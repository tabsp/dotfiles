//! Execute: run a Plan, produce a Run.
//!
//! Phase 3: orchestrate ops/{install, link, create, shell, clean} with retry
//! and real-time streaming output.

use crate::config::Config;
use crate::model::ActionStatus;
use crate::model::MAX_HISTORY_OUTPUT_LINES;
use crate::model::{Action, OutputLine, OutputStream, Plan, Run, RunAction, RunItem, RunStatus};
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
        item_index: usize,
        action_index: usize,
        item: String,
        action: String,
    },
    ActionFinished {
        item_index: usize,
        action_index: usize,
        item: String,
        action: String,
        status: ActionStatus,
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
    SudoPrompt {
        item: String,
        response: mpsc::Sender<bool>,
    },
    Aborted,
}

pub fn execute(plan: &Plan, config: &Config) -> Result<Run> {
    execute_with_events(plan, config, |_| {}, || false)
}

pub fn execute_with_events<F, C>(
    plan: &Plan,
    config: &Config,
    emit: F,
    should_abort: C,
) -> Result<Run>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
{
    execute_with_events_and_sudo(plan, config, emit, should_abort, |_| {
        shell::pre_cache_sudo().unwrap_or(false)
    })
}

pub fn execute_with_events_and_sudo<F, C, S>(
    plan: &Plan,
    config: &Config,
    mut emit: F,
    should_abort: C,
    mut sudo_auth: S,
) -> Result<Run>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
    S: FnMut(&str) -> bool,
{
    let started_at = now_iso();
    let mut items: Vec<RunItem> = Vec::new();
    let mut any_failed = false;
    let mut aborted = false;
    let mut not_run_start_index: Option<usize> = None;
    let config_dir = config
        .path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));

    for (index, plan_item) in plan.items.iter().enumerate() {
        if should_abort() {
            aborted = true;
            emit(ExecuteEvent::Aborted);
            not_run_start_index = Some(index);
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
                actions: plan_item
                    .actions
                    .iter()
                    .map(|action| RunAction {
                        kind: action_kind(action).into(),
                        name: action.describe(),
                        status: ActionStatus::WillSkip,
                        error: Some("skipped (not selected)".into()),
                        output: vec![],
                    })
                    .collect(),
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
        let mut run_actions: Vec<RunAction> = Vec::new();

        for (action_index, action) in plan_item.actions.iter().enumerate() {
            if should_abort() {
                aborted = true;
                emit(ExecuteEvent::Aborted);
                append_not_run_actions(
                    &mut run_actions,
                    &plan_item.actions,
                    action_index,
                    "not run (aborted)",
                );
                break;
            }
            let action_name = action.describe();
            let action_output_start = item_output.len();
            emit(ExecuteEvent::ActionStarted {
                item_index: index,
                action_index,
                item: plan_item.name.clone(),
                action: action_name.clone(),
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
                        &mut sudo_auth,
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
                    last_status = match &link_plan.action {
                        link::LinkAction::Skip => ActionStatus::NoChange,
                        link::LinkAction::Link | link::LinkAction::Relink => ActionStatus::WillLink,
                        link::LinkAction::Backup(_) => ActionStatus::WillBackupLink,
                        link::LinkAction::Fail(_) => ActionStatus::WillFail,
                    };
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
                Action::Create { target } => {
                    let expanded = crate::path::expand_home(&target.to_string_lossy())
                        .unwrap_or_else(|_| target.clone());
                    let existed = expanded.exists();
                    match create_dir(&expanded) {
                        Ok(()) => {
                            last_status = if existed {
                                ActionStatus::NoChange
                            } else {
                                ActionStatus::WillCreate
                            };
                            let action_desc = if existed { "exists" } else { "created" };
                            let msg = format!("{action_desc} {}", target.display());
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
                    }
                }
                Action::Shell {
                    command,
                    if_condition,
                    optional,
                    ..
                } => {
                    let condition_skipped = if let Some(cond) = if_condition
                        && !shell::condition_matches(cond).unwrap_or(false)
                    {
                        last_status = ActionStatus::WillSkip;
                        let msg = format!("condition skipped: {cond}");
                        emit(ExecuteEvent::ActionMessage {
                            item: plan_item.name.clone(),
                            message: msg.clone(),
                        });
                        push_output_line(&mut item_output, OutputStream::Action, &msg);
                        true
                    } else {
                        false
                    };
                    if !condition_skipped {
                        // Keep sudo fresh before commands that need it. This must
                        // stay non-interactive so abort can still work in the TUI.
                        if shell::command_contains_sudo(command)
                            && !ensure_sudo_session(&plan_item.name, &mut sudo_auth)
                        {
                            let msg =
                                "sudo session expired — re-run to re-authenticate".to_string();
                            emit(ExecuteEvent::ActionMessage {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut item_output, OutputStream::Action, &msg);
                            error = Some("sudo session expired".into());
                            last_status = ActionStatus::WillFail;
                        } else {
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
                                Some(0) => {
                                    last_status = ActionStatus::WillRun;
                                }
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
                    }
                }
                Action::Clean { target, force } => {
                    let expanded_target = crate::path::expand_home(&target.to_string_lossy())
                        .unwrap_or_else(|_| target.clone());
                    let clean_action = clean::plan_clean(&expanded_target, *force)?;
                    let action_desc = describe_clean_action(&clean_action);
                    last_status = match &clean_action {
                        clean::CleanAction::Skip => ActionStatus::NoChange,
                        clean::CleanAction::RemoveSymlink => ActionStatus::WillClean,
                        clean::CleanAction::BackupAndRemove(_) => ActionStatus::WillBackupRemove,
                    };
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

            let action_error = if matches!(last_status, ActionStatus::WillFail) {
                error.clone()
            } else {
                None
            };
            let action_output = item_output
                .get(action_output_start..)
                .map_or_else(Vec::new, ToOwned::to_owned);
            run_actions.push(RunAction {
                kind: action_kind(action).into(),
                name: action_name.clone(),
                status: last_status,
                error: action_error,
                output: action_output,
            });
            emit(ExecuteEvent::ActionFinished {
                item_index: index,
                action_index,
                item: plan_item.name.clone(),
                action: action_name,
                status: last_status,
            });

            if error.is_some() {
                append_not_run_actions(
                    &mut run_actions,
                    &plan_item.actions,
                    action_index.saturating_add(1),
                    "not run after previous failure",
                );
                break;
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
            actions: run_actions,
        });
        emit(ExecuteEvent::ItemFinished {
            index,
            name: plan_item.name.clone(),
            status: last_status,
        });

        if aborted {
            not_run_start_index = Some(index.saturating_add(1));
            break;
        }
    }

    if let Some(start_index) = not_run_start_index {
        append_remaining_not_run_items(plan, &mut items, start_index);
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

fn append_remaining_not_run_items(plan: &Plan, items: &mut Vec<RunItem>, start_index: usize) {
    for plan_item in plan
        .items
        .iter()
        .skip(start_index)
        .filter(|item| item.selected)
    {
        items.push(RunItem {
            id: plan_item.id.clone(),
            name: plan_item.name.clone(),
            status: ActionStatus::NotRun,
            started_at: None,
            finished_at: None,
            duration_ms: None,
            attempts: 0,
            error: Some("not run (aborted)".into()),
            output: vec![],
            actions: plan_item
                .actions
                .iter()
                .map(|action| not_run_action(action, "not run (aborted)"))
                .collect(),
        });
    }
}

fn append_not_run_actions(
    run_actions: &mut Vec<RunAction>,
    actions: &[Action],
    start_index: usize,
    reason: &str,
) {
    run_actions.extend(
        actions
            .iter()
            .skip(start_index)
            .map(|action| not_run_action(action, reason)),
    );
}

fn not_run_action(action: &Action, reason: &str) -> RunAction {
    RunAction {
        kind: action_kind(action).into(),
        name: action.describe(),
        status: ActionStatus::NotRun,
        error: Some(reason.into()),
        output: vec![],
    }
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
    sudo_auth: &mut impl FnMut(&str) -> bool,
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

    let pkg_mgr = crate::package_managers::resolve_pkg_mgr_name(pkg_mgrs)
        .unwrap_or_else(crate::package_managers::default_pkg_mgr_name);
    let platform_cmd = install_command_for_platform(&entry, &pkg_mgr);
    let presence_command = platform_cmd
        .as_ref()
        .and_then(|cmd| cmd.as_ref().ok())
        .map(String::as_str);
    if matches!(
        install::detect_presence(&entry, presence_command),
        install::InstallPresence::Present
    ) {
        let msg = format!("already installed: {}", entry.name);
        emit(ExecuteEvent::ActionMessage {
            item: item_name.to_string(),
            message: msg.clone(),
        });
        return Ok((
            ActionStatus::NoChange,
            None,
            0,
            vec![OutputLine {
                stream: OutputStream::Action,
                line: msg,
            }],
        ));
    }

    // Fonts can use package-manager installs on platforms where a package
    // exists, with source_url as a fallback for platforms without one.
    if entry.kind == "font"
        && (platform_cmd.is_none()
            || matches!(platform_cmd, Some(Err(InstallCommandError::UnsupportedOs))))
    {
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

        // Download + unzip as a single streaming shell command so curl
        // progress appears in real time.
        let installed_check = if entry.font_family.is_empty() {
            String::new()
        } else {
            format!(
                "if command -v fc-list >/dev/null 2>&1 && fc-list | grep -qi {}; then echo {}; exit 0; fi && ",
                shell_quote(&entry.font_family),
                shell_quote(&format!("font already installed: {}", entry.font_family)),
            )
        };
        let verify_fontconfig = if entry.font_family.is_empty() {
            String::new()
        } else {
            format!(
                " && if command -v fc-list >/dev/null 2>&1; then fc-list | grep -qi {} || {{ echo {}; exit 1; }}; fi",
                shell_quote(&entry.font_family),
                shell_quote(&format!(
                    "installed font files, but fontconfig did not report: {}",
                    entry.font_family
                )),
            )
        };
        let font_cmd = format!(
            "{}tmpdir=$(mktemp -d) && trap 'rm -rf \"$tmpdir\"' EXIT HUP INT TERM && mkdir -p {} && curl -fsSL {} -o \"$tmpdir/font.zip\" && unzip -o -q \"$tmpdir/font.zip\" -d \"$tmpdir/font\" && find \"$tmpdir/font\" -type f \\( -name '*.ttf' -o -name '*.otf' \\) -exec cp {{}} {}/ \\; && if command -v fc-cache >/dev/null 2>&1; then fc-cache -f {}; fi{}",
            installed_check,
            shell_quote(&fonts_dir),
            shell_quote(&entry.source_url),
            shell_quote(&fonts_dir),
            shell_quote(&fonts_dir),
            verify_fontconfig,
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
                return Ok((ActionStatus::WillInstall, None, 1, output));
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

    let cmd = match platform_cmd {
        Some(Ok(cmd)) => cmd,
        Some(Err(InstallCommandError::UnsupportedOs)) => {
            let os = crate::package_managers::os_name();
            let msg = format!("{} is not supported for {pkg_mgr} on {os}", entry.name);
            emit(ExecuteEvent::ActionMessage {
                item: item_name.to_string(),
                message: msg.clone(),
            });
            return Ok((ActionStatus::WillFail, Some(msg), 0, vec![]));
        }
        None | Some(Err(InstallCommandError::MissingPlatform)) => {
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

        // Keep sudo fresh before install commands that need it. This must stay
        // non-interactive so abort can still work in the TUI.
        if shell::command_contains_sudo(&cmd) && !ensure_sudo_session(item_name, sudo_auth) {
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
                return Ok((ActionStatus::WillInstall, None, attempt, all_output));
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

fn action_kind(action: &Action) -> &'static str {
    match action {
        Action::Install { .. } => "install",
        Action::Link { .. } => "link",
        Action::Create { .. } => "create",
        Action::Shell { .. } => "shell",
        Action::Clean { .. } => "clean",
    }
}

fn ensure_sudo_session(item: &str, sudo_auth: &mut impl FnMut(&str) -> bool) -> bool {
    shell::refresh_sudo().unwrap_or(false) || sudo_auth(item)
}

enum InstallCommandError {
    MissingPlatform,
    UnsupportedOs,
}

fn install_command_for_platform(
    entry: &install::ToolEntry,
    pkg_mgr: &str,
) -> Option<Result<String, InstallCommandError>> {
    let distro = crate::package_managers::distro_id();
    let mut candidates = vec![pkg_mgr.to_string()];
    if let Some(distro) = &distro
        && !candidates.iter().any(|candidate| candidate == distro)
    {
        candidates.push(distro.clone());
    }
    let os = crate::package_managers::os_name().to_string();
    if !candidates.iter().any(|candidate| candidate == &os) {
        candidates.push(os);
    }

    let mut saw_unsupported = false;
    for candidate in candidates {
        if let Some(c) = entry.platforms.get(&candidate) {
            if command_supports_current_platform(c, distro.as_deref()) {
                return Some(Ok(c.command().into()));
            }
            saw_unsupported = true;
        }
    }

    if saw_unsupported {
        Some(Err(InstallCommandError::UnsupportedOs))
    } else if entry.kind == "font" {
        None
    } else {
        Some(Err(InstallCommandError::MissingPlatform))
    }
}

fn command_supports_current_platform(
    command: &install::InstallCommand,
    distro: Option<&str>,
) -> bool {
    command.supports_os(crate::package_managers::os_name())
        || distro.is_some_and(|distro| command.supports_os(distro))
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
    use crate::model::{Action, HostInfo, Mode, Plan, PlanItem};
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
            default_shell: None,
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
            default_shell: None,
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
        assert_eq!(
            run.items.len(),
            plan.items.iter().filter(|item| item.selected).count()
        );
        assert!(
            run.items
                .iter()
                .all(|item| item.status == ActionStatus::NotRun)
        );
        assert!(
            run.items
                .iter()
                .all(|item| item.error.as_deref().is_some_and(|e| e.contains("aborted")))
        );
    }

    #[test]
    fn install_skips_when_binary_is_present() {
        fn deny_sudo(_: &str) -> bool {
            false
        }

        let mut events = Vec::new();
        let mut sudo_auth = deny_sudo;
        let (status, err, attempts, output) = run_install_streaming(
            "sh",
            &crate::config::PackageManagerConfig::default(),
            DEFAULT_INSTALL_RETRIES,
            "sh",
            &mut |event| events.push(event),
            &|| false,
            &mut sudo_auth,
        )
        .unwrap();

        assert_eq!(status, ActionStatus::NoChange);
        assert!(err.is_none());
        assert_eq!(attempts, 0);
        assert!(
            output
                .iter()
                .any(|line| line.line.contains("already installed: sh"))
        );
        assert!(events.iter().any(|event| {
            matches!(event, ExecuteEvent::ActionMessage { message, .. } if message.contains("already installed: sh"))
        }));
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
            default_shell: None,
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
            default_shell: None,
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
            default_shell: None,
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
        assert_eq!(run.items[0].status, ActionStatus::WillLink);
        assert!(messages.iter().any(|m| m.contains("linked")));
    }

    #[test]
    fn create_existing_directory_reports_no_change() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("already-there");
        std::fs::create_dir_all(&target).unwrap();

        let cfg = Config {
            path: dir.path().join("dotman.yaml"),
            package_managers: crate::config::PackageManagerConfig::default(),
            install: vec![],
            links: vec![],
            create: vec![target.clone()],
            shell: vec![],
            default_shell: None,
            clean: vec![],
            auto_install_pkg_manager: false,
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let run = execute(&plan, &cfg).unwrap();

        assert_eq!(run.status, RunStatus::Success);
        assert_eq!(run.items[0].status, ActionStatus::NoChange);
        assert!(
            run.items[0]
                .output
                .iter()
                .any(|line| line.line.contains("exists"))
        );
    }

    #[test]
    fn failed_action_stops_remaining_actions_in_item() {
        let dir = tempfile::tempdir().unwrap();
        let create_target = dir.path().join("should-not-exist");
        let cfg = Config {
            path: dir.path().join("dotman.yaml"),
            package_managers: crate::config::PackageManagerConfig::default(),
            install: vec![],
            links: vec![],
            create: vec![],
            shell: vec![],
            default_shell: None,
            clean: vec![],
            auto_install_pkg_manager: false,
        };
        let plan = Plan {
            id: "test-run".into(),
            mode: Mode::Deploy,
            created_at: "2026-01-01T00:00:00Z".into(),
            config_path: cfg.path.clone(),
            config_hash: "hash".into(),
            host: HostInfo {
                hostname: "test".into(),
                os: "Linux".into(),
                arch: "x86_64".into(),
                user: "test".into(),
                home: dir.path().to_path_buf(),
            },
            items: vec![PlanItem {
                id: "failing-item".into(),
                name: "failing item".into(),
                layer: "misc".into(),
                selected: true,
                actions: vec![
                    Action::Shell {
                        command: "false".into(),
                        description: None,
                        optional: false,
                        if_condition: None,
                    },
                    Action::Create {
                        target: create_target.clone(),
                    },
                ],
            }],
            auto_install_pkg_manager: false,
        };

        let run = execute(&plan, &cfg).unwrap();

        assert_eq!(run.status, RunStatus::Failed);
        assert!(!create_target.exists());
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
            default_shell: None,
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
