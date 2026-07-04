//! Nerd Font icon set (user has Nerd Font installed).

// UI controls
pub const ICON_EXPANDED: &str = "▾";
pub const ICON_COLLAPSED: &str = "▸";

// Checkbox states
pub const ICON_CHECKED: &str = "▣";
pub const ICON_UNCHECKED: &str = "▢";

// Dirty / clean
pub const ICON_DIRTY: &str = "●";
pub const ICON_CLEAN: &str = "○";

// Step status
pub const ICON_OK: &str = "✓";
pub const ICON_FAIL: &str = "✗";
pub const ICON_SKIP: &str = "⊘";
pub const ICON_RUNNING: &str = "⏵";
pub const ICON_WARN: &str = "⚠";
pub const ICON_PENDING: &str = "⏸";
pub const ICON_RETRY: &str = "↻";

// Spinners (braille, 10 frames)
pub const SPINNER_BRAILLE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// Spinners (circle, 4 frames)
pub const SPINNER_CIRCLE: &[&str] = &["◐", "◓", "◑", "◒"];

// Misc
pub const ICON_GEAR: &str = "⚙";
pub const ICON_INFO: &str = "ⓘ";
pub const ICON_LAPTOP: &str = "\u{f035b}";

// Progress bar
pub const PROGRESS_FULL: &str = "█";
pub const PROGRESS_EMPTY: &str = "░";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_are_non_empty() {
        assert!(!ICON_EXPANDED.is_empty());
        assert!(!ICON_COLLAPSED.is_empty());
        assert!(!ICON_CHECKED.is_empty());
        assert!(!ICON_UNCHECKED.is_empty());
        assert!(!ICON_DIRTY.is_empty());
        assert!(!ICON_CLEAN.is_empty());
        assert!(!ICON_OK.is_empty());
        assert!(!ICON_FAIL.is_empty());
        assert!(!ICON_SKIP.is_empty());
        assert!(!ICON_RUNNING.is_empty());
        assert!(!ICON_WARN.is_empty());
        assert!(!ICON_PENDING.is_empty());
        assert!(!ICON_RETRY.is_empty());
        assert!(!ICON_GEAR.is_empty());
        assert!(!ICON_INFO.is_empty());
        assert!(!ICON_LAPTOP.is_empty());
        assert!(!PROGRESS_FULL.is_empty());
        assert!(!PROGRESS_EMPTY.is_empty());
    }

    #[test]
    fn spinners_have_expected_length() {
        assert_eq!(SPINNER_BRAILLE.len(), 10);
        assert_eq!(SPINNER_CIRCLE.len(), 4);
    }
}
