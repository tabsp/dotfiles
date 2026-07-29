use super::*;

// ---------------- shared chrome ----------------

/// Apply the active background policy across the frame. Transparent themes use
/// `Color::Reset`; explicit solid themes provide an RGB canvas color.
pub(super) fn paint_background(f: &mut Frame) {
    f.render_widget(
        Block::default().style(Style::default().bg(CATPPUCCIN_MOCHA.bg)),
        f.area(),
    );
}

/// Panel/card layer. It remains transparent when the active theme does.
pub(super) fn surface_panel() -> Block<'static> {
    Block::default().style(Style::default().bg(CATPPUCCIN_MOCHA.surface))
}

pub(super) fn screen_header_prefix(icon: &str, name: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(
            format!("{icon}  "),
            Style::default().fg(CATPPUCCIN_MOCHA.primary),
        ),
        Span::styled("dotman", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(" · ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(
            name.to_string(),
            Style::default()
                .fg(CATPPUCCIN_MOCHA.fg)
                .add_modifier(Modifier::BOLD),
        ),
    ]
}

pub(super) fn screen_header(
    icon: &str,
    name: &str,
    meta: Vec<Span<'static>>,
    width: usize,
) -> Line<'static> {
    let mut spans = screen_header_prefix(icon, name);
    if !meta.is_empty() {
        spans.push(Span::raw("  "));
        spans.extend(meta);
    }
    let used: usize = spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum();
    let rule = width.saturating_sub(used + 1);
    if rule > 0 {
        spans.push(Span::styled(
            format!(" {}", "─".repeat(rule)),
            Style::default().fg(CATPPUCCIN_MOCHA.border_subtle),
        ));
    }
    Line::from(spans)
}

pub(super) struct FooterNotice<'a> {
    pub message: &'a str,
    pub kind: NoticeKind,
}

/// First footer line: transient notice. Info messages stay quiet (message
/// only); warnings and errors get an explicit bold label.
pub(super) fn footer_notice_line(notice: Option<FooterNotice<'_>>, width: usize) -> Line<'static> {
    let Some(notice) = notice.filter(|notice| !notice.message.is_empty()) else {
        return Line::from("");
    };
    let style = notice_style(notice.kind);
    if matches!(notice.kind, NoticeKind::Info) {
        return Line::from(Span::styled(
            fit_to_width(&format!("  {}", notice.message), width),
            style,
        ));
    }
    let prefix = format!("  {}  ", notice_label(notice.kind));
    Line::from(vec![
        Span::styled(prefix.clone(), style.add_modifier(Modifier::BOLD)),
        Span::styled(
            fit_to_width(notice.message, width.saturating_sub(display_width(&prefix))),
            style,
        ),
    ])
}

pub(super) fn screen_footer(
    notice: Option<FooterNotice<'_>>,
    help: Line<'static>,
    width: usize,
) -> Paragraph<'static> {
    let lines = if notice
        .as_ref()
        .is_some_and(|notice| !notice.message.is_empty())
    {
        vec![footer_notice_line(notice, width), help]
    } else {
        vec![help]
    };
    Paragraph::new(lines).style(Style::default().bg(CATPPUCCIN_MOCHA.surface))
}

pub(super) fn footer_height(message: &str) -> u16 {
    if message.is_empty() { 1 } else { 2 }
}

pub(super) fn vim_prefix_help(app: &App, help: Line<'static>, width: usize) -> Line<'static> {
    if !app.vim_pending_g {
        return help;
    }
    let prefix = vec![
        Span::styled(
            "g…",
            Style::default()
                .fg(CATPPUCCIN_MOCHA.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];
    let prefix_width = 4;
    if prefix_width + line_display_width(&help) > width {
        return Line::from(prefix);
    }
    let mut spans = prefix;
    spans.extend(help.spans);
    Line::from(spans)
}

pub(super) fn scroll_marker_line(above: bool, count: usize, width: usize) -> Line<'static> {
    let arrow = if above { "↑" } else { "↓" };
    Line::from(Span::styled(
        fit_to_width(&format!("  {arrow} {count} more"), width),
        Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
    ))
}

/// Group header: icon + label in bold group color, count dimmed.
pub(super) fn group_header_line(
    icon: &str,
    label: &str,
    count: usize,
    color: Color,
    width: usize,
) -> Line<'static> {
    let head = format!("{icon} {label}");
    let tail = format!(" ({count})");
    let padding = width.saturating_sub(display_width(&head) + display_width(&tail));
    Line::from(vec![
        Span::styled(
            head,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(tail, Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::raw(" ".repeat(padding)),
    ])
}

/// "2026-07-05T12:00:00+08:00" -> "Jul 05 12:00"; falls back to the date part.
pub(super) fn short_datetime(rfc3339: &str) -> String {
    let fallback = || rfc3339.split('T').next().unwrap_or(rfc3339).to_string();
    let Ok(parsed) =
        time::OffsetDateTime::parse(rfc3339, &time::format_description::well_known::Rfc3339)
    else {
        return fallback();
    };
    static FORMAT: &[time::format_description::FormatItem<'static>] =
        time::macros::format_description!("[month repr:short] [day] [hour]:[minute]");
    parsed.format(FORMAT).unwrap_or_else(|_| fallback())
}

pub(super) fn relative_datetime(rfc3339: &str) -> String {
    let Ok(parsed) =
        time::OffsetDateTime::parse(rfc3339, &time::format_description::well_known::Rfc3339)
    else {
        return short_datetime(rfc3339);
    };
    let elapsed = time::OffsetDateTime::now_utc() - parsed;
    if elapsed.is_negative() {
        return short_datetime(rfc3339);
    }
    let seconds = elapsed.whole_seconds();
    if seconds < 60 {
        "just now".into()
    } else if seconds < 3_600 {
        format!("{}m ago", seconds / 60)
    } else if seconds < 86_400 {
        format!("{}h ago", seconds / 3_600)
    } else if seconds < 2_592_000 {
        format!("{}d ago", seconds / 86_400)
    } else {
        short_datetime(rfc3339)
    }
}

pub(super) fn short_run_id(id: &str) -> String {
    let mut chars = id.chars();
    let short = chars.by_ref().take(8).collect::<String>();
    if chars.next().is_some() {
        format!("{short}…")
    } else {
        short
    }
}

pub(super) fn format_elapsed(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {:02}s", secs / 60, secs % 60)
    } else {
        format!("{}h {:02}m", secs / 3600, (secs % 3600) / 60)
    }
}

pub(super) fn plan_mode_label(mode: PlanMode) -> &'static str {
    match mode {
        PlanMode::Deploy => "deploy",
    }
}

pub(super) fn mode_label(mode: &Mode) -> &'static str {
    match mode {
        Mode::Menu => "menu",
        Mode::Deploy => "deploy",
        Mode::Plan => "plan",
        Mode::History => "history",
        Mode::Run(_) => "run",
    }
}

pub(super) fn span_bg_style(bg: Option<Color>) -> Style {
    if let Some(bg) = bg {
        Style::default().bg(bg)
    } else {
        Style::default()
    }
}

pub(super) fn focus_bg() -> Color {
    CATPPUCCIN_MOCHA.surface_active
}

pub(super) fn run_status_color(status: RunStatus) -> Color {
    match status {
        RunStatus::Running => CATPPUCCIN_MOCHA.running,
        RunStatus::Success => CATPPUCCIN_MOCHA.success,
        RunStatus::Failed => CATPPUCCIN_MOCHA.danger,
        RunStatus::Aborted => CATPPUCCIN_MOCHA.warning,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LayoutDensity {
    Compact,
    Normal,
}

pub(super) fn layout_density(width: u16, height: u16) -> LayoutDensity {
    if width < 72 || height < 22 {
        LayoutDensity::Compact
    } else {
        LayoutDensity::Normal
    }
}

pub(super) fn divider_style() -> Style {
    Style::default().fg(CATPPUCCIN_MOCHA.divider)
}

pub(super) fn notice_style(kind: NoticeKind) -> Style {
    Style::default().fg(match kind {
        NoticeKind::Info => CATPPUCCIN_MOCHA.text_muted,
        NoticeKind::Success => CATPPUCCIN_MOCHA.success,
        NoticeKind::Warning => CATPPUCCIN_MOCHA.warning,
        NoticeKind::Error => CATPPUCCIN_MOCHA.danger,
    })
}

pub(super) fn notice_label(kind: NoticeKind) -> &'static str {
    match kind {
        NoticeKind::Info => "info",
        NoticeKind::Success => "done",
        NoticeKind::Warning => "warning",
        NoticeKind::Error => "error",
    }
}

pub(super) fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

pub(super) fn fit_to_width(value: &str, width: usize) -> String {
    let len = display_width(value);
    if len <= width {
        let mut out = value.to_string();
        out.push_str(&" ".repeat(width - len));
        return out;
    }
    if width == 0 {
        return String::new();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let mut out = String::new();
    let target = width - 3;
    let mut used = 0;
    for ch in value.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if used + ch_width > target {
            break;
        }
        out.push(ch);
        used += ch_width;
    }
    out.push_str("...");
    out
}

pub(super) fn keycap(label: &'static str) -> Span<'static> {
    Span::styled(
        format!("[{label}]"),
        Style::default()
            .fg(CATPPUCCIN_MOCHA.text_muted)
            .add_modifier(Modifier::BOLD),
    )
}

pub(super) fn hint(label: &'static str) -> Span<'static> {
    Span::styled(label, Style::default().fg(CATPPUCCIN_MOCHA.fg_dim))
}

pub(super) fn plan_help_line(width: usize, read_only: bool) -> Line<'static> {
    let readonly_full = [
        ("↑↓", " Navigate  "),
        ("Space", " Toggle  "),
        ("s", " Save  "),
        ("q", " Back  "),
        ("read-only", ""),
    ];
    let readonly_compact = [
        ("↑↓", " "),
        ("Spc", " "),
        ("s", " "),
        ("q", " "),
        ("read-only", ""),
    ];
    let full = [
        ("↑↓", " Navigate  "),
        ("Space", " Toggle  "),
        ("s", " Save  "),
        ("r", " Review  "),
        ("q", " Back"),
    ];
    let short = [
        ("↑↓", " "),
        ("Space", " "),
        ("s", " "),
        ("r", " "),
        ("q", ""),
    ];
    let compact = [("↑↓", " "), ("Spc", " "), ("s", " "), ("r", " "), ("q", "")];

    let candidates: Vec<&[(&'static str, &'static str)]> = if read_only {
        vec![&readonly_full[..], &readonly_compact[..]]
    } else {
        vec![&full[..], &short[..], &compact[..]]
    };
    for parts in candidates {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }

    help_line_from_parts(&[("q", "")])
}

pub(super) fn review_help_line(width: usize) -> Line<'static> {
    let full = [
        ("↑↓", " Group  "),
        ("h/l", " Fold  "),
        ("Space", " Toggle  "),
        ("r", " Run  "),
        ("q", " Back"),
    ];
    let short = [
        ("↑↓", " "),
        ("h/l", " "),
        ("Spc", " "),
        ("r", " "),
        ("q", ""),
    ];
    let compact = [("h/l", " "), ("Spc", " "), ("r", " "), ("q", "")];

    for parts in [&full[..], &short[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }

    help_line_from_parts(&[("q", "")])
}

pub(super) fn run_help_line(
    width: usize,
    aborting: bool,
    finished: bool,
    log_follow: bool,
) -> Line<'static> {
    if aborting {
        let full = [("q/Esc", " Stopping")];
        let compact = [("q", "")];
        for parts in [&full[..], &compact[..]] {
            let line = help_line_from_parts(parts);
            if line_display_width(&line) <= width {
                return line;
            }
        }
        return help_line_from_parts(&[("q", "")]);
    }

    const PAUSED_FULL: &[(&str, &str)] = &[
        ("↑↓", " Scroll  "),
        ("h/l", " Filter  "),
        ("Enter", " Fold  "),
        ("f", " Follow  "),
        ("q/Esc", " Abort"),
    ];
    const FINISHED_FULL: &[(&str, &str)] = &[
        ("↑↓", " Scroll  "),
        ("h/l", " Filter  "),
        ("Enter", " Fold  "),
        ("q", " Back"),
    ];
    const RUNNING_FULL: &[(&str, &str)] = &[
        ("↑↓", " Scroll  "),
        ("h/l", " Filter  "),
        ("Enter", " Fold  "),
        ("q/Esc", " Abort"),
    ];
    const PAUSED_COMPACT: &[(&str, &str)] = &[("↑↓", " "), ("f", " "), ("q", "")];
    const COMPACT: &[(&str, &str)] = &[("↑↓", " "), ("q", "")];
    let full = if !log_follow {
        PAUSED_FULL
    } else if finished {
        FINISHED_FULL
    } else {
        RUNNING_FULL
    };
    let compact = if !log_follow { PAUSED_COMPACT } else { COMPACT };
    for parts in [full, compact] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }
    help_line_from_parts(&[("q", "")])
}

pub(super) fn help_line_from_parts(parts: &[(&'static str, &'static str)]) -> Line<'static> {
    let mut spans = Vec::with_capacity(parts.len() * 2);
    for (key, label) in parts {
        spans.push(keycap(key));
        if !label.is_empty() {
            spans.push(hint(label));
        }
    }
    Line::from(spans)
}

pub(super) fn line_display_width(line: &Line<'_>) -> usize {
    line.spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum()
}

pub(super) fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(ch) => ch.to_ascii_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}
