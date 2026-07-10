//! TUI module root: entry points, terminal loop, and screen dispatch.
//!
//! Phase 5+7 minimal: MainMenu, PlanView, RunView, HistoryView.

use crate::cli::Mode;
use crate::config;
use crate::execute::MAX_TUI_OUTPUT_LINES;
use crate::icons;
use crate::model::{
    Action, ActionStatus, Mode as PlanMode, Plan, PlanItem, Run, RunAction, RunItem, RunStatus,
    Selection,
};
use crate::ops::clean;
use crate::ops::install;
use crate::ops::link::{self, LinkAction, LinkSettings};
use crate::ops::shell;
use crate::store;
use crate::theme::CATPPUCCIN_MOCHA;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
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

pub use app::{App, LogLine, Screen};
use components::*;

/// Entry point when a config path has already been resolved by init.rs.
pub fn run_with_config(config_path: std::path::PathBuf, mode: Mode) -> Result<(), String> {
    let mut terminal = setup_terminal().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    if let Err(e) = app.load_config_from(&config_path) {
        // Defer error to first render; user sees message.
        app.status_message = e;
    }
    app::initialize_screen(&mut app);
    let res = run_event_loop(&mut app, &mut terminal);
    restore_terminal().map_err(|e| e.to_string())?;
    res.map_err(|e| e.to_string())
}

/// Entry point when no config is needed (menu, history).
pub fn run_no_config(mode: Mode) -> Result<(), String> {
    let mut terminal = setup_terminal().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    app::initialize_screen(&mut app);
    let res = run_event_loop(&mut app, &mut terminal);
    restore_terminal().map_err(|e| e.to_string())?;
    res.map_err(|e| e.to_string())
}

pub fn run(mode: Mode) -> Result<(), String> {
    let mut terminal = setup_terminal().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    if let Err(e) = app.load_config() {
        // Defer error to first render; user sees message.
        app.status_message = e;
    }
    app::initialize_screen(&mut app);
    let res = run_event_loop(&mut app, &mut terminal);
    restore_terminal().map_err(|e| e.to_string())?;
    res.map_err(|e| e.to_string())
}

fn setup_terminal() -> Result<DefaultTerminal, io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    ratatui::Terminal::new(backend)
}

fn restore_terminal() -> Result<(), io::Error> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

fn run_event_loop(app: &mut App, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut last_tick = Instant::now();
    loop {
        if app.should_quit {
            return Ok(());
        }

        // Recreate the terminal backend if sudo -v tore it down.
        if app.needs_terminal_reset {
            let backend = ratatui::backend::CrosstermBackend::new(io::stdout());
            *terminal = ratatui::Terminal::new(backend)?;
            app.needs_terminal_reset = false;
        }

        terminal.draw(|f| render(app, f))?;

        // Tick animation 100ms.
        if last_tick.elapsed() >= Duration::from_millis(100) {
            app.tick();
            last_tick = Instant::now();
        }

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            handle_key(app, key.code)?;
        }
    }
}

fn handle_key(app: &mut App, key: KeyCode) -> Result<()> {
    match app.screen {
        Screen::MainMenu => main_menu::handle_main_menu(app, key),
        Screen::PlanView => plan::handle_plan(app, key),
        Screen::ConfirmView => review::handle_confirm(app, key),
        Screen::RunView => run::handle_run(app, key),
        Screen::HistoryView => history::handle_history(app, key),
        Screen::RunReplay => history::handle_replay(app, key),
    }
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
use review::*;
#[cfg(test)]
use run::*;
#[cfg(test)]
mod tests;
