use super::MAX_RUN_EVENTS_PER_FRAME;
use super::log::{log_group_for_event, push_log, push_log_group, push_log_indented};
use super::view::{run_action_total, run_executed_action_total};
use crate::model::{ActionStatus, Run};
use crate::ops::shell;
use crate::theme::CATPPUCCIN_MOCHA;
use crate::tui::{App, LogKind, RunThreadResult, restore_terminal, setup_terminal};

pub(in crate::tui) fn apply_run_thread_result(app: &mut App, result: RunThreadResult) {
    if let Some(run) = result.run {
        app.run = Some(run.clone());
        sync_finished_run_state(app, &run);
    }
    if let Some(warning) = result.save_warning {
        app.run_save_warning = Some(warning.clone());
        push_log(app, &warning, Some(CATPPUCCIN_MOCHA.warning));
    }
    if let Some(error) = result.error {
        let message = format!("run failed: {error}");
        app.run_error = Some(message.clone());
        clear_active_run_state(app);
        push_log(app, &message, Some(CATPPUCCIN_MOCHA.danger));
    }
}

pub(in crate::tui::run) fn clear_active_run_state(app: &mut App) {
    app.current_item = None;
    app.current_action = None;
    app.active_log_group = None;
}

pub(in crate::tui) fn sync_finished_run_state(app: &mut App, run: &Run) {
    let total = app.progress.1.max(run_action_total(run));
    let done = run_executed_action_total(run).min(total);
    app.progress = (done, total);
    app.current_item = None;
    app.current_action = None;
    app.run_item_statuses = run.items.iter().map(|item| Some(item.status)).collect();
    app.run_action_statuses = run
        .items
        .iter()
        .map(|item| {
            item.actions
                .iter()
                .map(|action| Some(action.status))
                .collect()
        })
        .collect();
}

pub(in crate::tui) fn drain_run_events(app: &mut App) -> bool {
    let Some(rx) = app.run_events.take() else {
        return false;
    };
    let mut drained = false;
    for _ in 0..MAX_RUN_EVENTS_PER_FRAME {
        let Ok(event) = rx.try_recv() else {
            break;
        };
        drained = true;
        match event {
            crate::execute::ExecuteEvent::ItemStarted { index, name } => {
                app.current_item = Some(index);
                app.last_item_index = Some(index);
                app.current_action = None;
                app.active_log_group = None;
                push_log_group(app, &name);
                push_log_indented(app, "started", None, 1, LogKind::System);
            }
            crate::execute::ExecuteEvent::ActionStarted {
                item_index,
                action_index,
                item,
                action,
            } => {
                app.current_action = Some((item_index, action_index));
                app.current_item = Some(item_index);
                app.last_item_index = Some(item_index);
                let group = format!("{item} / {action}");
                app.active_log_group = Some(group.clone());
                push_log_group(app, &group);
            }
            crate::execute::ExecuteEvent::ActionFinished {
                item_index,
                action_index,
                item: _,
                action,
                status,
            } => {
                if let Some(statuses) = app.run_action_statuses.get_mut(item_index)
                    && let Some(slot) = statuses.get_mut(action_index)
                {
                    *slot = Some(status);
                }
                app.progress.0 = app.progress.0.saturating_add(1).min(app.progress.1);
                push_log_indented(
                    app,
                    &format!("finished {action}: {}", status.result_label()),
                    None,
                    1,
                    if matches!(status, ActionStatus::WillFail | ActionStatus::Aborted) {
                        LogKind::Stderr
                    } else {
                        LogKind::System
                    },
                );
                app.current_action = None;
                app.active_log_group = None;
            }
            crate::execute::ExecuteEvent::Output { item, stream, line } => {
                let color = match stream {
                    crate::model::OutputStream::Stderr => Some(CATPPUCCIN_MOCHA.danger),
                    crate::model::OutputStream::Stdout => None,
                    crate::model::OutputStream::Action => Some(CATPPUCCIN_MOCHA.primary),
                };
                let kind = match stream {
                    crate::model::OutputStream::Stderr => LogKind::Stderr,
                    crate::model::OutputStream::Stdout => LogKind::Stdout,
                    crate::model::OutputStream::Action => LogKind::Action,
                };
                let group = log_group_for_event(app, &item);
                push_log_group(app, &group);
                push_log_indented(app, &line, color, 1, kind);
            }
            crate::execute::ExecuteEvent::ActionMessage { item, message } => {
                let group = log_group_for_event(app, &item);
                push_log_group(app, &group);
                push_log_indented(
                    app,
                    &message,
                    Some(CATPPUCCIN_MOCHA.primary),
                    1,
                    LogKind::Action,
                );
            }
            crate::execute::ExecuteEvent::ActionError { item, message } => {
                let group = log_group_for_event(app, &item);
                push_log_group(app, &group);
                push_log_indented(
                    app,
                    &message,
                    Some(CATPPUCCIN_MOCHA.danger),
                    1,
                    LogKind::Stderr,
                );
            }
            crate::execute::ExecuteEvent::ItemFinished {
                index,
                name,
                status,
            } => {
                if let Some(plan) = app.plan.as_ref()
                    && plan
                        .items
                        .get(index)
                        .is_some_and(|item| item.selected && item.actions.is_empty())
                {
                    app.progress.0 = app.progress.0.saturating_add(1).min(app.progress.1);
                }
                app.current_item = None;
                app.current_action = None;
                app.active_log_group = None;
                app.last_item_index = Some(index);
                if let Some(slot) = app.run_item_statuses.get_mut(index) {
                    *slot = Some(status);
                }
                push_log_group(app, &name);
                push_log_indented(
                    app,
                    &format!("finished: {}", status.result_label()),
                    None,
                    1,
                    LogKind::System,
                );
            }
            crate::execute::ExecuteEvent::Aborted => {
                push_log(app, "run aborted", Some(CATPPUCCIN_MOCHA.warning));
            }
            crate::execute::ExecuteEvent::SudoPrompt { item, response } => {
                let group = log_group_for_event(app, &item);
                push_log_group(app, &group);
                push_log_indented(
                    app,
                    "sudo session expired; re-authenticating",
                    Some(CATPPUCCIN_MOCHA.warning),
                    1,
                    LogKind::Action,
                );
                let ok = match restore_terminal() {
                    Ok(()) => {
                        let ok = shell::pre_cache_sudo().unwrap_or(false);
                        let _ = setup_terminal();
                        app.needs_terminal_reset = true;
                        ok
                    }
                    Err(_) => false,
                };
                let _ = response.send(ok);
                if !ok {
                    push_log_indented(
                        app,
                        "sudo authentication failed",
                        Some(CATPPUCCIN_MOCHA.danger),
                        1,
                        LogKind::Stderr,
                    );
                }
            }
        }
    }
    app.run_events = Some(rx);
    drained
}

pub(in crate::tui) fn drain_all_run_events(app: &mut App) {
    while drain_run_events(app) {}
}
