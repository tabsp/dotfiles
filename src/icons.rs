//! Icon sets for TUI rendering.
//!
//! `DOTMAN_ICONS=nerd` forces Nerd Font glyphs, `DOTMAN_ICONS=plain` uses
//! portable Unicode symbols. Without an override, fish shells default to Nerd
//! Font and other shells default to plain symbols.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconMode {
    Nerd,
    Plain,
}

pub struct IconSet {
    pub expanded: &'static str,
    pub collapsed: &'static str,
    pub selected: &'static str,
    pub unselected: &'static str,
    pub success: &'static str,
    pub failed: &'static str,
    pub skipped: &'static str,
    pub running: &'static str,
    pub warning: &'static str,
    pub pending: &'static str,
    pub retry: &'static str,
    pub app: &'static str,
    pub info: &'static str,
    pub host: &'static str,
    pub menu_deploy: &'static str,
    pub menu_plan: &'static str,
    pub menu_history: &'static str,
    pub menu_quit: &'static str,
}

pub const NERD: IconSet = IconSet {
    expanded: "\u{f47c}",     // nf-oct-chevron_down
    collapsed: "\u{f460}",    // nf-oct-chevron_right
    selected: "\u{f0132}",    // nf-md-checkbox_marked
    unselected: "\u{f0131}",  // nf-md-checkbox_blank_outline
    success: "\u{f058}",      // nf-fa-check_circle
    failed: "\u{f057}",       // nf-fa-times_circle
    skipped: "\u{f529}",      // nf-oct-skip
    running: "\u{f04b}",      // nf-fa-play
    warning: "\u{f071}",      // nf-fa-warning
    pending: "\u{f04c}",      // nf-fa-pause
    retry: "\u{f021}",        // nf-fa-refresh
    app: "\u{f013}",          // nf-fa-gear
    info: "\u{f05a}",         // nf-fa-info_circle
    host: "\u{f109}",         // nf-fa-laptop
    menu_deploy: "\u{f135}",  // nf-fa-rocket
    menu_plan: "\u{f0ae}",    // nf-fa-tasks
    menu_history: "\u{f1da}", // nf-fa-history
    menu_quit: "\u{f011}",    // nf-fa-power_off
};

pub const PLAIN: IconSet = IconSet {
    expanded: "▾",
    collapsed: "▸",
    selected: "■",
    unselected: "□",
    success: "✓",
    failed: "✗",
    skipped: "⊘",
    running: "▶",
    warning: "⚠",
    pending: "‖",
    retry: "↻",
    app: "⚙",
    info: "i",
    host: "⌂",
    menu_deploy: ">",
    menu_plan: "?",
    menu_history: "↺",
    menu_quit: "×",
};

// Spinners (braille, 10 frames)
pub const SPINNER_BRAILLE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// Spinners (circle, 4 frames)
pub const SPINNER_CIRCLE: &[&str] = &["◐", "◓", "◑", "◒"];

// Progress bar
pub const PROGRESS_FULL: &str = "█";
pub const PROGRESS_EMPTY: &str = "░";

pub fn current() -> &'static IconSet {
    match mode() {
        IconMode::Nerd => &NERD,
        IconMode::Plain => &PLAIN,
    }
}

pub fn mode() -> IconMode {
    if let Some(mode) = std::env::var("DOTMAN_ICONS")
        .ok()
        .and_then(|value| parse_mode(&value))
    {
        return mode;
    }
    if is_fish_shell() {
        IconMode::Nerd
    } else {
        IconMode::Plain
    }
}

fn parse_mode(value: &str) -> Option<IconMode> {
    match value.trim().to_ascii_lowercase().as_str() {
        "nerd" | "nerdfont" | "nerd-font" => Some(IconMode::Nerd),
        "plain" | "unicode" | "fallback" => Some(IconMode::Plain),
        _ => None,
    }
}

fn is_fish_shell() -> bool {
    std::env::var("SHELL")
        .ok()
        .and_then(|shell| {
            std::path::Path::new(&shell)
                .file_name()
                .map(|name| name.to_string_lossy().to_ascii_lowercase())
        })
        .is_some_and(|shell| shell == "fish")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_sets_are_non_empty() {
        for set in [&NERD, &PLAIN] {
            assert!(!set.expanded.is_empty());
            assert!(!set.collapsed.is_empty());
            assert!(!set.selected.is_empty());
            assert!(!set.unselected.is_empty());
            assert!(!set.success.is_empty());
            assert!(!set.failed.is_empty());
            assert!(!set.skipped.is_empty());
            assert!(!set.running.is_empty());
            assert!(!set.warning.is_empty());
            assert!(!set.pending.is_empty());
            assert!(!set.retry.is_empty());
            assert!(!set.app.is_empty());
            assert!(!set.info.is_empty());
            assert!(!set.host.is_empty());
            assert!(!set.menu_deploy.is_empty());
            assert!(!set.menu_plan.is_empty());
            assert!(!set.menu_history.is_empty());
            assert!(!set.menu_quit.is_empty());
        }
    }

    #[test]
    fn parses_icon_modes() {
        assert_eq!(parse_mode("nerd"), Some(IconMode::Nerd));
        assert_eq!(parse_mode("nerd-font"), Some(IconMode::Nerd));
        assert_eq!(parse_mode("plain"), Some(IconMode::Plain));
        assert_eq!(parse_mode("unicode"), Some(IconMode::Plain));
        assert_eq!(parse_mode("unknown"), None);
    }

    #[test]
    fn spinners_have_expected_length() {
        assert_eq!(SPINNER_BRAILLE.len(), 10);
        assert_eq!(SPINNER_CIRCLE.len(), 4);
    }
}
