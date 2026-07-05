//! Execute: run a Plan, produce a Run.
//!
//! Phase 3: orchestrate ops/{install, link, create, shell, clean} with retry
//! and real-time streaming output.

use crate::config::Config;
use crate::model::ActionStatus;
use crate::model::MAX_HISTORY_OUTPUT_LINES;
use crate::model::{Action, OutputLine, OutputStream, Plan, Run, RunItem, RunStatus};
use crate::ops::clean::{self};
use crate::ops::create::create_dir;
use crate::ops::install::{self};
use crate::ops::link::{self, LinkSettings};
use crate::ops::shell::{self, StreamLine};
use anyhow::Result;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

/// Default retry config (used when item doesn't override).
const DEFAULT_INSTALL_RETRIES: u32 = 2;
const RETRY_INITIAL_DELAY_SECS: u64 = 5;

/// Maximum output lines per step in TUI (before truncation).
pub const MAX_TUI_OUTPUT_LINES: usize = 1000;

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
    /// Real-time stdout/stderr output line.
    Output {
        item: String,
        stream: OutputStream,
        line: String,
    },
    /// Structured action feedback (link, create, clean).
    ActionMessage {
        item: String,
        message: String,
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
    let config_dir = config
        .path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    for (index, plan_item) in plan.items.iter().enumerate() {
        if should_abort() {
            aborted = true;
            emit(ExecuteEvent::Aborted);
            break;
        }

        if !plan_item.selected {
            items.push(RunItem {
                id: plan_item.id.clone(),
                name: plan_item.name.clone(),
                status: ActionStatus::WillSkip,
                started_at: None,
                finished_at: None,
                duration_ms: None,
                attempts: 0,
                error: Some("skipped (not selected)".into()),
                output: vec![],
            });
            continue;
        }

        emit(ExecuteEvent::ItemStarted {
            index,
            name: plan_item.name.clone(),
        });
        let item_started_at = now_iso();
        let started = Instant::now();
        let mut error: Option<String> = None;
        let mut attempts: u32 = 0;
        let mut last_status = ActionStatus::WillRun;
        let mut item_output: Vec<OutputLine> = Vec::new();

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
                    let (status, err, n, output) = run_install_streaming(
                        binary,
                        &config.package_managers,
                        DEFAULT_INSTALL_RETRIES,
                        &plan_item.name,
                        &mut emit,
                        &should_abort,
                    )?;
                    item_output.extend(output);
                    cap_output_len(&mut item_output);
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
                    let link_plan = link::plan_link(config_dir, target, source, settings)?;
                    let action_desc = describe_link_action(&link_plan.action);
                    match link::apply_link(link_plan) {
                        Ok(()) => {
                            let msg = format!(
                                "linked {} -> {} ({action_desc})",
                                target.display(),
                                source.display()
                            );
                            emit(ExecuteEvent::ActionMessage {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut item_output, OutputStream::Action, &msg);
                        }
                        Err(e) => {
                            let msg = format!("link failed: {e}");
                            emit(ExecuteEvent::ActionMessage {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut item_output, OutputStream::Action, &msg);
                            error = Some(e.to_string());
                            last_status = ActionStatus::WillFail;
                        }
                    }
                }
                Action::Create { target } => match create_dir(
                    &crate::path::expand_home(&target.to_string_lossy())
                        .unwrap_or_else(|_| target.clone()),
                ) {
                    Ok(()) => {
                        let msg = format!("created {}", target.display());
                        emit(ExecuteEvent::ActionMessage {
                            item: plan_item.name.clone(),
                            message: msg.clone(),
                        });
                        push_output_line(&mut item_output, OutputStream::Action, &msg);
                    }
                    Err(e) => {
                        let msg = format!("create failed: {e}");
                        emit(ExecuteEvent::ActionMessage {
                            item: plan_item.name.clone(),
                            message: msg.clone(),
                        });
                        push_output_line(&mut item_output, OutputStream::Action, &msg);
                        error = Some(e.to_string());
                        last_status = ActionStatus::WillFail;
                    }
                },
                Action::Shell {
                    command,
                    if_condition,
                    optional,
                    ..
                } => {
                    if let Some(cond) = if_condition
                        && !shell::condition_matches(cond).unwrap_or(false)
                    {
                        last_status = ActionStatus::WillSkip;
                        let msg = format!("condition skipped: {cond}");
                        emit(ExecuteEvent::ActionMessage {
                            item: plan_item.name.clone(),
                            message: msg.clone(),
                        });
                        push_output_line(&mut item_output, OutputStream::Action, &msg);
                        continue;
                    }
                    // Refresh sudo timestamp before commands that use sudo.
                    // The initial pre_cache_sudo() prompts interactively;
                    // refresh_sudo() is non-interactive and just extends
                    // the cached session so it doesn't expire mid-run.
                    if shell::command_contains_sudo(command)
                        && !shell::refresh_sudo().unwrap_or(false)
                    {
                        let msg = "sudo session expired — re-run to re-authenticate".to_string();
                        emit(ExecuteEvent::ActionMessage {
                            item: plan_item.name.clone(),
                            message: msg.clone(),
                        });
                        push_output_line(&mut item_output, OutputStream::Action, &msg);
                        error = Some("sudo session expired".into());
                        last_status = ActionStatus::WillFail;
                        break;
                    }
                    let (exit_code, output) = run_command_streaming_with_events(
                        command,
                        config_dir,
                        &plan_item.name,
                        &mut emit,
                        &should_abort,
                    )?;
                    item_output.extend(output);
                    cap_output_len(&mut item_output);
                    match exit_code {
                        Some(0) => {}
                        Some(code) if *optional => {
                            let msg = format!("optional command failed (exit {code})");
                            emit(ExecuteEvent::ActionMessage {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut item_output, OutputStream::Action, &msg);
                            last_status = ActionStatus::NoChange;
                        }
                        Some(code) => {
                            error = Some(format!("exit code {code}"));
                            last_status = ActionStatus::WillFail;
                        }
                        None => {
                            error = Some("killed (aborted)".into());
                            last_status = ActionStatus::WillFail;
                            aborted = true;
                        }
                    }
                }
                Action::Clean { target, force } => {
                    let expanded_target = crate::path::expand_home(&target.to_string_lossy())
                        .unwrap_or_else(|_| target.clone());
                    let clean_action = clean::plan_clean(&expanded_target, *force)?;
                    let action_desc = describe_clean_action(&clean_action);
                    match clean::apply_clean(clean_action, &expanded_target) {
                        Ok(()) => {
                            let msg =
                                format!("cleaned {} ({action_desc})", expanded_target.display());
                            emit(ExecuteEvent::ActionMessage {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut item_output, OutputStream::Action, &msg);
                        }
                        Err(e) => {
                            let msg = format!("clean failed: {e}");
                            emit(ExecuteEvent::ActionMessage {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut item_output, OutputStream::Action, &msg);
                            error = Some(e.to_string());
                            last_status = ActionStatus::WillFail;
                        }
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
            started_at: Some(item_started_at),
            finished_at: Some(now_iso()),
            duration_ms: Some(started.elapsed().as_millis() as u64),
            attempts,
            error,
            output: item_output,
        });
        emit(ExecuteEvent::ItemFinished {
            index,
            name: plan_item.name.clone(),
            status: last_status,
        });

        if aborted {
            break;
        }
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
        mode: plan.mode,
        started_at,
        finished_at: Some(now_iso()),
        status,
        config_hash: plan.config_hash.clone(),
        items,
    })
}

/// Run a shell command with real-time streaming output via events.
///
/// Spawns the command in a background thread, drains output through the emit
/// callback, and checks the abort signal periodically.
fn run_command_streaming_with_events<F, C>(
    command: &str,
    config_dir: &Path,
    item_name: &str,
    emit: &mut F,
    should_abort: &C,
) -> Result<(Option<i32>, Vec<OutputLine>)>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
{
    // Check abort before spawning.
    if should_abort() {
        return Ok((None, vec![]));
    }

    let cmd_owned = command.to_string();
    let dir_owned = config_dir.to_path_buf();
    let (tx, rx) = mpsc::channel::<StreamLine>();
    let abort = Arc::new(AtomicBool::new(false));
    let cmd_abort = Arc::clone(&abort);

    let handle =
        thread::spawn(move || shell::run_command_streaming(&cmd_owned, &dir_owned, &tx, cmd_abort));

    let mut output_lines: Vec<OutputLine> = Vec::new();

    // Drain output while command runs.
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                emit(ExecuteEvent::Output {
                    item: item_name.to_string(),
                    stream: line.stream,
                    line: line.line.clone(),
                });
                push_output_line(&mut output_lines, line.stream, &line.line);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if should_abort() {
                    abort.store(true, Ordering::SeqCst);
                }
                if handle.is_finished() {
                    // Command process has exited but reader threads may still have
                    // buffered lines in the channel. Drain them before returning to
                    // avoid losing tail output.
                    while let Ok(line) = rx.try_recv() {
                        emit(ExecuteEvent::Output {
                            item: item_name.to_string(),
                            stream: line.stream,
                            line: line.line.clone(),
                        });
                        push_output_line(&mut output_lines, line.stream, &line.line);
                    }
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let exit_code = handle
        .join()
        .map_err(|_| anyhow::anyhow!("command thread panicked"))??;

    Ok((exit_code, output_lines))
}

/// Install with retry and streaming output.
fn run_install_streaming<F, C>(
    binary: &str,
    pkg_mgrs: &crate::config::PackageManagerConfig,
    max_retries: u32,
    item_name: &str,
    emit: &mut F,
    should_abort: &C,
) -> Result<(ActionStatus, Option<String>, u32, Vec<OutputLine>)>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
{
    let db = install::load_db()?;
    let entry = install::find(&db, binary);

    let entry = match entry {
        Some(e) => e,
        None => {
            let msg = format!("tool '{binary}' not in tool db");
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }
    };

    // Font install: build a shell command from the tool entry and run it
    // through the streaming pipeline. Fonts are source-url installs, not
    // package-manager installs, so they do not require a platform command.
    if entry.kind == "font" {
        if entry.source_url.is_empty() {
            let msg = format!("font {} missing source_url", entry.name);
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }

        let home = std::env::var("HOME").unwrap_or_default();
        if home.is_empty() {
            let msg = "HOME not set".to_string();
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }
        let fonts_dir = format!("{home}/.local/share/fonts");

        // Check already installed.
        if std::path::Path::new(&fonts_dir)
            .join(&entry.binary)
            .exists()
        {
            let msg = format!("font {} already installed", entry.name);
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            let mut output = Vec::new();
            push_output_line(&mut output, OutputStream::Action, &msg);
            return Ok((ActionStatus::NoChange, None, 1, output));
        }

        // Download + unzip as a single streaming shell command so curl
        // progress appears in real time.
        let zip = format!("{fonts_dir}/{}.zip", entry.name);
        let font_cmd = format!(
            "mkdir -p {} && curl -fsSL {} -o {} && unzip -o -q {} -d {} && rm -f {}",
            shell_quote(&fonts_dir),
            shell_quote(&entry.source_url),
            shell_quote(&zip),
            shell_quote(&zip),
            shell_quote(&fonts_dir),
            shell_quote(&zip),
        );

        let config_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let (exit_code, mut output) = run_command_streaming_with_events(
            &font_cmd,
            &config_dir,
            item_name,
            emit,
            should_abort,
        )?;
        match exit_code {
            Some(0) => {
                let msg = format!("font {} installed to {fonts_dir}", entry.name);
                emit(ExecuteEvent::ActionMessage {
                    item: item_name.to_string(),
                    message: msg.clone(),
                });
                push_output_line(&mut output, OutputStream::Action, &msg);
                return Ok((ActionStatus::NoChange, None, 1, output));
            }
            Some(code) => {
                let msg = format!("font install failed (exit {code})");
                emit(ExecuteEvent::ActionMessage {
                    item: item_name.to_string(),
                    message: msg.clone(),
                });
                push_output_line(&mut output, OutputStream::Action, &msg);
                return Ok((ActionStatus::WillFail, Some(msg), 1, output));
            }
            None => {
                return Ok((
                    ActionStatus::WillFail,
                    Some("killed (aborted)".into()),
                    1,
                    output,
                ));
            }
        }
    }

    let pkg_mgr = crate::package_managers::resolve_pkg_mgr_name(pkg_mgrs)
        .unwrap_or_else(crate::package_managers::default_pkg_mgr_name);

    let cmd = match entry.platforms.get(&pkg_mgr) {
        Some(c) if c.supports_os(crate::package_managers::os_name()) => c.command().to_string(),
        Some(_) => {
            let os = crate::package_managers::os_name();
            let msg = format!("{} is not supported for {pkg_mgr} on {os}", entry.name);
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }
        None => {
            let msg = format!("no install command for {pkg_mgr}");
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }
    };

    let config_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let mut last_err: Option<String> = None;
    let mut all_output: Vec<OutputLine> = Vec::new();
    let mut attempt = 0u32;
    let max = max_retries + 1;

    while attempt < max {
        if should_abort() {
            return Ok((
                ActionStatus::WillFail,
                Some("aborted".into()),
                attempt,
                all_output,
            ));
        }

        attempt += 1;
        let status_line = format!("install {binary}: attempt {attempt}/{max}");
        emit(ExecuteEvent::ActionMessage {
            item: item_name.to_string(),
            message: status_line.clone(),
        });
        push_output_line(&mut all_output, OutputStream::Action, &status_line);

        // Refresh sudo timestamp before install commands that use sudo.
        if shell::command_contains_sudo(&cmd) && !shell::refresh_sudo().unwrap_or(false) {
            let msg = "sudo session expired — re-run to re-authenticate".to_string();
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            push_output_line(&mut all_output, OutputStream::Action, &msg);
            return Ok((
                ActionStatus::WillFail,
                Some("sudo session expired".into()),
                attempt,
                all_output,
            ));
        }

        let (exit_code, output) =
            run_command_streaming_with_events(&cmd, &config_dir, item_name, emit, should_abort)?;
        all_output.extend(output);
        cap_output_len(&mut all_output);

        match exit_code {
            Some(0) => {
                return Ok((ActionStatus::NoChange, None, attempt, all_output));
            }
            Some(code) => {
                last_err = Some(format!("install failed (exit {code})"));
            }
            None => {
                last_err = Some("killed (aborted)".into());
                break;
            }
        }

        if attempt < max {
            let delay = RETRY_INITIAL_DELAY_SECS * 2u64.pow(attempt - 1);
            let retry_msg = format!("retrying {binary} in {delay}s");
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: retry_msg.clone(),
            });
            push_output_line(&mut all_output, OutputStream::Action, &retry_msg);

            // Sleep in 1-second increments, checking abort each time.
            for _ in 0..delay {
                if should_abort() {
                    return Ok((
                        ActionStatus::WillFail,
                        Some("aborted".into()),
                        attempt,
                        all_output,
                    ));
                }
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    Ok((ActionStatus::WillFail, last_err, attempt, all_output))
}

/// Fallback OS key used when no package manager is configured for the current platform.
fn describe_link_action(action: &link::LinkAction) -> String {
    match action {
        link::LinkAction::Skip => "skip: already linked".into(),
        link::LinkAction::Link => "link: create symlink".into(),
        link::LinkAction::Backup(path) => format!("backup then link: {}", path.display()),
        link::LinkAction::Relink => "relink: replace wrong symlink".into(),
        link::LinkAction::Fail(reason) => format!("fail: {reason}"),
    }
}

fn describe_clean_action(action: &clean::CleanAction) -> String {
    match action {
        clean::CleanAction::Skip => "skip".into(),
        clean::CleanAction::RemoveSymlink => "remove symlink".into(),
        clean::CleanAction::BackupAndRemove(path) => {
            format!("backup to {} then remove", path.display())
        }
    }
}

fn push_output_line(output: &mut Vec<OutputLine>, stream: OutputStream, line: &str) {
    output.push(OutputLine {
        stream,
        line: line.to_string(),
    });
    cap_output_len(output);
}

fn cap_output_len(output: &mut Vec<OutputLine>) {
    if output.len() > MAX_HISTORY_OUTPUT_LINES {
        let drop = output.len() - MAX_HISTORY_OUTPUT_LINES;
        output.drain(0..drop);
    }
}

fn now_iso() -> String {
    time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::new())
}

/// Return a single-quoted shell string safe for embedding in `sh -c` commands.
fn shell_quote(s: &str) -> String {
    let escaped = s.replace('\'', "'\\''");
    format!("'{escaped}'")
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

    #[test]
    fn streaming_shell_command_emits_output_events() {
        let cfg = Config {
            path: PathBuf::from("/tmp/dotman.yaml"),
            package_managers: crate::config::PackageManagerConfig::default(),
            install: vec![],
            links: vec![],
            create: vec![],
            shell: vec![crate::config::ShellEntry {
                command: "echo hello".into(),
                description: Some("test shell".into()),
                optional: false,
                if_condition: None,
            }],
            clean: vec![],
            auto_install_pkg_manager: false,
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let mut outputs: Vec<String> = Vec::new();
        let run = execute_with_events(
            &plan,
            &cfg,
            |event| {
                if let ExecuteEvent::Output { line, .. } = &event {
                    outputs.push(line.clone());
                }
            },
            || false,
        )
        .unwrap();

        assert_eq!(run.status, RunStatus::Success);
        assert!(outputs.iter().any(|l| l.contains("hello")));
    }

    #[test]
    fn shell_command_runs_when_config_path_has_no_parent() {
        let dir = tempfile::tempdir().unwrap();
        let original_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let cfg = Config {
            path: PathBuf::from("dotman.yaml"),
            package_managers: crate::config::PackageManagerConfig::default(),
            install: vec![],
            links: vec![],
            create: vec![],
            shell: vec![crate::config::ShellEntry {
                command: "echo relative-config-path".into(),
                description: Some("relative config path shell".into()),
                optional: false,
                if_condition: None,
            }],
            clean: vec![],
            auto_install_pkg_manager: false,
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let run = execute(&plan, &cfg).unwrap();

        std::env::set_current_dir(original_cwd).unwrap();
        assert_eq!(run.status, RunStatus::Success);
        assert!(run.items.iter().any(|item| {
            item.output
                .iter()
                .any(|line| line.line.contains("relative-config-path"))
        }));
    }

    #[test]
    fn link_emits_action_message() {
        let dir = tempfile::tempdir().unwrap();
        // Create source file.
        let src = dir.path().join("test_src");
        std::fs::write(&src, "content").unwrap();
        let target = dir.path().join("test_link");

        let cfg = Config {
            path: dir.path().join("dotman.yaml"),
            package_managers: crate::config::PackageManagerConfig::default(),
            install: vec![],
            links: vec![crate::config::LinkEntry {
                target: target.clone(),
                source: src.clone(),
                backup: None,
                relink: None,
            }],
            create: vec![],
            shell: vec![],
            clean: vec![],
            auto_install_pkg_manager: false,
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let mut messages: Vec<String> = Vec::new();
        let run = execute_with_events(
            &plan,
            &cfg,
            |event| {
                if let ExecuteEvent::ActionMessage { message, .. } = &event {
                    messages.push(message.clone());
                }
            },
            || false,
        )
        .unwrap();

        assert_eq!(run.status, RunStatus::Success);
        assert!(messages.iter().any(|m| m.contains("linked")));
    }

    #[test]
    fn abort_preserves_current_item_in_run() {
        // When a shell action is aborted, the current RunItem should still
        // appear in run.items with its output and error.
        let cfg = Config {
            path: std::path::PathBuf::from("/tmp/dotman.yaml"),
            package_managers: crate::config::PackageManagerConfig::default(),
            install: vec![],
            links: vec![],
            create: vec![],
            shell: vec![crate::config::ShellEntry {
                command: "echo started && sleep 10".into(),
                description: Some("long shell".into()),
                optional: false,
                if_condition: None,
            }],
            clean: vec![],
            auto_install_pkg_manager: false,
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let abort_flag = Arc::new(AtomicBool::new(false));
        let emit_flag = Arc::clone(&abort_flag);
        let check_flag = Arc::clone(&abort_flag);
        let run = execute_with_events(
            &plan,
            &cfg,
            move |event| {
                if matches!(event, ExecuteEvent::Output { line, .. } if line.contains("started"))
                    && !emit_flag.load(Ordering::SeqCst)
                {
                    emit_flag.store(true, Ordering::SeqCst);
                }
            },
            move || check_flag.load(Ordering::SeqCst),
        )
        .unwrap();

        assert_eq!(run.status, RunStatus::Aborted);
        // The shell item must be present in the run.
        let shell_item = run.items.iter().find(|i| i.name != "fish").unwrap();
        assert!(shell_item.error.is_some());
        assert!(
            shell_item.output.iter().any(|l| l.line.contains("started")),
            "output should contain 'started' line"
        );
    }

    #[test]
    fn history_output_respects_500_line_cap() {
        use crate::model::{OutputLine, OutputStream};
        // Build a vec with 600 lines, push them through push_output_line,
        // and verify only the last 500 are retained.
        let mut output: Vec<OutputLine> = Vec::new();
        for i in 0..600 {
            push_output_line(&mut output, OutputStream::Stdout, &format!("line {i}"));
        }
        assert_eq!(output.len(), 500);
        assert_eq!(output.first().unwrap().line, "line 100");
        assert_eq!(output.last().unwrap().line, "line 599");
    }

    #[test]
    fn cap_output_after_extend() {
        use crate::model::{OutputLine, OutputStream};
        // Simulate the install retry pattern: push 1 action line,
        // extend 500 output lines, push another action line — must stay at 500.
        let mut all_output: Vec<OutputLine> = Vec::new();
        push_output_line(&mut all_output, OutputStream::Action, "attempt 1/3");
        for i in 0..500 {
            push_output_line(&mut all_output, OutputStream::Stdout, &format!("line {i}"));
        }
        assert_eq!(all_output.len(), 500);
        // Now "extend" another batch (simulating retry output)
        let mut retry_output = Vec::new();
        for i in 0..100 {
            retry_output.push(OutputLine {
                stream: OutputStream::Stdout,
                line: format!("retry {i}"),
            });
        }
        all_output.extend(retry_output);
        cap_output_len(&mut all_output);
        push_output_line(&mut all_output, OutputStream::Action, "final");
        assert_eq!(all_output.len(), 500);
        assert_eq!(all_output.last().unwrap().line, "final");
    }

    #[test]
    fn shell_quote_handles_single_quotes() {
        assert_eq!(shell_quote("hello"), "'hello'");
        assert_eq!(shell_quote("it's working"), "'it'\\''s working'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn font_command_starts_with_mkdir_not_quoted_command() {
        // shell_quote must be called on individual arguments, not entire
        // commands — otherwise the shell tries to execute a binary named
        // "mkdir -p /path" instead of running mkdir with arguments.
        let source_url = "https://example.com/font.zip";
        let home = "/home/user";
        let name = "test-font";
        let fonts_dir = format!("{home}/.local/share/fonts");
        let zip = format!("{fonts_dir}/{name}.zip");

        let cmd = format!(
            "mkdir -p {} && curl -fsSL {} -o {} && unzip -o -q {} -d {} && rm -f {}",
            shell_quote(&fonts_dir),
            shell_quote(source_url),
            shell_quote(&zip),
            shell_quote(&zip),
            shell_quote(&fonts_dir),
            shell_quote(&zip),
        );

        // Must start with mkdir, not a quoted string containing mkdir.
        assert!(
            cmd.starts_with("mkdir"),
            "font command should start with 'mkdir', got: {cmd}"
        );
        // Must contain the quoted paths.
        assert!(cmd.contains("'/home/user/.local/share/fonts'"));
        // Must not quote the entire mkdir command.
        assert!(!cmd.contains("'mkdir"));
    }
}
