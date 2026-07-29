//! TUI module root: entry points, terminal loop, and screen dispatch.
//!
//! Phase 5+7 minimal: MainMenu, PlanView, RunView, HistoryView.

use crate::cli::Mode;
use crate::config;
use crate::icons;
use crate::model::{
    Action, ActionStatus, Mode as PlanMode, OutputStream, Plan, PlanItem, Run, RunAction, RunItem,
    RunStatus, Selection,
};
use crate::ops::clean;
use crate::ops::install;
use crate::ops::link::{self, LinkAction, LinkSettings};
use crate::ops::shell;
use crate::store;
use crate::theme::CATPPUCCIN_MOCHA;
use anyhow::Result;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use std::sync::{Arc, atomic::AtomicBool, mpsc};
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

mod app;
mod components;
mod history;
mod main_menu;
mod plan;
mod review;
mod run;
pub mod web_demo;

pub use app::{App, LogFilter, LogKind, LogLine, NoticeKind, RunThreadResult, Screen};
use components::*;

/// Entry point when a config path has already been resolved by init.rs.
pub fn run_with_config(config_path: std::path::PathBuf, mode: Mode) -> Result<(), String> {
    let (_guard, mut terminal) = setup_terminal_guarded().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    if let Err(e) = app.load_config_from(&config_path) {
        // Defer error to first render; user sees message.
        app.status_message = e;
        app.status_kind = NoticeKind::Error;
    }
    app::initialize_screen(&mut app);
    let res = run_event_loop(&mut app, &mut terminal);
    res.map_err(|e| e.to_string())
}

/// Entry point when no config is needed (menu, history).
pub fn run_no_config(mode: Mode) -> Result<(), String> {
    let (_guard, mut terminal) = setup_terminal_guarded().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    app::initialize_screen(&mut app);
    let res = run_event_loop(&mut app, &mut terminal);
    res.map_err(|e| e.to_string())
}

pub fn run(mode: Mode) -> Result<(), String> {
    let (_guard, mut terminal) = setup_terminal_guarded().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    if let Err(e) = app.load_config() {
        // Defer error to first render; user sees message.
        app.status_message = e;
        app.status_kind = NoticeKind::Error;
    }
    app::initialize_screen(&mut app);
    let res = run_event_loop(&mut app, &mut terminal);
    res.map_err(|e| e.to_string())
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = restore_terminal();
    }
}

fn setup_terminal_guarded() -> Result<(TerminalGuard, DefaultTerminal), io::Error> {
    enable_raw_mode()?;
    let guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let terminal = ratatui::Terminal::new(backend)?;
    Ok((guard, terminal))
}

fn setup_terminal() -> Result<DefaultTerminal, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    ratatui::Terminal::new(backend)
}

fn restore_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen)?;
    Ok(())
}

fn run_event_loop(app: &mut App, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut last_tick = Instant::now();
    let mut needs_draw = true;
    loop {
        if app.should_quit {
            return Ok(());
        }

        // Recreate the terminal backend if sudo -v tore it down.
        if app.needs_terminal_reset {
            let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
            *terminal = ratatui::Terminal::new(backend)?;
            app.needs_terminal_reset = false;
            needs_draw = true;
        }

        if matches!(app.screen, Screen::RunView) && run::drain_run_events(app) {
            needs_draw = true;
        }
        if matches!(app.screen, Screen::ConfirmView) && review::poll_review_thread(app) {
            needs_draw = true;
        }

        if needs_draw {
            terminal.draw(|f| render(app, f))?;
            needs_draw = false;
        }

        // Tick animation 100ms.
        if last_tick.elapsed() >= Duration::from_millis(100) {
            let key_prefix_expired = app.tick();
            last_tick = Instant::now();
            if matches!(app.screen, Screen::RunView) || key_prefix_expired {
                needs_draw = true;
            }
        }

        if event::poll(Duration::from_millis(50))?
            && let event = event::read()?
        {
            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key_event(app, key)?;
                    needs_draw = true;
                }
                Event::Resize(_, _) => {
                    needs_draw = true;
                }
                Event::Mouse(mouse)
                    if matches!(app.screen, Screen::RunView)
                        && matches!(
                            mouse.kind,
                            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
                        ) =>
                {
                    run::handle_run_mouse(app, mouse.kind);
                    needs_draw = true;
                }
                _ => {}
            }
        }
    }
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<()> {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        clear_vim_prefix(app);
        return match key.code {
            KeyCode::Char('c') => dispatch_key(app, KeyCode::Char('q')),
            KeyCode::Char('d') => half_page(app, 1),
            KeyCode::Char('u') => half_page(app, -1),
            KeyCode::Char('f') => dispatch_key(app, KeyCode::PageDown),
            KeyCode::Char('b') => dispatch_key(app, KeyCode::PageUp),
            _ => Ok(()),
        };
    }

    let key = key.code;
    if app.vim_pending_g {
        clear_vim_prefix(app);
        if key == KeyCode::Char('g') {
            return jump_to_top(app);
        }
        if key == KeyCode::Esc {
            return Ok(());
        }
    } else if key == KeyCode::Char('g') {
        app.vim_pending_g = true;
        app.vim_pending_g_since = Some(Instant::now());
        return Ok(());
    }
    if key == KeyCode::Char('G') {
        return jump_to_bottom(app);
    }
    if key == KeyCode::Home {
        return jump_to_top(app);
    }
    if key == KeyCode::End {
        return jump_to_bottom(app);
    }
    dispatch_key(app, key)
}

#[cfg(test)]
fn handle_key(app: &mut App, key: KeyCode) -> Result<()> {
    handle_key_event(app, KeyEvent::new(key, KeyModifiers::NONE))
}

fn clear_vim_prefix(app: &mut App) {
    app.vim_pending_g = false;
    app.vim_pending_g_since = None;
}

fn dispatch_key(app: &mut App, key: KeyCode) -> Result<()> {
    match app.screen {
        Screen::MainMenu => main_menu::handle_main_menu(app, key),
        Screen::PlanView => plan::handle_plan(app, key),
        Screen::ConfirmView => review::handle_confirm(app, key),
        Screen::RunView => run::handle_run(app, key),
        Screen::HistoryView => history::handle_history(app, key),
        Screen::RunReplay => history::handle_replay(app, key),
    }
}

fn half_page(app: &mut App, direction: isize) -> Result<()> {
    match app.screen {
        Screen::MainMenu => {
            for _ in 0..2 {
                main_menu::handle_main_menu(
                    app,
                    if direction < 0 {
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    },
                )?;
            }
        }
        Screen::PlanView => {
            for _ in 0..4 {
                plan::handle_plan(
                    app,
                    if direction < 0 {
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    },
                )?;
            }
        }
        Screen::ConfirmView => review::scroll_review(app, direction * 4),
        Screen::RunView => run::scroll_run_half_page(app, direction),
        Screen::HistoryView => {
            for _ in 0..4 {
                history::handle_history(
                    app,
                    if direction < 0 {
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    },
                )?;
            }
        }
        Screen::RunReplay => {
            for _ in 0..4 {
                history::handle_replay(
                    app,
                    if direction < 0 {
                        KeyCode::Up
                    } else {
                        KeyCode::Down
                    },
                )?;
            }
        }
    }
    Ok(())
}

fn jump_to_top(app: &mut App) -> Result<()> {
    match app.screen {
        Screen::MainMenu => app.menu_state.select(Some(0)),
        Screen::PlanView => plan::jump_plan_top(app),
        Screen::ConfirmView => review::jump_review_top(app),
        Screen::RunView => run::jump_run_top(app),
        Screen::HistoryView => history::jump_history_top(app),
        Screen::RunReplay => history::jump_replay_top(app),
    }
    Ok(())
}

fn jump_to_bottom(app: &mut App) -> Result<()> {
    match app.screen {
        Screen::MainMenu => app.menu_state.select(Some(3)),
        Screen::PlanView => plan::jump_plan_bottom(app),
        Screen::ConfirmView => review::jump_review_bottom(app),
        Screen::RunView => run::jump_run_bottom(app),
        Screen::HistoryView => history::jump_history_bottom(app),
        Screen::RunReplay => history::jump_replay_bottom(app),
    }
    Ok(())
}

// ---------------- render dispatch ----------------

fn render(app: &mut App, f: &mut Frame) {
    paint_background(f);
    match app.screen {
        Screen::MainMenu => main_menu::render_main_menu(f, app),
        Screen::PlanView => plan::render_plan(f, app),
        Screen::ConfirmView => review::render_confirm(f, app),
        Screen::RunView => run::render_run(f, app),
        Screen::HistoryView => history::render_history(f, app),
        Screen::RunReplay => history::render_replay(f, app),
    }
}

#[cfg(test)]
use plan::*;
#[cfg(test)]
use review::*;
#[cfg(test)]
use run::events::{apply_run_thread_result, drain_all_run_events, sync_finished_run_state};
#[cfg(test)]
use run::log::{
    log_bottom_scroll, log_scroll_offset, push_log, push_log_group, push_log_indented,
    sanitize_tui_log_line, scroll_run_log, toggle_current_log_group, visible_log_lines,
};
#[cfg(test)]
use run::view::{
    RunDisplayLine, RunGroup, action_kind_for_view, current_run_item_name, finished_run_for_view,
    grouped_run_lines, live_run_display_lines, ordered_run_lines, run_action_kind_icon,
    run_action_total, run_body_lines, run_executed_action_total, run_group_for_status,
    run_group_header_line, run_is_aborting, run_is_terminal, run_log_panel_height,
    run_progress_bar, run_status_icon, run_status_label_for_view, run_status_line, run_title,
    selected_run_action_total,
};
#[cfg(test)]
use run::*;
#[cfg(test)]
mod tests;
