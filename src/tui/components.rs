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
        ("Enter", " Expand  "),
        ("S", " Save  "),
        ("Q", " Back  "),
        ("read-only", ""),
    ];
    let readonly_compact = [
        ("↑↓", " "),
        ("Spc", " "),
        ("Ent", " "),
        ("S", " "),
        ("Q", " "),
        ("read-only", ""),
    ];
    let full = [
        ("↑↓", " Navigate  "),
        ("Space", " Toggle  "),
        ("Enter", " Expand  "),
        ("1-6", " Jump  "),
        ("S", " Save  "),
        ("R", " Review  "),
        ("Q", " Back"),
    ];
    let short = [
        ("↑↓", " "),
        ("Space", " "),
        ("Enter", " "),
        ("1-6", " "),
        ("S", " "),
        ("R", " "),
        ("Q", ""),
    ];
    let compact = [
        ("↑↓", " "),
        ("Spc", " "),
        ("Ent", " "),
        ("1-6", " "),
        ("S", " "),
        ("R", " "),
        ("Q", ""),
    ];

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

    help_line_from_parts(&[("Q", "")])
}

pub(super) fn review_help_line(width: usize) -> Line<'static> {
    let full = [
        ("↑↓/j/k", " Scroll  "),
        ("Pg", " Page  "),
        ("Enter/R", " Run  "),
        ("E/Q", " Edit"),
    ];
    let short = [("↑↓/jk", " "), ("Pg", " "), ("Enter/R", " "), ("E/Q", "")];
    let compact = [("↑↓", " "), ("Pg", " "), ("R", " "), ("E/Q", "")];

    for parts in [&full[..], &short[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }

    help_line_from_parts(&[("Q", "")])
}

pub(super) fn run_help_line(
    width: usize,
    aborting: bool,
    finished: bool,
    log_follow: bool,
) -> Line<'static> {
    if aborting {
        let full = [("Q/Esc", " Stopping")];
        let compact = [("Q", "")];
        for parts in [&full[..], &compact[..]] {
            let line = help_line_from_parts(parts);
            if line_display_width(&line) <= width {
                return line;
            }
        }
        return help_line_from_parts(&[("Q", "")]);
    }

    const PAUSED_FULL: &[(&str, &str)] = &[
        ("Pg/Home/End", " Log  "),
        ("Tab", " Filter  "),
        ("Enter", " Fold  "),
        ("F", " Follow  "),
        ("Q/Esc", " Back"),
    ];
    const FINISHED_FULL: &[(&str, &str)] = &[
        ("Pg/Home/End", " Log  "),
        ("Tab", " Filter  "),
        ("Enter", " Fold  "),
        ("Q/Esc", " Back"),
    ];
    const RUNNING_FULL: &[(&str, &str)] = &[
        ("Pg/Home/End", " Log  "),
        ("Tab", " Filter  "),
        ("Enter", " Fold  "),
        ("Q/Esc", " Abort"),
    ];
    const PAUSED_COMPACT: &[(&str, &str)] = &[("Pg", " "), ("F", " "), ("Q", "")];
    const COMPACT: &[(&str, &str)] = &[("Pg", " "), ("Q", "")];
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
    help_line_from_parts(&[("Q", "")])
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
