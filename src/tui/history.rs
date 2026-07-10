use super::*;

// ---------------- HistoryView ----------------

pub(super) fn handle_history(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let next = match app.list_state.selected() {
                Some(i) if i + 1 < app.runs.len() => i + 1,
                Some(_) => app.runs.len().saturating_sub(1),
                None => 0,
            };
            app.list_state.select(Some(next));
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let prev = match app.list_state.selected() {
                Some(0) | None => 0,
                Some(i) => i - 1,
            };
            app.list_state.select(Some(prev));
        }
        KeyCode::Enter => {
            if let Some(idx) = app.list_state.selected()
                && let Some(run) = app.runs.get(idx)
            {
                app.run = Some(run.clone());
                app.screen = Screen::RunReplay;
            }
        }
        KeyCode::Char('d') => {
            if let Some(idx) = app.list_state.selected() {
                let id = app.runs[idx].id.clone();
                if store::delete(&id).is_ok() {
                    app.runs.remove(idx);
                }
            }
        }
        _ => {}
    }
    Ok(())
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

    if app.runs.is_empty() {
        f.render_widget(
            Paragraph::new("no runs yet").alignment(Alignment::Center),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = app
            .runs
            .iter()
            .enumerate()
            .map(|(idx, r)| {
                let selected = app.list_state.selected() == Some(idx);
                let status = format!("{:?}", r.status).to_lowercase();
                let mode = format!("{:?}", r.mode).to_lowercase();
                history_run_line(r, &mode, &status, selected, usize::from(chunks[1].width))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(Style::default())
            .highlight_symbol("");
        f.render_stateful_widget(list, chunks[1], &mut app.list_state);
    }

    let help = Paragraph::new(history_help_line(usize::from(chunks[2].width)));
    f.render_widget(help, chunks[2]);
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
    let left = format!("{}  {}  ", run.started_at, mode);
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

fn run_status_color(status: RunStatus) -> Color {
    match status {
        RunStatus::Running => CATPPUCCIN_MOCHA.running,
        RunStatus::Success => CATPPUCCIN_MOCHA.success,
        RunStatus::Failed => CATPPUCCIN_MOCHA.danger,
        RunStatus::Aborted => CATPPUCCIN_MOCHA.warning,
    }
}

fn history_help_line(width: usize) -> Line<'static> {
    let full = [
        ("↑↓/j/k", " Navigate  "),
        ("Enter", " View  "),
        ("D", " Delete  "),
        ("Q", " Back"),
    ];
    let compact = [("↑↓", " "), ("Enter", " "), ("D", " "), ("Q", "")];
    for parts in [&full[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }
    help_line_from_parts(&[("Q", "")])
}

// ---------------- RunReplay ----------------

pub(super) fn handle_replay(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::HistoryView;
        }
        _ => {}
    }
    Ok(())
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

    let lines = run::grouped_run_lines(
        run::finished_run_display_lines(run, usize::from(chunks[2].width)),
        usize::from(chunks[2].width),
        usize::from(chunks[2].height),
    );

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::NONE)),
        chunks[2],
    );

    let help = Paragraph::new(replay_help_line(usize::from(chunks[3].width)));
    f.render_widget(help, chunks[3]);
}

fn replay_help_line(width: usize) -> Line<'static> {
    let full = [("Q/Esc", " Back")];
    let compact = [("Q", "")];
    for parts in [&full[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }
    help_line_from_parts(&[("Q", "")])
}
