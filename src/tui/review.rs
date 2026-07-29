use super::*;

mod inspection;

pub(super) use inspection::*;

const REVIEW_PAGE_SIZE: isize = 8;

pub(super) fn default_expanded_groups() -> BTreeSet<ReviewGroup> {
    BTreeSet::from([ReviewGroup::Attention, ReviewGroup::WillRun])
}

// ---------------- ConfirmView ----------------

pub(super) fn start_review(app: &mut App) {
    let Some(plan) = app.plan.clone() else {
        return;
    };
    let config = app.config.clone();
    app.review_entries.clear();
    app.review_scroll = 0;
    app.review_group_focus = 0;
    app.review_follow_group_focus = true;
    app.review_expanded_groups = default_expanded_groups();
    app.review_danger_confirmed = false;
    app.status_message = "checking action conditions...".into();
    app.status_kind = NoticeKind::Info;
    app.status_is_focus_info = false;
    app.screen = Screen::ConfirmView;
    app.review_thread = Some(std::thread::spawn(move || {
        review_entries(&plan, config.as_ref())
    }));
}

pub(super) fn poll_review_thread(app: &mut App) -> bool {
    let Some(handle) = app.review_thread.as_ref() else {
        return false;
    };
    if !handle.is_finished() {
        return false;
    }
    let handle = app.review_thread.take().expect("review thread exists");
    match handle.join() {
        Ok(entries) => {
            app.review_entries = entries;
            app.review_group_focus = first_visible_review_group(&app.review_entries);
            app.review_follow_group_focus = true;
            app.status_message.clear();
            app.status_kind = NoticeKind::Info;
        }
        Err(_) => {
            app.review_entries.clear();
            app.status_message = "review condition check panicked".into();
            app.status_kind = NoticeKind::Error;
        }
    }
    true
}

pub(super) fn handle_confirm(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
            move_review_group_focus(app, 1);
        }
        KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
            move_review_group_focus(app, -1);
        }
        KeyCode::PageDown => scroll_review(app, REVIEW_PAGE_SIZE),
        KeyCode::PageUp => scroll_review(app, -REVIEW_PAGE_SIZE),
        KeyCode::Home => {
            app.review_scroll = 0;
            app.review_follow_group_focus = false;
        }
        KeyCode::End => {
            app.review_scroll = usize::MAX;
            app.review_follow_group_focus = false;
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.review_expanded_groups
                .remove(&focused_review_group(app));
            app.review_scroll = 0;
            app.review_follow_group_focus = true;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.review_expanded_groups.insert(focused_review_group(app));
            app.review_scroll = 0;
            app.review_follow_group_focus = true;
        }
        KeyCode::Char(' ') => {
            let group = focused_review_group(app);
            if !app.review_expanded_groups.insert(group) {
                app.review_expanded_groups.remove(&group);
            }
            app.review_scroll = 0;
            app.review_follow_group_focus = true;
        }
        KeyCode::Char('!') if review_entries_have_danger(&app.review_entries) => {
            app.review_danger_confirmed = true;
            app.status_message = "danger confirmed; press r to run once".into();
            app.status_kind = NoticeKind::Warning;
        }
        KeyCode::Char('r') | KeyCode::Enter => {
            if app.review_thread.is_some() {
                app.status_message = "review checks are still running".into();
                app.status_kind = NoticeKind::Info;
                return Ok(());
            }
            if review_entries_have_danger(&app.review_entries) && !app.review_danger_confirmed {
                app.status_message =
                    "plan has danger items; press ! to confirm before running".into();
                app.status_kind = NoticeKind::Error;
                return Ok(());
            }
            // Pre-cache sudo credentials before executing if the plan needs them.
            if review_entries_need_sudo(&app.review_entries) {
                restore_terminal()?;
                let ok = shell::pre_cache_sudo().unwrap_or(false);
                // Re-enter raw mode; the event loop will recreate the Terminal
                // backend on the next tick.
                setup_terminal()?;
                app.needs_terminal_reset = true;
                if !ok {
                    app.status_message = "sudo authentication failed".into();
                    app.status_kind = NoticeKind::Error;
                    return Ok(());
                }
            }
            app.review_danger_confirmed = false;
            app.status_message.clear();
            app.status_kind = NoticeKind::Info;
            app.status_is_focus_info = false;
            run::start_run(app);
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::PlanView;
        }
        _ => {}
    }
    Ok(())
}

fn focused_review_group(app: &App) -> ReviewGroup {
    REVIEW_GROUPS[app.review_group_focus.min(REVIEW_GROUPS.len() - 1)]
}

fn move_review_group_focus(app: &mut App, direction: isize) {
    app.review_group_focus =
        next_visible_review_group(&app.review_entries, app.review_group_focus, direction);
    app.review_scroll = 0;
    app.review_follow_group_focus = true;
}

pub(super) fn jump_review_top(app: &mut App) {
    app.review_group_focus = first_visible_review_group(&app.review_entries);
    app.review_scroll = 0;
    app.review_follow_group_focus = true;
}

pub(super) fn jump_review_bottom(app: &mut App) {
    app.review_group_focus = next_visible_review_group(
        &app.review_entries,
        first_visible_review_group(&app.review_entries),
        -1,
    );
    app.review_scroll = usize::MAX;
    app.review_follow_group_focus = true;
}

pub(super) fn scroll_review(app: &mut App, delta: isize) {
    app.review_follow_group_focus = false;
    if delta < 0 {
        app.review_scroll = app.review_scroll.saturating_sub(delta.unsigned_abs());
    } else {
        app.review_scroll = app.review_scroll.saturating_add(delta as usize);
    }
}

pub(super) fn render_confirm(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let density = layout_density(area.width, area.height);
    let summary_height = match density {
        LayoutDensity::Compact => 1,
        LayoutDensity::Normal => 3,
    };
    let footer_size = footer_height(&app.status_message);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(summary_height),
            Constraint::Min(3),
            Constraint::Length(footer_size),
        ])
        .split(area);

    let Some(plan) = &app.plan else {
        f.render_widget(
            Paragraph::new("no plan loaded").alignment(Alignment::Center),
            chunks[2],
        );
        return;
    };

    let selected = plan.items.iter().filter(|item| item.selected).count();
    let skipped = plan.items.len().saturating_sub(selected);
    let actions = plan
        .items
        .iter()
        .filter(|item| item.selected)
        .map(|item| item.actions.len())
        .sum::<usize>();
    let entries = &app.review_entries;
    let review_count = entries.len();
    let change_count = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.severity,
                ReviewSeverity::Run | ReviewSeverity::Warning
            )
        })
        .count();
    let risk_count = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.severity,
                ReviewSeverity::Warning | ReviewSeverity::Danger
            )
        })
        .count();

    let meta = vec![Span::styled(
        mode_label(&app.mode),
        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
    )];
    f.render_widget(
        Paragraph::new(screen_header(
            icon_set.info,
            "Review",
            meta,
            usize::from(chunks[0].width),
        )),
        chunks[0],
    );

    let summary = if density == LayoutDensity::Compact {
        vec![Line::from(vec![
            Span::styled("Selected ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
            Span::styled(
                selected.to_string(),
                Style::default().fg(CATPPUCCIN_MOCHA.accent),
            ),
            Span::styled(
                format!(" · {actions} actions · "),
                Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
            ),
            Span::styled(
                format!("{risk_count} attention"),
                Style::default().fg(if risk_count > 0 {
                    CATPPUCCIN_MOCHA.warning
                } else {
                    CATPPUCCIN_MOCHA.text_muted
                }),
            ),
        ])]
    } else {
        let dim = Style::default().fg(CATPPUCCIN_MOCHA.fg_dim);
        vec![
            Line::from(vec![
                Span::styled("Selected: ", dim),
                Span::styled(
                    selected.to_string(),
                    Style::default().fg(CATPPUCCIN_MOCHA.accent),
                ),
                Span::styled(" steps · ", dim),
                Span::styled(
                    actions.to_string(),
                    Style::default().fg(CATPPUCCIN_MOCHA.accent),
                ),
                Span::styled(" actions", dim),
            ]),
            Line::from(vec![
                Span::styled("Skipped: ", dim),
                Span::styled(
                    skipped.to_string(),
                    Style::default().fg(CATPPUCCIN_MOCHA.fg),
                ),
                Span::styled(" steps", dim),
            ]),
            Line::from(vec![
                Span::styled("Review: ", dim),
                Span::styled(
                    format!("{review_count} actions · {change_count} active"),
                    Style::default().fg(CATPPUCCIN_MOCHA.fg),
                ),
                Span::styled(" · ", dim),
                Span::styled(
                    format!("{risk_count} attention"),
                    Style::default().fg(if risk_count > 0 {
                        CATPPUCCIN_MOCHA.warning
                    } else {
                        CATPPUCCIN_MOCHA.text_muted
                    }),
                ),
            ]),
        ]
    };
    f.render_widget(Paragraph::new(summary), chunks[1]);

    let body_width = usize::from(chunks[2].width);
    let body_height = usize::from(chunks[2].height);
    let follow_group_focus = app.review_follow_group_focus;
    let body = if entries.is_empty() {
        vec![Line::from("No selected actions.")]
    } else {
        review_body_lines(
            entries,
            body_width,
            body_height,
            &mut app.review_scroll,
            &app.review_expanded_groups,
            app.review_group_focus,
            follow_group_focus,
        )
    };
    app.review_follow_group_focus = false;
    f.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), chunks[2]);

    let notice = (!app.status_message.is_empty()).then(|| FooterNotice {
        message: &app.status_message,
        kind: app.status_kind,
    });
    let footer = screen_footer(
        notice,
        vim_prefix_help(
            app,
            review_help_line(usize::from(chunks[3].width)),
            usize::from(chunks[3].width),
        ),
        usize::from(chunks[3].width),
    );
    f.render_widget(footer, chunks[3]);
}

pub(super) fn selected_item_count(plan: Option<&Plan>) -> usize {
    plan.map(|plan| plan.items.iter().filter(|item| item.selected).count())
        .unwrap_or(0)
}

pub(super) fn selected_action_count(plan: Option<&Plan>) -> usize {
    plan.map(|plan| {
        plan.items
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.actions.len())
            .sum()
    })
    .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReviewGroup {
    Attention,
    WillRun,
    AlreadyOk,
    Skipped,
}

const REVIEW_GROUPS: [ReviewGroup; 4] = [
    ReviewGroup::Attention,
    ReviewGroup::WillRun,
    ReviewGroup::AlreadyOk,
    ReviewGroup::Skipped,
];

fn first_visible_review_group(entries: &[ReviewEntry]) -> usize {
    REVIEW_GROUPS
        .iter()
        .position(|group| {
            entries
                .iter()
                .any(|entry| review_group_for(entry) == *group)
        })
        .unwrap_or(0)
}

fn next_visible_review_group(entries: &[ReviewEntry], current: usize, delta: isize) -> usize {
    let mut candidate = current.min(REVIEW_GROUPS.len() - 1);
    for _ in 0..REVIEW_GROUPS.len() {
        candidate = if delta < 0 {
            candidate.checked_sub(1).unwrap_or(REVIEW_GROUPS.len() - 1)
        } else {
            (candidate + 1) % REVIEW_GROUPS.len()
        };
        if entries
            .iter()
            .any(|entry| review_group_for(entry) == REVIEW_GROUPS[candidate])
        {
            return candidate;
        }
    }
    current
}

pub(super) fn review_body_lines(
    entries: &[ReviewEntry],
    width: usize,
    height: usize,
    scroll: &mut usize,
    expanded_groups: &BTreeSet<ReviewGroup>,
    focused_group: usize,
    follow_group_focus: bool,
) -> Vec<Line<'static>> {
    if height == 0 {
        return Vec::new();
    }
    let mut all_lines = Vec::new();
    let mut focused_line = None;
    for (group_index, group) in REVIEW_GROUPS.into_iter().enumerate() {
        let group_entries = sorted_review_group_entries(entries, group);
        if group_entries.is_empty() {
            continue;
        }
        let expanded = expanded_groups.contains(&group);
        if group_index == focused_group {
            focused_line = Some(all_lines.len());
        }
        all_lines.push(review_group_header_line(
            group,
            group_entries.len(),
            width,
            expanded,
            group_index == focused_group,
        ));
        if expanded {
            for entry in group_entries {
                all_lines.extend(review_entry_lines(entry, width));
            }
        } else {
            all_lines.push(review_group_summary_line(&group_entries, width));
        }
    }
    if all_lines.len() <= height {
        *scroll = 0;
        return all_lines;
    }

    let max_scroll = all_lines.len().saturating_sub(height);
    if follow_group_focus && let Some(focused_line) = focused_line {
        *scroll = focused_line.saturating_sub(1).min(max_scroll);
    }
    *scroll = (*scroll).min(max_scroll);

    let mut visible = all_lines
        .iter()
        .skip(*scroll)
        .take(height)
        .cloned()
        .collect::<Vec<_>>();
    if *scroll > 0
        && let Some(first) = visible.first_mut()
    {
        *first = scroll_marker_line(true, *scroll, width);
    }
    let below = all_lines.len().saturating_sub(*scroll + height);
    if below > 0
        && let Some(last) = visible.last_mut()
    {
        *last = scroll_marker_line(false, below, width);
    }
    visible
}

pub(super) fn sorted_review_group_entries(
    entries: &[ReviewEntry],
    group: ReviewGroup,
) -> Vec<&ReviewEntry> {
    let mut group_entries = entries
        .iter()
        .filter(|entry| review_group_for(entry) == group)
        .collect::<Vec<_>>();
    group_entries.sort_by_key(|entry| (review_kind_rank(entry.kind), entry.order));
    group_entries
}

pub(super) fn review_kind_rank(kind: &str) -> usize {
    match kind {
        "install" => 0,
        "link" => 1,
        "create" => 2,
        "shell" => 3,
        "clean" => 4,
        _ => usize::MAX,
    }
}

pub(super) fn review_group_for(entry: &ReviewEntry) -> ReviewGroup {
    match entry.severity {
        ReviewSeverity::Warning | ReviewSeverity::Danger => ReviewGroup::Attention,
        ReviewSeverity::Run => ReviewGroup::WillRun,
        ReviewSeverity::Success => ReviewGroup::AlreadyOk,
        ReviewSeverity::Skip => ReviewGroup::Skipped,
    }
}

pub(super) fn review_group_header_line(
    group: ReviewGroup,
    count: usize,
    width: usize,
    expanded: bool,
    focused: bool,
) -> Line<'static> {
    let icon_set = icons::current();
    let (status_icon, label, color) = match group {
        ReviewGroup::Attention => (icon_set.warning, "Attention", CATPPUCCIN_MOCHA.warning),
        ReviewGroup::WillRun => (icon_set.running, "Will Run", CATPPUCCIN_MOCHA.running),
        ReviewGroup::AlreadyOk => (icon_set.success, "Already OK", CATPPUCCIN_MOCHA.success),
        ReviewGroup::Skipped => (icon_set.skipped, "Skipped", CATPPUCCIN_MOCHA.skip),
    };
    let disclosure = if expanded {
        icon_set.expanded
    } else {
        icon_set.collapsed
    };
    let marker = if focused { "▎" } else { " " };
    let head = format!("{marker} {disclosure} {status_icon} {label}");
    let tail = format!("  ({count})");
    let padding = width.saturating_sub(display_width(&head) + display_width(&tail));
    Line::from(vec![
        Span::styled(
            head,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(padding)),
        Span::styled(tail, Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
    ])
}

fn review_group_summary_line(entries: &[&ReviewEntry], width: usize) -> Line<'static> {
    let count = |kind: &str| entries.iter().filter(|entry| entry.kind == kind).count();
    let mut parts = Vec::new();
    for (kind, label) in [
        ("install", "installs"),
        ("link", "links"),
        ("create", "directories"),
        ("shell", "shell steps"),
        ("clean", "cleanups"),
    ] {
        let value = count(kind);
        if value > 0 {
            parts.push(format!("{value} {label}"));
        }
    }
    Line::from(Span::styled(
        fit_to_width(&format!("    {}", parts.join(" · ")), width),
        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
    ))
}

pub(super) fn review_entry_lines(entry: &ReviewEntry, width: usize) -> Vec<Line<'static>> {
    let left = format!("{} {:<7} {}", entry.kind_icon, entry.kind, entry.item);
    let detail = review_entry_detail(entry);
    let single = if let Some(detail) = detail {
        format!("{left}  {}  {detail}", entry.status)
    } else {
        format!("{left}  {}", entry.status)
    };
    if display_width(&single) <= width.saturating_sub(2) {
        return vec![Line::from(vec![
            Span::raw("    "),
            Span::styled(single, Style::default().fg(CATPPUCCIN_MOCHA.fg)),
        ])];
    }

    let first = format!("{left}  {}", entry.status);
    let mut lines = vec![Line::from(vec![
        Span::raw("    "),
        Span::styled(
            fit_to_width(&first, width.saturating_sub(4)),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
    ])];
    if let Some(detail) = detail {
        lines.push(Line::from(Span::styled(
            fit_to_width(&format!("      {detail}"), width),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        )));
    }
    lines
}

pub(super) fn review_entry_detail(entry: &ReviewEntry) -> Option<&str> {
    let detail = entry.detail.trim();
    (!detail.is_empty() && detail != entry.item.trim()).then_some(detail)
}

pub(super) fn review_entries_need_sudo(entries: &[ReviewEntry]) -> bool {
    entries.iter().any(review_entry_needs_sudo)
}

pub(super) fn review_entries_have_danger(entries: &[ReviewEntry]) -> bool {
    entries
        .iter()
        .any(|entry| matches!(entry.severity, ReviewSeverity::Danger))
}

pub(super) fn review_entry_needs_sudo(entry: &ReviewEntry) -> bool {
    matches!(
        entry.severity,
        ReviewSeverity::Run | ReviewSeverity::Warning
    ) && (entry.status.contains("sudo") || shell::command_contains_sudo(&entry.detail))
}

#[cfg(test)]
pub(super) fn review_status_icon(
    icon_set: &'static icons::IconSet,
    severity: ReviewSeverity,
) -> &'static str {
    match severity {
        ReviewSeverity::Success => icon_set.success,
        ReviewSeverity::Skip => icon_set.skipped,
        ReviewSeverity::Run => icon_set.running,
        ReviewSeverity::Warning => icon_set.warning,
        ReviewSeverity::Danger => icon_set.failed,
    }
}

#[cfg(test)]
pub(super) fn review_status_style(severity: ReviewSeverity) -> Style {
    Style::default().fg(match severity {
        ReviewSeverity::Success => CATPPUCCIN_MOCHA.success,
        ReviewSeverity::Skip => CATPPUCCIN_MOCHA.skip,
        ReviewSeverity::Run => CATPPUCCIN_MOCHA.running,
        ReviewSeverity::Warning => CATPPUCCIN_MOCHA.warning,
        ReviewSeverity::Danger => CATPPUCCIN_MOCHA.danger,
    })
}
