use super::*;

// ---------------- ConfirmView ----------------

pub(super) fn handle_confirm(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Down | KeyCode::Char('j') => scroll_review(app, 1),
        KeyCode::Up | KeyCode::Char('k') => scroll_review(app, -1),
        KeyCode::PageDown => scroll_review(app, 8),
        KeyCode::PageUp => scroll_review(app, -8),
        KeyCode::Home => app.review_scroll = 0,
        KeyCode::End => app.review_scroll = usize::MAX,
        KeyCode::Enter | KeyCode::Char('r') => {
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
                    return Ok(());
                }
            }
            run::start_run(app);
        }
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('e') => {
            app.screen = Screen::PlanView;
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn scroll_review(app: &mut App, delta: isize) {
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
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(summary_height),
            Constraint::Min(3),
            Constraint::Length(1),
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

    let title_prefix = format!("{}  dotman - Review  {:?} ", icon_set.info, app.mode);
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

    let summary = if density == LayoutDensity::Compact {
        vec![Line::from(vec![
            Span::styled("Selected ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
            Span::styled(
                selected.to_string(),
                Style::default().fg(CATPPUCCIN_MOCHA.success),
            ),
            Span::raw(format!(" · {actions} actions · ")),
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
        vec![
            Line::from(vec![
                Span::styled("Selected: ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
                Span::styled(
                    selected.to_string(),
                    Style::default().fg(CATPPUCCIN_MOCHA.success),
                ),
                Span::raw(" steps, "),
                Span::styled(
                    actions.to_string(),
                    Style::default().fg(CATPPUCCIN_MOCHA.accent),
                ),
                Span::raw(" actions"),
            ]),
            Line::from(vec![
                Span::styled("Skipped: ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
                Span::raw(skipped.to_string()),
                Span::raw(" steps"),
            ]),
            Line::from(vec![
                Span::styled("Review: ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
                Span::raw(format!(
                    "{review_count} actions, {change_count} active, {risk_count} attention"
                )),
            ]),
        ]
    };
    f.render_widget(Paragraph::new(summary), chunks[1]);

    let body_width = usize::from(chunks[2].width);
    let body_height = usize::from(chunks[2].height);
    let body = if entries.is_empty() {
        vec![Line::from("No selected actions.")]
    } else {
        review_body_lines(entries, body_width, body_height, &mut app.review_scroll)
    };
    f.render_widget(Paragraph::new(body).wrap(Wrap { trim: false }), chunks[2]);

    let help_line = if app.status_message.is_empty() {
        review_help_line(usize::from(chunks[3].width))
    } else {
        Line::from(vec![
            Span::styled("  error  ", Style::default().fg(CATPPUCCIN_MOCHA.danger)),
            Span::styled(
                fit_to_width(
                    &app.status_message,
                    usize::from(chunks[3].width).saturating_sub(9),
                ),
                Style::default().fg(CATPPUCCIN_MOCHA.danger),
            ),
        ])
    };
    let help = Paragraph::new(help_line);
    f.render_widget(help, chunks[3]);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReviewSeverity {
    Success,
    Skip,
    Run,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReviewGroup {
    Attention,
    WillRun,
    AlreadyOk,
    Skipped,
}

#[derive(Debug, Clone)]
pub(super) struct ReviewEntry {
    pub(super) order: usize,
    pub(super) item: String,
    pub(super) kind: &'static str,
    pub(super) kind_icon: &'static str,
    pub(super) severity: ReviewSeverity,
    pub(super) status: String,
    pub(super) detail: String,
}

pub(super) fn review_entries(plan: &Plan, config: Option<&config::Config>) -> Vec<ReviewEntry> {
    let icon_set = icons::current();
    let config_dir = plan.config_path.parent().unwrap_or(Path::new("."));
    let mut entries = Vec::new();
    for item in plan.items.iter().filter(|item| item.selected) {
        for action in &item.actions {
            let mut entry = match action {
                Action::Install {
                    pkg_mgr,
                    binary,
                    source,
                } => review_install_entry(item, config, pkg_mgr, binary, source),
                Action::Link { target, source } => {
                    review_link_entry(item, config_dir, target, source, icon_set.action_link)
                }
                Action::Create { target } => {
                    review_create_entry(item, target, icon_set.action_create)
                }
                Action::Shell {
                    command,
                    description,
                    optional,
                    if_condition,
                } => review_shell_entry(
                    item,
                    command,
                    description.as_deref(),
                    *optional,
                    if_condition.as_deref(),
                    icon_set.action_shell,
                ),
                Action::Clean { target, force } => {
                    review_clean_entry(item, target, *force, icon_set.action_clean)
                }
            };
            entry.order = entries.len();
            entries.push(entry);
        }
    }
    entries
}

pub(super) fn review_install_entry(
    item: &PlanItem,
    config: Option<&config::Config>,
    pkg_mgr: &str,
    binary: &str,
    source: &str,
) -> ReviewEntry {
    let icon_set = icons::current();
    let resolved_pkg_mgr = config
        .and_then(|cfg| crate::package_managers::resolve_pkg_mgr_name(&cfg.package_managers))
        .unwrap_or_else(|| {
            if pkg_mgr == "auto" {
                crate::package_managers::default_pkg_mgr_name()
            } else {
                pkg_mgr.to_string()
            }
        });
    let command = install_command_summary(binary, &resolved_pkg_mgr).unwrap_or_else(|| {
        if source.trim().is_empty() {
            format!("install {binary}")
        } else {
            source.to_string()
        }
    });

    let db = install::load_db().ok();
    let entry = db.as_ref().and_then(|db| install::find(db, binary));
    match entry
        .as_ref()
        .map(|entry| install::detect_presence(entry, Some(&command)))
        .unwrap_or(install::InstallPresence::Unknown)
    {
        install::InstallPresence::Present => ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Success,
            status: "present".into(),
            detail: command,
        },
        install::InstallPresence::Missing => ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Run,
            status: "missing".into(),
            detail: command,
        },
        install::InstallPresence::Unknown => ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Warning,
            status: "unknown".into(),
            detail: command,
        },
    }
}

pub(super) fn review_link_entry(
    item: &PlanItem,
    config_dir: &Path,
    target: &Path,
    source: &Path,
    kind_icon: &'static str,
) -> ReviewEntry {
    let (severity, status) = match link::plan_link(
        config_dir,
        target,
        source,
        LinkSettings {
            create: true,
            relative: true,
            backup: true,
            relink: false,
        },
    ) {
        Ok(link_plan) => describe_link_review(&link_plan.action),
        Err(e) => (ReviewSeverity::Danger, format!("inspect failed: {e}")),
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "link",
        kind_icon,
        severity,
        status,
        detail: format!("{} -> {}", target.display(), source.display()),
    }
}

pub(super) fn review_create_entry(
    item: &PlanItem,
    target: &Path,
    kind_icon: &'static str,
) -> ReviewEntry {
    let expanded = crate::path::expand_home(&target.to_string_lossy())
        .unwrap_or_else(|_| target.to_path_buf());
    let (severity, status) = if expanded.exists() {
        (ReviewSeverity::Success, "exists".into())
    } else {
        (ReviewSeverity::Run, "create".into())
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "create",
        kind_icon,
        severity,
        status,
        detail: target.display().to_string(),
    }
}

pub(super) fn review_shell_entry(
    item: &PlanItem,
    command: &str,
    description: Option<&str>,
    optional: bool,
    if_condition: Option<&str>,
    kind_icon: &'static str,
) -> ReviewEntry {
    let mut status = if optional {
        "optional".to_string()
    } else {
        "run".to_string()
    };
    let mut severity = ReviewSeverity::Run;
    if let Some(cond) = if_condition {
        match shell::condition_matches(cond) {
            Ok(true) => status = format!("if ok · {status}"),
            Ok(false) => {
                status = "if skip".into();
                severity = ReviewSeverity::Skip;
            }
            Err(_) => {
                status = "if unknown".into();
                severity = ReviewSeverity::Warning;
            }
        }
    }
    if shell::command_contains_sudo(command) && !matches!(severity, ReviewSeverity::Skip) {
        status = format!("{status} · sudo");
        severity = ReviewSeverity::Warning;
    }
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "shell",
        kind_icon,
        severity,
        status,
        detail: description.unwrap_or(command).to_string(),
    }
}

pub(super) fn review_clean_entry(
    item: &PlanItem,
    target: &Path,
    force: bool,
    kind_icon: &'static str,
) -> ReviewEntry {
    let (severity, status) = match clean::plan_clean(target, force) {
        Ok(clean::CleanAction::Skip) => (ReviewSeverity::Skip, "skip".into()),
        Ok(clean::CleanAction::RemoveSymlink) => (ReviewSeverity::Warning, "remove symlink".into()),
        Ok(clean::CleanAction::BackupAndRemove(_)) => {
            (ReviewSeverity::Warning, "backup remove".into())
        }
        Err(e) => (ReviewSeverity::Danger, format!("inspect failed: {e}")),
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "clean",
        kind_icon,
        severity,
        status,
        detail: target.display().to_string(),
    }
}

pub(super) fn describe_link_review(action: &LinkAction) -> (ReviewSeverity, String) {
    match action {
        LinkAction::Skip => (ReviewSeverity::Success, "linked".into()),
        LinkAction::Link => (ReviewSeverity::Run, "link".into()),
        LinkAction::Backup(_) => (ReviewSeverity::Warning, "backup link".into()),
        LinkAction::Relink => (ReviewSeverity::Warning, "relink".into()),
        LinkAction::Fail(reason) => (ReviewSeverity::Danger, format!("fail: {reason}")),
    }
}

pub(super) fn review_body_lines(
    entries: &[ReviewEntry],
    width: usize,
    height: usize,
    scroll: &mut usize,
) -> Vec<Line<'static>> {
    if height == 0 {
        return Vec::new();
    }
    let mut all_lines = Vec::new();
    for group in [
        ReviewGroup::Attention,
        ReviewGroup::WillRun,
        ReviewGroup::AlreadyOk,
        ReviewGroup::Skipped,
    ] {
        let group_entries = sorted_review_group_entries(entries, group);
        if group_entries.is_empty() {
            continue;
        }
        all_lines.push(review_group_header_line(group, group_entries.len(), width));
        for entry in group_entries {
            all_lines.extend(review_entry_lines(entry, width));
        }
    }
    if all_lines.len() <= height {
        *scroll = 0;
        return all_lines;
    }

    let max_scroll = all_lines.len().saturating_sub(height);
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
        *first = Line::from(Span::styled(
            fit_to_width(&format!("  ... {} above", *scroll), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    let below = all_lines.len().saturating_sub(*scroll + height);
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
) -> Line<'static> {
    let icon_set = icons::current();
    let (icon, label, style) = match group {
        ReviewGroup::Attention => (
            icon_set.warning,
            "Attention",
            Style::default().fg(CATPPUCCIN_MOCHA.warning),
        ),
        ReviewGroup::WillRun => (
            icon_set.running,
            "Will Run",
            Style::default().fg(CATPPUCCIN_MOCHA.running),
        ),
        ReviewGroup::AlreadyOk => (
            icon_set.success,
            "Already OK",
            Style::default().fg(CATPPUCCIN_MOCHA.success),
        ),
        ReviewGroup::Skipped => (
            icon_set.skipped,
            "Skipped",
            Style::default().fg(CATPPUCCIN_MOCHA.skip),
        ),
    };
    Line::from(Span::styled(
        fit_to_width(&format!("{icon} {label} ({count})"), width),
        style.add_modifier(Modifier::BOLD),
    ))
}

pub(super) fn review_entry_lines(entry: &ReviewEntry, width: usize) -> Vec<Line<'static>> {
    let icon_set = icons::current();
    let status_icon = review_status_icon(icon_set, entry.severity);
    let status_style = review_status_style(entry.severity);
    let left = format!("{} {:<7} {}", entry.kind_icon, entry.kind, entry.item);
    let detail = review_entry_detail(entry);
    let single = if let Some(detail) = detail {
        format!("{left}  {}  {detail}", entry.status)
    } else {
        format!("{left}  {}", entry.status)
    };
    if display_width(&single) <= width.saturating_sub(2) {
        return vec![Line::from(vec![
            Span::styled(status_icon, status_style),
            Span::raw(" "),
            Span::styled(single, Style::default().fg(CATPPUCCIN_MOCHA.fg)),
        ])];
    }

    let first = format!("{left}  {}", entry.status);
    let mut lines = vec![Line::from(vec![
        Span::styled(status_icon, status_style),
        Span::raw(" "),
        Span::styled(
            fit_to_width(&first, width.saturating_sub(2)),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
    ])];
    if let Some(detail) = detail {
        lines.push(Line::from(Span::styled(
            fit_to_width(&format!("  {} {detail}", icon_set.info), width),
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

pub(super) fn review_entry_needs_sudo(entry: &ReviewEntry) -> bool {
    matches!(
        entry.severity,
        ReviewSeverity::Run | ReviewSeverity::Warning
    ) && (entry.status.contains("sudo") || shell::command_contains_sudo(&entry.detail))
}

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

pub(super) fn review_status_style(severity: ReviewSeverity) -> Style {
    Style::default().fg(match severity {
        ReviewSeverity::Success => CATPPUCCIN_MOCHA.success,
        ReviewSeverity::Skip => CATPPUCCIN_MOCHA.skip,
        ReviewSeverity::Run => CATPPUCCIN_MOCHA.running,
        ReviewSeverity::Warning => CATPPUCCIN_MOCHA.warning,
        ReviewSeverity::Danger => CATPPUCCIN_MOCHA.danger,
    })
}

pub(super) fn install_command_summary(binary: &str, pkg_mgr: &str) -> Option<String> {
    let db = install::load_db().ok()?;
    let entry = install::find(&db, binary)?;
    install::command_for_current_platform(&entry, pkg_mgr)
}
