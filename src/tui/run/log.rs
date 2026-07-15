use crate::execute::MAX_TUI_OUTPUT_LINES;
use crate::theme::CATPPUCCIN_MOCHA;
use crate::tui::{App, LogFilter, LogKind, LogLine};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use std::collections::BTreeSet;

pub(in crate::tui) fn log_group_for_event(app: &App, item: &str) -> String {
    app.active_log_group
        .clone()
        .unwrap_or_else(|| item.to_string())
}

pub(in crate::tui) fn push_log(app: &mut App, line: &str, fg: Option<Color>) {
    push_log_indented(app, line, fg, 0, LogKind::System);
}

pub(in crate::tui) fn push_log_group(app: &mut App, group: &str) {
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

pub(in crate::tui) fn push_log_indented(
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
    while app.current_log.len() > MAX_TUI_OUTPUT_LINES {
        let remove_index = if app.current_log.len() > 1
            && app.current_log[0].kind == LogKind::Header
            && app.current_log[0].group == app.current_log[1].group
        {
            1
        } else {
            0
        };
        app.current_log.remove(remove_index);
        app.log_dropped_count = app.log_dropped_count.saturating_add(1);
        app.log_scroll = app.log_scroll.saturating_sub(1);
    }
    if let Some(first) = app.current_log.first()
        && first.kind != LogKind::Header
        && let Some(group) = first.group.clone()
    {
        if app.current_log.len() >= MAX_TUI_OUTPUT_LINES {
            app.current_log.remove(0);
            app.log_dropped_count = app.log_dropped_count.saturating_add(1);
            app.log_scroll = app.log_scroll.saturating_sub(1);
        }
        app.current_log.insert(
            0,
            LogLine {
                text: group.clone(),
                fg: Some(CATPPUCCIN_MOCHA.primary),
                indent: 0,
                group: Some(group),
                kind: LogKind::Header,
            },
        );
    }
    if app.log_follow {
        app.log_scroll = log_bottom_scroll(app, app.log_viewport_height.max(1));
    }
}

pub(in crate::tui) fn scroll_run_log(app: &mut App, pages: isize) {
    let viewport_height = app.log_viewport_height.max(1);
    scroll_run_log_lines(app, pages.saturating_mul(viewport_height as isize));
}

pub(in crate::tui) fn scroll_run_log_lines(app: &mut App, lines: isize) {
    let viewport_height = app.log_viewport_height.max(1);
    if app.log_follow {
        app.log_scroll = log_bottom_scroll(app, viewport_height);
    }
    app.log_follow = false;
    if lines.is_negative() {
        app.log_scroll = app.log_scroll.saturating_sub(lines.unsigned_abs());
    } else {
        app.log_scroll = app.log_scroll.saturating_add(lines as usize);
    }
    clamp_log_scroll(app, viewport_height);
}

pub(in crate::tui) fn log_scroll_offset(app: &App, viewport_height: usize) -> u16 {
    if app.log_follow {
        return log_bottom_scroll(app, viewport_height).min(u16::MAX as usize) as u16;
    }
    app.log_scroll
        .min(log_bottom_scroll(app, viewport_height))
        .min(u16::MAX as usize) as u16
}

pub(in crate::tui) fn log_bottom_scroll(app: &App, viewport_height: usize) -> usize {
    visible_log_len(app).saturating_sub(viewport_height)
}

pub(in crate::tui) fn visible_log_len(app: &App) -> usize {
    let base = if app.current_log.is_empty() {
        1
    } else {
        filtered_log_entries(app).len().max(1)
    };
    base + usize::from(app.log_dropped_count > 0)
}

pub(in crate::tui) fn clamp_log_scroll(app: &mut App, viewport_height: usize) {
    app.log_scroll = app.log_scroll.min(log_bottom_scroll(app, viewport_height));
}

pub(in crate::tui) fn visible_log_lines(app: &App, _viewport_height: usize) -> Vec<Line<'static>> {
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

pub(in crate::tui) fn filtered_log_entries(app: &App) -> Vec<&LogLine> {
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

pub(in crate::tui) fn current_log_filter_group(app: &App) -> Option<String> {
    app.active_log_group
        .clone()
        .or_else(|| app.log_group.clone())
}

pub(in crate::tui) fn error_log_entries(app: &App) -> Vec<&LogLine> {
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

pub(in crate::tui) fn log_entry_matches_filter(
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

pub(in crate::tui) fn is_error_log_entry(entry: &LogLine) -> bool {
    entry.kind == LogKind::Stderr
}

pub(in crate::tui) fn log_line_for_view(entry: &LogLine) -> Line<'static> {
    let style = entry.fg.map(|c| Style::default().fg(c)).unwrap_or_default();
    let indent = "  ".repeat(entry.indent);
    Line::styled(format!("{indent}{}", entry.text), style)
}

pub(in crate::tui) fn collapsed_log_header_line(entry: &LogLine) -> Line<'static> {
    let style = entry.fg.map(|c| Style::default().fg(c)).unwrap_or_default();
    Line::from(vec![
        Span::styled(entry.text.clone(), style),
        Span::styled(
            "  (collapsed)",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    ])
}

pub(in crate::tui) fn log_empty_filter_message(filter: LogFilter) -> &'static str {
    match filter {
        LogFilter::All => "log is empty; waiting for output",
        LogFilter::Current => "no log lines for current action",
        LogFilter::Errors => "no error log lines",
    }
}

pub(in crate::tui) fn toggle_current_log_group(app: &mut App) {
    let Some(group) = app.log_group.clone() else {
        return;
    };
    if !app.collapsed_log_groups.remove(&group) {
        app.collapsed_log_groups.insert(group);
    }
}

pub(in crate::tui) fn run_log_title(app: &App) -> String {
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

pub(in crate::tui) fn sanitize_tui_log_line(line: &str) -> String {
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
