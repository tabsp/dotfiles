use super::*;

const REPLAY_PAGE_SIZE: usize = 8;
const REPLAY_OUTPUT_PREVIEW_LINES: usize = 12;

// ---------------- HistoryView ----------------

pub(super) fn handle_history(app: &mut App, key: KeyCode) -> Result<()> {
    if key != KeyCode::Char('d') && app.history_delete_pending.take().is_some() {
        app.status_message.clear();
    }
    match key {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('h') | KeyCode::Left => {
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
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => open_selected_history_run(app),
        KeyCode::Char('d') => {
            if let Some(idx) = app.history_state.selected()
                && let Some(run) = app.runs.get(idx)
            {
                let id = run.id.clone();
                if app.history_delete_pending.as_deref() != Some(&id) {
                    app.history_delete_pending = Some(id.clone());
                    app.status_message =
                        format!("delete run {}? press d again to confirm", short_run_id(&id));
                    app.status_kind = NoticeKind::Warning;
                    return Ok(());
                }
                app.history_delete_pending = None;
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

fn open_selected_history_run(app: &mut App) {
    let Some(run) = app
        .history_state
        .selected()
        .and_then(|index| app.runs.get(index))
        .cloned()
    else {
        return;
    };
    app.replay_state
        .select((!run.items.is_empty()).then_some(0));
    app.run = Some(run);
    app.replay_scroll = 0;
    app.replay_expanded.clear();
    app.screen = Screen::RunReplay;
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
    let footer_size = footer_height(&app.status_message);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(footer_size),
        ])
        .split(area);

    let meta = vec![Span::styled(
        format!("{} runs", app.runs.len()),
        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
    )];
    f.render_widget(
        Paragraph::new(screen_header(
            icon_set.app,
            "History",
            meta,
            usize::from(chunks[0].width),
        )),
        chunks[0],
    );

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
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = app
            .runs
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let selected = app.history_state.selected() == Some(idx);
                let status = r.status.result_label();
                let mode = plan_mode_label(r.mode);
                history_run_line(r, mode, status, selected, usize::from(chunks[1].width))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(Style::default())
            .highlight_symbol("");
        f.render_stateful_widget(list, chunks[1], &mut app.history_state);
    }

    if let Some(run) = app
        .history_state
        .selected()
        .and_then(|index| app.runs.get(index))
    {
        let detail = format!(
            "  {} · {} · {}",
            short_datetime(&run.started_at),
            short_run_id(&run.id),
            run::final_run_summary(run)
        );
        f.render_widget(
            Paragraph::new(Span::styled(
                fit_to_width(&detail, usize::from(chunks[2].width)),
                Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
            )),
            chunks[2],
        );
    }

    let notice = (!app.status_message.is_empty()).then(|| FooterNotice {
        message: &app.status_message,
        kind: app.status_kind,
    });
    let footer = screen_footer(
        notice,
        vim_prefix_help(
            app,
            history_help_line(usize::from(chunks[3].width)),
            usize::from(chunks[3].width),
        ),
        usize::from(chunks[3].width),
    );
    f.render_widget(footer, chunks[3]);
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
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("dotman.yaml");
    let date = relative_datetime(&run.started_at);
    let id = short_run_id(&run.id);
    let content_width = width.saturating_sub(display_width(marker));
    // The run id is the first thing to go when the terminal gets narrow; the
    // date, mode, and config name carry more meaning in the list.
    let id_suffix = format!("  {id}");
    let with_id = content_width >= 64;
    let right_width = display_width(status)
        + if with_id {
            display_width(&id_suffix)
        } else {
            0
        };
    let left = if content_width >= 56 {
        format!("{date:<12}  {mode:<7}  {config_label}")
    } else {
        format!("{date}  {mode}  {config_label}")
    };
    let left_width = content_width.saturating_sub(right_width + 2);
    let gap = content_width
        .saturating_sub(left_width + right_width)
        .max(1);
    let mut spans = vec![
        Span::styled(marker.to_string(), prefix_style),
        Span::styled(fit_to_width(&left, left_width), text_style),
        Span::styled(" ".repeat(gap), span_bg_style(bg)),
        Span::styled(
            status.to_string(),
            status_style.add_modifier(Modifier::BOLD),
        ),
    ];
    if with_id {
        spans.push(Span::styled(id_suffix, muted_style));
    }
    ListItem::new(Line::from(spans))
}

fn history_help_line(width: usize) -> Line<'static> {
    let full = [
        ("↑↓", " Navigate  "),
        ("l", " Open  "),
        ("dd", " Delete  "),
        ("h", " Back"),
    ];
    let compact = [("↑↓", " "), ("l", " "), ("dd", " "), ("h", "")];
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
        KeyCode::Left | KeyCode::Char('h') => {
            if let Some(key) = selected_replay_key(app)
                && app.replay_expanded.remove(&key)
            {
                return Ok(());
            }
            app.screen = Screen::HistoryView;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if let Some(key) = selected_replay_key(app) {
                app.replay_expanded.insert(key);
            }
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
            Constraint::Length(footer_height(&app.status_message)),
        ])
        .split(area);

    let meta = vec![Span::styled(
        plan_mode_label(run.mode),
        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
    )];
    f.render_widget(
        Paragraph::new(screen_header(
            icon_set.app,
            "Replay",
            meta,
            usize::from(chunks[0].width),
        )),
        chunks[0],
    );

    let summary = format!(
        "  {}  {}  {}",
        short_run_id(&run.id),
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

    f.render_widget(Paragraph::new(lines), chunks[2]);

    let notice = (!app.status_message.is_empty()).then(|| FooterNotice {
        message: &app.status_message,
        kind: app.status_kind,
    });
    let footer = screen_footer(
        notice,
        vim_prefix_help(
            app,
            replay_help_line(usize::from(chunks[3].width)),
            usize::from(chunks[3].width),
        ),
        usize::from(chunks[3].width),
    );
    f.render_widget(footer, chunks[3]);
}

pub(super) fn replay_help_line(width: usize) -> Line<'static> {
    let full = [
        ("↑↓", " Navigate  "),
        ("h/l", " Fold  "),
        ("Space", " Toggle  "),
        ("q", " Back"),
    ];
    let compact = [("↑↓", " "), ("h/l", " "), ("q", "")];
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
    let icon_set = icons::current();
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
        let prefix = if expanded {
            icon_set.expanded
        } else {
            icon_set.collapsed
        };
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
            let guide = || Span::styled("    │ ", Style::default().fg(CATPPUCCIN_MOCHA.guide));
            let detail_width = width.saturating_sub(6);
            if let Some(error) = &entry.action.error {
                lines.push(Line::from(vec![
                    guide(),
                    Span::styled(
                        fit_to_width(&format!("error: {error}"), detail_width),
                        Style::default().fg(CATPPUCCIN_MOCHA.danger),
                    ),
                ]));
            }
            let output = &entry.action.output;
            if output.is_empty() {
                lines.push(Line::from(vec![
                    guide(),
                    Span::styled(
                        "no saved output",
                        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
                    ),
                ]));
            } else {
                for line in output.iter().take(REPLAY_OUTPUT_PREVIEW_LINES) {
                    lines.push(Line::from(vec![
                        guide(),
                        Span::styled(
                            fit_to_width(&line.line, detail_width),
                            Style::default().fg(match line.stream {
                                OutputStream::Stderr => CATPPUCCIN_MOCHA.warning,
                                OutputStream::Stdout => CATPPUCCIN_MOCHA.fg_dim,
                                OutputStream::Action => CATPPUCCIN_MOCHA.primary,
                            }),
                        ),
                    ]));
                }
                if output.len() > REPLAY_OUTPUT_PREVIEW_LINES {
                    lines.push(Line::from(vec![
                        guide(),
                        Span::styled(
                            format!(
                                "... {} more saved output lines",
                                output.len() - REPLAY_OUTPUT_PREVIEW_LINES
                            ),
                            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
                        ),
                    ]));
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
