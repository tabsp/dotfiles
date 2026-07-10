use super::*;

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
        NoticeKind::Success => "saved",
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
    let full = [("↑↓", " Scroll  "), ("r", " Run  "), ("q", " Back")];
    let short = [("↑↓", " "), ("r", " "), ("q", "")];
    let compact = [("↑↓", " "), ("r", " "), ("q", "")];

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
        let full = [("q", " Stopping")];
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
        ("Tab", " Filter  "),
        ("Enter", " Fold  "),
        ("f", " Follow  "),
        ("q", " Abort"),
    ];
    const FINISHED_FULL: &[(&str, &str)] = &[
        ("↑↓", " Scroll  "),
        ("Tab", " Filter  "),
        ("Enter", " Fold  "),
        ("q", " Back"),
    ];
    const RUNNING_FULL: &[(&str, &str)] = &[
        ("↑↓", " Scroll  "),
        ("Tab", " Filter  "),
        ("Enter", " Fold  "),
        ("q", " Abort"),
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
