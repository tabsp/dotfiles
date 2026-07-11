use super::*;

const REPLAY_PAGE_SIZE: usize = 8;
const REPLAY_OUTPUT_PREVIEW_LINES: usize = 12;

// ---------------- HistoryView ----------------

pub(super) fn handle_history(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            clamp_menu_selection(app);
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let next = match app.history_state.selected() {
                Some(i) if i + 1 < app.runs.len() => i + 1,
                Some(_) => app.runs.len().saturating_sub(1),
                None => 0,
            };
            app.history_state
                .select((!app.runs.is_empty()).then_some(next));
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let prev = match app.history_state.selected() {
                Some(0) | None => 0,
                Some(i) => i - 1,
            };
            app.history_state
                .select((!app.runs.is_empty()).then_some(prev));
        }
        KeyCode::PageDown => {
            let next = app
                .history_state
                .selected()
                .unwrap_or(0)
                .saturating_add(REPLAY_PAGE_SIZE)
                .min(app.runs.len().saturating_sub(1));
            app.history_state
                .select((!app.runs.is_empty()).then_some(next));
        }
        KeyCode::PageUp => {
            let prev = app
                .history_state
                .selected()
                .unwrap_or(0)
                .saturating_sub(REPLAY_PAGE_SIZE);
            app.history_state
                .select((!app.runs.is_empty()).then_some(prev));
        }
        KeyCode::Enter => {
            if let Some(idx) = app.history_state.selected()
                && let Some(run) = app.runs.get(idx)
            {
                app.run = Some(run.clone());
                app.replay_state
                    .select((!run.items.is_empty()).then_some(0));
                app.replay_scroll = 0;
                app.replay_expanded.clear();
                app.screen = Screen::RunReplay;
            }
        }
        KeyCode::Char('d') => {
            if let Some(idx) = app.history_state.selected()
                && let Some(run) = app.runs.get(idx)
            {
                let id = run.id.clone();
                match store::delete(&id) {
                    Ok(()) => {
                        app.runs.remove(idx);
                        clamp_history_selection(app);
                        app.status_message = format!("deleted run {id}");
                        app.status_kind = NoticeKind::Success;
                    }
                    Err(error) => {
                        app.status_message = format!("failed to delete run {id}: {error}");
                        app.status_kind = NoticeKind::Error;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn jump_history_top(app: &mut App) {
    app.history_state
        .select((!app.runs.is_empty()).then_some(0));
}

pub(super) fn jump_history_bottom(app: &mut App) {
    app.history_state
        .select((!app.runs.is_empty()).then_some(app.runs.len().saturating_sub(1)));
}

pub(super) fn render_history(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    let title_prefix = format!(
        "{}  dotman - History  {} runs ",
        icon_set.app,
        app.runs.len()
    );
    let divider_width = usize::from(chunks[0].width).saturating_sub(display_width(&title_prefix));
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            title_prefix,
            Style::default()
                .fg(CATPPUCCIN_MOCHA.fg_dim)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", "─".repeat(divider_width)),
            Style::default().fg(CATPPUCCIN_MOCHA.border_subtle),
        ),
    ]));
    f.render_widget(title, chunks[0]);

    let content_area = if !app.status_message.is_empty() {
        let line = Paragraph::new(Line::from(Span::styled(
            fit_to_width(&app.status_message, usize::from(chunks[1].width)),
            notice_style(app.status_kind),
        )));
        let banner = Rect::new(chunks[1].x, chunks[1].y, chunks[1].width, 1);
        f.render_widget(line, banner);
        history_content_area(chunks[1], true)
    } else {
        chunks[1]
    };

    if app.runs.is_empty() {
        f.render_widget(
            Paragraph::new(vec![
                Line::from("no runs yet"),
                Line::from(Span::styled(
                    "q back - start deploy from the main menu",
                    Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
                )),
            ])
            .alignment(Alignment::Center),
            content_area,
        );
    } else {
        let items: Vec<ListItem> = app
            .runs
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let selected = app.history_state.selected() == Some(idx);
                let status = r.status.result_label();
                let mode = format!("{:?}", r.mode).to_lowercase();
                history_run_line(r, &mode, status, selected, usize::from(content_area.width))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(Style::default())
            .highlight_symbol("");
        f.render_stateful_widget(list, content_area, &mut app.history_state);
    }

    let help = Paragraph::new(history_help_line(usize::from(chunks[2].width)));
    f.render_widget(help, chunks[2]);
}

pub(super) fn history_content_area(area: Rect, has_notice: bool) -> Rect {
    if !has_notice {
        return area;
    }
    Rect::new(
        area.x,
        area.y.saturating_add(1),
        area.width,
        area.height.saturating_sub(1),
    )
}

pub(super) fn clamp_history_selection(app: &mut App) {
    clamp_list_state(&mut app.history_state, app.runs.len());
}

pub(super) fn clamp_menu_selection(app: &mut App) {
    clamp_list_state(&mut app.menu_state, 4);
}

pub(super) fn clamp_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        *state.offset_mut() = 0;
        return;
    }
    let selected = state.selected().unwrap_or(0).min(len - 1);
    state.select(Some(selected));
}

fn history_run_line(
    run: &Run,
    mode: &str,
    status: &str,
    selected: bool,
    width: usize,
) -> ListItem<'static> {
    let bg = selected.then(focus_bg);
    let marker = if selected { "▎ " } else { "  " };
    let prefix_style = span_bg_style(bg).fg(if selected {
        CATPPUCCIN_MOCHA.focus_marker
    } else {
        CATPPUCCIN_MOCHA.fg_dim
    });
    let status_style = span_bg_style(bg).fg(run_status_color(run.status));
    let text_style = span_bg_style(bg).fg(CATPPUCCIN_MOCHA.fg);
    let muted_style = span_bg_style(bg).fg(CATPPUCCIN_MOCHA.fg_dim);
    let config_label = run
        .config_path
        .as_ref()
        .and_then(|path| path.parent())
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-config");
    let left = format!("{}  {}  {}  ", run.started_at, mode, config_label);
    let right = format!("  {}", run.id);
    let fixed = display_width(marker) + display_width(&left) + display_width(status);
    let id_width = width.saturating_sub(fixed);
    ListItem::new(Line::from(vec![
        Span::styled(marker.to_string(), prefix_style),
        Span::styled(left, text_style),
        Span::styled(
            status.to_string(),
            status_style.add_modifier(Modifier::BOLD),
        ),
        Span::styled(fit_to_width(&right, id_width), muted_style),
    ]))
}

fn history_help_line(width: usize) -> Line<'static> {
    let full = [
        ("↑↓", " Navigate  "),
        ("Enter", " Open  "),
        ("d", " Delete  "),
        ("q", " Back"),
    ];
    let compact = [("↑↓", " "), ("Ent", " "), ("d", " "), ("q", "")];
    for parts in [&full[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }
    help_line_from_parts(&[("q", "")])
}

// ---------------- RunReplay ----------------

pub(super) fn handle_replay(app: &mut App, key: KeyCode) -> Result<()> {
    let len = replay_action_count(app.run.as_ref());
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::HistoryView;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let next = app
                .replay_state
                .selected()
                .unwrap_or(0)
                .saturating_add(1)
                .min(len.saturating_sub(1));
            app.replay_state.select((len > 0).then_some(next));
            app.replay_follow_selection = true;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let prev = app.replay_state.selected().unwrap_or(0).saturating_sub(1);
            app.replay_state.select((len > 0).then_some(prev));
            app.replay_follow_selection = true;
        }
        KeyCode::PageDown => {
            app.replay_scroll = app.replay_scroll.saturating_add(REPLAY_PAGE_SIZE);
        }
        KeyCode::PageUp => {
            app.replay_scroll = app.replay_scroll.saturating_sub(REPLAY_PAGE_SIZE);
        }
        KeyCode::Home => {
            app.replay_scroll = 0;
            app.replay_state.select((len > 0).then_some(0));
        }
        KeyCode::End => {
            app.replay_state
                .select((len > 0).then_some(len.saturating_sub(1)));
            app.replay_scroll = usize::MAX;
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(key) = selected_replay_key(app)
                && !app.replay_expanded.insert(key.clone())
            {
                app.replay_expanded.remove(&key);
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn jump_replay_top(app: &mut App) {
    let len = replay_action_count(app.run.as_ref());
    app.replay_state.select((len > 0).then_some(0));
    app.replay_scroll = 0;
    app.replay_follow_selection = true;
}

pub(super) fn jump_replay_bottom(app: &mut App) {
    let len = replay_action_count(app.run.as_ref());
    app.replay_state
        .select((len > 0).then_some(len.saturating_sub(1)));
    app.replay_scroll = usize::MAX;
    app.replay_follow_selection = true;
}

pub(super) fn render_replay(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let run = match &app.run {
        Some(r) => r,
        None => {
            f.render_widget(
                Paragraph::new("no run loaded").alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    let title_prefix = format!("{}  dotman - Replay  {:?} ", icon_set.app, run.mode);
    let divider_width = usize::from(chunks[0].width).saturating_sub(display_width(&title_prefix));
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            title_prefix,
            Style::default()
                .fg(CATPPUCCIN_MOCHA.fg_dim)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", "─".repeat(divider_width)),
            Style::default().fg(CATPPUCCIN_MOCHA.border_subtle),
        ),
    ]));
    f.render_widget(title, chunks[0]);

    let summary = format!(
        "  {}  {}  {}",
        run.id,
        run::run_status_label(run.status).to_lowercase(),
        run::final_run_summary(run)
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            fit_to_width(&summary, usize::from(chunks[1].width)),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ))),
        chunks[1],
    );

    let mut lines = replay_lines(app, usize::from(chunks[2].width));
    let viewport_height = usize::from(chunks[2].height).max(1);
    if app.replay_follow_selection {
        if let Some(selected_line) = selected_replay_line_index(&lines) {
            if selected_line < app.replay_scroll {
                app.replay_scroll = selected_line;
            } else if selected_line >= app.replay_scroll.saturating_add(viewport_height) {
                app.replay_scroll = selected_line + 1 - viewport_height;
            }
        }
        app.replay_follow_selection = false;
    }
    let max_scroll = lines.len().saturating_sub(viewport_height);
    app.replay_scroll = app.replay_scroll.min(max_scroll);
    lines = lines
        .into_iter()
        .skip(app.replay_scroll)
        .take(usize::from(chunks[2].height))
        .collect();

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::NONE)),
        chunks[2],
    );

    let help = Paragraph::new(replay_help_line(usize::from(chunks[3].width)));
    f.render_widget(help, chunks[3]);
}

pub(super) fn replay_help_line(width: usize) -> Line<'static> {
    let full = [
        ("↑↓", " Navigate  "),
        ("Space", " Toggle  "),
        ("q", " Back"),
    ];
    let compact = [("↑↓", " "), ("Spc", " "), ("q", "")];
    for parts in [&full[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }
    help_line_from_parts(&[("q", "")])
}

fn replay_action_count(run: Option<&Run>) -> usize {
    run.map(|run| run.items.iter().map(|item| item.actions.len().max(1)).sum())
        .unwrap_or(0)
}

fn selected_replay_key(app: &App) -> Option<String> {
    let run = app.run.as_ref()?;
    let selected = app.replay_state.selected()?;
    replay_entries(run)
        .get(selected)
        .map(|entry| replay_key(entry.item_index, entry.action_index))
}

fn replay_key(item_index: usize, action_index: usize) -> String {
    format!("{item_index}:{action_index}")
}

struct ReplayEntry<'a> {
    item_index: usize,
    action_index: usize,
    item: &'a RunItem,
    action: &'a RunAction,
}

fn replay_entries(run: &Run) -> Vec<ReplayEntry<'_>> {
    let mut entries = Vec::new();
    for (item_index, item) in run.items.iter().enumerate() {
        for (action_index, action) in item.actions.iter().enumerate() {
            entries.push(ReplayEntry {
                item_index,
                action_index,
                item,
                action,
            });
        }
    }
    entries
}

pub(super) fn replay_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let Some(run) = app.run.as_ref() else {
        return vec![Line::from("no run loaded")];
    };
    let selected = app.replay_state.selected();
    let mut lines = Vec::new();
    for (idx, entry) in replay_entries(run).iter().enumerate() {
        let selected_row = selected == Some(idx);
        let key = replay_key(entry.item_index, entry.action_index);
        let expanded = app.replay_expanded.contains(&key);
        let status = entry.action.status;
        let name = format!("{} / {}", entry.item.name, entry.action.name);
        let row_bg = selected_row.then(focus_bg);
        let marker = if selected_row { "▎ " } else { "  " };
        let status_style = run::run_status_style(Some(status), false);
        let status_style = row_bg.map_or(status_style, |bg| status_style.bg(bg));
        let prefix = if expanded { "▾" } else { "▸" };
        lines.push(Line::from(vec![
            Span::styled(
                marker,
                span_bg_style(row_bg).fg(if selected_row {
                    CATPPUCCIN_MOCHA.focus_marker
                } else {
                    CATPPUCCIN_MOCHA.fg_dim
                }),
            ),
            Span::styled(prefix, span_bg_style(row_bg).fg(CATPPUCCIN_MOCHA.fg_dim)),
            Span::styled(" ", span_bg_style(row_bg)),
            Span::styled(
                fit_to_width(&name, width.saturating_sub(16)),
                span_bg_style(row_bg).fg(CATPPUCCIN_MOCHA.fg),
            ),
            Span::styled(run::run_item_status_label(status), status_style),
        ]));
        if expanded {
            if let Some(error) = &entry.action.error {
                lines.push(Line::from(Span::styled(
                    format!("    error: {error}"),
                    Style::default().fg(CATPPUCCIN_MOCHA.danger),
                )));
            }
            let output = &entry.action.output;
            if output.is_empty() {
                lines.push(Line::from(Span::styled(
                    "    no saved output",
                    Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
                )));
            } else {
                for line in output.iter().take(REPLAY_OUTPUT_PREVIEW_LINES) {
                    let stream = match line.stream {
                        OutputStream::Stdout => "stdout",
                        OutputStream::Stderr => "stderr",
                        OutputStream::Action => "action",
                    };
                    lines.push(Line::from(Span::styled(
                        fit_to_width(&format!("    {stream}: {}", line.line), width),
                        Style::default().fg(match line.stream {
                            OutputStream::Stderr => CATPPUCCIN_MOCHA.warning,
                            OutputStream::Stdout => CATPPUCCIN_MOCHA.fg_dim,
                            OutputStream::Action => CATPPUCCIN_MOCHA.primary,
                        }),
                    )));
                }
                if output.len() > REPLAY_OUTPUT_PREVIEW_LINES {
                    lines.push(Line::from(Span::styled(
                        format!(
                            "    ... {} more saved output lines",
                            output.len() - REPLAY_OUTPUT_PREVIEW_LINES
                        ),
                        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
                    )));
                }
            }
        }
    }
    lines
}

fn selected_replay_line_index(lines: &[Line<'static>]) -> Option<usize> {
    lines.iter().position(|line| {
        line.spans
            .first()
            .is_some_and(|span| span.content.as_ref() == "▎ ")
    })
}
