use super::ExecuteEvent;
use super::command::{ensure_sudo_session, run_command_streaming_with_events};
use super::installer::{DEFAULT_INSTALL_RETRIES, run_install_streaming};
use super::result::{
    action_kind, append_not_run_actions, append_remaining_not_run_items, cap_output_len,
    describe_clean_action, describe_link_action, now_iso, push_output_line,
};
use crate::config::Config;
use crate::model::{
    Action, ActionStatus, OutputLine, OutputStream, Plan, Run, RunAction, RunItem, RunStatus,
};
use crate::ops::clean;
use crate::ops::create::create_dir;
use crate::ops::link::{self, LinkSettings};
use crate::ops::shell;
use anyhow::Result;
use std::path::Path;
use std::time::Instant;

pub(super) fn run<F, C, S>(
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
                last_status = ActionStatus::Aborted;
                error = Some("aborted".into());
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
            let mut action_output: Vec<OutputLine> = Vec::new();
            let mut action_error: Option<String> = None;
            emit(ExecuteEvent::ActionStarted {
                item_index: index,
                action_index,
                item: plan_item.name.clone(),
                action: action_name.clone(),
            });
            match action {
                Action::Install { spec } => {
                    let install_result = run_install_streaming(
                        spec,
                        DEFAULT_INSTALL_RETRIES,
                        &plan_item.name,
                        &mut emit,
                        &should_abort,
                        &mut sudo_auth,
                    );
                    match install_result {
                        Ok((status, err, n, output)) => {
                            action_output.extend(output);
                            attempts = n;
                            if let Some(e) = err {
                                action_error = Some(e);
                            }
                            last_status = status;
                        }
                        Err(e) => {
                            let msg = format!("install failed: {e}");
                            emit(ExecuteEvent::ActionError {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut action_output, OutputStream::Action, &msg);
                            action_error = Some(e.to_string());
                            last_status = ActionStatus::WillFail;
                        }
                    }
                }
                Action::Link {
                    target,
                    source,
                    backup,
                    relink,
                } => {
                    let settings = LinkSettings {
                        create: true,
                        relative: true,
                        backup: *backup,
                        relink: *relink,
                    };
                    match link::plan_link(config_dir, target, source, settings) {
                        Ok(link_plan) => {
                            let action_desc = describe_link_action(&link_plan.action);
                            last_status = match &link_plan.action {
                                link::LinkAction::Skip => ActionStatus::NoChange,
                                link::LinkAction::Link | link::LinkAction::Relink => {
                                    ActionStatus::WillLink
                                }
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
                                    push_output_line(
                                        &mut action_output,
                                        OutputStream::Action,
                                        &msg,
                                    );
                                }
                                Err(e) => {
                                    let msg = format!("link failed: {e}");
                                    emit(ExecuteEvent::ActionError {
                                        item: plan_item.name.clone(),
                                        message: msg.clone(),
                                    });
                                    push_output_line(
                                        &mut action_output,
                                        OutputStream::Action,
                                        &msg,
                                    );
                                    action_error = Some(e.to_string());
                                    last_status = ActionStatus::WillFail;
                                }
                            }
                        }
                        Err(e) => {
                            let msg = format!("link planning failed: {e}");
                            emit(ExecuteEvent::ActionError {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut action_output, OutputStream::Action, &msg);
                            action_error = Some(e.to_string());
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
                            push_output_line(&mut action_output, OutputStream::Action, &msg);
                        }
                        Err(e) => {
                            let msg = format!("create failed: {e}");
                            emit(ExecuteEvent::ActionError {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut action_output, OutputStream::Action, &msg);
                            action_error = Some(e.to_string());
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
                    let condition_skipped = if let Some(cond) = if_condition {
                        match shell::condition_matches(cond, config_dir) {
                            Ok(shell::ConditionResult::Matched) => false,
                            Ok(shell::ConditionResult::NotMatched) => {
                                last_status = ActionStatus::WillSkip;
                                let msg = format!("condition skipped: {cond}");
                                emit(ExecuteEvent::ActionMessage {
                                    item: plan_item.name.clone(),
                                    message: msg.clone(),
                                });
                                push_output_line(&mut action_output, OutputStream::Action, &msg);
                                true
                            }
                            Ok(shell::ConditionResult::Error(err)) => {
                                let msg = format!("condition error: {err}");
                                emit(ExecuteEvent::ActionError {
                                    item: plan_item.name.clone(),
                                    message: msg.clone(),
                                });
                                push_output_line(&mut action_output, OutputStream::Action, &msg);
                                action_error = Some(err);
                                last_status = ActionStatus::WillFail;
                                true
                            }
                            Err(e) => {
                                let msg = format!("condition error: {e}");
                                emit(ExecuteEvent::ActionError {
                                    item: plan_item.name.clone(),
                                    message: msg.clone(),
                                });
                                push_output_line(&mut action_output, OutputStream::Action, &msg);
                                action_error = Some(e.to_string());
                                last_status = ActionStatus::WillFail;
                                true
                            }
                        }
                    } else {
                        false
                    };
                    if !condition_skipped && action_error.is_none() {
                        // Keep sudo fresh before commands that need it. This must
                        // stay non-interactive so abort can still work in the TUI.
                        if shell::command_contains_sudo(command)
                            && !ensure_sudo_session(&plan_item.name, &mut sudo_auth)
                        {
                            let msg =
                                "sudo session expired — re-run to re-authenticate".to_string();
                            emit(ExecuteEvent::ActionError {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut action_output, OutputStream::Action, &msg);
                            action_error = Some("sudo session expired".into());
                            last_status = ActionStatus::WillFail;
                        } else {
                            let command_result = run_command_streaming_with_events(
                                command,
                                config_dir,
                                &plan_item.name,
                                &mut emit,
                                &should_abort,
                            );
                            match command_result {
                                Ok((exit_code, output)) => {
                                    action_output.extend(output);
                                    cap_output_len(&mut action_output);
                                    match exit_code {
                                        Some(0) => {
                                            last_status = ActionStatus::Executed;
                                        }
                                        Some(code) if *optional => {
                                            let msg =
                                                format!("optional command failed (exit {code})");
                                            emit(ExecuteEvent::ActionMessage {
                                                item: plan_item.name.clone(),
                                                message: msg.clone(),
                                            });
                                            push_output_line(
                                                &mut action_output,
                                                OutputStream::Action,
                                                &msg,
                                            );
                                            last_status = ActionStatus::NoChange;
                                        }
                                        Some(code) => {
                                            let msg =
                                                format!("command failed with exit code {code}");
                                            emit(ExecuteEvent::ActionError {
                                                item: plan_item.name.clone(),
                                                message: msg.clone(),
                                            });
                                            push_output_line(
                                                &mut action_output,
                                                OutputStream::Action,
                                                &msg,
                                            );
                                            action_error = Some(format!("exit code {code}"));
                                            last_status = ActionStatus::WillFail;
                                        }
                                        None => {
                                            action_error = Some("aborted".into());
                                            last_status = ActionStatus::Aborted;
                                            aborted = true;
                                        }
                                    }
                                }
                                Err(e) => {
                                    let msg = format!("command failed: {e}");
                                    emit(ExecuteEvent::ActionError {
                                        item: plan_item.name.clone(),
                                        message: msg.clone(),
                                    });
                                    push_output_line(
                                        &mut action_output,
                                        OutputStream::Action,
                                        &msg,
                                    );
                                    action_error = Some(e.to_string());
                                    last_status = ActionStatus::WillFail;
                                }
                            }
                        }
                    }
                }
                Action::Clean { target, force } => {
                    let expanded_target = crate::path::expand_home(&target.to_string_lossy())
                        .unwrap_or_else(|_| target.clone());
                    match clean::plan_clean(&expanded_target, *force) {
                        Ok(clean_action) => {
                            let action_desc = describe_clean_action(&clean_action);
                            last_status = match &clean_action {
                                clean::CleanAction::Skip => ActionStatus::NoChange,
                                clean::CleanAction::RemoveSymlink => ActionStatus::WillClean,
                                clean::CleanAction::BackupAndRemove(_) => {
                                    ActionStatus::WillBackupRemove
                                }
                            };
                            match clean::apply_clean(clean_action, &expanded_target) {
                                Ok(()) => {
                                    let msg = format!(
                                        "cleaned {} ({action_desc})",
                                        expanded_target.display()
                                    );
                                    emit(ExecuteEvent::ActionMessage {
                                        item: plan_item.name.clone(),
                                        message: msg.clone(),
                                    });
                                    push_output_line(
                                        &mut action_output,
                                        OutputStream::Action,
                                        &msg,
                                    );
                                }
                                Err(e) => {
                                    let msg = format!("clean failed: {e}");
                                    emit(ExecuteEvent::ActionError {
                                        item: plan_item.name.clone(),
                                        message: msg.clone(),
                                    });
                                    push_output_line(
                                        &mut action_output,
                                        OutputStream::Action,
                                        &msg,
                                    );
                                    action_error = Some(e.to_string());
                                    last_status = ActionStatus::WillFail;
                                }
                            }
                        }
                        Err(e) => {
                            let msg = format!("clean planning failed: {e}");
                            emit(ExecuteEvent::ActionError {
                                item: plan_item.name.clone(),
                                message: msg.clone(),
                            });
                            push_output_line(&mut action_output, OutputStream::Action, &msg);
                            action_error = Some(e.to_string());
                            last_status = ActionStatus::WillFail;
                        }
                    }
                }
            }

            if last_status == ActionStatus::Aborted {
                aborted = true;
                emit(ExecuteEvent::Aborted);
            }

            cap_output_len(&mut action_output);
            item_output.extend(action_output.clone());
            cap_output_len(&mut item_output);
            if action_error.is_some() {
                error = action_error.clone();
            }
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
        id: crate::store::new_run_id(),
        plan_id: Some(plan.id.clone()),
        mode: plan.mode,
        started_at,
        finished_at: Some(now_iso()),
        status,
        config_hash: plan.config_hash.clone(),
        config_path: Some(plan.config_path.clone()),
        host: Some(plan.host.clone()),
        items,
    })
}
