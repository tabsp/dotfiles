use super::*;

// ---------------- MainMenu ----------------

pub(super) fn handle_main_menu(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('d') => {
            app.mode = Mode::Deploy;
            if let Err(e) = app.build_plan() {
                app.status_message = e;
            }
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('p') => {
            app.mode = Mode::Plan;
            if let Err(e) = app.build_plan() {
                app.status_message = e;
            }
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('h') => {
            app.runs = store::list().unwrap_or_default();
            app.screen = Screen::HistoryView;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app.list_state.selected().unwrap_or(0);
            if i + 1 < 4 {
                app.list_state.select(Some(i + 1));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app.list_state.selected().unwrap_or(0);
            if i > 0 {
                app.list_state.select(Some(i - 1));
            }
        }
        KeyCode::Enter => match app.list_state.selected() {
            Some(0) => {
                app.mode = Mode::Deploy;
                if let Err(e) = app.build_plan() {
                    app.status_message = e;
                }
                app.screen = Screen::PlanView;
            }
            Some(1) => {
                app.mode = Mode::Plan;
                if let Err(e) = app.build_plan() {
                    app.status_message = e;
                }
                app.screen = Screen::PlanView;
            }
            Some(2) => {
                app.runs = store::list().unwrap_or_default();
                app.screen = Screen::HistoryView;
            }
            Some(3) => app.should_quit = true,
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

pub(super) fn render_main_menu(f: &mut Frame, app: &mut App) {
    fn fmt_date(s: &str) -> String {
        // RFC 3339: "2026-07-05T12:00:00+08:00" → "2026-07-05"
        s.split('T').next().unwrap_or(s).to_string()
    }

    let icon_set = icons::current();
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.fg_dim));
    f.render_widget(block, area);

    let has_run = !app.runs.is_empty();
    let summary_size: u16 = if has_run { 3 } else { 2 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(15),
            Constraint::Length(summary_size),
            Constraint::Length(1),
        ])
        .split(area);

    let title_prefix = format!("{}  dotman - Main Menu ", icon_set.app);
    let divider_width = usize::from(chunks[0].width).saturating_sub(display_width(&title_prefix));
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{}  dotman - Main Menu", icon_set.app),
            Style::default()
                .fg(CATPPUCCIN_MOCHA.fg_dim)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", "─".repeat(divider_width)),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    ]));
    f.render_widget(title, chunks[0]);

    // Menu items with two-line layout (title + description)
    let menu_items: [(&str, &str, &str); 4] = [
        (
            icon_set.menu_deploy,
            "Deploy",
            "Sync dotfiles to this machine",
        ),
        (
            icon_set.menu_plan,
            "Plan only",
            "Preview changes without executing",
        ),
        (
            icon_set.menu_history,
            "History",
            "Browse past deployment records",
        ),
        (icon_set.menu_quit, "Quit", "Exit dotman"),
    ];
    let mut styled_items: Vec<ListItem> = Vec::new();
    let area_width = usize::from(chunks[1].width);
    for (i, &(icon, title, desc)) in menu_items.iter().enumerate() {
        let is_sel = app.list_state.selected() == Some(i);
        let title_text = format!("{} {}", icon, title);
        if is_sel {
            let bg = focus_bg();
            let title_content_w = 2 + display_width(&title_text);
            let desc_content_w = 4 + display_width(desc);
            let mut lines = vec![
                Line::from(vec![
                    Span::styled(" ", Style::default().bg(bg)),
                    Span::styled("▎", Style::default().fg(CATPPUCCIN_MOCHA.primary).bg(bg)),
                    Span::styled(
                        title_text.clone(),
                        Style::default()
                            .bg(bg)
                            .fg(CATPPUCCIN_MOCHA.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " ".repeat(area_width.saturating_sub(title_content_w)),
                        Style::default().bg(bg),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" ", Style::default().bg(bg)),
                    Span::styled("▎", Style::default().fg(CATPPUCCIN_MOCHA.primary).bg(bg)),
                    Span::styled("  ", Style::default().bg(bg)),
                    Span::styled(desc, Style::default().bg(bg).fg(CATPPUCCIN_MOCHA.fg_dim)),
                    Span::styled(
                        " ".repeat(area_width.saturating_sub(desc_content_w)),
                        Style::default().bg(bg),
                    ),
                ]),
            ];
            if i == 0 {
                lines.insert(0, Line::from(" "));
            }
            if i < 4 {
                lines.push(Line::from(" "));
            }
            styled_items.push(ListItem::new(lines));
        } else {
            let mut lines = vec![
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(title_text.clone(), Style::default().fg(CATPPUCCIN_MOCHA.fg)),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(desc, Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
                ]),
            ];
            if i == 0 {
                lines.insert(0, Line::from(" "));
            }
            if i < 4 {
                lines.push(Line::from(" "));
            }
            styled_items.push(ListItem::new(lines));
        }
    }
    let list = List::new(styled_items)
        .highlight_style(Style::default())
        .highlight_symbol("");
    f.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // Summary
    let cfg = app.config.as_ref();
    let pkg = cfg.map(|c| c.install.len()).unwrap_or(0);
    let links = cfg.map(|c| c.links.len()).unwrap_or(0);
    let dirs = cfg.map(|c| c.create.len()).unwrap_or(0);
    let shells = cfg.map(|c| c.shell.len()).unwrap_or(0);

    let os_part = if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    let arch_part = std::env::consts::ARCH;
    let summary_line_str = format!(
        "  {} {os_part} {arch_part} · {pkg} packages · {links} links · {dirs} directories · {shells} shell steps",
        icon_set.host
    );

    let summary_width = usize::from(chunks[2].width).saturating_sub(2);
    let summary_divider = format!("  {}", "─".repeat(summary_width));

    let mut summary_lines = vec![
        Line::from(vec![Span::styled(
            summary_divider,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        )]),
        Line::from(vec![Span::styled(
            summary_line_str,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        )]),
    ];

    if let Some(run) = app.runs.first() {
        let status_icon = match run.status {
            RunStatus::Success => icon_set.success,
            RunStatus::Failed => icon_set.failed,
            RunStatus::Aborted => icon_set.warning,
            RunStatus::Running => icon_set.running,
        };
        let status_color = match run.status {
            RunStatus::Success => CATPPUCCIN_MOCHA.success,
            RunStatus::Failed => CATPPUCCIN_MOCHA.danger,
            RunStatus::Aborted | RunStatus::Running => CATPPUCCIN_MOCHA.warning,
        };
        let mode_str = format!("{:?}", run.mode).to_lowercase();
        let date_str = fmt_date(&run.started_at);
        let total = run.items.len();
        let failed = run.items.iter().filter(|i| i.error.is_some()).count();
        summary_lines.push(Line::from(vec![
            Span::styled(
                format!("  last run: {date_str}  "),
                Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
            ),
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::styled(
                format!(" {mode_str} ({total} items, {failed} fail)"),
                Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(summary_lines), chunks[2]);

    let help = Paragraph::new(Line::from(vec![
        keycap("↑↓"),
        hint(" move  "),
        keycap("enter"),
        hint(" select  "),
        keycap("d"),
        hint(" deploy  "),
        keycap("p"),
        hint(" plan  "),
        keycap("h"),
        hint(" history  "),
        keycap("q"),
        hint(" quit"),
    ]));
    f.render_widget(help, chunks[3]);
}
