//! Catppuccin Mocha theme.
//!
//! Matches the user's ghostty / tmux config for visual consistency.

use ratatui::style::Color;

pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub surface_active: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub text_muted: Color,
    pub text_disabled: Color,
    pub primary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub running: Color,
    pub skip: Color,
    pub border_subtle: Color,
    pub border_strong: Color,
    pub focus_marker: Color,
    pub guide: Color,
    pub divider: Color,
}

pub const CATPPUCCIN_MOCHA: Theme = Theme {
    bg: Color::Rgb(30, 30, 46),      // #1e1e2e
    surface: Color::Rgb(24, 24, 37), // #181825 mantle
    surface_active: Color::Rgb(39, 36, 52),
    fg: Color::Rgb(205, 214, 244),          // #cdd6f4
    fg_dim: Color::Rgb(108, 112, 134),      // #6c7086
    text_muted: Color::Rgb(147, 153, 178),  // #9399b2 overlay2
    text_disabled: Color::Rgb(88, 91, 112), // #585b70 surface2
    primary: Color::Rgb(203, 166, 247),     // #cba6f7 mauve
    accent: Color::Rgb(137, 180, 250),      // #89b4fa blue
    success: Color::Rgb(166, 227, 161),     // #a6e3a1 green
    warning: Color::Rgb(249, 226, 175),     // #f9e2af yellow
    danger: Color::Rgb(243, 139, 168),      // #f38ba8 red
    running: Color::Rgb(250, 179, 135),     // #fab387 peach
    skip: Color::Rgb(88, 91, 112),          // #585b70 gray
    border_subtle: Color::Rgb(55, 57, 73),
    border_strong: Color::Rgb(108, 112, 134),
    focus_marker: Color::Rgb(203, 166, 247),
    guide: Color::Rgb(46, 48, 62),
    divider: Color::Rgb(55, 57, 73),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catppuccin_mocha_bg_matches_expected() {
        assert_eq!(CATPPUCCIN_MOCHA.bg, Color::Rgb(30, 30, 46));
    }

    #[test]
    fn catppuccin_mocha_fg_matches_expected() {
        assert_eq!(CATPPUCCIN_MOCHA.fg, Color::Rgb(205, 214, 244));
    }

    #[test]
    fn catppuccin_mocha_primary_matches_expected() {
        assert_eq!(CATPPUCCIN_MOCHA.primary, Color::Rgb(203, 166, 247));
    }

    #[test]
    fn catppuccin_mocha_success_matches_expected() {
        assert_eq!(CATPPUCCIN_MOCHA.success, Color::Rgb(166, 227, 161));
    }

    #[test]
    fn all_theme_colors_are_non_default() {
        assert_ne!(CATPPUCCIN_MOCHA.bg, Color::Reset);
        assert_ne!(CATPPUCCIN_MOCHA.fg, Color::Reset);
    }
}
