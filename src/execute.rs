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
use std::time::Instant;

/// Default retry config (used when item doesn't override).
const DEFAULT_INSTALL_RETRIES: u32 = 2;
const RETRY_INITIAL_DELAY_SECS: u64 = 5;

#[derive(Debug, Clone)]
pub enum ExecuteEvent {
    ItemStarted {
        index: usize,
        name: String,
    },
    ActionStarted {
        item: String,
        action: String,
    },
    ActionOutput {
        item: String,
        output: String,
    },
    ItemFinished {
        index: usize,
        name: String,
        status: ActionStatus,
    },
    Aborted,
}

pub fn execute(plan: &Plan, config: &Config) -> Result<Run> {
    execute_with_events(plan, config, |_| {}, || false)
}

pub fn execute_with_events<F, C>(
    plan: &Plan,
    config: &Config,
    mut emit: F,
    should_abort: C,
) -> Result<Run>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
{
    let started_at = now_iso();
    let mut items: Vec<RunItem> = Vec::new();
    let mut any_failed = false;
    let mut aborted = false;

    for (index, plan_item) in plan.items.iter().enumerate() {
        if should_abort() {
            aborted = true;
            emit(ExecuteEvent::Aborted);
            break;
        }

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

        emit(ExecuteEvent::ItemStarted {
            index,
            name: plan_item.name.clone(),
        });
        let started = Instant::now();
        let mut error: Option<String> = None;
        let mut attempts: u32 = 0;
        let mut last_status = ActionStatus::WillRun;

        for action in &plan_item.actions {
            if should_abort() {
                aborted = true;
                emit(ExecuteEvent::Aborted);
                break;
            }
            emit(ExecuteEvent::ActionStarted {
                item: plan_item.name.clone(),
                action: action.describe(),
            });
            match action {
                Action::Install { binary, .. } => {
                    let (status, err, n, logs) = run_install_with_retry(
                        binary,
                        &config.package_managers,
                        DEFAULT_INSTALL_RETRIES,
                    )?;
                    emit_output(&mut emit, &plan_item.name, logs);
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
                    } else {
                        emit(ExecuteEvent::ActionOutput {
                            item: plan_item.name.clone(),
                            output: format!("linked {}", target.display()),
                        });
                    }
                }
                Action::Create { target } => {
                    if let Err(e) = create_dir(target) {
                        error = Some(e.to_string());
                        last_status = ActionStatus::WillFail;
                    } else {
                        emit(ExecuteEvent::ActionOutput {
                            item: plan_item.name.clone(),
                            output: format!("created {}", target.display()),
                        });
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
                        emit(ExecuteEvent::ActionOutput {
                            item: plan_item.name.clone(),
                            output: format!("condition skipped: {cond}"),
                        });
                        continue;
                    }
                    let out =
                        shell::run_shell(command, config.path.parent().unwrap_or(Path::new(".")))?;
                    emit_command_output(&mut emit, &plan_item.name, &out.stdout, &out.stderr);
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
                    } else {
                        emit(ExecuteEvent::ActionOutput {
                            item: plan_item.name.clone(),
                            output: format!("cleaned {}", target.display()),
                        });
                    }
                }
            }
        }

        if aborted {
            break;
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
        emit(ExecuteEvent::ItemFinished {
            index,
            name: plan_item.name.clone(),
            status: last_status,
        });
    }

    let status = if aborted {
        RunStatus::Aborted
    } else if any_failed {
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

fn emit_output<F>(emit: &mut F, item: &str, logs: Vec<String>)
where
    F: FnMut(ExecuteEvent),
{
    for output in logs {
        emit(ExecuteEvent::ActionOutput {
            item: item.to_string(),
            output,
        });
    }
}

fn emit_command_output<F>(emit: &mut F, item: &str, stdout: &str, stderr: &str)
where
    F: FnMut(ExecuteEvent),
{
    for line in stdout.lines().chain(stderr.lines()) {
        if !line.trim().is_empty() {
            emit(ExecuteEvent::ActionOutput {
                item: item.to_string(),
                output: line.to_string(),
            });
        }
    }
}

/// Install with retry: try the install command, on failure wait + retry up to max_retries.
fn run_install_with_retry(
    binary: &str,
    pkg_mgrs: &crate::config::PackageManagerConfig,
    max_retries: u32,
) -> Result<(ActionStatus, Option<String>, u32, Vec<String>)> {
    let db = install::load_db()?;
    let entry = install::find(&db, binary);

    let os = crate::package_managers::detect_os();
    let pkg_mgr = crate::package_managers::resolve_pkg_mgr_name(pkg_mgrs)
        .unwrap_or_else(|| fallback_pkg_mgr_key(os));

    let entry = match entry {
        Some(e) => e,
        None => {
            return Ok((
                ActionStatus::WillFail,
                Some(format!("tool '{binary}' not in tool db")),
                0,
                Vec::new(),
            ));
        }
    };

    let mut last_err: Option<String> = None;
    let mut logs = Vec::new();
    let mut attempt = 0u32;
    let max = max_retries + 1;

    while attempt < max {
        attempt += 1;
        logs.push(format!("install {binary}: attempt {attempt}/{max}"));
        match install::install(entry, &pkg_mgr) {
            Ok(out) if out.exit_code == 0 => {
                emit_install_output(&mut logs, &out.stdout, &out.stderr);
                return Ok((ActionStatus::NoChange, None, attempt, logs));
            }
            Ok(out) => {
                emit_install_output(&mut logs, &out.stdout, &out.stderr);
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
            logs.push(format!("retrying {binary} in {delay}s"));
            std::thread::sleep(std::time::Duration::from_secs(delay));
        }
    }

    Ok((ActionStatus::WillFail, last_err, attempt, logs))
}

/// Fallback OS key used when no package manager is configured for the current platform.
fn fallback_pkg_mgr_key(os: crate::package_managers::Os) -> String {
    use crate::package_managers::Os;
    match os {
        Os::Mac => "macos".into(),
        Os::Linux => "linux".into(),
        Os::Unknown => "unknown".into(),
    }
}

fn emit_install_output(logs: &mut Vec<String>, stdout: &str, stderr: &str) {
    for line in stdout.lines().chain(stderr.lines()) {
        if !line.trim().is_empty() {
            logs.push(line.to_string());
        }
    }
}

fn now_iso() -> String {
    time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::new())
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

    #[test]
    fn execute_with_events_can_abort_before_running() {
        let cfg = Config {
            path: PathBuf::from("/tmp/dotman.yaml"),
            package_managers: crate::config::PackageManagerConfig::default(),
            install: vec!["fish".into()],
            links: vec![],
            create: vec![],
            shell: vec![],
            clean: vec![],
            auto_install_pkg_manager: false,
            auto_clone_repo: None,
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let mut saw_abort = false;
        let run = execute_with_events(
            &plan,
            &cfg,
            |event| {
                if matches!(event, ExecuteEvent::Aborted) {
                    saw_abort = true;
                }
            },
            || true,
        )
        .unwrap();

        assert!(saw_abort);
        assert_eq!(run.status, RunStatus::Aborted);
        assert!(run.items.is_empty());
    }
}
