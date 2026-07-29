use super::*;

// ---------------- MainMenu ----------------

pub(super) fn handle_main_menu(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('d') => {
            app.mode = Mode::Deploy;
            if let Err(e) = app.build_plan() {
                app.status_message = e;
                app.status_kind = NoticeKind::Error;
            }
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('p') => {
            app.mode = Mode::Plan;
            if let Err(e) = app.build_plan() {
                app.status_message = e;
                app.status_kind = NoticeKind::Error;
            }
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('h') => {
            app::load_runs(app);
            history::clamp_history_selection(app);
            app.screen = Screen::HistoryView;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app.menu_state.selected().unwrap_or(0);
            if i + 1 < 4 {
                app.menu_state.select(Some(i + 1));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app.menu_state.selected().unwrap_or(0);
            if i > 0 {
                app.menu_state.select(Some(i - 1));
            }
        }
        KeyCode::PageDown => {
            let i = app.menu_state.selected().unwrap_or(0);
            app.menu_state.select(Some((i + 3).min(3)));
        }
        KeyCode::PageUp => {
            let i = app.menu_state.selected().unwrap_or(0);
            app.menu_state.select(Some(i.saturating_sub(3)));
        }
        KeyCode::Enter => match app.menu_state.selected() {
            Some(0) => {
                app.mode = Mode::Deploy;
                if let Err(e) = app.build_plan() {
                    app.status_message = e;
                    app.status_kind = NoticeKind::Error;
                }
                app.screen = Screen::PlanView;
            }
            Some(1) => {
                app.mode = Mode::Plan;
                if let Err(e) = app.build_plan() {
                    app.status_message = e;
                    app.status_kind = NoticeKind::Error;
                }
                app.screen = Screen::PlanView;
            }
            Some(2) => {
                app::load_runs(app);
                history::clamp_history_selection(app);
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
    let icon_set = icons::current();
    let area = f.area();
    let density = layout_density(area.width, area.height);
    // Keep the launcher as one compact composition at every terminal height.
    // Extra rows belong below the content; stretching command gaps makes focus
    // movement feel discontinuous and disconnects recent activity from actions.
    const MENU_HEIGHT: u16 = 8;
    const MENU_ACTIVITY_GAP: u16 = 1;
    const ACTIVITY_HEIGHT: u16 = 4;
    const CONTENT_MAX_WIDTH: u16 = 96;
    let content_height = MENU_HEIGHT + MENU_ACTIVITY_GAP + ACTIVITY_HEIGHT;
    let footer_size = footer_height(&app.status_message);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(content_height),
            Constraint::Length(footer_size),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(screen_header(
            icon_set.app,
            "Main Menu",
            Vec::new(),
            usize::from(chunks[0].width),
        )),
        chunks[0],
    );

    let menu_items: [(&str, &str, &str); 4] = [
        (
            icon_set.menu_deploy,
            "Deploy",
            "Sync dotfiles to this machine",
        ),
        (
            icon_set.menu_plan,
            "Plan",
            "Preview changes without executing",
        ),
        (
            icon_set.menu_history,
            "History",
            "Browse past deployment records",
        ),
        (icon_set.menu_quit, "Quit", "Exit dotman"),
    ];
    // One stable command rail: every description remains on the same row and
    // moving focus changes only emphasis, never geometry.
    let horizontal_gutter = match density {
        LayoutDensity::Compact => 2,
        LayoutDensity::Normal => 8,
    };
    let content_width =
        CONTENT_MAX_WIDTH.min(chunks[1].width.saturating_sub(horizontal_gutter).max(1));
    let vertical_offset = match density {
        LayoutDensity::Compact => 0,
        LayoutDensity::Normal => chunks[1].height.saturating_sub(content_height).min(2),
    };
    let content_area = Rect::new(
        chunks[1].x + chunks[1].width.saturating_sub(content_width) / 2,
        chunks[1].y + vertical_offset,
        content_width,
        content_height.min(chunks[1].height.saturating_sub(vertical_offset)),
    );
    let content_rows = Layout::vertical([
        Constraint::Length(MENU_HEIGHT),
        Constraint::Length(MENU_ACTIVITY_GAP),
        Constraint::Length(ACTIVITY_HEIGHT),
    ])
    .split(content_area);
    let selected = app
        .menu_state
        .selected()
        .unwrap_or(0)
        .min(menu_items.len() - 1);
    let menu_width = usize::from(content_rows[0].width);
    let mut menu_lines = Vec::with_capacity(usize::from(MENU_HEIGHT));
    for (index, &(icon, title, description)) in menu_items.iter().enumerate() {
        menu_lines.push(main_menu_command_line(
            icon,
            title,
            description,
            index == selected,
            index == 3,
            menu_width,
        ));
        let gap_after = [1, 1, 1, 0][index];
        for _ in 0..gap_after {
            menu_lines.push(Line::from(""));
        }
    }
    f.render_widget(Paragraph::new(menu_lines), content_rows[0]);

    let cfg = app.config.as_ref();
    let packages = cfg.map(|c| c.install.len()).unwrap_or(0);
    let links = cfg.map(|c| c.links.len()).unwrap_or(0);
    let directories = cfg.map(|c| c.create.len()).unwrap_or(0);
    let shell_steps = cfg.map(|c| c.shell.len()).unwrap_or(0);
    let os = if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    let host = format!(
        "{} {os} {} · {packages} packages · {links} links · {directories} directories · {shell_steps} shell steps",
        icon_set.host,
        std::env::consts::ARCH,
    );
    let activity_width = usize::from(content_rows[2].width);
    let mut activity_lines = vec![
        Line::from(Span::styled("─".repeat(activity_width), divider_style())),
        Line::from(""),
    ];
    if let Some(run) = app.runs.first() {
        let summary = crate::model::RunSummary::from_run(run);
        let (status_icon, status_text) = match run.status {
            RunStatus::Success => (icon_set.success, "last deployment succeeded"),
            RunStatus::Failed => (icon_set.failed, "last deployment failed"),
            RunStatus::Aborted => (icon_set.warning, "last deployment aborted"),
            RunStatus::Running => (icon_set.running, "deployment in progress"),
        };
        let status_color = run_status_color(run.status);
        let total = run
            .items
            .iter()
            .filter(|item| !matches!(item.status, ActionStatus::WillSkip))
            .count();
        let items_label = if total == 1 { "item" } else { "items" };
        let details = format!(
            "  {} · {} · {total} {items_label} · {} failed",
            main_menu_short_date(&run.started_at),
            plan_mode_label(run.mode),
            summary.failed,
        );
        let status_width = display_width(status_icon) + 1 + display_width(status_text);
        activity_lines.push(Line::from(vec![
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::styled(
                format!(" {status_text}"),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                fit_to_width(&details, activity_width.saturating_sub(status_width)),
                Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
            ),
        ]));
    } else {
        activity_lines.push(Line::from(Span::styled(
            "no deployments recorded yet",
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        )));
    }
    activity_lines.push(Line::from(Span::styled(
        fit_to_width(&host, activity_width),
        Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
    )));
    f.render_widget(Paragraph::new(activity_lines), content_rows[2]);

    let notice = (!app.status_message.is_empty()).then(|| FooterNotice {
        message: &app.status_message,
        kind: app.status_kind,
    });
    let footer = screen_footer(
        notice,
        vim_prefix_help(
            app,
            main_menu_help_line(usize::from(chunks[2].width)),
            usize::from(chunks[2].width),
        ),
        usize::from(chunks[2].width),
    );
    f.render_widget(footer, chunks[2]);
}

fn main_menu_command_line(
    icon: &'static str,
    title: &'static str,
    description: &'static str,
    selected: bool,
    muted: bool,
    width: usize,
) -> Line<'static> {
    let label_width = if width >= 72 { 18 } else { 14 };
    let description_width = width.saturating_sub(2 + label_width + 2);
    let background = selected.then(focus_bg);
    let base = span_bg_style(background);
    let marker = if selected { "▎ " } else { "  " };
    let label_color = if selected {
        CATPPUCCIN_MOCHA.primary
    } else if muted {
        CATPPUCCIN_MOCHA.text_muted
    } else {
        CATPPUCCIN_MOCHA.fg
    };
    let description_color = if muted {
        CATPPUCCIN_MOCHA.fg_dim
    } else {
        CATPPUCCIN_MOCHA.text_muted
    };
    let mut label_style = base.fg(label_color);
    if selected {
        label_style = label_style.add_modifier(Modifier::BOLD);
    }

    Line::from(vec![
        Span::styled(
            marker,
            base.fg(if selected {
                CATPPUCCIN_MOCHA.focus_marker
            } else {
                label_color
            }),
        ),
        Span::styled(
            fit_to_width(&format!("{icon} {title}"), label_width),
            label_style,
        ),
        Span::styled("  ", base),
        Span::styled(
            fit_to_width(description, description_width),
            base.fg(description_color),
        ),
    ])
}

fn main_menu_short_date(rfc3339: &str) -> String {
    short_datetime(rfc3339)
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

fn main_menu_help_line(width: usize) -> Line<'static> {
    let full = [("↑↓", " Navigate  "), ("Enter", " Open  "), ("q", " Quit")];
    let compact = [("↑↓", " "), ("Ent", " "), ("q", "")];
    for parts in [&full[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }
    help_line_from_parts(&[("q", "")])
}
