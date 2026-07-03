//! Catppuccin Mocha theme.
//!
//! Matches the user's ghostty / tmux config for visual consistency.

use ratatui::style::Color;

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub primary: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub running: Color,
    pub skip: Color,
}

pub const CATPPUCCIN_MOCHA: Theme = Theme {
    bg: Color::Rgb(30, 30, 46),         // #1e1e2e
    fg: Color::Rgb(205, 214, 244),      // #cdd6f4
    fg_dim: Color::Rgb(108, 112, 134),  // #6c7086
    primary: Color::Rgb(203, 166, 247), // #cba6f7 mauve
    accent: Color::Rgb(137, 180, 250),  // #89b4fa blue
    success: Color::Rgb(166, 227, 161), // #a6e3a1 green
    warning: Color::Rgb(249, 226, 175), // #f9e2af yellow
    danger: Color::Rgb(243, 139, 168),  // #f38ba8 red
    running: Color::Rgb(250, 179, 135), // #fab387 peach
    skip: Color::Rgb(88, 91, 112),      // #585b70 gray
};
