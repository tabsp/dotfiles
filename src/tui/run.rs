use super::*;

// ---------------- RunView ----------------

pub(super) fn handle_run(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::PageUp => scroll_run_log(app, -1),
        KeyCode::PageDown => scroll_run_log(app, 1),
        KeyCode::Home => {
            app.log_follow = false;
            app.log_scroll = 0;
        }
        KeyCode::End | KeyCode::Char('f') => {
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, 0);
        }
        KeyCode::Tab => {
            app.log_filter = app.log_filter.next();
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, 0);
        }
        KeyCode::Char('c') => {
            app.log_filter = LogFilter::Current;
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, 0);
        }
        KeyCode::Char('e') => {
            app.log_filter = LogFilter::Errors;
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, 0);
        }
        KeyCode::Enter => toggle_current_log_group(app),
        KeyCode::Char('q') | KeyCode::Esc => {
            if let Some(flag) = &app.abort_flag {
                flag.store(true, Ordering::SeqCst);
                app.status_message = "abort requested; waiting for current action".into();
                push_log(app, "abort requested; waiting for current action", None);
            } else if app.plan.is_some() {
                app.screen = Screen::PlanView;
            } else {
                app.screen = Screen::MainMenu;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn start_run(app: &mut App) {
    if app.plan.is_none() || app.config.is_none() {
        return;
    }
    let plan = app.plan.clone().unwrap();
    let cfg = app.config.clone().unwrap();
    let total = selected_run_action_total(&plan);
    app.progress = (0, total);
    app.current_item = None;
    app.last_item_index = None;
    app.current_action = None;
    app.run_started = Some(Instant::now());
    app.current_log.clear();
    app.log_scroll = 0;
    app.log_follow = true;
    app.log_dropped_count = 0;
    app.log_group = None;
    app.active_log_group = None;
    app.log_filter = LogFilter::All;
    app.collapsed_log_groups.clear();
    app.run_error = None;
    app.run_save_warning = None;
    app.run = None;
    app.run_item_statuses = vec![None; plan.items.len()];
    app.run_action_statuses = plan
        .items
        .iter()
        .map(|item| vec![None; item.actions.len()])
        .collect();
    app.screen = Screen::RunView;

    let (tx, rx) = mpsc::channel();
    let sudo_tx = tx.clone();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let thread_abort_flag = Arc::clone(&abort_flag);
    let handle = std::thread::spawn(move || -> RunThreadResult {
        let result = match crate::execute::execute_with_events_and_sudo(
            &plan,
            &cfg,
            |event| {
                let _ = tx.send(event);
            },
            || thread_abort_flag.load(Ordering::SeqCst),
            |item| {
                let (response_tx, response_rx) = mpsc::channel();
                let _ = sudo_tx.send(crate::execute::ExecuteEvent::SudoPrompt {
                    item: item.to_string(),
                    response: response_tx,
                });
                response_rx.recv().unwrap_or(false)
            },
        ) {
            Ok(run) => run,
            Err(error) => {
                return RunThreadResult {
                    run: None,
                    error: Some(error.to_string()),
                    save_warning: None,
                };
            }
        };
        let save_warning = crate::store::save(&result)
            .err()
            .map(|error| format!("history save failed: {error}"));
        RunThreadResult {
            run: Some(result),
            error: None,
            save_warning,
        }
    });
    app.run_thread = Some(handle);
    app.run_events = Some(rx);
    app.abort_flag = Some(abort_flag);
}

pub(super) fn render_run(f: &mut Frame, app: &mut App) {
    let area = f.area();
    drain_run_events(app);

    // Try to join the run thread (non-blocking).
    if let Some(handle) = &app.run_thread
        && handle.is_finished()
    {
        let handle = app.run_thread.take().unwrap();
        match handle.join() {
            Ok(result) => {
                apply_run_thread_result(app, result);
                app.abort_flag = None;
                app.run_events = None;
            }
            Err(_) => {
                app.run_error = Some("run thread panicked".into());
                push_log(app, "run thread panicked", Some(CATPPUCCIN_MOCHA.danger));
                app.abort_flag = None;
                app.run_events = None;
            }
        }
    }

    let aborting = run_is_aborting(app);
    let finished = finished_run_for_view(app).is_some();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(run_log_panel_height(area.height)),
            Constraint::Length(1),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(run_title_line(app, usize::from(chunks[0].width))),
        chunks[0],
    );

    f.render_widget(
        Paragraph::new(run_status_line(app, usize::from(chunks[1].width))),
        chunks[1],
    );

    let step_lines = run_body_lines(app, usize::from(chunks[2].width), chunks[2].height as usize);
    f.render_widget(
        Paragraph::new(step_lines).block(Block::default().borders(Borders::NONE)),
        chunks[2],
    );

    let log_lines = visible_log_lines(app, chunks[3].height.saturating_sub(2) as usize);
    let log_scroll = log_scroll_offset(app, chunks[3].height.saturating_sub(2) as usize);
    f.render_widget(
        Paragraph::new(log_lines)
            .block(
                Block::default()
                    .title(run_log_title(app))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Plain),
            )
            .scroll((log_scroll, 0)),
        chunks[3],
    );

    let help = Paragraph::new(run_help_line(
        usize::from(chunks[4].width),
        aborting,
        finished,
        app.log_follow,
    ));
    f.render_widget(help, chunks[4]);
}

pub(super) fn apply_run_thread_result(app: &mut App, result: RunThreadResult) {
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
        push_log(app, &message, Some(CATPPUCCIN_MOCHA.danger));
    }
}

pub(super) fn sync_finished_run_state(app: &mut App, run: &Run) {
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

pub(super) fn finished_run_for_view(app: &App) -> Option<&Run> {
    if app.run_thread.is_none() && app.run_error.is_none() {
        app.run.as_ref()
    } else {
        None
    }
}

pub(super) fn run_is_aborting(app: &App) -> bool {
    app.abort_flag
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::SeqCst))
}

pub(super) fn run_border_color(app: &App, aborting: bool) -> Color {
    if aborting {
        CATPPUCCIN_MOCHA.warning
    } else if app.run_error.is_some() {
        CATPPUCCIN_MOCHA.danger
    } else if let Some(run) = finished_run_for_view(app) {
        match run.status {
            RunStatus::Running => CATPPUCCIN_MOCHA.running,
            RunStatus::Success => CATPPUCCIN_MOCHA.success,
            RunStatus::Failed => CATPPUCCIN_MOCHA.danger,
            RunStatus::Aborted => CATPPUCCIN_MOCHA.warning,
        }
    } else {
        CATPPUCCIN_MOCHA.running
    }
}

#[cfg(test)]
pub(super) fn run_title(app: &App, width: usize) -> String {
    line_to_plain_string(&run_title_line(app, width))
}

pub(super) fn run_title_line(app: &App, width: usize) -> Line<'static> {
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

pub(super) fn run_status_line(app: &App, width: usize) -> Line<'static> {
    let (label, text, style) = if let Some(error) = &app.run_error {
        (
            "  error    ",
            error.clone(),
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
        )
    } else if let Some(warning) = &app.run_save_warning {
        (
            "  warning  ",
            warning.clone(),
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

pub(super) fn run_status_label(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Running => "Running",
        RunStatus::Success => "Success",
        RunStatus::Failed => "Failed",
        RunStatus::Aborted => "Aborted",
    }
}

#[cfg(test)]
pub(super) fn line_to_plain_string(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("")
}

pub(super) fn final_run_summary(run: &Run) -> String {
    let mut changed = 0;
    let mut no_change = 0;
    let mut failed = 0;
    let mut not_run = 0;
    for item in &run.items {
        if item.actions.is_empty() {
            match run_group_for_status(Some(item.status), false) {
                RunGroup::Changed => changed += 1,
                RunGroup::NoChange => no_change += 1,
                RunGroup::Failed => failed += 1,
                RunGroup::NotRun => not_run += 1,
                _ => {}
            }
            continue;
        }
        for action in &item.actions {
            match run_group_for_status(Some(action.status), false) {
                RunGroup::Changed => changed += 1,
                RunGroup::NoChange => no_change += 1,
                RunGroup::Failed => failed += 1,
                RunGroup::NotRun => not_run += 1,
                _ => {}
            }
        }
    }
    if not_run > 0 {
        format!("{changed} changed, {no_change} no change, {failed} failed, {not_run} not run")
    } else {
        format!("{changed} changed, {no_change} no change, {failed} failed")
    }
}

pub(super) fn current_run_item_name(app: &App) -> Option<&str> {
    let plan = app.plan.as_ref()?;
    let index = app.current_item.or(app.last_item_index)?;
    plan.items.get(index).map(|item| item.name.as_str())
}

pub(super) fn current_run_action_name(
    app: &App,
    item_index: usize,
    action_index: usize,
) -> Option<String> {
    let plan = app.plan.as_ref()?;
    let item = plan.items.get(item_index)?;
    let action = item.actions.get(action_index)?;
    Some(format!("{} / {}", item.name, action.describe()))
}

pub(super) fn selected_run_action_total(plan: &Plan) -> usize {
    plan.items
        .iter()
        .filter(|item| item.selected)
        .map(|item| item.actions.len().max(usize::from(item.actions.is_empty())))
        .sum()
}

pub(super) fn run_action_total(run: &Run) -> usize {
    run.items
        .iter()
        .filter(|item| item.error.as_deref() != Some("skipped (not selected)"))
        .map(|item| item.actions.len().max(usize::from(item.actions.is_empty())))
        .sum()
}

pub(super) fn run_executed_action_total(run: &Run) -> usize {
    run.items
        .iter()
        .filter(|item| item.error.as_deref() != Some("skipped (not selected)"))
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

pub(super) fn run_progress_bar(done: usize, total: usize, width: usize) -> String {
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

pub(super) fn run_log_panel_height(total_height: u16) -> u16 {
    let available = total_height.saturating_sub(2);
    let max_log = available.saturating_sub(4);
    let desired = if available >= 24 { 10 } else { 7 };
    desired.min(max_log)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RunGroup {
    Failed,
    Running,
    Changed,
    NoChange,
    NotRun,
    Skipped,
    Pending,
}

pub(super) struct RunDisplayLine {
    pub(super) group: RunGroup,
    pub(super) line: Line<'static>,
    pub(super) active: bool,
}

pub(super) fn run_body_lines(app: &App, width: usize, height: usize) -> Vec<Line<'static>> {
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

pub(super) fn live_run_display_lines(app: &App, plan: &Plan, width: usize) -> Vec<RunDisplayLine> {
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

pub(super) fn finished_run_display_lines(run: &Run, width: usize) -> Vec<RunDisplayLine> {
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

pub(super) fn grouped_run_lines(
    display_lines: Vec<RunDisplayLine>,
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let mut all_lines = Vec::new();
    let mut active_line = None;
    for group in [
        RunGroup::Failed,
        RunGroup::Running,
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

pub(super) fn ordered_run_lines(
    display_lines: Vec<RunDisplayLine>,
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let all_lines = display_lines
        .into_iter()
        .map(|display_line| display_line.line)
        .collect::<Vec<_>>();
    if all_lines.len() <= height {
        return all_lines;
    }
    let focus = all_lines
        .iter()
        .position(|line| line_text_contains(line, "running"))
        .unwrap_or_else(|| all_lines.len().saturating_sub(1));
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

pub(super) fn line_text_contains(line: &Line<'_>, needle: &str) -> bool {
    line.spans
        .iter()
        .any(|span| span.content.as_ref().contains(needle))
}

pub(super) fn run_group_for_status(status: Option<ActionStatus>, active: bool) -> RunGroup {
    if active {
        return RunGroup::Running;
    }
    match status {
        Some(ActionStatus::WillFail) => RunGroup::Failed,
        Some(ActionStatus::NoChange) => RunGroup::NoChange,
        Some(ActionStatus::NotRun) => RunGroup::NotRun,
        Some(ActionStatus::WillSkip) => RunGroup::Skipped,
        Some(_) => RunGroup::Changed,
        None => RunGroup::Pending,
    }
}

pub(super) fn run_group_header_line(group: RunGroup, count: usize, width: usize) -> Line<'static> {
    let icon_set = icons::current();
    let (icon, label, style) = match group {
        RunGroup::Failed => (
            icon_set.failed,
            "Failed",
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
        ),
        RunGroup::Running => (
            icon_set.running,
            "Running",
            Style::default().fg(CATPPUCCIN_MOCHA.running),
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

pub(super) fn finished_action_line(
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

pub(super) struct RunActionLineSpec<'a> {
    pub(super) kind: &'a str,
    pub(super) item_name: &'a str,
    pub(super) action_name: &'a str,
    pub(super) status_label: &'static str,
    pub(super) status: Option<ActionStatus>,
    pub(super) active: bool,
    pub(super) width: usize,
    pub(super) spinner_frame: usize,
}

pub(super) fn run_action_line(spec: RunActionLineSpec<'_>) -> Line<'static> {
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

pub(super) fn run_status_label_for_view(
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

pub(super) fn run_status_icon(status: Option<ActionStatus>) -> Span<'static> {
    let icon_set = icons::current();
    match status {
        Some(ActionStatus::WillFail) => Span::styled(
            icon_set.failed,
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
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

pub(super) fn run_status_style(status: Option<ActionStatus>, active: bool) -> Style {
    Style::default().fg(if active {
        CATPPUCCIN_MOCHA.running
    } else {
        match status {
            Some(ActionStatus::WillFail) => CATPPUCCIN_MOCHA.danger,
            Some(ActionStatus::WillSkip) => CATPPUCCIN_MOCHA.fg_dim,
            Some(ActionStatus::NotRun) => CATPPUCCIN_MOCHA.fg_dim,
            Some(ActionStatus::NoChange) => CATPPUCCIN_MOCHA.text_muted,
            Some(_) => CATPPUCCIN_MOCHA.success,
            None => CATPPUCCIN_MOCHA.fg_dim,
        }
    })
}

pub(super) fn action_kind_for_view(action: &Action) -> &'static str {
    match action {
        Action::Install { .. } => "install",
        Action::Link { .. } => "link",
        Action::Create { .. } => "create",
        Action::Shell { .. } => "shell",
        Action::Clean { .. } => "clean",
    }
}

pub(super) fn run_action_kind_icon(kind: &str) -> &'static str {
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

pub(super) fn drain_run_events(app: &mut App) {
    let Some(rx) = app.run_events.take() else {
        return;
    };
    while let Ok(event) = rx.try_recv() {
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
                    &format!("finished {action}: {status:?}"),
                    None,
                    1,
                    if status == ActionStatus::WillFail {
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
                    &format!("finished: {status:?}"),
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
}

pub(super) fn log_group_for_event(app: &App, item: &str) -> String {
    app.active_log_group
        .clone()
        .unwrap_or_else(|| item.to_string())
}

pub(super) fn push_log(app: &mut App, line: &str, fg: Option<Color>) {
    push_log_indented(app, line, fg, 0, LogKind::System);
}

pub(super) fn push_log_group(app: &mut App, group: &str) {
    let sanitized = sanitize_tui_log_line(group);
    if app.log_group.as_deref() == Some(sanitized.as_str()) {
        return;
    }
    app.log_group = Some(sanitized.clone());
    push_log_indented(
        app,
        &sanitized,
        Some(CATPPUCCIN_MOCHA.primary),
        0,
        LogKind::Header,
    );
}

pub(super) fn push_log_indented(
    app: &mut App,
    line: &str,
    fg: Option<Color>,
    indent: usize,
    kind: LogKind,
) {
    app.current_log.push(LogLine {
        text: sanitize_tui_log_line(line),
        fg,
        indent,
        group: app.log_group.clone(),
        kind,
    });
    // Cap per-step TUI log at MAX_TUI_OUTPUT_LINES (1000).
    if app.current_log.len() > MAX_TUI_OUTPUT_LINES {
        let drop_count = app.current_log.len() - MAX_TUI_OUTPUT_LINES;
        app.current_log.drain(0..drop_count);
        app.log_dropped_count = app.log_dropped_count.saturating_add(drop_count);
        app.log_scroll = app.log_scroll.saturating_sub(drop_count);
    }
    if app.log_follow {
        app.log_scroll = log_bottom_scroll(app, 0);
    }
}

pub(super) fn scroll_run_log(app: &mut App, pages: isize) {
    let step = 8usize;
    if app.log_follow {
        app.log_scroll = log_bottom_scroll(app, 0);
    }
    app.log_follow = false;
    if pages.is_negative() {
        app.log_scroll = app
            .log_scroll
            .saturating_sub(step.saturating_mul(pages.unsigned_abs()));
    } else {
        app.log_scroll = app
            .log_scroll
            .saturating_add(step.saturating_mul(pages as usize));
    }
    clamp_log_scroll(app, 0);
}

pub(super) fn log_scroll_offset(app: &App, viewport_height: usize) -> u16 {
    if app.log_follow {
        return log_bottom_scroll(app, viewport_height).min(u16::MAX as usize) as u16;
    }
    app.log_scroll
        .min(log_bottom_scroll(app, viewport_height))
        .min(u16::MAX as usize) as u16
}

pub(super) fn log_bottom_scroll(app: &App, viewport_height: usize) -> usize {
    visible_log_len(app).saturating_sub(viewport_height)
}

pub(super) fn visible_log_len(app: &App) -> usize {
    let base = if app.current_log.is_empty() {
        1
    } else {
        filtered_log_entries(app).len().max(1)
    };
    base + usize::from(app.log_dropped_count > 0)
}

pub(super) fn clamp_log_scroll(app: &mut App, viewport_height: usize) {
    app.log_scroll = app.log_scroll.min(log_bottom_scroll(app, viewport_height));
}

pub(super) fn visible_log_lines(app: &App, _viewport_height: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if app.log_dropped_count > 0 {
        lines.push(Line::from(Span::styled(
            format!("... {} earlier log lines truncated", app.log_dropped_count),
            Style::default().fg(CATPPUCCIN_MOCHA.warning),
        )));
    }
    if app.current_log.is_empty() {
        lines.push(Line::from(Span::styled(
            "log is empty; waiting for output",
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        )));
        return lines;
    }
    lines.extend(filtered_log_entries(app).into_iter().map(|entry| {
        if entry.kind == LogKind::Header
            && entry
                .group
                .as_ref()
                .is_some_and(|group| app.collapsed_log_groups.contains(group))
        {
            collapsed_log_header_line(entry)
        } else {
            log_line_for_view(entry)
        }
    }));
    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            log_empty_filter_message(app.log_filter),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        )));
    }
    lines
}

pub(super) fn filtered_log_entries(app: &App) -> Vec<&LogLine> {
    if app.log_filter == LogFilter::Errors {
        return error_log_entries(app);
    }
    let mut visible = Vec::new();
    let current_group = current_log_filter_group(app);
    for entry in &app.current_log {
        if !log_entry_matches_filter(entry, app.log_filter, current_group.as_deref()) {
            continue;
        }
        if entry.kind != LogKind::Header
            && entry
                .group
                .as_ref()
                .is_some_and(|group| app.collapsed_log_groups.contains(group))
        {
            continue;
        }
        visible.push(entry);
    }
    visible
}

pub(super) fn current_log_filter_group(app: &App) -> Option<String> {
    app.active_log_group
        .clone()
        .or_else(|| app.log_group.clone())
}

pub(super) fn error_log_entries(app: &App) -> Vec<&LogLine> {
    let mut error_groups = BTreeSet::new();
    let mut include_ungrouped_error = false;
    for entry in &app.current_log {
        if is_error_log_entry(entry) {
            if let Some(group) = &entry.group {
                error_groups.insert(group.clone());
            } else {
                include_ungrouped_error = true;
            }
        }
    }

    let mut visible = Vec::new();
    for entry in &app.current_log {
        match &entry.group {
            Some(group)
                if error_groups.contains(group)
                    && (entry.kind == LogKind::Header || is_error_log_entry(entry)) =>
            {
                visible.push(entry);
            }
            None if include_ungrouped_error && is_error_log_entry(entry) => visible.push(entry),
            _ => {}
        }
    }
    visible
}

pub(super) fn log_entry_matches_filter(
    entry: &LogLine,
    filter: LogFilter,
    current_group: Option<&str>,
) -> bool {
    match filter {
        LogFilter::All => true,
        LogFilter::Current => entry.group.as_deref() == current_group,
        LogFilter::Errors => is_error_log_entry(entry),
    }
}

pub(super) fn is_error_log_entry(entry: &LogLine) -> bool {
    let text = entry.text.to_ascii_lowercase();
    entry.kind == LogKind::Stderr || text.contains("failed") || text.contains("aborted")
}

pub(super) fn log_line_for_view(entry: &LogLine) -> Line<'static> {
    let style = entry.fg.map(|c| Style::default().fg(c)).unwrap_or_default();
    let indent = "  ".repeat(entry.indent);
    Line::styled(format!("{indent}{}", entry.text), style)
}

pub(super) fn collapsed_log_header_line(entry: &LogLine) -> Line<'static> {
    let style = entry.fg.map(|c| Style::default().fg(c)).unwrap_or_default();
    Line::from(vec![
        Span::styled(entry.text.clone(), style),
        Span::styled(
            "  (collapsed)",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    ])
}

pub(super) fn log_empty_filter_message(filter: LogFilter) -> &'static str {
    match filter {
        LogFilter::All => "log is empty; waiting for output",
        LogFilter::Current => "no log lines for current action",
        LogFilter::Errors => "no error log lines",
    }
}

pub(super) fn toggle_current_log_group(app: &mut App) {
    let Some(group) = app.log_group.clone() else {
        return;
    };
    if !app.collapsed_log_groups.remove(&group) {
        app.collapsed_log_groups.insert(group);
    }
}

pub(super) fn run_log_title(app: &App) -> String {
    let follow = if app.log_follow { "follow" } else { "paused" };
    if app.collapsed_log_groups.is_empty() {
        format!(" log: {follow} · {} ", app.log_filter.label())
    } else {
        format!(
            " log: {follow} · {} · {} collapsed ",
            app.log_filter.label(),
            app.collapsed_log_groups.len()
        )
    }
}

pub(super) fn sanitize_tui_log_line(line: &str) -> String {
    let mut sanitized = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }

        if ch.is_control() {
            if ch == '\t' {
                sanitized.push(' ');
            }
            continue;
        }

        sanitized.push(ch);
    }

    sanitized
}

pub(super) fn run_item_status_label(status: ActionStatus) -> &'static str {
    match status {
        ActionStatus::WillFail => "failed",
        ActionStatus::WillSkip => "skipped",
        ActionStatus::NotRun => "not run",
        ActionStatus::NoChange => "no change",
        ActionStatus::WillRun => "ran",
        _ => "changed",
    }
}
