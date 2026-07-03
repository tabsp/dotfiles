//! Execute: run a Plan, produce a Run.
//!
//! Phase 3: orchestrate ops/{install, link, create, shell, clean} with retry.

use crate::config::Config;
use crate::model::ActionStatus;
use crate::model::{Action, Plan, Run, RunItem, RunStatus};
use crate::ops::clean::{self};
use crate::ops::create::create_dir;
use crate::ops::install::{self};
use crate::ops::link::{self, LinkSettings};
use crate::ops::shell::{self};
use anyhow::Result;
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Default retry config (used when item doesn't override).
const DEFAULT_INSTALL_RETRIES: u32 = 2;
const RETRY_INITIAL_DELAY_SECS: u64 = 5;

pub fn execute(plan: &Plan, config: &Config) -> Result<Run> {
    let started_at = now_iso();
    let mut items: Vec<RunItem> = Vec::new();
    let mut any_failed = false;

    for plan_item in &plan.items {
        if !plan_item.selected {
            // Skip items the user deselected.
            items.push(RunItem {
                id: plan_item.id.clone(),
                name: plan_item.name.clone(),
                status: ActionStatus::WillSkip,
                started_at: None,
                finished_at: None,
                duration_ms: None,
                attempts: 0,
                error: Some("skipped (not selected)".into()),
            });
            continue;
        }

        let started = Instant::now();
        let mut error: Option<String> = None;
        let mut attempts: u32 = 0;
        let mut last_status = ActionStatus::WillRun;

        for action in &plan_item.actions {
            match action {
                Action::Install { binary, .. } => {
                    let (status, err, n) = run_install_with_retry(
                        binary,
                        &config.package_managers,
                        DEFAULT_INSTALL_RETRIES,
                    )?;
                    attempts = n;
                    if let Some(e) = err {
                        error = Some(e);
                    }
                    last_status = status;
                }
                Action::Link { target, source } => {
                    let settings = LinkSettings {
                        create: true,
                        relative: true,
                        backup: true,
                        relink: false,
                    };
                    let plan = link::plan_link(
                        config.path.parent().unwrap_or(Path::new(".")),
                        target,
                        source,
                        settings,
                    )?;
                    if let Err(e) = link::apply_link(plan) {
                        error = Some(e.to_string());
                        last_status = ActionStatus::WillFail;
                    }
                }
                Action::Create { target } => {
                    if let Err(e) = create_dir(target) {
                        error = Some(e.to_string());
                        last_status = ActionStatus::WillFail;
                    }
                }
                Action::Shell {
                    command,
                    if_condition,
                    ..
                } => {
                    if let Some(cond) = if_condition
                        && !shell::condition_matches(cond).unwrap_or(false)
                    {
                        last_status = ActionStatus::WillSkip;
                        continue;
                    }
                    let out =
                        shell::run_shell(command, config.path.parent().unwrap_or(Path::new(".")))?;
                    if out.exit_code != 0 {
                        error = Some(format!("exit code {}", out.exit_code));
                        last_status = ActionStatus::WillFail;
                    }
                }
                Action::Clean { target, force } => {
                    let action = clean::plan_clean(target, *force)?;
                    if let Err(e) = clean::apply_clean(action, target) {
                        error = Some(e.to_string());
                        last_status = ActionStatus::WillFail;
                    }
                }
            }
        }

        if error.is_some() {
            any_failed = true;
        }

        items.push(RunItem {
            id: plan_item.id.clone(),
            name: plan_item.name.clone(),
            status: last_status,
            started_at: Some(started_at.clone()),
            finished_at: Some(now_iso()),
            duration_ms: Some(started.elapsed().as_millis() as u64),
            attempts,
            error,
        });
    }

    let status = if any_failed {
        RunStatus::Failed
    } else {
        RunStatus::Success
    };

    Ok(Run {
        id: plan.id.clone(),
        plan_id: plan.id.clone(),
        mode: plan.mode,
        started_at,
        finished_at: Some(now_iso()),
        status,
        config_hash: plan.config_hash.clone(),
        items,
    })
}

/// Install with retry: try the install command, on failure wait + retry up to max_retries.
fn run_install_with_retry(
    binary: &str,
    pkg_mgrs: &crate::config::PackageManagerConfig,
    max_retries: u32,
) -> Result<(ActionStatus, Option<String>, u32)> {
    let db = install::load_db()?;
    let entry = install::find(&db, binary);

    let os = crate::package_managers::detect_os();
    let pkg_mgr = pkg_mgr_for(pkg_mgrs, os);

    let entry = match entry {
        Some(e) => e,
        None => {
            return Ok((
                ActionStatus::WillFail,
                Some(format!("tool '{binary}' not in tool db")),
                0,
            ));
        }
    };

    let mut last_err: Option<String> = None;
    let mut attempt = 0u32;
    let max = max_retries + 1;

    while attempt < max {
        attempt += 1;
        match install::install(entry, &pkg_mgr) {
            Ok(out) if out.exit_code == 0 => {
                return Ok((ActionStatus::NoChange, None, attempt));
            }
            Ok(out) => {
                last_err = Some(format!(
                    "install failed (exit {}): {}",
                    out.exit_code,
                    out.stderr.trim()
                ));
            }
            Err(e) => {
                last_err = Some(e.to_string());
            }
        }

        if attempt < max {
            let delay = RETRY_INITIAL_DELAY_SECS * 2u64.pow(attempt - 1);
            std::thread::sleep(std::time::Duration::from_secs(delay));
        }
    }

    Ok((ActionStatus::WillFail, last_err, attempt))
}

fn pkg_mgr_for(
    cfg: &crate::config::PackageManagerConfig,
    os: crate::package_managers::Os,
) -> String {
    use crate::package_managers::Os;
    let key = match os {
        Os::Mac => "macos",
        Os::Linux => "linux",
        Os::Unknown => "unknown",
    };
    // Try the OS key first, then fall back to a default.
    // We do per-distro lookup inline here for simplicity.
    if let Some(p) = detect_distro_pkg_mgr(cfg) {
        return p;
    }
    key.to_string()
}

fn detect_distro_pkg_mgr(cfg: &crate::config::PackageManagerConfig) -> Option<String> {
    // /etc/os-release has ID=ubuntu / ID=debian / ID=arch / ID=fedora.
    let contents = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("ID=") {
            let id = rest.trim().trim_matches('"');
            let key = match id {
                "ubuntu" => "ubuntu",
                "debian" => "debian",
                "arch" => "arch",
                "fedora" => "fedora",
                _ => return None,
            };
            return match key {
                "macos" => cfg.macos.clone(),
                "ubuntu" => cfg.ubuntu.clone(),
                "debian" => cfg.debian.clone(),
                "arch" => cfg.arch.clone(),
                "fedora" => cfg.fedora.clone(),
                _ => None,
            };
        }
    }
    None
}

fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("epoch:{now}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Mode;
    use crate::plan::build;
    use std::path::PathBuf;

    #[test]
    fn execute_empty_plan_runs_no_actions() {
        let cfg = Config {
            path: PathBuf::from("/tmp/dotman.yaml"),
            package_managers: crate::config::PackageManagerConfig::default(),
            install: vec![],
            links: vec![],
            create: vec![],
            shell: vec![],
            clean: vec![],
            auto_install_pkg_manager: false,
            auto_clone_repo: None,
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let run = execute(&plan, &cfg).unwrap();
        assert_eq!(run.status, RunStatus::Success);
        assert_eq!(run.items.len(), 0);
    }
}
