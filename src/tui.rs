//! TUI module root: entry points, terminal loop, and screen dispatch.
//!
//! Phase 5+7 minimal: MainMenu, PlanView, RunView, HistoryView.

use crate::cli::Mode;
use crate::config;
use crate::execute::MAX_TUI_OUTPUT_LINES;
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
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::collections::BTreeSet;
use std::io;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

mod app;
mod components;
mod history;
mod main_menu;
mod plan;
mod review;
mod run;

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
            app.tick();
            last_tick = Instant::now();
            if matches!(app.screen, Screen::RunView) {
                needs_draw = true;
            }
        }

        if event::poll(Duration::from_millis(50))?
            && let event = event::read()?
        {
            match event {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    handle_key(app, key.code)?;
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

fn handle_key(app: &mut App, key: KeyCode) -> Result<()> {
    if app.vim_pending_g {
        app.vim_pending_g = false;
        if key == KeyCode::Char('g') {
            return jump_to_top(app);
        }
    } else if key == KeyCode::Char('g') {
        app.vim_pending_g = true;
        return Ok(());
    }
    if key == KeyCode::Char('G') {
        return jump_to_bottom(app);
    }
    match app.screen {
        Screen::MainMenu => main_menu::handle_main_menu(app, key),
        Screen::PlanView => plan::handle_plan(app, key),
        Screen::ConfirmView => review::handle_confirm(app, key),
        Screen::RunView => run::handle_run(app, key),
        Screen::HistoryView => history::handle_history(app, key),
        Screen::RunReplay => history::handle_replay(app, key),
    }
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
    let area = f.area();
    f.render_widget(Clear, area);
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
use run::*;
#[cfg(test)]
mod tests;
