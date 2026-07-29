//! Runtime-selectable TUI themes.
//!
//! `DOTMAN_THEME=auto` keeps the terminal background transparent while choosing
//! Mocha or Latte foreground colors when the background is known. It falls back
//! to terminal-safe ANSI colors otherwise. Explicit `mocha` and `latte` use
//! their solid backgrounds. `terminal` remains a transparent compatibility
//! alias.

use ratatui::style::Color;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

pub const MOCHA: Theme = Theme {
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

pub const LATTE: Theme = Theme {
    bg: Color::Rgb(239, 241, 245),
    surface: Color::Rgb(230, 233, 239),
    surface_active: Color::Rgb(220, 224, 232),
    fg: Color::Rgb(76, 79, 105),
    fg_dim: Color::Rgb(140, 143, 161),
    text_muted: Color::Rgb(108, 111, 133),
    text_disabled: Color::Rgb(156, 160, 176),
    primary: Color::Rgb(136, 57, 239),
    accent: Color::Rgb(30, 102, 245),
    success: Color::Rgb(64, 160, 43),
    warning: Color::Rgb(223, 142, 29),
    danger: Color::Rgb(210, 15, 57),
    running: Color::Rgb(254, 100, 11),
    skip: Color::Rgb(156, 160, 176),
    border_subtle: Color::Rgb(204, 208, 218),
    border_strong: Color::Rgb(140, 143, 161),
    focus_marker: Color::Rgb(136, 57, 239),
    guide: Color::Rgb(204, 208, 218),
    divider: Color::Rgb(204, 208, 218),
};

pub const ANSI: Theme = Theme {
    bg: Color::Reset,
    surface: Color::Reset,
    surface_active: Color::Reset,
    fg: Color::Reset,
    fg_dim: Color::DarkGray,
    text_muted: Color::Gray,
    text_disabled: Color::DarkGray,
    primary: Color::LightMagenta,
    accent: Color::LightBlue,
    success: Color::LightGreen,
    warning: Color::LightYellow,
    danger: Color::LightRed,
    running: Color::LightCyan,
    skip: Color::DarkGray,
    border_subtle: Color::DarkGray,
    border_strong: Color::Gray,
    focus_marker: Color::LightMagenta,
    guide: Color::DarkGray,
    divider: Color::DarkGray,
};

pub const NO_COLOR_THEME: Theme = Theme {
    bg: Color::Reset,
    surface: Color::Reset,
    surface_active: Color::Reset,
    fg: Color::Reset,
    fg_dim: Color::Reset,
    text_muted: Color::Reset,
    text_disabled: Color::Reset,
    primary: Color::Reset,
    accent: Color::Reset,
    success: Color::Reset,
    warning: Color::Reset,
    danger: Color::Reset,
    running: Color::Reset,
    skip: Color::Reset,
    border_subtle: Color::Reset,
    border_strong: Color::Reset,
    focus_marker: Color::Reset,
    guide: Color::Reset,
    divider: Color::Reset,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeChoice {
    Mocha,
    Latte,
    Ansi,
    NoColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackgroundChoice {
    Transparent,
    Solid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ThemeSelection {
    palette: ThemeChoice,
    background: BackgroundChoice,
}

/// Kept under the original name so existing render code remains compact.
/// Unlike the old constant, this is the active runtime palette.
pub static CATPPUCCIN_MOCHA: LazyLock<Theme> = LazyLock::new(active_theme);

fn active_theme() -> Theme {
    // Website fixtures are product screenshots, not an interactive terminal;
    // keep them deterministic even when the export process inherits NO_COLOR.
    if std::env::args()
        .next()
        .and_then(|path| {
            std::path::Path::new(&path)
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .is_some_and(|name| name == "dotman-web-export")
    {
        return MOCHA;
    }
    let requested = std::env::var("DOTMAN_THEME").ok();
    let colorfgbg = std::env::var("COLORFGBG").ok();
    let no_color = std::env::var_os("NO_COLOR").is_some();
    let selection = resolve_theme(requested.as_deref(), colorfgbg.as_deref(), no_color);
    theme_for_selection(selection)
}

fn theme_for_selection(selection: ThemeSelection) -> Theme {
    let theme = match selection.palette {
        ThemeChoice::Mocha => MOCHA,
        ThemeChoice::Latte => LATTE,
        ThemeChoice::Ansi => ANSI,
        ThemeChoice::NoColor => NO_COLOR_THEME,
    };
    match selection.background {
        BackgroundChoice::Transparent => transparent(theme),
        BackgroundChoice::Solid => theme,
    }
}

fn transparent(mut theme: Theme) -> Theme {
    theme.bg = Color::Reset;
    theme.surface = Color::Reset;
    theme.surface_active = Color::Reset;
    theme
}

fn resolve_theme(
    requested: Option<&str>,
    colorfgbg: Option<&str>,
    no_color: bool,
) -> ThemeSelection {
    if no_color {
        return ThemeSelection {
            palette: ThemeChoice::NoColor,
            background: BackgroundChoice::Transparent,
        };
    }
    match requested
        .unwrap_or("auto")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "mocha" | "dark" => ThemeSelection {
            palette: ThemeChoice::Mocha,
            background: BackgroundChoice::Solid,
        },
        "latte" | "light" => ThemeSelection {
            palette: ThemeChoice::Latte,
            background: BackgroundChoice::Solid,
        },
        "ansi" | "16" => ThemeSelection {
            palette: ThemeChoice::Ansi,
            background: BackgroundChoice::Transparent,
        },
        "terminal" | "native" | "none" | "auto" => ThemeSelection {
            palette: auto_palette(colorfgbg),
            background: BackgroundChoice::Transparent,
        },
        _ => ThemeSelection {
            palette: auto_palette(colorfgbg),
            background: BackgroundChoice::Transparent,
        },
    }
}

fn auto_palette(colorfgbg: Option<&str>) -> ThemeChoice {
    let background = colorfgbg
        .and_then(|value| value.rsplit(';').next())
        .and_then(|value| value.parse::<u8>().ok());
    match background {
        Some(0..=6 | 8) => ThemeChoice::Mocha,
        Some(_) => ThemeChoice::Latte,
        // Unknown terminal backgrounds cannot safely use either Catppuccin
        // foreground palette with transparency. Preserve the native foreground
        // and use portable ANSI semantic accents instead.
        None => ThemeChoice::Ansi,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catppuccin_mocha_bg_matches_expected() {
        assert_eq!(MOCHA.bg, Color::Rgb(30, 30, 46));
    }

    #[test]
    fn catppuccin_mocha_fg_matches_expected() {
        assert_eq!(MOCHA.fg, Color::Rgb(205, 214, 244));
    }

    #[test]
    fn catppuccin_mocha_primary_matches_expected() {
        assert_eq!(MOCHA.primary, Color::Rgb(203, 166, 247));
    }

    #[test]
    fn catppuccin_mocha_success_matches_expected() {
        assert_eq!(MOCHA.success, Color::Rgb(166, 227, 161));
    }

    #[test]
    fn all_theme_colors_are_non_default() {
        assert_ne!(MOCHA.bg, Color::Reset);
        assert_ne!(MOCHA.fg, Color::Reset);
    }

    #[test]
    fn no_color_disables_palette_and_background() {
        assert_eq!(
            resolve_theme(Some("mocha"), Some("15;0"), true),
            ThemeSelection {
                palette: ThemeChoice::NoColor,
                background: BackgroundChoice::Transparent,
            }
        );
    }

    #[test]
    fn auto_theme_uses_terminal_safe_ansi_when_background_is_unknown() {
        assert_eq!(
            resolve_theme(None, None, false),
            ThemeSelection {
                palette: ThemeChoice::Ansi,
                background: BackgroundChoice::Transparent,
            }
        );
        assert_eq!(theme_for_selection(resolve_theme(None, None, false)), ANSI);
    }

    #[test]
    fn auto_theme_detects_palette_without_filling_background() {
        assert_eq!(
            resolve_theme(Some("auto"), Some("15;0"), false),
            ThemeSelection {
                palette: ThemeChoice::Mocha,
                background: BackgroundChoice::Transparent,
            }
        );
        assert_eq!(
            resolve_theme(Some("auto"), Some("0;15"), false),
            ThemeSelection {
                palette: ThemeChoice::Latte,
                background: BackgroundChoice::Transparent,
            }
        );
    }

    #[test]
    fn explicit_catppuccin_themes_keep_solid_backgrounds() {
        assert_eq!(
            resolve_theme(Some("mocha"), None, false),
            ThemeSelection {
                palette: ThemeChoice::Mocha,
                background: BackgroundChoice::Solid,
            }
        );
        assert_eq!(
            resolve_theme(Some("latte"), None, false),
            ThemeSelection {
                palette: ThemeChoice::Latte,
                background: BackgroundChoice::Solid,
            }
        );
    }

    #[test]
    fn transparent_theme_keeps_semantic_colors() {
        let theme = transparent(MOCHA);
        assert_eq!(theme.bg, Color::Reset);
        assert_eq!(theme.surface, Color::Reset);
        assert_eq!(theme.surface_active, Color::Reset);
        assert_eq!(theme.primary, MOCHA.primary);
        assert_eq!(theme.success, MOCHA.success);
    }

    #[test]
    fn auto_theme_produces_a_transparent_catppuccin_palette() {
        let selection = resolve_theme(Some("auto"), Some("15;0"), false);
        let theme = theme_for_selection(selection);

        assert_eq!(theme.bg, Color::Reset);
        assert_eq!(theme.surface, Color::Reset);
        assert_eq!(theme.surface_active, Color::Reset);
        assert_eq!(theme.fg, MOCHA.fg);
        assert_eq!(theme.primary, MOCHA.primary);
    }

    #[test]
    fn terminal_is_an_alias_for_transparent_auto_not_a_palette() {
        let auto = resolve_theme(Some("auto"), Some("0;15"), false);
        let terminal = resolve_theme(Some("terminal"), Some("0;15"), false);

        assert_eq!(terminal, auto);
        assert_eq!(theme_for_selection(terminal), transparent(LATTE));
    }
}
