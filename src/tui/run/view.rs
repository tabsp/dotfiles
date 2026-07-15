use crate::icons;
use crate::model::{Action, ActionStatus, Plan, Run, RunAction, RunItem, RunStatus};
use crate::theme::CATPPUCCIN_MOCHA;
use crate::tui::{App, display_width, fit_to_width, run_status_color};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::sync::atomic::Ordering;

pub(in crate::tui) fn finished_run_for_view(app: &App) -> Option<&Run> {
    if app.run_thread.is_none() && app.run_error.is_none() {
        app.run.as_ref()
    } else {
        None
    }
}

pub(in crate::tui) fn run_is_terminal(app: &App) -> bool {
    app.run_thread.is_none() && (app.run_error.is_some() || app.run.is_some())
}

pub(in crate::tui) fn run_is_aborting(app: &App) -> bool {
    app.abort_flag
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::SeqCst))
}

pub(in crate::tui) fn run_border_color(app: &App, aborting: bool) -> Color {
    if aborting {
        CATPPUCCIN_MOCHA.warning
    } else if app.run_error.is_some() {
        CATPPUCCIN_MOCHA.danger
    } else if let Some(run) = finished_run_for_view(app) {
        run_status_color(run.status)
    } else {
        CATPPUCCIN_MOCHA.running
    }
}

#[cfg(test)]
pub(in crate::tui) fn run_title(app: &App, width: usize) -> String {
    line_to_plain_string(&run_title_line(app, width))
}

pub(in crate::tui) fn run_title_line(app: &App, width: usize) -> Line<'static> {
    let icon_set = icons::current();
    let (state, done, total) = if app.run_error.is_some() {
        ("Failed", app.progress.0, app.progress.1)
    } else if let Some(run) = finished_run_for_view(app) {
        let total = app.progress.1.max(run_action_total(run));
        let done = if app.progress.1 == 0 {
            run_executed_action_total(run)
        } else {
            app.progress.0
        };
        (run_status_label(run.status), done, total)
    } else if run_is_aborting(app) {
        ("Stopping", app.progress.0, app.progress.1)
    } else {
        ("Running", app.progress.0, app.progress.1)
    };
    let prefix = format!("{}  Run  ", icon_set.running);
    let progress = format!(
        "{state}  {done}/{total}  {}",
        run_progress_bar(done, total, 10)
    );
    let divider_width = width.saturating_sub(display_width(&prefix) + display_width(&progress));
    Line::from(vec![
        Span::styled(
            prefix,
            Style::default()
                .fg(CATPPUCCIN_MOCHA.fg_dim)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "─".repeat(divider_width),
            Style::default().fg(CATPPUCCIN_MOCHA.border_subtle),
        ),
        Span::styled(
            progress,
            Style::default().fg(run_border_color(app, run_is_aborting(app))),
        ),
    ])
}

pub(in crate::tui) fn run_status_line(app: &App, width: usize) -> Line<'static> {
    let (label, text, style) = if let Some(error) = &app.run_error {
        (
            "  error    ",
            error.clone(),
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
        )
    } else if let Some(warning) = &app.run_save_warning {
        let summary = finished_run_for_view(app)
            .map(final_run_summary)
            .unwrap_or_else(|| "run finished".into());
        (
            "  result   ",
            format!("{summary} · warning: {warning}"),
            Style::default().fg(CATPPUCCIN_MOCHA.warning),
        )
    } else if let Some(run) = finished_run_for_view(app) {
        (
            "  current  ",
            final_run_summary(run),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        )
    } else if let Some((item_idx, action_idx)) = app.current_action {
        (
            "  current  ",
            current_run_action_name(app, item_idx, action_idx).unwrap_or_else(|| {
                current_run_item_name(app)
                    .map(str::to_string)
                    .unwrap_or_else(|| "waiting".into())
            }),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        )
    } else {
        (
            "  current  ",
            current_run_item_name(app)
                .map(str::to_string)
                .unwrap_or_else(|| "waiting".into()),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        )
    };
    Line::from(vec![
        Span::styled(label, Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(fit_to_width(&text, width.saturating_sub(11)), style),
    ])
}

pub(in crate::tui) fn run_status_label(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Running => "Running",
        RunStatus::Success => "Success",
        RunStatus::Failed => "Failed",
        RunStatus::Aborted => "Aborted",
    }
}

#[cfg(test)]
pub(in crate::tui) fn line_to_plain_string(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("")
}

pub(in crate::tui) fn final_run_summary(run: &Run) -> String {
    crate::model::RunSummary::from_run(run).display()
}

pub(in crate::tui) fn current_run_item_name(app: &App) -> Option<&str> {
    let plan = app.plan.as_ref()?;
    let index = app.current_item.or(app.last_item_index)?;
    plan.items.get(index).map(|item| item.name.as_str())
}

pub(in crate::tui) fn current_run_action_name(
    app: &App,
    item_index: usize,
    action_index: usize,
) -> Option<String> {
    let plan = app.plan.as_ref()?;
    let item = plan.items.get(item_index)?;
    let action = item.actions.get(action_index)?;
    Some(format!("{} / {}", item.name, action.describe()))
}

pub(in crate::tui) fn selected_run_action_total(plan: &Plan) -> usize {
    plan.items
        .iter()
        .filter(|item| item.selected)
        .map(|item| item.actions.len().max(usize::from(item.actions.is_empty())))
        .sum()
}

pub(in crate::tui) fn run_action_total(run: &Run) -> usize {
    run.items
        .iter()
        .filter(|item| !run_item_was_unselected(item))
        .map(|item| item.actions.len().max(usize::from(item.actions.is_empty())))
        .sum()
}

pub(in crate::tui) fn run_executed_action_total(run: &Run) -> usize {
    run.items
        .iter()
        .filter(|item| !run_item_was_unselected(item))
        .map(|item| {
            if item.actions.is_empty() {
                usize::from(item.status != ActionStatus::NotRun)
            } else {
                item.actions
                    .iter()
                    .filter(|action| action.status != ActionStatus::NotRun)
                    .count()
            }
        })
        .sum()
}

fn run_item_was_unselected(item: &RunItem) -> bool {
    item.status == ActionStatus::WillSkip && item.started_at.is_none()
}

pub(in crate::tui) fn run_progress_bar(done: usize, total: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if total == 0 {
        return "░".repeat(width);
    }
    let filled = (done.saturating_mul(width) + total / 2) / total;
    format!(
        "{}{}",
        "█".repeat(filled.min(width)),
        "░".repeat(width.saturating_sub(filled))
    )
}

pub(in crate::tui) fn run_log_panel_height(total_height: u16) -> u16 {
    let available = total_height.saturating_sub(2);
    let max_log = available.saturating_sub(4);
    let desired = if available >= 24 { 10 } else { 7 };
    desired.min(max_log)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum RunGroup {
    Failed,
    Aborted,
    Running,
    Ran,
    Changed,
    NoChange,
    NotRun,
    Skipped,
    Pending,
}

pub(in crate::tui) struct RunDisplayLine {
    pub(in crate::tui) group: RunGroup,
    pub(in crate::tui) line: Line<'static>,
    pub(in crate::tui) active: bool,
}

pub(in crate::tui) fn run_body_lines(app: &App, width: usize, height: usize) -> Vec<Line<'static>> {
    if height == 0 {
        return Vec::new();
    }
    let display_lines = if let Some(run) = finished_run_for_view(app) {
        finished_run_display_lines(run, width)
    } else if let Some(plan) = &app.plan {
        live_run_display_lines(app, plan, width)
    } else {
        return vec![Line::from("loading...")];
    };
    if finished_run_for_view(app).is_some() {
        grouped_run_lines(display_lines, width, height)
    } else {
        ordered_run_lines(display_lines, width, height)
    }
}

pub(in crate::tui) fn live_run_display_lines(
    app: &App,
    plan: &Plan,
    width: usize,
) -> Vec<RunDisplayLine> {
    let mut lines = Vec::new();
    for (item_index, item) in plan.items.iter().enumerate() {
        if item.actions.is_empty() {
            let active = Some(item_index) == app.current_item;
            let status = if !item.selected {
                Some(ActionStatus::WillSkip)
            } else if active {
                None
            } else {
                app.run_item_statuses
                    .get(item_index)
                    .and_then(|status| *status)
            };
            lines.push(RunDisplayLine {
                group: run_group_for_status(status, active),
                line: run_action_line(RunActionLineSpec {
                    kind: "shell",
                    item_name: &item.name,
                    action_name: "",
                    status_label: run_status_label_for_view(status, active),
                    status,
                    active,
                    width,
                    spinner_frame: app.spinner_frame,
                }),
                active,
            });
            continue;
        }

        for (action_index, action) in item.actions.iter().enumerate() {
            let active = app.current_action == Some((item_index, action_index));
            let status = if !item.selected {
                Some(ActionStatus::WillSkip)
            } else if active {
                None
            } else {
                app.run_action_statuses
                    .get(item_index)
                    .and_then(|statuses| statuses.get(action_index))
                    .and_then(|status| *status)
            };
            lines.push(RunDisplayLine {
                group: run_group_for_status(status, active),
                line: run_action_line(RunActionLineSpec {
                    kind: action_kind_for_view(action),
                    item_name: &item.name,
                    action_name: &action.describe(),
                    status_label: run_status_label_for_view(status, active),
                    status,
                    active,
                    width,
                    spinner_frame: app.spinner_frame,
                }),
                active,
            });
        }
    }
    lines
}

pub(in crate::tui) fn finished_run_display_lines(run: &Run, width: usize) -> Vec<RunDisplayLine> {
    let mut lines = Vec::new();
    for item in &run.items {
        if item.actions.is_empty() {
            let status = Some(item.status);
            lines.push(RunDisplayLine {
                group: run_group_for_status(status, false),
                line: run_action_line(RunActionLineSpec {
                    kind: "shell",
                    item_name: &item.name,
                    action_name: "",
                    status_label: run_status_label_for_view(status, false),
                    status,
                    active: false,
                    width,
                    spinner_frame: 0,
                }),
                active: false,
            });
            continue;
        }

        for action in &item.actions {
            let status = Some(action.status);
            lines.push(RunDisplayLine {
                group: run_group_for_status(status, false),
                line: finished_action_line(item, action, width),
                active: false,
            });
        }
    }
    lines
}

pub(in crate::tui) fn grouped_run_lines(
    display_lines: Vec<RunDisplayLine>,
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let mut all_lines = Vec::new();
    let mut active_line = None;
    for group in [
        RunGroup::Failed,
        RunGroup::Aborted,
        RunGroup::Running,
        RunGroup::Ran,
        RunGroup::Changed,
        RunGroup::NoChange,
        RunGroup::NotRun,
        RunGroup::Skipped,
        RunGroup::Pending,
    ] {
        let group_lines = display_lines
            .iter()
            .filter(|line| line.group == group)
            .collect::<Vec<_>>();
        if group_lines.is_empty() {
            continue;
        }
        all_lines.push(run_group_header_line(group, group_lines.len(), width));
        for display_line in group_lines {
            if display_line.active {
                active_line = Some(all_lines.len());
            }
            all_lines.push(display_line.line.clone());
        }
    }

    if all_lines.len() <= height {
        return all_lines;
    }
    let focus = active_line.unwrap_or_else(|| all_lines.len().saturating_sub(1));
    let mut start = focus.saturating_sub(height / 2);
    if start + height > all_lines.len() {
        start = all_lines.len() - height;
    }
    let end = start + height;
    let mut visible = all_lines[start..end].to_vec();
    if start > 0
        && let Some(first) = visible.first_mut()
    {
        *first = Line::from(Span::styled(
            fit_to_width(&format!("  ... {start} above"), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    let below = all_lines.len().saturating_sub(end);
    if below > 0
        && let Some(last) = visible.last_mut()
    {
        *last = Line::from(Span::styled(
            fit_to_width(&format!("  ... {below} below"), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    visible
}

pub(in crate::tui) fn ordered_run_lines(
    display_lines: Vec<RunDisplayLine>,
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let focus = display_lines
        .iter()
        .position(|display_line| display_line.active)
        .unwrap_or_else(|| display_lines.len().saturating_sub(1));
    let all_lines = display_lines
        .into_iter()
        .map(|display_line| display_line.line)
        .collect::<Vec<_>>();
    if all_lines.len() <= height {
        return all_lines;
    }
    let mut start = focus.saturating_sub(height / 2);
    if start + height > all_lines.len() {
        start = all_lines.len() - height;
    }
    let end = start + height;
    let mut visible = all_lines[start..end].to_vec();
    if start > 0
        && let Some(first) = visible.first_mut()
    {
        *first = Line::from(Span::styled(
            fit_to_width(&format!("  ... {start} above"), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    let below = all_lines.len().saturating_sub(end);
    if below > 0
        && let Some(last) = visible.last_mut()
    {
        *last = Line::from(Span::styled(
            fit_to_width(&format!("  ... {below} below"), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    visible
}

pub(in crate::tui) fn run_group_for_status(status: Option<ActionStatus>, active: bool) -> RunGroup {
    if active {
        return RunGroup::Running;
    }
    match status {
        Some(ActionStatus::WillFail) => RunGroup::Failed,
        Some(ActionStatus::Aborted) => RunGroup::Aborted,
        Some(ActionStatus::NoChange) => RunGroup::NoChange,
        Some(ActionStatus::NotRun) => RunGroup::NotRun,
        Some(ActionStatus::WillSkip) => RunGroup::Skipped,
        Some(ActionStatus::WillRun | ActionStatus::Executed) => RunGroup::Ran,
        Some(_) => RunGroup::Changed,
        None => RunGroup::Pending,
    }
}

pub(in crate::tui) fn run_group_header_line(
    group: RunGroup,
    count: usize,
    width: usize,
) -> Line<'static> {
    let icon_set = icons::current();
    let (icon, label, style) = match group {
        RunGroup::Failed => (
            icon_set.failed,
            "Failed",
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
        ),
        RunGroup::Aborted => (
            icon_set.warning,
            "Aborted",
            Style::default().fg(CATPPUCCIN_MOCHA.warning),
        ),
        RunGroup::Running => (
            icon_set.running,
            "Running",
            Style::default().fg(CATPPUCCIN_MOCHA.running),
        ),
        RunGroup::Ran => (
            icon_set.success,
            "Ran",
            Style::default().fg(CATPPUCCIN_MOCHA.success),
        ),
        RunGroup::Changed => (
            icon_set.success,
            "Changed",
            Style::default().fg(CATPPUCCIN_MOCHA.success),
        ),
        RunGroup::NoChange => (
            icon_set.info,
            "No Change",
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ),
        RunGroup::NotRun => (
            icon_set.skipped,
            "Not Run",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        RunGroup::Skipped => (
            icon_set.skipped,
            "Skipped",
            Style::default().fg(CATPPUCCIN_MOCHA.skip),
        ),
        RunGroup::Pending => (
            icon_set.pending,
            "Pending",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    };
    Line::from(Span::styled(
        fit_to_width(&format!("{icon} {label} ({count})"), width),
        style.add_modifier(Modifier::BOLD),
    ))
}

pub(in crate::tui) fn finished_action_line(
    item: &RunItem,
    action: &RunAction,
    width: usize,
) -> Line<'static> {
    run_action_line(RunActionLineSpec {
        kind: &action.kind,
        item_name: &item.name,
        action_name: &action.name,
        status_label: run_status_label_for_view(Some(action.status), false),
        status: Some(action.status),
        active: false,
        width,
        spinner_frame: 0,
    })
}

pub(in crate::tui) struct RunActionLineSpec<'a> {
    pub(in crate::tui) kind: &'a str,
    pub(in crate::tui) item_name: &'a str,
    pub(in crate::tui) action_name: &'a str,
    pub(in crate::tui) status_label: &'static str,
    pub(in crate::tui) status: Option<ActionStatus>,
    pub(in crate::tui) active: bool,
    pub(in crate::tui) width: usize,
    pub(in crate::tui) spinner_frame: usize,
}

pub(in crate::tui) fn run_action_line(spec: RunActionLineSpec<'_>) -> Line<'static> {
    let status_width = 10;
    let left_width = spec.width.saturating_sub(status_width + 3);
    let name = if spec.action_name.is_empty() || spec.action_name == spec.item_name {
        spec.item_name.to_string()
    } else {
        format!("{} / {}", spec.item_name, spec.action_name)
    };
    let icon = if spec.active {
        Span::styled(
            icons::SPINNER_BRAILLE[spec.spinner_frame % icons::SPINNER_BRAILLE.len()],
            Style::default().fg(CATPPUCCIN_MOCHA.running),
        )
    } else {
        run_status_icon(spec.status)
    };
    let status_style = run_status_style(spec.status, spec.active);
    Line::from(vec![
        icon,
        Span::raw(" "),
        Span::styled(
            run_action_kind_icon(spec.kind),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::raw(" "),
        Span::styled(
            fit_to_width(&name, left_width),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
        Span::raw(" "),
        Span::styled(
            fit_to_width(spec.status_label, status_width),
            status_style.add_modifier(if spec.active {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ])
}

pub(in crate::tui) fn run_status_label_for_view(
    status: Option<ActionStatus>,
    active: bool,
) -> &'static str {
    if active {
        "running"
    } else {
        match status {
            Some(status) => run_item_status_label(status),
            None => "pending",
        }
    }
}

pub(in crate::tui) fn run_status_icon(status: Option<ActionStatus>) -> Span<'static> {
    let icon_set = icons::current();
    match status {
        Some(ActionStatus::WillFail) => Span::styled(
            icon_set.failed,
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
        ),
        Some(ActionStatus::Aborted) => Span::styled(
            icon_set.warning,
            Style::default().fg(CATPPUCCIN_MOCHA.warning),
        ),
        Some(ActionStatus::WillSkip) => Span::styled(
            icon_set.skipped,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Some(ActionStatus::NotRun) => Span::styled(
            icon_set.skipped,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Some(ActionStatus::NoChange) => Span::styled(
            icon_set.info,
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ),
        Some(_) => Span::styled(
            icon_set.success,
            Style::default().fg(CATPPUCCIN_MOCHA.success),
        ),
        None => Span::styled(
            icon_set.pending,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    }
}

pub(in crate::tui) fn run_status_style(status: Option<ActionStatus>, active: bool) -> Style {
    Style::default().fg(if active {
        CATPPUCCIN_MOCHA.running
    } else {
        match status {
            Some(ActionStatus::WillFail) => CATPPUCCIN_MOCHA.danger,
            Some(ActionStatus::Aborted) => CATPPUCCIN_MOCHA.warning,
            Some(ActionStatus::WillSkip) => CATPPUCCIN_MOCHA.fg_dim,
            Some(ActionStatus::NotRun) => CATPPUCCIN_MOCHA.fg_dim,
            Some(ActionStatus::NoChange) => CATPPUCCIN_MOCHA.text_muted,
            Some(_) => CATPPUCCIN_MOCHA.success,
            None => CATPPUCCIN_MOCHA.fg_dim,
        }
    })
}

pub(in crate::tui) fn action_kind_for_view(action: &Action) -> &'static str {
    match action {
        Action::Install { .. } => "install",
        Action::Link { .. } => "link",
        Action::Create { .. } => "create",
        Action::Shell { .. } => "shell",
        Action::Clean { .. } => "clean",
    }
}

pub(in crate::tui) fn run_action_kind_icon(kind: &str) -> &'static str {
    let icon_set = icons::current();
    match kind {
        "install" => icon_set.action_install,
        "link" => icon_set.action_link,
        "create" => icon_set.action_create,
        "clean" => icon_set.action_clean,
        "shell" => icon_set.action_shell,
        _ => icon_set.info,
    }
}

pub(in crate::tui) fn run_item_status_label(status: ActionStatus) -> &'static str {
    status.result_label()
}
