use crate::model::{
    Action, ActionStatus, MAX_HISTORY_OUTPUT_LINES, OutputLine, OutputStream, Plan, RunAction,
    RunItem,
};
use crate::ops::{clean, link};

pub(super) fn append_remaining_not_run_items(
    plan: &Plan,
    items: &mut Vec<RunItem>,
    start_index: usize,
) {
    for plan_item in plan.items.iter().skip(start_index) {
        let (status, reason) = if plan_item.selected {
            (ActionStatus::NotRun, "not run (aborted)")
        } else {
            (ActionStatus::WillSkip, "skipped (not selected)")
        };
        items.push(RunItem {
            id: plan_item.id.clone(),
            name: plan_item.name.clone(),
            status,
            started_at: None,
            finished_at: None,
            duration_ms: None,
            attempts: 0,
            error: Some(reason.into()),
            output: vec![],
            actions: plan_item
                .actions
                .iter()
                .map(|action| skipped_or_not_run_action(action, status, reason))
                .collect(),
        });
    }
}

pub(super) fn append_not_run_actions(
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

pub(super) fn not_run_action(action: &Action, reason: &str) -> RunAction {
    skipped_or_not_run_action(action, ActionStatus::NotRun, reason)
}

pub(super) fn skipped_or_not_run_action(
    action: &Action,
    status: ActionStatus,
    reason: &str,
) -> RunAction {
    RunAction {
        kind: action_kind(action).into(),
        name: action.describe(),
        status,
        error: Some(reason.into()),
        output: vec![],
    }
}

/// Fallback OS key used when no package manager is configured for the current platform.
pub(super) fn describe_link_action(action: &link::LinkAction) -> String {
    match action {
        link::LinkAction::Skip => "skip: already linked".into(),
        link::LinkAction::Link => "link: create symlink".into(),
        link::LinkAction::Backup(path) => format!("backup then link: {}", path.display()),
        link::LinkAction::Relink => "relink: replace wrong symlink".into(),
        link::LinkAction::Fail(reason) => format!("fail: {reason}"),
    }
}

pub(super) fn action_kind(action: &Action) -> &'static str {
    match action {
        Action::Install { .. } => "install",
        Action::Link { .. } => "link",
        Action::Create { .. } => "create",
        Action::Shell { .. } => "shell",
        Action::Clean { .. } => "clean",
    }
}

pub(super) fn describe_clean_action(action: &clean::CleanAction) -> String {
    match action {
        clean::CleanAction::Skip => "skip".into(),
        clean::CleanAction::RemoveSymlink => "remove symlink".into(),
        clean::CleanAction::BackupAndRemove(path) => {
            format!("backup to {} then remove", path.display())
        }
    }
}

pub(super) fn push_output_line(output: &mut Vec<OutputLine>, stream: OutputStream, line: &str) {
    output.push(OutputLine {
        stream,
        line: line.to_string(),
    });
    cap_output_len(output);
}

pub(super) fn cap_output_len(output: &mut Vec<OutputLine>) {
    if output.len() > MAX_HISTORY_OUTPUT_LINES {
        let drop = output.len() - MAX_HISTORY_OUTPUT_LINES;
        output.drain(0..drop);
    }
}

pub(super) fn now_iso() -> String {
    time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::new())
}
