use super::*;

// ---------------- RunView ----------------

pub(super) fn handle_run(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
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
    app.current_action = None;
    app.run_started = Some(Instant::now());
    app.current_log.clear();
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
    let handle = std::thread::spawn(move || -> anyhow::Result<Run> {
        let result = crate::execute::execute_with_events_and_sudo(
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
        )?;
        let _ = crate::store::save(&result)?;
        Ok(result)
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
            Ok(Ok(run)) => {
                app.run = Some(run.clone());
                sync_finished_run_state(app, &run);
                app.abort_flag = None;
                app.run_events = None;
            }
            Ok(Err(e)) => {
                app.status_message = format!("run failed: {e}");
                app.abort_flag = None;
                app.run_events = None;
            }
            Err(_) => {
                app.status_message = "run thread panicked".into();
                app.abort_flag = None;
                app.run_events = None;
            }
        }
    }

    let aborting = run_is_aborting(app);
    let finished = finished_run_for_view(app).is_some();
    let border_color = run_border_color(app, aborting);
    let block = Block::default()
        .title(run_title(app, usize::from(area.width.saturating_sub(4))))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(run_log_panel_height(area.height)),
            Constraint::Length(1),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(run_status_line(app, usize::from(chunks[0].width))),
        chunks[0],
    );

    let step_lines = run_body_lines(app, usize::from(chunks[1].width), chunks[1].height as usize);
    f.render_widget(
        Paragraph::new(step_lines).block(Block::default().borders(Borders::NONE)),
        chunks[1],
    );

    // Log.
    let log_lines: Vec<Line> = app
        .current_log
        .iter()
        .map(|entry| {
            let style = entry.fg.map(|c| Style::default().fg(c)).unwrap_or_default();
            Line::styled(entry.text.clone(), style)
        })
        .collect();
    let log_height = chunks[2].height.saturating_sub(2) as usize;
    let log_scroll = app
        .current_log
        .len()
        .saturating_sub(log_height)
        .min(u16::MAX as usize) as u16;
    f.render_widget(
        Paragraph::new(log_lines)
            .block(
                Block::default()
                    .title(" log ")
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            )
            .scroll((log_scroll, 0)),
        chunks[2],
    );

    let help = Paragraph::new(run_help_line(
        usize::from(chunks[3].width),
        aborting,
        finished,
    ));
    f.render_widget(help, chunks[3]);
}

pub(super) fn sync_finished_run_state(app: &mut App, run: &Run) {
    let total = run_action_total(run);
    app.progress = (total, total);
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
    if app.run_thread.is_none() {
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

pub(super) fn run_title(app: &App, width: usize) -> String {
    let icon_set = icons::current();
    let (state, done, total) = if let Some(run) = finished_run_for_view(app) {
        let total = run_action_total(run);
        (run_status_label(run.status), total, total)
    } else if run_is_aborting(app) {
        ("Stopping", app.progress.0, app.progress.1)
    } else {
        ("Running", app.progress.0, app.progress.1)
    };
    let title = format!(
        " {} dotman - {:?}  {state}  {}/{}  {} ",
        icon_set.running,
        app.mode,
        done,
        total,
        run_progress_bar(done, total, 10),
    );
    fit_to_width(&title, width)
}

pub(super) fn run_status_line(app: &App, width: usize) -> Line<'static> {
    let text = if let Some(run) = finished_run_for_view(app) {
        final_run_summary(run)
    } else if let Some((item_idx, action_idx)) = app.current_action {
        current_run_action_name(app, item_idx, action_idx).unwrap_or_else(|| {
            current_run_item_name(app)
                .map(str::to_string)
                .unwrap_or_else(|| "waiting".into())
        })
    } else {
        current_run_item_name(app)
            .map(str::to_string)
            .unwrap_or_else(|| "waiting".into())
    };
    Line::from(vec![
        Span::styled("  current  ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(
            fit_to_width(&text, width.saturating_sub(11)),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
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

pub(super) fn final_run_summary(run: &Run) -> String {
    let mut changed = 0;
    let mut no_change = 0;
    let mut failed = 0;
    for item in &run.items {
        if item.actions.is_empty() {
            match run_group_for_status(Some(item.status), false) {
                RunGroup::Changed => changed += 1,
                RunGroup::NoChange => no_change += 1,
                RunGroup::Failed => failed += 1,
                _ => {}
            }
            continue;
        }
        for action in &item.actions {
            match run_group_for_status(Some(action.status), false) {
                RunGroup::Changed => changed += 1,
                RunGroup::NoChange => no_change += 1,
                RunGroup::Failed => failed += 1,
                _ => {}
            }
        }
    }
    format!("{changed} changed, {no_change} no change, {failed} failed")
}

pub(super) fn current_run_item_name(app: &App) -> Option<&str> {
    let plan = app.plan.as_ref()?;
    let index = app.current_item.or_else(|| app.progress.0.checked_sub(1))?;
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
        .filter(|item| item.started_at.is_some())
        .map(|item| item.actions.len().max(usize::from(item.actions.is_empty())))
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
                line: run_action_line(
                    "shell",
                    &item.name,
                    "",
                    run_status_label_for_view(status, active),
                    status,
                    active,
                    width,
                ),
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
                line: run_action_line(
                    action_kind_for_view(action),
                    &item.name,
                    &action.describe(),
                    run_status_label_for_view(status, active),
                    status,
                    active,
                    width,
                ),
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
                line: run_action_line(
                    "shell",
                    &item.name,
                    "",
                    run_status_label_for_view(status, false),
                    status,
                    false,
                    width,
                ),
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
    run_action_line(
        &action.kind,
        &item.name,
        &action.name,
        run_status_label_for_view(Some(action.status), false),
        Some(action.status),
        false,
        width,
    )
}

pub(super) fn run_action_line(
    kind: &str,
    item_name: &str,
    action_name: &str,
    status_label: &'static str,
    status: Option<ActionStatus>,
    active: bool,
    width: usize,
) -> Line<'static> {
    let status_width = 10;
    let left_width = width.saturating_sub(status_width + 3);
    let name = if action_name.is_empty() || action_name == item_name {
        item_name.to_string()
    } else {
        format!("{item_name} / {action_name}")
    };
    let icon = if active {
        Span::styled(
            icons::SPINNER_BRAILLE[0],
            Style::default().fg(CATPPUCCIN_MOCHA.running),
        )
    } else {
        run_status_icon(status)
    };
    let status_style = run_status_style(status, active);
    Line::from(vec![
        icon,
        Span::raw(" "),
        Span::styled(
            run_action_kind_icon(kind),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::raw(" "),
        Span::styled(
            fit_to_width(&name, left_width),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
        Span::raw(" "),
        Span::styled(
            fit_to_width(status_label, status_width),
            status_style.add_modifier(if active {
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
                app.current_action = None;
                push_log(app, &format!("started {name}"), None);
            }
            crate::execute::ExecuteEvent::ActionStarted {
                item_index,
                action_index,
                item,
                action,
            } => {
                app.current_action = Some((item_index, action_index));
                push_log(app, &format!("{item}: {action}"), None);
            }
            crate::execute::ExecuteEvent::ActionFinished {
                item_index,
                action_index,
                item,
                action,
                status,
            } => {
                if let Some(statuses) = app.run_action_statuses.get_mut(item_index)
                    && let Some(slot) = statuses.get_mut(action_index)
                {
                    *slot = Some(status);
                }
                app.progress.0 = app.progress.0.saturating_add(1).min(app.progress.1);
                app.current_action = None;
                push_log(app, &format!("{item}: finished {action}: {status:?}"), None);
            }
            crate::execute::ExecuteEvent::Output { item, stream, line } => {
                let color = match stream {
                    crate::model::OutputStream::Stderr => Some(CATPPUCCIN_MOCHA.danger),
                    crate::model::OutputStream::Stdout => None,
                    crate::model::OutputStream::Action => Some(CATPPUCCIN_MOCHA.primary),
                };
                push_log(app, &format!("{item}: {line}"), color);
            }
            crate::execute::ExecuteEvent::ActionMessage { item, message } => {
                push_log(
                    app,
                    &format!("{item}: {message}"),
                    Some(CATPPUCCIN_MOCHA.primary),
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
                if let Some(slot) = app.run_item_statuses.get_mut(index) {
                    *slot = Some(status);
                }
                push_log(app, &format!("finished {name}: {status:?}"), None);
            }
            crate::execute::ExecuteEvent::Aborted => {
                push_log(app, "run aborted", Some(CATPPUCCIN_MOCHA.warning));
            }
            crate::execute::ExecuteEvent::SudoPrompt { item, response } => {
                push_log(
                    app,
                    &format!("{item}: sudo session expired; re-authenticating"),
                    Some(CATPPUCCIN_MOCHA.warning),
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
                    push_log(
                        app,
                        &format!("{item}: sudo authentication failed"),
                        Some(CATPPUCCIN_MOCHA.danger),
                    );
                }
            }
        }
    }
    app.run_events = Some(rx);
}

pub(super) fn push_log(app: &mut App, line: &str, fg: Option<Color>) {
    app.current_log.push(LogLine {
        text: sanitize_tui_log_line(line),
        fg,
    });
    // Cap per-step TUI log at MAX_TUI_OUTPUT_LINES (1000).
    if app.current_log.len() > MAX_TUI_OUTPUT_LINES {
        let drop_count = app.current_log.len() - MAX_TUI_OUTPUT_LINES;
        app.current_log.drain(0..drop_count);
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
        ActionStatus::NoChange => "no change",
        ActionStatus::WillRun => "ran",
        _ => "changed",
    }
}
