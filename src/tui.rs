//! TUI: app + all screens in one file.
//!
//! Phase 5+7 minimal: MainMenu, PlanView, RunView, ResultView, HistoryView.

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
use crate::plan;
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

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    MainMenu,
    PlanView,
    ConfirmView,
    RunView,
    ResultView,
    HistoryView,
    RunReplay,
}

#[derive(Debug, Clone)]
enum PlanRow {
    Header {
        layer: String,
        ordinal: usize,
        enabled: usize,
        total: usize,
    },
    Item(usize),
    InlineItems(Vec<usize>),
    Divider,
}

const GRID_COLUMNS: usize = 3;

pub struct App {
    pub screen: Screen,
    pub mode: Mode,
    pub config: Option<config::Config>,
    pub plan: Option<Plan>,
    pub run: Option<Run>,
    pub runs: Vec<Run>,
    review_entries: Vec<ReviewEntry>,
    review_scroll: usize,
    pub list_state: ListState,
    pub grid_col: usize,
    pub plan_columns: usize,
    pub collapsed_layers: BTreeSet<String>,
    pub status_message: String,
    pub status_is_focus_info: bool,
    pub should_quit: bool,
    pub dirty: bool,
    // For RunView
    pub spinner_frame: usize,
    pub run_thread: Option<std::thread::JoinHandle<anyhow::Result<Run>>>,
    pub run_events: Option<mpsc::Receiver<crate::execute::ExecuteEvent>>,
    pub abort_flag: Option<Arc<AtomicBool>>,
    pub progress: (usize, usize), // (done, total)
    pub current_log: Vec<LogLine>,
    pub current_item: Option<usize>,
    pub current_action: Option<(usize, usize)>,
    pub run_item_statuses: Vec<Option<ActionStatus>>,
    pub run_action_statuses: Vec<Vec<Option<ActionStatus>>>,
    pub run_started: Option<Instant>,
    /// Set to true after `sudo -v` restores the terminal; signals the event
    /// loop to recreate the Terminal backend on the next tick.
    pub needs_terminal_reset: bool,
}

/// A single log line with optional color.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub text: String,
    pub fg: Option<Color>,
}

impl App {
    pub fn new(mode: Mode) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            screen: Screen::MainMenu,
            mode,
            config: None,
            plan: None,
            run: None,
            runs: Vec::new(),
            review_entries: Vec::new(),
            review_scroll: 0,
            list_state,
            grid_col: 0,
            plan_columns: GRID_COLUMNS,
            collapsed_layers: BTreeSet::new(),
            status_message: String::new(),
            status_is_focus_info: false,
            should_quit: false,
            dirty: false,
            spinner_frame: 0,
            run_thread: None,
            run_events: None,
            abort_flag: None,
            progress: (0, 0),
            current_log: Vec::new(),
            current_item: None,
            current_action: None,
            run_item_statuses: Vec::new(),
            run_action_statuses: Vec::new(),
            run_started: None,
            needs_terminal_reset: false,
        }
    }

    pub fn load_config(&mut self) -> Result<(), String> {
        let path = if std::path::Path::new("dotman.yaml").exists() {
            std::path::PathBuf::from("dotman.yaml")
        } else if let Ok(Some(p)) = crate::profile::active_config_path() {
            p
        } else {
            return Err("no dotman.yaml found in current directory or active profile".into());
        };
        let cfg = config::load(&path).map_err(|e| e.to_string())?;
        self.config = Some(cfg);
        Ok(())
    }

    pub fn load_config_from(&mut self, config_path: &std::path::Path) -> Result<(), String> {
        let cfg = config::load(config_path).map_err(|e| e.to_string())?;
        self.config = Some(cfg);
        Ok(())
    }

    pub fn build_plan(&mut self) -> Result<(), String> {
        let cfg = self.config.as_ref().ok_or("config not loaded")?;
        let plan_mode = match self.mode {
            Mode::Menu => PlanMode::Deploy,
            Mode::Deploy | Mode::Plan => PlanMode::Deploy,
            _ => PlanMode::Deploy,
        };
        let mut plan = plan::build(cfg, plan_mode).map_err(|e| e.to_string())?;
        apply_saved_selection(&mut plan)?;
        plan.sync_auto_steps();
        self.plan = Some(plan);
        self.review_entries.clear();
        self.review_scroll = 0;
        select_first_plan_row(
            &mut self.list_state,
            self.plan.as_ref(),
            &self.collapsed_layers,
            self.plan_columns,
        );
        self.dirty = false;
        Ok(())
    }

    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % icons::SPINNER_BRAILLE.len();
    }
}

/// Entry point when a config path has already been resolved by init.rs.
pub fn run_with_config(config_path: std::path::PathBuf, mode: Mode) -> Result<(), String> {
    let mut terminal = setup_terminal().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    if let Err(e) = app.load_config_from(&config_path) {
        // Defer error to first render; user sees message.
        app.status_message = e;
    }
    initialize_screen(&mut app);
    let res = run_event_loop(&mut app, &mut terminal);
    restore_terminal().map_err(|e| e.to_string())?;
    res.map_err(|e| e.to_string())
}

/// Entry point when no config is needed (menu, history).
pub fn run_no_config(mode: Mode) -> Result<(), String> {
    let mut terminal = setup_terminal().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    initialize_screen(&mut app);
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
    initialize_screen(&mut app);
    let res = run_event_loop(&mut app, &mut terminal);
    restore_terminal().map_err(|e| e.to_string())?;
    res.map_err(|e| e.to_string())
}

fn initialize_screen(app: &mut App) {
    match app.mode.clone() {
        Mode::Menu => {
            app.runs = store::list().unwrap_or_default();
        }
        Mode::Deploy | Mode::Plan => {
            if let Err(e) = app.build_plan() {
                app.status_message = e;
            }
            app.screen = Screen::PlanView;
        }
        Mode::History => {
            app.runs = store::list().unwrap_or_default();
            app.screen = Screen::HistoryView;
        }
        Mode::Run(id) => match store::load(&id) {
            Ok(run) => {
                app.run = Some(run);
                app.screen = Screen::RunReplay;
            }
            Err(e) => {
                app.status_message = e.to_string();
                app.screen = Screen::HistoryView;
            }
        },
    }
}

fn apply_saved_selection(plan: &mut Plan) -> Result<(), String> {
    let selection = store::load_selection().map_err(|e| e.to_string())?;
    for item in &mut plan.items {
        if let Some(selected) = selection.items.get(&item.id) {
            item.selected = *selected;
        }
    }
    Ok(())
}

fn save_current_selection(app: &mut App) -> Result<(), String> {
    let plan = app.plan.as_ref().ok_or("no plan loaded")?;
    let selection = Selection {
        items: plan
            .items
            .iter()
            .map(|item| (item.id.clone(), item.selected))
            .collect(),
    };
    let path = store::save_selection(&selection).map_err(|e| e.to_string())?;
    app.dirty = false;
    app.status_message = format!("saved selection to {}", path.display());
    app.status_is_focus_info = false;
    Ok(())
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
        Screen::MainMenu => handle_main_menu(app, key),
        Screen::PlanView => handle_plan(app, key),
        Screen::ConfirmView => handle_confirm(app, key),
        Screen::RunView => handle_run(app, key),
        Screen::ResultView => handle_result(app, key),
        Screen::HistoryView => handle_history(app, key),
        Screen::RunReplay => handle_replay(app, key),
    }
}

// ---------------- MainMenu ----------------

fn handle_main_menu(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('d') => {
            app.mode = Mode::Deploy;
            if let Err(e) = app.build_plan() {
                app.status_message = e;
            }
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('p') => {
            app.mode = Mode::Plan;
            if let Err(e) = app.build_plan() {
                app.status_message = e;
            }
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('h') => {
            app.runs = store::list().unwrap_or_default();
            app.screen = Screen::HistoryView;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let i = app.list_state.selected().unwrap_or(0);
            if i + 1 < 4 {
                app.list_state.select(Some(i + 1));
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            let i = app.list_state.selected().unwrap_or(0);
            if i > 0 {
                app.list_state.select(Some(i - 1));
            }
        }
        KeyCode::Enter => match app.list_state.selected() {
            Some(0) => {
                app.mode = Mode::Deploy;
                if let Err(e) = app.build_plan() {
                    app.status_message = e;
                }
                app.screen = Screen::PlanView;
            }
            Some(1) => {
                app.mode = Mode::Plan;
                if let Err(e) = app.build_plan() {
                    app.status_message = e;
                }
                app.screen = Screen::PlanView;
            }
            Some(2) => {
                app.runs = store::list().unwrap_or_default();
                app.screen = Screen::HistoryView;
            }
            Some(3) => app.should_quit = true,
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

fn render_main_menu(f: &mut Frame, app: &mut App) {
    fn fmt_date(s: &str) -> String {
        // RFC 3339: "2026-07-05T12:00:00+08:00" → "2026-07-05"
        s.split('T').next().unwrap_or(s).to_string()
    }

    let icon_set = icons::current();
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.fg_dim));
    f.render_widget(block, area);

    let has_run = !app.runs.is_empty();
    let summary_size: u16 = if has_run { 3 } else { 2 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(15),
            Constraint::Length(summary_size),
            Constraint::Length(1),
        ])
        .split(area);

    let title_prefix = format!("{}  dotman - Main Menu ", icon_set.app);
    let divider_width = usize::from(chunks[0].width).saturating_sub(display_width(&title_prefix));
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{}  dotman - Main Menu", icon_set.app),
            Style::default()
                .fg(CATPPUCCIN_MOCHA.fg_dim)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", "─".repeat(divider_width)),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    ]));
    f.render_widget(title, chunks[0]);

    // Menu items with two-line layout (title + description)
    let menu_items: [(&str, &str, &str); 4] = [
        (
            icon_set.menu_deploy,
            "Deploy",
            "Sync dotfiles to this machine",
        ),
        (
            icon_set.menu_plan,
            "Plan only",
            "Preview changes without executing",
        ),
        (
            icon_set.menu_history,
            "History",
            "Browse past deployment records",
        ),
        (icon_set.menu_quit, "Quit", "Exit dotman"),
    ];
    let mut styled_items: Vec<ListItem> = Vec::new();
    let area_width = usize::from(chunks[1].width);
    for (i, &(icon, title, desc)) in menu_items.iter().enumerate() {
        let is_sel = app.list_state.selected() == Some(i);
        let title_text = format!("{} {}", icon, title);
        if is_sel {
            let bg = focus_bg();
            let title_content_w = 2 + display_width(&title_text);
            let desc_content_w = 4 + display_width(desc);
            let mut lines = vec![
                Line::from(vec![
                    Span::styled(" ", Style::default().bg(bg)),
                    Span::styled("▎", Style::default().fg(CATPPUCCIN_MOCHA.primary).bg(bg)),
                    Span::styled(
                        title_text.clone(),
                        Style::default()
                            .bg(bg)
                            .fg(CATPPUCCIN_MOCHA.primary)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " ".repeat(area_width.saturating_sub(title_content_w)),
                        Style::default().bg(bg),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(" ", Style::default().bg(bg)),
                    Span::styled("▎", Style::default().fg(CATPPUCCIN_MOCHA.primary).bg(bg)),
                    Span::styled("  ", Style::default().bg(bg)),
                    Span::styled(desc, Style::default().bg(bg).fg(CATPPUCCIN_MOCHA.fg_dim)),
                    Span::styled(
                        " ".repeat(area_width.saturating_sub(desc_content_w)),
                        Style::default().bg(bg),
                    ),
                ]),
            ];
            if i == 0 {
                lines.insert(0, Line::from(" "));
            }
            if i < 4 {
                lines.push(Line::from(" "));
            }
            styled_items.push(ListItem::new(lines));
        } else {
            let mut lines = vec![
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(title_text.clone(), Style::default().fg(CATPPUCCIN_MOCHA.fg)),
                ]),
                Line::from(vec![
                    Span::raw("    "),
                    Span::styled(desc, Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
                ]),
            ];
            if i == 0 {
                lines.insert(0, Line::from(" "));
            }
            if i < 4 {
                lines.push(Line::from(" "));
            }
            styled_items.push(ListItem::new(lines));
        }
    }
    let list = List::new(styled_items)
        .highlight_style(Style::default())
        .highlight_symbol("");
    f.render_stateful_widget(list, chunks[1], &mut app.list_state);

    // Summary
    let cfg = app.config.as_ref();
    let pkg = cfg.map(|c| c.install.len()).unwrap_or(0);
    let links = cfg.map(|c| c.links.len()).unwrap_or(0);
    let dirs = cfg.map(|c| c.create.len()).unwrap_or(0);
    let shells = cfg.map(|c| c.shell.len()).unwrap_or(0);

    let os_part = if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    let arch_part = std::env::consts::ARCH;
    let summary_line_str = format!(
        "  {} {os_part} {arch_part} · {pkg} packages · {links} links · {dirs} directories · {shells} shell steps",
        icon_set.host
    );

    let summary_width = usize::from(chunks[2].width).saturating_sub(2);
    let summary_divider = format!("  {}", "─".repeat(summary_width));

    let mut summary_lines = vec![
        Line::from(vec![Span::styled(
            summary_divider,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        )]),
        Line::from(vec![Span::styled(
            summary_line_str,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        )]),
    ];

    if let Some(run) = app.runs.first() {
        let status_icon = match run.status {
            RunStatus::Success => icon_set.success,
            RunStatus::Failed => icon_set.failed,
            RunStatus::Aborted => icon_set.warning,
            RunStatus::Running => icon_set.running,
        };
        let status_color = match run.status {
            RunStatus::Success => CATPPUCCIN_MOCHA.success,
            RunStatus::Failed => CATPPUCCIN_MOCHA.danger,
            RunStatus::Aborted | RunStatus::Running => CATPPUCCIN_MOCHA.warning,
        };
        let mode_str = format!("{:?}", run.mode).to_lowercase();
        let date_str = fmt_date(&run.started_at);
        let total = run.items.len();
        let failed = run.items.iter().filter(|i| i.error.is_some()).count();
        summary_lines.push(Line::from(vec![
            Span::styled(
                format!("  last run: {date_str}  "),
                Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
            ),
            Span::styled(status_icon, Style::default().fg(status_color)),
            Span::styled(
                format!(" {mode_str} ({total} items, {failed} fail)"),
                Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
            ),
        ]));
    }

    f.render_widget(Paragraph::new(summary_lines), chunks[2]);

    let help = Paragraph::new(Line::from(vec![
        keycap("↑↓"),
        hint(" move  "),
        keycap("enter"),
        hint(" select  "),
        keycap("d"),
        hint(" deploy  "),
        keycap("p"),
        hint(" plan  "),
        keycap("h"),
        hint(" history  "),
        keycap("q"),
        hint(" quit"),
    ]));
    f.render_widget(help, chunks[3]);
}

// ---------------- PlanView ----------------

fn handle_plan(app: &mut App, key: KeyCode) -> Result<()> {
    let rows = app
        .plan
        .as_ref()
        .map(|plan| build_plan_rows(plan, &app.collapsed_layers, app.plan_columns))
        .unwrap_or_default();
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('s') => {
            if let Err(e) = save_current_selection(app) {
                app.status_message = e;
                app.status_is_focus_info = false;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            select_next_plan_row(app, &rows);
            update_plan_focus_info(app);
        }
        KeyCode::Char('k') | KeyCode::Up => {
            select_prev_plan_row(app, &rows);
            update_plan_focus_info(app);
        }
        KeyCode::Char('h') | KeyCode::Left => {
            move_grid_col(app, &rows, -1);
            update_plan_focus_info(app);
        }
        KeyCode::Char('l') | KeyCode::Right => {
            move_grid_col(app, &rows, 1);
            update_plan_focus_info(app);
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(plan) = &mut app.plan
                && let Some(row_idx) = app.list_state.selected()
            {
                match rows.get(row_idx) {
                    Some(PlanRow::Header { layer, .. }) => {
                        toggle_layer(&mut app.collapsed_layers, layer);
                        keep_selection_in_range(app);
                    }
                    Some(PlanRow::Item(item_idx)) => {
                        if let Some(item) = plan.items.get_mut(*item_idx) {
                            item.selected = !item.selected;
                            app.dirty = true;
                        }
                    }
                    Some(PlanRow::InlineItems(item_indices)) => {
                        let col = app.grid_col.min(item_indices.len().saturating_sub(1));
                        if let Some(item_idx) = item_indices.get(col)
                            && let Some(item) = plan.items.get_mut(*item_idx)
                        {
                            item.selected = !item.selected;
                            app.dirty = true;
                        }
                    }
                    _ => {}
                }
            }
            update_plan_focus_info(app);
        }
        KeyCode::Char('1') => {
            toggle_layer_by_number(app, 1);
            update_plan_focus_info(app);
        }
        KeyCode::Char('2') => {
            toggle_layer_by_number(app, 2);
            update_plan_focus_info(app);
        }
        KeyCode::Char('3') => {
            toggle_layer_by_number(app, 3);
            update_plan_focus_info(app);
        }
        KeyCode::Char('4') => {
            toggle_layer_by_number(app, 4);
            update_plan_focus_info(app);
        }
        KeyCode::Char('5') => {
            toggle_layer_by_number(app, 5);
            update_plan_focus_info(app);
        }
        KeyCode::Char('6') => {
            toggle_layer_by_number(app, 6);
            update_plan_focus_info(app);
        }
        KeyCode::Char('a') => {
            if let Some(plan) = &mut app.plan {
                for item in plan.items.iter_mut() {
                    item.selected = true;
                }
                app.dirty = true;
            }
        }
        KeyCode::Char('n') => {
            if let Some(plan) = &mut app.plan {
                for item in plan.items.iter_mut() {
                    item.selected = false;
                }
                app.dirty = true;
            }
        }
        KeyCode::Char('r') => {
            if matches!(app.mode, Mode::Plan) {
                app.status_message = "plan mode is read-only; choose deploy to run".into();
                app.status_is_focus_info = false;
            } else if selected_item_count(app.plan.as_ref()) == 0 {
                app.status_message = "nothing selected".into();
                app.status_is_focus_info = false;
            } else {
                app.review_entries = if let Some(plan) = app.plan.as_ref() {
                    review_entries(plan, app.config.as_ref())
                } else {
                    Vec::new()
                };
                app.review_scroll = 0;
                app.screen = Screen::ConfirmView;
            }
        }
        _ => {}
    }
    Ok(())
}

fn render_plan(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    app.plan_columns = plan_grid_columns(usize::from(chunks[1].width));
    app.grid_col = clamped_grid_col_for_selection(app);

    let plan = match &app.plan {
        Some(p) => p,
        None => {
            let msg = Paragraph::new("no plan loaded").alignment(Alignment::Center);
            f.render_widget(msg, chunks[0]);
            return;
        }
    };

    let selected = selected_item_count(Some(plan));
    let actions = selected_action_count(Some(plan));
    let state = if app.dirty { "unsaved" } else { "saved" };
    let status_prefix = format!(
        "{}  dotman - Plan (○ {state})  {selected} selected · {actions} actions ",
        icon_set.app
    );
    let divider_width = usize::from(chunks[0].width).saturating_sub(display_width(&status_prefix));
    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            format!("{}  dotman - Plan (", icon_set.app),
            Style::default()
                .fg(CATPPUCCIN_MOCHA.fg_dim)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            if app.dirty {
                "○ unsaved"
            } else {
                "○ saved"
            },
            Style::default().fg(if app.dirty {
                CATPPUCCIN_MOCHA.warning
            } else {
                CATPPUCCIN_MOCHA.text_muted
            }),
        ),
        Span::styled(")  ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(
            format!("{selected} selected · {actions} actions"),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::styled(
            format!(" {}", "─".repeat(divider_width)),
            Style::default().fg(CATPPUCCIN_MOCHA.border_subtle),
        ),
    ]));
    f.render_widget(status, chunks[0]);

    let rows = build_plan_rows(plan, &app.collapsed_layers, app.plan_columns);
    let mut items: Vec<ListItem> = Vec::new();
    let row_width = usize::from(chunks[1].width);
    let cell_width = grid_cell_width(row_width, app.plan_columns);
    for (row_index, row) in rows.iter().enumerate() {
        let selected_row = app
            .list_state
            .selected()
            .is_some_and(|selected| selected == row_index);
        match row {
            PlanRow::Header {
                layer,
                ordinal,
                enabled,
                total,
            } => {
                items.push(plan_header_line(
                    layer,
                    *ordinal,
                    *enabled,
                    *total,
                    app.collapsed_layers.contains(layer),
                    selected_row,
                    row_width,
                ));
            }
            PlanRow::Item(item_idx) => {
                let it = &plan.items[*item_idx];
                if selected_row {
                    items.push(selected_item_line(it, row_width));
                } else {
                    items.push(plan_item_line(it, row_width));
                }
            }
            PlanRow::InlineItems(item_indices) => {
                let mut spans = vec![Span::raw("  ")];
                for (i, item_idx) in item_indices.iter().enumerate() {
                    let it = &plan.items[*item_idx];
                    let selected_cell = selected_row && app.grid_col == i;
                    spans.extend(grid_cell_spans(it, cell_width, selected_cell));
                    if i + 1 < item_indices.len() {
                        spans.push(Span::raw("    "));
                    }
                }
                items.push(ListItem::new(Line::from(spans)));
            }
            PlanRow::Divider => {
                items.push(ListItem::new(Line::from(Span::styled(
                    format!("  {}", "─".repeat(row_width.saturating_sub(2))),
                    divider_style(),
                ))));
            }
        }
    }

    let list = List::new(items)
        .highlight_style(Style::default())
        .highlight_symbol("");
    f.render_stateful_widget(list, chunks[1], &mut app.list_state);

    let status_line = if app.status_message.is_empty() {
        Line::from("")
    } else {
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                &app.status_message,
                Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
            ),
        ])
    };
    let help = Paragraph::new(vec![
        status_line,
        plan_help_line(usize::from(chunks[2].width)),
    ]);
    f.render_widget(help, chunks[2]);
}

fn build_plan_rows(
    plan: &Plan,
    collapsed_layers: &BTreeSet<String>,
    grid_columns: usize,
) -> Vec<PlanRow> {
    let layers = [
        "terminal",
        "shell",
        "multiplexer",
        "software",
        "enhancement",
        "misc",
    ];
    let mut rows = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        let layer_items: Vec<usize> = plan
            .items
            .iter()
            .enumerate()
            .filter_map(|(idx, it)| (it.layer == *layer).then_some(idx))
            .collect();
        if layer_items.is_empty() {
            continue;
        }
        let enabled = layer_items
            .iter()
            .filter(|idx| plan.items[**idx].selected)
            .count();
        rows.push(PlanRow::Header {
            layer: (*layer).to_string(),
            ordinal: i + 1,
            enabled,
            total: layer_items.len(),
        });
        if !collapsed_layers.contains(*layer) {
            if i < 3 || grid_columns == 1 {
                rows.extend(layer_items.into_iter().map(PlanRow::Item));
            } else {
                for chunk in layer_items.chunks(grid_columns) {
                    rows.push(PlanRow::InlineItems(chunk.to_vec()));
                }
            }
        }
        if i + 1 < layers.len() {
            rows.push(PlanRow::Divider);
        }
    }
    rows
}

fn plan_header_line(
    layer: &str,
    ordinal: usize,
    enabled: usize,
    total: usize,
    collapsed: bool,
    focused: bool,
    width: usize,
) -> ListItem<'static> {
    let icon_set = icons::current();
    let icon = if collapsed {
        icon_set.collapsed
    } else {
        icon_set.expanded
    };
    let left = format!("{} {:02}  {}", icon, ordinal, capitalize(layer));
    let right = format!("{enabled} / {total}");
    let content_width = width.saturating_sub(2);
    let right_width = display_width(&right);
    let left_width = content_width.saturating_sub(right_width + 1);
    let gap = content_width
        .saturating_sub(left_width + right_width)
        .max(1);

    if focused {
        let bg = focus_bg();
        ListItem::new(Line::from(vec![
            Span::styled(
                "▎",
                Style::default().fg(CATPPUCCIN_MOCHA.focus_marker).bg(bg),
            ),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(
                fit_to_width(&left, left_width),
                Style::default().bg(bg).fg(CATPPUCCIN_MOCHA.text_muted),
            ),
            Span::styled(" ".repeat(gap), Style::default().bg(bg)),
            Span::styled(
                right,
                Style::default().bg(bg).fg(CATPPUCCIN_MOCHA.text_muted),
            ),
        ]))
    } else {
        ListItem::new(Line::from(vec![
            Span::raw("  "),
            Span::styled(
                fit_to_width(&left, left_width),
                Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
            ),
            Span::raw(" ".repeat(gap)),
            Span::styled(right, Style::default().fg(CATPPUCCIN_MOCHA.text_muted)),
        ]))
    }
}

fn plan_item_line(item: &PlanItem, width: usize) -> ListItem<'static> {
    let label = item_label(item);
    let prefix_width = 6;
    let available = width.saturating_sub(prefix_width);

    ListItem::new(Line::from(vec![
        Span::raw("  "),
        Span::raw("  "),
        checkbox_span(item.selected, false),
        Span::raw("  "),
        Span::styled(
            fit_to_width(&label, available),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
    ]))
}

fn select_first_plan_row(
    list_state: &mut ListState,
    plan: Option<&Plan>,
    collapsed_layers: &BTreeSet<String>,
    grid_columns: usize,
) {
    let Some(plan) = plan else {
        select_plan_row(list_state, 0, true);
        return;
    };
    let rows = build_plan_rows(plan, collapsed_layers, grid_columns);
    let first = rows.iter().position(is_selectable_plan_row).unwrap_or(0);
    select_plan_row(list_state, first, true);
}

fn select_next_plan_row(app: &mut App, rows: &[PlanRow]) {
    if rows.is_empty() {
        select_plan_row(&mut app.list_state, 0, true);
        return;
    }
    let current = app.list_state.selected().unwrap_or(0);
    let start = app.list_state.selected().unwrap_or(0).saturating_add(1);
    let next = (start..rows.len())
        .find(|idx| is_selectable_plan_row(&rows[*idx]))
        .or_else(|| rows.iter().position(is_selectable_plan_row))
        .unwrap_or(0);
    clamp_grid_col(app, rows.get(next));
    select_plan_row(&mut app.list_state, next, next < current);
}

fn select_prev_plan_row(app: &mut App, rows: &[PlanRow]) {
    if rows.is_empty() {
        select_plan_row(&mut app.list_state, 0, true);
        return;
    }
    let current = app
        .list_state
        .selected()
        .unwrap_or(rows.len())
        .min(rows.len());
    let start = app
        .list_state
        .selected()
        .unwrap_or(rows.len())
        .min(rows.len());
    let prev = (0..start)
        .rev()
        .find(|idx| is_selectable_plan_row(&rows[*idx]))
        .or_else(|| rows.iter().rposition(is_selectable_plan_row))
        .unwrap_or(0);
    clamp_grid_col(app, rows.get(prev));
    select_plan_row(&mut app.list_state, prev, prev > current);
}

fn select_plan_row(list_state: &mut ListState, idx: usize, reset_offset: bool) {
    list_state.select(Some(idx));
    if reset_offset {
        *list_state.offset_mut() = 0;
    }
}

fn is_selectable_plan_row(row: &PlanRow) -> bool {
    matches!(
        row,
        PlanRow::Header { .. } | PlanRow::Item(_) | PlanRow::InlineItems(_)
    )
}

fn move_grid_col(app: &mut App, rows: &[PlanRow], delta: isize) {
    let Some(row_idx) = app.list_state.selected() else {
        return;
    };
    let Some(PlanRow::InlineItems(item_indices)) = rows.get(row_idx) else {
        return;
    };
    let max_col = item_indices.len().saturating_sub(1);
    let next = if delta.is_negative() {
        app.grid_col.saturating_sub(delta.unsigned_abs())
    } else {
        app.grid_col.saturating_add(delta as usize)
    };
    app.grid_col = next.min(max_col);
}

fn clamp_grid_col(app: &mut App, row: Option<&PlanRow>) {
    if let Some(PlanRow::InlineItems(item_indices)) = row {
        app.grid_col = app.grid_col.min(item_indices.len().saturating_sub(1));
    } else {
        app.grid_col = 0;
    }
}

fn clamped_grid_col_for_selection(app: &App) -> usize {
    let Some(plan) = &app.plan else {
        return 0;
    };
    let rows = build_plan_rows(plan, &app.collapsed_layers, app.plan_columns);
    let Some(row_idx) = app.list_state.selected() else {
        return 0;
    };
    match rows.get(row_idx) {
        Some(PlanRow::InlineItems(item_indices)) => {
            app.grid_col.min(item_indices.len().saturating_sub(1))
        }
        _ => 0,
    }
}

fn toggle_layer(collapsed_layers: &mut BTreeSet<String>, layer: &str) {
    if !collapsed_layers.remove(layer) {
        collapsed_layers.insert(layer.to_string());
    }
}

fn toggle_layer_by_number(app: &mut App, number: usize) {
    if let Some(layer) = layer_by_number(number) {
        toggle_layer(&mut app.collapsed_layers, layer);
        keep_selection_in_range(app);
    }
}

fn layer_by_number(number: usize) -> Option<&'static str> {
    match number {
        1 => Some("terminal"),
        2 => Some("shell"),
        3 => Some("multiplexer"),
        4 => Some("software"),
        5 => Some("enhancement"),
        6 => Some("misc"),
        _ => None,
    }
}

fn keep_selection_in_range(app: &mut App) {
    let rows = app
        .plan
        .as_ref()
        .map(|plan| build_plan_rows(plan, &app.collapsed_layers, app.plan_columns))
        .unwrap_or_default();
    let selected = app.list_state.selected().unwrap_or(0);
    if selected >= rows.len() || !rows.get(selected).is_some_and(is_selectable_plan_row) {
        let first = rows.iter().position(is_selectable_plan_row).unwrap_or(0);
        clamp_grid_col(app, rows.get(first));
        select_plan_row(&mut app.list_state, first, true);
    } else {
        clamp_grid_col(app, rows.get(selected));
    }
}

fn update_plan_focus_info(app: &mut App) {
    if let Some(info) = focused_plan_item_info(app) {
        app.status_message = info;
        app.status_is_focus_info = true;
    } else {
        clear_focus_info(app);
    }
}

fn focused_plan_item_info(app: &App) -> Option<String> {
    let plan = app.plan.as_ref()?;
    let row_idx = app.list_state.selected()?;
    let rows = build_plan_rows(plan, &app.collapsed_layers, app.plan_columns);
    match rows.get(row_idx)? {
        PlanRow::Item(item_idx) => plan.items.get(*item_idx).map(plan_item_info),
        PlanRow::InlineItems(item_indices) => {
            let col = app.grid_col.min(item_indices.len().saturating_sub(1));
            item_indices
                .get(col)
                .and_then(|item_idx| plan.items.get(*item_idx))
                .map(plan_item_info)
        }
        PlanRow::Header { .. } | PlanRow::Divider => None,
    }
}

fn clear_focus_info(app: &mut App) {
    if app.status_is_focus_info {
        app.status_message.clear();
        app.status_is_focus_info = false;
    }
}

fn plan_item_info(item: &PlanItem) -> String {
    let actions = item
        .actions
        .iter()
        .map(Action::describe)
        .collect::<Vec<_>>()
        .join(" · ");
    if actions.is_empty() {
        item.name.clone()
    } else {
        format!("{}: {actions}", item.name)
    }
}

fn selected_item_line(item: &PlanItem, width: usize) -> ListItem<'static> {
    let bg = focus_bg();
    let label = item_label(item);
    let fixed_width = 7;
    let label_width = width.saturating_sub(fixed_width);
    ListItem::new(Line::from(vec![
        Span::styled("  ", Style::default().bg(bg)),
        Span::styled(
            "▎",
            Style::default().fg(CATPPUCCIN_MOCHA.focus_marker).bg(bg),
        ),
        Span::styled(" ", Style::default().bg(bg)),
        checkbox_span(item.selected, true),
        Span::styled("  ", Style::default().bg(bg)),
        Span::styled(
            fit_to_width(&label, label_width),
            Style::default().bg(bg).fg(CATPPUCCIN_MOCHA.fg),
        ),
    ]))
}

fn plan_grid_columns(width: usize) -> usize {
    if width < 90 {
        1
    } else if width < 120 {
        2
    } else {
        GRID_COLUMNS
    }
}

fn grid_cell_width(row_width: usize, columns: usize) -> usize {
    let indent = 4;
    let gaps = columns.saturating_sub(1) * 4;
    row_width
        .saturating_sub(indent + gaps)
        .checked_div(columns)
        .unwrap_or(18)
        .max(18)
}

fn grid_cell_spans(item: &PlanItem, width: usize, focused: bool) -> Vec<Span<'static>> {
    let bg = focused.then(focus_bg);
    let label = item_label(item);
    let prefix_width = 2;
    let fixed_width = prefix_width + 3;
    let label_width = width.saturating_sub(fixed_width);
    let prefix = if focused { "▎ " } else { "  " };
    let mut prefix_style = Style::default();
    if focused {
        prefix_style = prefix_style
            .fg(CATPPUCCIN_MOCHA.focus_marker)
            .bg(focus_bg());
    }
    vec![
        Span::styled(prefix, prefix_style),
        checkbox_span(item.selected, focused),
        Span::styled("  ", span_bg_style(bg)),
        Span::styled(
            fit_to_width(&label, label_width),
            span_bg_style(bg)
                .fg(CATPPUCCIN_MOCHA.fg)
                .add_modifier(Modifier::empty()),
        ),
    ]
}

fn checkbox_span(selected: bool, highlighted: bool) -> Span<'static> {
    let icon_set = icons::current();
    Span::styled(
        if selected {
            icon_set.selected
        } else {
            icon_set.unselected
        },
        span_bg_style(highlighted.then(focus_bg)).fg(if selected {
            CATPPUCCIN_MOCHA.success
        } else {
            CATPPUCCIN_MOCHA.fg_dim
        }),
    )
}

fn item_label(item: &PlanItem) -> String {
    if item.actions.len() > 1 {
        format!("{} (+{})", item.name, item.actions.len() - 1)
    } else {
        item.name.clone()
    }
}

fn span_bg_style(bg: Option<Color>) -> Style {
    if let Some(bg) = bg {
        Style::default().bg(bg)
    } else {
        Style::default()
    }
}

fn focus_bg() -> Color {
    CATPPUCCIN_MOCHA.surface_active
}

fn divider_style() -> Style {
    Style::default().fg(CATPPUCCIN_MOCHA.divider)
}

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(value)
}

fn fit_to_width(value: &str, width: usize) -> String {
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

fn keycap(label: &'static str) -> Span<'static> {
    Span::styled(
        format!("[{label}]"),
        Style::default()
            .fg(CATPPUCCIN_MOCHA.text_muted)
            .add_modifier(Modifier::BOLD),
    )
}

fn hint(label: &'static str) -> Span<'static> {
    Span::styled(label, Style::default().fg(CATPPUCCIN_MOCHA.fg_dim))
}

fn plan_help_line(width: usize) -> Line<'static> {
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

    for parts in [&full[..], &short[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }

    help_line_from_parts(&[("Q", "")])
}

fn review_help_line(width: usize) -> Line<'static> {
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

fn run_help_line(width: usize, aborting: bool, finished: bool) -> Line<'static> {
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

    let full = if finished {
        [("Q/Esc", " Back")]
    } else {
        [("Q/Esc", " Abort")]
    };
    let compact = [("Q", "")];
    for parts in [&full[..], &compact[..]] {
        let line = help_line_from_parts(parts);
        if line_display_width(&line) <= width {
            return line;
        }
    }
    help_line_from_parts(&[("Q", "")])
}

fn help_line_from_parts(parts: &[(&'static str, &'static str)]) -> Line<'static> {
    let mut spans = Vec::with_capacity(parts.len() * 2);
    for (key, label) in parts {
        spans.push(keycap(key));
        if !label.is_empty() {
            spans.push(hint(label));
        }
    }
    Line::from(spans)
}

fn line_display_width(line: &Line<'_>) -> usize {
    line.spans
        .iter()
        .map(|span| display_width(span.content.as_ref()))
        .sum()
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(ch) => ch.to_ascii_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

// ---------------- ConfirmView ----------------

fn handle_confirm(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Down | KeyCode::Char('j') => scroll_review(app, 1),
        KeyCode::Up | KeyCode::Char('k') => scroll_review(app, -1),
        KeyCode::PageDown => scroll_review(app, 8),
        KeyCode::PageUp => scroll_review(app, -8),
        KeyCode::Home => app.review_scroll = 0,
        KeyCode::End => app.review_scroll = usize::MAX,
        KeyCode::Enter | KeyCode::Char('r') => {
            // Pre-cache sudo credentials before executing if the plan needs them.
            if review_entries_need_sudo(&app.review_entries) {
                restore_terminal()?;
                let ok = shell::pre_cache_sudo().unwrap_or(false);
                // Re-enter raw mode; the event loop will recreate the Terminal
                // backend on the next tick.
                setup_terminal()?;
                app.needs_terminal_reset = true;
                if !ok {
                    app.status_message = "sudo authentication failed".into();
                    return Ok(());
                }
            }
            start_run(app);
        }
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('e') => {
            app.screen = Screen::PlanView;
        }
        _ => {}
    }
    Ok(())
}

fn scroll_review(app: &mut App, delta: isize) {
    if delta < 0 {
        app.review_scroll = app.review_scroll.saturating_sub(delta.unsigned_abs());
    } else {
        app.review_scroll = app.review_scroll.saturating_add(delta as usize);
    }
}

fn render_confirm(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let block = Block::default()
        .title(format!(" {} Review — {:?} ", icon_set.info, app.mode))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.border_strong));
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    let Some(plan) = &app.plan else {
        f.render_widget(
            Paragraph::new("no plan loaded").alignment(Alignment::Center),
            chunks[1],
        );
        return;
    };

    let selected = plan.items.iter().filter(|item| item.selected).count();
    let skipped = plan.items.len().saturating_sub(selected);
    let actions = plan
        .items
        .iter()
        .filter(|item| item.selected)
        .map(|item| item.actions.len())
        .sum::<usize>();
    let entries = &app.review_entries;
    let review_count = entries.len();
    let change_count = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.severity,
                ReviewSeverity::Run | ReviewSeverity::Warning
            )
        })
        .count();
    let risk_count = entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.severity,
                ReviewSeverity::Warning | ReviewSeverity::Danger
            )
        })
        .count();

    let summary = vec![
        Line::from(vec![
            Span::styled("Selected: ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
            Span::styled(
                selected.to_string(),
                Style::default().fg(CATPPUCCIN_MOCHA.success),
            ),
            Span::raw(" steps, "),
            Span::styled(
                actions.to_string(),
                Style::default().fg(CATPPUCCIN_MOCHA.accent),
            ),
            Span::raw(" actions"),
        ]),
        Line::from(vec![
            Span::styled("Skipped: ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
            Span::raw(skipped.to_string()),
            Span::raw(" steps"),
        ]),
        Line::from(vec![
            Span::styled("Review: ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
            Span::raw(format!(
                "{review_count} actions, {change_count} active, {risk_count} attention"
            )),
        ]),
    ];
    f.render_widget(Paragraph::new(summary), chunks[0]);

    let body_width = usize::from(chunks[1].width).saturating_sub(2);
    let body_height = usize::from(chunks[1].height).saturating_sub(2);
    let body = if entries.is_empty() {
        vec![Line::from("No selected actions.")]
    } else {
        review_body_lines(entries, body_width, body_height, &mut app.review_scroll)
    };
    f.render_widget(
        Paragraph::new(body)
            .block(
                Block::default()
                    .title(" actions ")
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );

    let help = Paragraph::new(review_help_line(usize::from(chunks[2].width)));
    f.render_widget(help, chunks[2]);
}

fn selected_item_count(plan: Option<&Plan>) -> usize {
    plan.map(|plan| plan.items.iter().filter(|item| item.selected).count())
        .unwrap_or(0)
}

fn selected_action_count(plan: Option<&Plan>) -> usize {
    plan.map(|plan| {
        plan.items
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.actions.len())
            .sum()
    })
    .unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewSeverity {
    Success,
    Skip,
    Run,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewGroup {
    Attention,
    WillRun,
    AlreadyOk,
    Skipped,
}

#[derive(Debug, Clone)]
struct ReviewEntry {
    order: usize,
    item: String,
    kind: &'static str,
    kind_icon: &'static str,
    severity: ReviewSeverity,
    status: String,
    detail: String,
}

fn review_entries(plan: &Plan, config: Option<&config::Config>) -> Vec<ReviewEntry> {
    let icon_set = icons::current();
    let config_dir = plan.config_path.parent().unwrap_or(Path::new("."));
    let mut entries = Vec::new();
    for item in plan.items.iter().filter(|item| item.selected) {
        for action in &item.actions {
            let mut entry = match action {
                Action::Install {
                    pkg_mgr,
                    binary,
                    source,
                } => review_install_entry(item, config, pkg_mgr, binary, source),
                Action::Link { target, source } => {
                    review_link_entry(item, config_dir, target, source, icon_set.action_link)
                }
                Action::Create { target } => {
                    review_create_entry(item, target, icon_set.action_create)
                }
                Action::Shell {
                    command,
                    description,
                    optional,
                    if_condition,
                } => review_shell_entry(
                    item,
                    command,
                    description.as_deref(),
                    *optional,
                    if_condition.as_deref(),
                    icon_set.action_shell,
                ),
                Action::Clean { target, force } => {
                    review_clean_entry(item, target, *force, icon_set.action_clean)
                }
            };
            entry.order = entries.len();
            entries.push(entry);
        }
    }
    entries
}

fn review_install_entry(
    item: &PlanItem,
    config: Option<&config::Config>,
    pkg_mgr: &str,
    binary: &str,
    source: &str,
) -> ReviewEntry {
    let icon_set = icons::current();
    let resolved_pkg_mgr = config
        .and_then(|cfg| crate::package_managers::resolve_pkg_mgr_name(&cfg.package_managers))
        .unwrap_or_else(|| {
            if pkg_mgr == "auto" {
                crate::package_managers::default_pkg_mgr_name()
            } else {
                pkg_mgr.to_string()
            }
        });
    let command = install_command_summary(binary, &resolved_pkg_mgr).unwrap_or_else(|| {
        if source.trim().is_empty() {
            format!("install {binary}")
        } else {
            source.to_string()
        }
    });

    let db = install::load_db().ok();
    let entry = db.as_ref().and_then(|db| install::find(db, binary));
    match entry
        .as_ref()
        .map(|entry| install::detect_presence(entry, Some(&command)))
        .unwrap_or(install::InstallPresence::Unknown)
    {
        install::InstallPresence::Present => ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Success,
            status: "present".into(),
            detail: command,
        },
        install::InstallPresence::Missing => ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Run,
            status: "missing".into(),
            detail: command,
        },
        install::InstallPresence::Unknown => ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Warning,
            status: "unknown".into(),
            detail: command,
        },
    }
}

fn review_link_entry(
    item: &PlanItem,
    config_dir: &Path,
    target: &Path,
    source: &Path,
    kind_icon: &'static str,
) -> ReviewEntry {
    let (severity, status) = match link::plan_link(
        config_dir,
        target,
        source,
        LinkSettings {
            create: true,
            relative: true,
            backup: true,
            relink: false,
        },
    ) {
        Ok(link_plan) => describe_link_review(&link_plan.action),
        Err(e) => (ReviewSeverity::Danger, format!("inspect failed: {e}")),
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "link",
        kind_icon,
        severity,
        status,
        detail: format!("{} -> {}", target.display(), source.display()),
    }
}

fn review_create_entry(item: &PlanItem, target: &Path, kind_icon: &'static str) -> ReviewEntry {
    let expanded = crate::path::expand_home(&target.to_string_lossy())
        .unwrap_or_else(|_| target.to_path_buf());
    let (severity, status) = if expanded.exists() {
        (ReviewSeverity::Success, "exists".into())
    } else {
        (ReviewSeverity::Run, "create".into())
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "create",
        kind_icon,
        severity,
        status,
        detail: target.display().to_string(),
    }
}

fn review_shell_entry(
    item: &PlanItem,
    command: &str,
    description: Option<&str>,
    optional: bool,
    if_condition: Option<&str>,
    kind_icon: &'static str,
) -> ReviewEntry {
    let mut status = if optional {
        "optional".to_string()
    } else {
        "run".to_string()
    };
    let mut severity = ReviewSeverity::Run;
    if let Some(cond) = if_condition {
        match shell::condition_matches(cond) {
            Ok(true) => status = format!("if ok · {status}"),
            Ok(false) => {
                status = "if skip".into();
                severity = ReviewSeverity::Skip;
            }
            Err(_) => {
                status = "if unknown".into();
                severity = ReviewSeverity::Warning;
            }
        }
    }
    if shell::command_contains_sudo(command) && !matches!(severity, ReviewSeverity::Skip) {
        status = format!("{status} · sudo");
        severity = ReviewSeverity::Warning;
    }
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "shell",
        kind_icon,
        severity,
        status,
        detail: description.unwrap_or(command).to_string(),
    }
}

fn review_clean_entry(
    item: &PlanItem,
    target: &Path,
    force: bool,
    kind_icon: &'static str,
) -> ReviewEntry {
    let (severity, status) = match clean::plan_clean(target, force) {
        Ok(clean::CleanAction::Skip) => (ReviewSeverity::Skip, "skip".into()),
        Ok(clean::CleanAction::RemoveSymlink) => (ReviewSeverity::Warning, "remove symlink".into()),
        Ok(clean::CleanAction::BackupAndRemove(_)) => {
            (ReviewSeverity::Warning, "backup remove".into())
        }
        Err(e) => (ReviewSeverity::Danger, format!("inspect failed: {e}")),
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "clean",
        kind_icon,
        severity,
        status,
        detail: target.display().to_string(),
    }
}

fn describe_link_review(action: &LinkAction) -> (ReviewSeverity, String) {
    match action {
        LinkAction::Skip => (ReviewSeverity::Success, "linked".into()),
        LinkAction::Link => (ReviewSeverity::Run, "link".into()),
        LinkAction::Backup(_) => (ReviewSeverity::Warning, "backup link".into()),
        LinkAction::Relink => (ReviewSeverity::Warning, "relink".into()),
        LinkAction::Fail(reason) => (ReviewSeverity::Danger, format!("fail: {reason}")),
    }
}

fn review_body_lines(
    entries: &[ReviewEntry],
    width: usize,
    height: usize,
    scroll: &mut usize,
) -> Vec<Line<'static>> {
    if height == 0 {
        return Vec::new();
    }
    let mut all_lines = Vec::new();
    for group in [
        ReviewGroup::Attention,
        ReviewGroup::WillRun,
        ReviewGroup::AlreadyOk,
        ReviewGroup::Skipped,
    ] {
        let group_entries = sorted_review_group_entries(entries, group);
        if group_entries.is_empty() {
            continue;
        }
        all_lines.push(review_group_header_line(group, group_entries.len(), width));
        for entry in group_entries {
            all_lines.extend(review_entry_lines(entry, width));
        }
    }
    if all_lines.len() <= height {
        *scroll = 0;
        return all_lines;
    }

    let max_scroll = all_lines.len().saturating_sub(height);
    *scroll = (*scroll).min(max_scroll);

    let mut visible = all_lines
        .iter()
        .skip(*scroll)
        .take(height)
        .cloned()
        .collect::<Vec<_>>();
    if *scroll > 0
        && let Some(first) = visible.first_mut()
    {
        *first = Line::from(Span::styled(
            fit_to_width(&format!("  ... {} above", *scroll), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    let below = all_lines.len().saturating_sub(*scroll + height);
    if below > 0
        && let Some(last) = visible.last_mut()
    {
        *last = Line::from(Span::styled(
            fit_to_width(&format!("  ... {below} below"), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    visible
}

fn sorted_review_group_entries(entries: &[ReviewEntry], group: ReviewGroup) -> Vec<&ReviewEntry> {
    let mut group_entries = entries
        .iter()
        .filter(|entry| review_group_for(entry) == group)
        .collect::<Vec<_>>();
    group_entries.sort_by_key(|entry| (review_kind_rank(entry.kind), entry.order));
    group_entries
}

fn review_kind_rank(kind: &str) -> usize {
    match kind {
        "install" => 0,
        "link" => 1,
        "create" => 2,
        "shell" => 3,
        "clean" => 4,
        _ => usize::MAX,
    }
}

fn review_group_for(entry: &ReviewEntry) -> ReviewGroup {
    match entry.severity {
        ReviewSeverity::Warning | ReviewSeverity::Danger => ReviewGroup::Attention,
        ReviewSeverity::Run => ReviewGroup::WillRun,
        ReviewSeverity::Success => ReviewGroup::AlreadyOk,
        ReviewSeverity::Skip => ReviewGroup::Skipped,
    }
}

fn review_group_header_line(group: ReviewGroup, count: usize, width: usize) -> Line<'static> {
    let icon_set = icons::current();
    let (icon, label, style) = match group {
        ReviewGroup::Attention => (
            icon_set.warning,
            "Attention",
            Style::default().fg(CATPPUCCIN_MOCHA.warning),
        ),
        ReviewGroup::WillRun => (
            icon_set.running,
            "Will Run",
            Style::default().fg(CATPPUCCIN_MOCHA.running),
        ),
        ReviewGroup::AlreadyOk => (
            icon_set.success,
            "Already OK",
            Style::default().fg(CATPPUCCIN_MOCHA.success),
        ),
        ReviewGroup::Skipped => (
            icon_set.skipped,
            "Skipped",
            Style::default().fg(CATPPUCCIN_MOCHA.skip),
        ),
    };
    Line::from(Span::styled(
        fit_to_width(&format!("{icon} {label} ({count})"), width),
        style.add_modifier(Modifier::BOLD),
    ))
}

fn review_entry_lines(entry: &ReviewEntry, width: usize) -> Vec<Line<'static>> {
    let icon_set = icons::current();
    let status_icon = review_status_icon(icon_set, entry.severity);
    let status_style = review_status_style(entry.severity);
    let left = format!("{} {:<7} {}", entry.kind_icon, entry.kind, entry.item);
    let detail = review_entry_detail(entry);
    let single = if let Some(detail) = detail {
        format!("{left}  {}  {detail}", entry.status)
    } else {
        format!("{left}  {}", entry.status)
    };
    if display_width(&single) <= width.saturating_sub(2) {
        return vec![Line::from(vec![
            Span::styled(status_icon, status_style),
            Span::raw(" "),
            Span::styled(single, Style::default().fg(CATPPUCCIN_MOCHA.fg)),
        ])];
    }

    let first = format!("{left}  {}", entry.status);
    let mut lines = vec![Line::from(vec![
        Span::styled(status_icon, status_style),
        Span::raw(" "),
        Span::styled(
            fit_to_width(&first, width.saturating_sub(2)),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
    ])];
    if let Some(detail) = detail {
        lines.push(Line::from(Span::styled(
            fit_to_width(&format!("  {} {detail}", icon_set.info), width),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        )));
    }
    lines
}

fn review_entry_detail(entry: &ReviewEntry) -> Option<&str> {
    let detail = entry.detail.trim();
    (!detail.is_empty() && detail != entry.item.trim()).then_some(detail)
}

fn review_entries_need_sudo(entries: &[ReviewEntry]) -> bool {
    entries.iter().any(review_entry_needs_sudo)
}

fn review_entry_needs_sudo(entry: &ReviewEntry) -> bool {
    matches!(
        entry.severity,
        ReviewSeverity::Run | ReviewSeverity::Warning
    ) && (entry.status.contains("sudo") || shell::command_contains_sudo(&entry.detail))
}

fn review_status_icon(icon_set: &'static icons::IconSet, severity: ReviewSeverity) -> &'static str {
    match severity {
        ReviewSeverity::Success => icon_set.success,
        ReviewSeverity::Skip => icon_set.skipped,
        ReviewSeverity::Run => icon_set.running,
        ReviewSeverity::Warning => icon_set.warning,
        ReviewSeverity::Danger => icon_set.failed,
    }
}

fn review_status_style(severity: ReviewSeverity) -> Style {
    Style::default().fg(match severity {
        ReviewSeverity::Success => CATPPUCCIN_MOCHA.success,
        ReviewSeverity::Skip => CATPPUCCIN_MOCHA.skip,
        ReviewSeverity::Run => CATPPUCCIN_MOCHA.running,
        ReviewSeverity::Warning => CATPPUCCIN_MOCHA.warning,
        ReviewSeverity::Danger => CATPPUCCIN_MOCHA.danger,
    })
}

fn install_command_summary(binary: &str, pkg_mgr: &str) -> Option<String> {
    let db = install::load_db().ok()?;
    let entry = install::find(&db, binary)?;
    install::command_for_current_platform(&entry, pkg_mgr)
}

// ---------------- RunView ----------------

fn handle_run(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            if let Some(flag) = &app.abort_flag {
                flag.store(true, Ordering::SeqCst);
                app.status_message = "abort requested; waiting for current action".into();
                push_log(app, "abort requested; waiting for current action", None);
            } else if app.plan.is_some() {
                app.screen = Screen::PlanView;
            } else {
                app.screen = Screen::MainMenu;
            }
        }
        _ => {}
    }
    Ok(())
}

fn start_run(app: &mut App) {
    if app.plan.is_none() || app.config.is_none() {
        return;
    }
    let plan = app.plan.clone().unwrap();
    let cfg = app.config.clone().unwrap();
    let total = plan.items.iter().filter(|i| i.selected).count();
    app.progress = (0, total);
    app.current_item = None;
    app.current_action = None;
    app.run_started = Some(Instant::now());
    app.current_log.clear();
    app.run = None;
    app.run_item_statuses = vec![None; plan.items.len()];
    app.run_action_statuses = plan
        .items
        .iter()
        .map(|item| vec![None; item.actions.len()])
        .collect();
    app.screen = Screen::RunView;

    let (tx, rx) = mpsc::channel();
    let sudo_tx = tx.clone();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let thread_abort_flag = Arc::clone(&abort_flag);
    let handle = std::thread::spawn(move || -> anyhow::Result<Run> {
        let result = crate::execute::execute_with_events_and_sudo(
            &plan,
            &cfg,
            |event| {
                let _ = tx.send(event);
            },
            || thread_abort_flag.load(Ordering::SeqCst),
            |item| {
                let (response_tx, response_rx) = mpsc::channel();
                let _ = sudo_tx.send(crate::execute::ExecuteEvent::SudoPrompt {
                    item: item.to_string(),
                    response: response_tx,
                });
                response_rx.recv().unwrap_or(false)
            },
        )?;
        let _ = crate::store::save(&result)?;
        Ok(result)
    });
    app.run_thread = Some(handle);
    app.run_events = Some(rx);
    app.abort_flag = Some(abort_flag);
}

fn render_run(f: &mut Frame, app: &mut App) {
    let area = f.area();
    drain_run_events(app);

    // Try to join the run thread (non-blocking).
    if let Some(handle) = &app.run_thread
        && handle.is_finished()
    {
        let handle = app.run_thread.take().unwrap();
        match handle.join() {
            Ok(Ok(run)) => {
                app.run = Some(run.clone());
                sync_finished_run_state(app, &run);
                app.abort_flag = None;
                app.run_events = None;
            }
            Ok(Err(e)) => {
                app.status_message = format!("run failed: {e}");
                app.abort_flag = None;
                app.run_events = None;
            }
            Err(_) => {
                app.status_message = "run thread panicked".into();
                app.abort_flag = None;
                app.run_events = None;
            }
        }
    }

    let aborting = run_is_aborting(app);
    let finished = finished_run_for_view(app).is_some();
    let border_color = run_border_color(app, aborting);
    let block = Block::default()
        .title(run_title(app, usize::from(area.width.saturating_sub(4))))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(border_color));
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(run_log_panel_height(area.height)),
            Constraint::Length(1),
        ])
        .split(area);

    let step_lines = run_body_lines(app, usize::from(chunks[0].width), chunks[0].height as usize);
    f.render_widget(
        Paragraph::new(step_lines).block(Block::default().borders(Borders::NONE)),
        chunks[0],
    );

    // Log.
    let log_lines: Vec<Line> = app
        .current_log
        .iter()
        .map(|entry| {
            let style = entry.fg.map(|c| Style::default().fg(c)).unwrap_or_default();
            Line::styled(entry.text.clone(), style)
        })
        .collect();
    let log_height = chunks[1].height.saturating_sub(2) as usize;
    let log_scroll = app
        .current_log
        .len()
        .saturating_sub(log_height)
        .min(u16::MAX as usize) as u16;
    f.render_widget(
        Paragraph::new(log_lines)
            .block(
                Block::default()
                    .title(" log ")
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            )
            .scroll((log_scroll, 0)),
        chunks[1],
    );

    let help = Paragraph::new(run_help_line(
        usize::from(chunks[2].width),
        aborting,
        finished,
    ));
    f.render_widget(help, chunks[2]);
}

fn sync_finished_run_state(app: &mut App, run: &Run) {
    app.progress = (run.items.len(), run.items.len());
    app.current_item = None;
    app.current_action = None;
    app.run_item_statuses = run.items.iter().map(|item| Some(item.status)).collect();
    app.run_action_statuses = run
        .items
        .iter()
        .map(|item| {
            item.actions
                .iter()
                .map(|action| Some(action.status))
                .collect()
        })
        .collect();
}

fn finished_run_for_view(app: &App) -> Option<&Run> {
    if app.run_thread.is_none() {
        app.run.as_ref()
    } else {
        None
    }
}

fn run_is_aborting(app: &App) -> bool {
    app.abort_flag
        .as_ref()
        .is_some_and(|flag| flag.load(Ordering::SeqCst))
}

fn run_border_color(app: &App, aborting: bool) -> Color {
    if aborting {
        CATPPUCCIN_MOCHA.warning
    } else if let Some(run) = finished_run_for_view(app) {
        match run.status {
            RunStatus::Running => CATPPUCCIN_MOCHA.running,
            RunStatus::Success => CATPPUCCIN_MOCHA.success,
            RunStatus::Failed => CATPPUCCIN_MOCHA.danger,
            RunStatus::Aborted => CATPPUCCIN_MOCHA.warning,
        }
    } else {
        CATPPUCCIN_MOCHA.running
    }
}

fn run_title(app: &App, width: usize) -> String {
    let icon_set = icons::current();
    let (state, done, total, current) = if let Some(run) = finished_run_for_view(app) {
        (
            run_status_label(run.status),
            run.items.len(),
            run.items.len(),
            final_run_summary(run),
        )
    } else if run_is_aborting(app) {
        (
            "Stopping",
            app.progress.0,
            app.progress.1,
            current_run_item_name(app).unwrap_or("waiting").to_string(),
        )
    } else {
        (
            "Running",
            app.progress.0,
            app.progress.1,
            current_run_item_name(app).unwrap_or("waiting").to_string(),
        )
    };
    let title = format!(
        " {} {state}: {:?}  {}/{}  {}  {} ",
        icon_set.running,
        app.mode,
        done,
        total,
        run_progress_bar(done, total, 10),
        current
    );
    fit_to_width(&title, width)
}

fn run_status_label(status: RunStatus) -> &'static str {
    match status {
        RunStatus::Running => "Running",
        RunStatus::Success => "Success",
        RunStatus::Failed => "Failed",
        RunStatus::Aborted => "Aborted",
    }
}

fn final_run_summary(run: &Run) -> String {
    let mut changed = 0;
    let mut no_change = 0;
    let mut failed = 0;
    for item in &run.items {
        if item.actions.is_empty() {
            match run_group_for_status(Some(item.status), false) {
                RunGroup::Changed => changed += 1,
                RunGroup::NoChange => no_change += 1,
                RunGroup::Failed => failed += 1,
                _ => {}
            }
            continue;
        }
        for action in &item.actions {
            match run_group_for_status(Some(action.status), false) {
                RunGroup::Changed => changed += 1,
                RunGroup::NoChange => no_change += 1,
                RunGroup::Failed => failed += 1,
                _ => {}
            }
        }
    }
    format!("{changed} changed, {no_change} no change, {failed} failed")
}

fn current_run_item_name(app: &App) -> Option<&str> {
    let plan = app.plan.as_ref()?;
    let index = app.current_item.or_else(|| app.progress.0.checked_sub(1))?;
    plan.items.get(index).map(|item| item.name.as_str())
}

fn run_progress_bar(done: usize, total: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if total == 0 {
        return "░".repeat(width);
    }
    let filled = (done.saturating_mul(width) + total / 2) / total;
    format!(
        "{}{}",
        "█".repeat(filled.min(width)),
        "░".repeat(width.saturating_sub(filled))
    )
}

fn run_log_panel_height(total_height: u16) -> u16 {
    let available = total_height.saturating_sub(2);
    let max_log = available.saturating_sub(4);
    let desired = if available >= 24 { 10 } else { 7 };
    desired.min(max_log)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunGroup {
    Failed,
    Running,
    Changed,
    NoChange,
    Skipped,
    Pending,
}

struct RunDisplayLine {
    group: RunGroup,
    line: Line<'static>,
    active: bool,
}

fn run_body_lines(app: &App, width: usize, height: usize) -> Vec<Line<'static>> {
    if height == 0 {
        return Vec::new();
    }
    let display_lines = if let Some(run) = finished_run_for_view(app) {
        finished_run_display_lines(run, width)
    } else if let Some(plan) = &app.plan {
        live_run_display_lines(app, plan, width)
    } else {
        return vec![Line::from("loading...")];
    };
    grouped_run_lines(display_lines, width, height)
}

fn live_run_display_lines(app: &App, plan: &Plan, width: usize) -> Vec<RunDisplayLine> {
    let mut lines = Vec::new();
    for (item_index, item) in plan.items.iter().enumerate() {
        if item.actions.is_empty() {
            let active = Some(item_index) == app.current_item;
            let status = if !item.selected {
                Some(ActionStatus::WillSkip)
            } else if active {
                None
            } else {
                app.run_item_statuses
                    .get(item_index)
                    .and_then(|status| *status)
            };
            lines.push(RunDisplayLine {
                group: run_group_for_status(status, active),
                line: run_action_line(
                    "shell",
                    &item.name,
                    "",
                    run_status_label_for_view(status, active),
                    status,
                    active,
                    width,
                ),
                active,
            });
            continue;
        }

        for (action_index, action) in item.actions.iter().enumerate() {
            let active = app.current_action == Some((item_index, action_index));
            let status = if !item.selected {
                Some(ActionStatus::WillSkip)
            } else if active {
                None
            } else {
                app.run_action_statuses
                    .get(item_index)
                    .and_then(|statuses| statuses.get(action_index))
                    .and_then(|status| *status)
            };
            lines.push(RunDisplayLine {
                group: run_group_for_status(status, active),
                line: run_action_line(
                    action_kind_for_view(action),
                    &item.name,
                    &action.describe(),
                    run_status_label_for_view(status, active),
                    status,
                    active,
                    width,
                ),
                active,
            });
        }
    }
    lines
}

fn finished_run_display_lines(run: &Run, width: usize) -> Vec<RunDisplayLine> {
    let mut lines = Vec::new();
    for item in &run.items {
        if item.actions.is_empty() {
            let status = Some(item.status);
            lines.push(RunDisplayLine {
                group: run_group_for_status(status, false),
                line: run_action_line(
                    "shell",
                    &item.name,
                    "",
                    run_status_label_for_view(status, false),
                    status,
                    false,
                    width,
                ),
                active: false,
            });
            continue;
        }

        for action in &item.actions {
            let status = Some(action.status);
            lines.push(RunDisplayLine {
                group: run_group_for_status(status, false),
                line: finished_action_line(item, action, width),
                active: false,
            });
        }
    }
    lines
}

fn grouped_run_lines(
    display_lines: Vec<RunDisplayLine>,
    width: usize,
    height: usize,
) -> Vec<Line<'static>> {
    let mut all_lines = Vec::new();
    let mut active_line = None;
    for group in [
        RunGroup::Failed,
        RunGroup::Running,
        RunGroup::Changed,
        RunGroup::NoChange,
        RunGroup::Skipped,
        RunGroup::Pending,
    ] {
        let group_lines = display_lines
            .iter()
            .filter(|line| line.group == group)
            .collect::<Vec<_>>();
        if group_lines.is_empty() {
            continue;
        }
        all_lines.push(run_group_header_line(group, group_lines.len(), width));
        for display_line in group_lines {
            if display_line.active {
                active_line = Some(all_lines.len());
            }
            all_lines.push(display_line.line.clone());
        }
    }

    if all_lines.len() <= height {
        return all_lines;
    }
    let focus = active_line.unwrap_or_else(|| all_lines.len().saturating_sub(1));
    let mut start = focus.saturating_sub(height / 2);
    if start + height > all_lines.len() {
        start = all_lines.len() - height;
    }
    let end = start + height;
    let mut visible = all_lines[start..end].to_vec();
    if start > 0
        && let Some(first) = visible.first_mut()
    {
        *first = Line::from(Span::styled(
            fit_to_width(&format!("  ... {start} above"), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    let below = all_lines.len().saturating_sub(end);
    if below > 0
        && let Some(last) = visible.last_mut()
    {
        *last = Line::from(Span::styled(
            fit_to_width(&format!("  ... {below} below"), width),
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ));
    }
    visible
}

fn run_group_for_status(status: Option<ActionStatus>, active: bool) -> RunGroup {
    if active {
        return RunGroup::Running;
    }
    match status {
        Some(ActionStatus::WillFail) => RunGroup::Failed,
        Some(ActionStatus::NoChange) => RunGroup::NoChange,
        Some(ActionStatus::WillSkip) => RunGroup::Skipped,
        Some(_) => RunGroup::Changed,
        None => RunGroup::Pending,
    }
}

fn run_group_header_line(group: RunGroup, count: usize, width: usize) -> Line<'static> {
    let icon_set = icons::current();
    let (icon, label, style) = match group {
        RunGroup::Failed => (
            icon_set.failed,
            "Failed",
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
        ),
        RunGroup::Running => (
            icon_set.running,
            "Running",
            Style::default().fg(CATPPUCCIN_MOCHA.running),
        ),
        RunGroup::Changed => (
            icon_set.success,
            "Changed",
            Style::default().fg(CATPPUCCIN_MOCHA.success),
        ),
        RunGroup::NoChange => (
            icon_set.info,
            "No Change",
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ),
        RunGroup::Skipped => (
            icon_set.skipped,
            "Skipped",
            Style::default().fg(CATPPUCCIN_MOCHA.skip),
        ),
        RunGroup::Pending => (
            icon_set.pending,
            "Pending",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    };
    Line::from(Span::styled(
        fit_to_width(&format!("{icon} {label} ({count})"), width),
        style.add_modifier(Modifier::BOLD),
    ))
}

fn finished_action_line(item: &RunItem, action: &RunAction, width: usize) -> Line<'static> {
    run_action_line(
        &action.kind,
        &item.name,
        &action.name,
        run_status_label_for_view(Some(action.status), false),
        Some(action.status),
        false,
        width,
    )
}

fn run_action_line(
    kind: &str,
    item_name: &str,
    action_name: &str,
    status_label: &'static str,
    status: Option<ActionStatus>,
    active: bool,
    width: usize,
) -> Line<'static> {
    let status_width = 10;
    let left_width = width.saturating_sub(status_width + 3);
    let name = if action_name.is_empty() || action_name == item_name {
        item_name.to_string()
    } else {
        format!("{item_name} / {action_name}")
    };
    let icon = if active {
        Span::styled(
            icons::SPINNER_BRAILLE[0],
            Style::default().fg(CATPPUCCIN_MOCHA.running),
        )
    } else {
        run_status_icon(status)
    };
    let status_style = run_status_style(status, active);
    Line::from(vec![
        icon,
        Span::raw(" "),
        Span::styled(
            run_action_kind_icon(kind),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::raw(" "),
        Span::styled(
            fit_to_width(&name, left_width),
            Style::default().fg(CATPPUCCIN_MOCHA.fg),
        ),
        Span::raw(" "),
        Span::styled(
            fit_to_width(status_label, status_width),
            status_style.add_modifier(if active {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
        ),
    ])
}

fn run_status_label_for_view(status: Option<ActionStatus>, active: bool) -> &'static str {
    if active {
        "running"
    } else {
        match status {
            Some(status) => run_item_status_label(status),
            None => "pending",
        }
    }
}

fn run_status_icon(status: Option<ActionStatus>) -> Span<'static> {
    let icon_set = icons::current();
    match status {
        Some(ActionStatus::WillFail) => Span::styled(
            icon_set.failed,
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
        ),
        Some(ActionStatus::WillSkip) => Span::styled(
            icon_set.skipped,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Some(ActionStatus::NoChange) => Span::styled(
            icon_set.info,
            Style::default().fg(CATPPUCCIN_MOCHA.text_muted),
        ),
        Some(_) => Span::styled(
            icon_set.success,
            Style::default().fg(CATPPUCCIN_MOCHA.success),
        ),
        None => Span::styled(
            icon_set.pending,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    }
}

fn run_status_style(status: Option<ActionStatus>, active: bool) -> Style {
    Style::default().fg(if active {
        CATPPUCCIN_MOCHA.running
    } else {
        match status {
            Some(ActionStatus::WillFail) => CATPPUCCIN_MOCHA.danger,
            Some(ActionStatus::WillSkip) => CATPPUCCIN_MOCHA.fg_dim,
            Some(ActionStatus::NoChange) => CATPPUCCIN_MOCHA.text_muted,
            Some(_) => CATPPUCCIN_MOCHA.success,
            None => CATPPUCCIN_MOCHA.fg_dim,
        }
    })
}

fn action_kind_for_view(action: &Action) -> &'static str {
    match action {
        Action::Install { .. } => "install",
        Action::Link { .. } => "link",
        Action::Create { .. } => "create",
        Action::Shell { .. } => "shell",
        Action::Clean { .. } => "clean",
    }
}

fn run_action_kind_icon(kind: &str) -> &'static str {
    let icon_set = icons::current();
    match kind {
        "install" => icon_set.action_install,
        "link" => icon_set.action_link,
        "create" => icon_set.action_create,
        "clean" => icon_set.action_clean,
        "shell" => icon_set.action_shell,
        _ => icon_set.info,
    }
}

fn drain_run_events(app: &mut App) {
    let Some(rx) = app.run_events.take() else {
        return;
    };
    while let Ok(event) = rx.try_recv() {
        match event {
            crate::execute::ExecuteEvent::ItemStarted { index, name } => {
                app.current_item = Some(index);
                app.current_action = None;
                push_log(app, &format!("started {name}"), None);
            }
            crate::execute::ExecuteEvent::ActionStarted {
                item_index,
                action_index,
                item,
                action,
            } => {
                app.current_action = Some((item_index, action_index));
                push_log(app, &format!("{item}: {action}"), None);
            }
            crate::execute::ExecuteEvent::ActionFinished {
                item_index,
                action_index,
                item,
                action,
                status,
            } => {
                if let Some(statuses) = app.run_action_statuses.get_mut(item_index)
                    && let Some(slot) = statuses.get_mut(action_index)
                {
                    *slot = Some(status);
                }
                app.current_action = None;
                push_log(app, &format!("{item}: finished {action}: {status:?}"), None);
            }
            crate::execute::ExecuteEvent::Output { item, stream, line } => {
                let color = match stream {
                    crate::model::OutputStream::Stderr => Some(CATPPUCCIN_MOCHA.danger),
                    crate::model::OutputStream::Stdout => None,
                    crate::model::OutputStream::Action => Some(CATPPUCCIN_MOCHA.primary),
                };
                push_log(app, &format!("{item}: {line}"), color);
            }
            crate::execute::ExecuteEvent::ActionMessage { item, message } => {
                push_log(
                    app,
                    &format!("{item}: {message}"),
                    Some(CATPPUCCIN_MOCHA.primary),
                );
            }
            crate::execute::ExecuteEvent::ItemFinished {
                index,
                name,
                status,
            } => {
                app.progress.0 = app.progress.0.max(index.saturating_add(1));
                app.current_item = None;
                app.current_action = None;
                if let Some(slot) = app.run_item_statuses.get_mut(index) {
                    *slot = Some(status);
                }
                push_log(app, &format!("finished {name}: {status:?}"), None);
            }
            crate::execute::ExecuteEvent::Aborted => {
                push_log(app, "run aborted", Some(CATPPUCCIN_MOCHA.warning));
            }
            crate::execute::ExecuteEvent::SudoPrompt { item, response } => {
                push_log(
                    app,
                    &format!("{item}: sudo session expired; re-authenticating"),
                    Some(CATPPUCCIN_MOCHA.warning),
                );
                let ok = match restore_terminal() {
                    Ok(()) => {
                        let ok = shell::pre_cache_sudo().unwrap_or(false);
                        let _ = setup_terminal();
                        app.needs_terminal_reset = true;
                        ok
                    }
                    Err(_) => false,
                };
                let _ = response.send(ok);
                if !ok {
                    push_log(
                        app,
                        &format!("{item}: sudo authentication failed"),
                        Some(CATPPUCCIN_MOCHA.danger),
                    );
                }
            }
        }
    }
    app.run_events = Some(rx);
}

fn push_log(app: &mut App, line: &str, fg: Option<Color>) {
    app.current_log.push(LogLine {
        text: sanitize_tui_log_line(line),
        fg,
    });
    // Cap per-step TUI log at MAX_TUI_OUTPUT_LINES (1000).
    if app.current_log.len() > MAX_TUI_OUTPUT_LINES {
        let drop_count = app.current_log.len() - MAX_TUI_OUTPUT_LINES;
        app.current_log.drain(0..drop_count);
    }
}

fn sanitize_tui_log_line(line: &str) -> String {
    let mut sanitized = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }

        if ch.is_control() {
            if ch == '\t' {
                sanitized.push(' ');
            }
            continue;
        }

        sanitized.push(ch);
    }

    sanitized
}

fn run_item_status_label(status: ActionStatus) -> &'static str {
    match status {
        ActionStatus::WillFail => "failed",
        ActionStatus::WillSkip => "skipped",
        ActionStatus::NoChange => "no change",
        ActionStatus::WillRun => "ran",
        _ => "changed",
    }
}

// ---------------- ResultView ----------------

fn handle_result(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Enter | KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('e') => {
            app.screen = Screen::PlanView;
        }
        _ => {}
    }
    Ok(())
}

fn render_result(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let run = match &app.run {
        Some(r) => r,
        None => {
            f.render_widget(
                Paragraph::new("no run result").alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let ok = run.items.iter().filter(|i| run_item_succeeded(i)).count();
    let failed = run.items.iter().filter(|i| i.error.is_some()).count();
    let total = run.items.len();
    let title = format!(
        " {} Run {} — {} ok, {} failed, {} total ",
        icon_set.app,
        if matches!(run.status, RunStatus::Success) {
            icon_set.success
        } else {
            icon_set.failed
        },
        ok,
        failed,
        total
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(if matches!(run.status, RunStatus::Success) {
            Style::default().fg(CATPPUCCIN_MOCHA.success)
        } else {
            Style::default().fg(CATPPUCCIN_MOCHA.danger)
        });
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let mut lines: Vec<Line> = Vec::new();
    for item in &run.items {
        let icon = if item.error.is_some() {
            Span::styled(
                icon_set.failed,
                Style::default().fg(CATPPUCCIN_MOCHA.danger),
            )
        } else if matches!(item.status, ActionStatus::WillSkip) {
            Span::styled(
                icon_set.skipped,
                Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
            )
        } else {
            Span::styled(
                icon_set.success,
                Style::default().fg(CATPPUCCIN_MOCHA.success),
            )
        };
        let name = &item.name;
        let mut spans = vec![icon, Span::raw(" "), Span::raw(name.clone())];
        if let Some(err) = &item.error {
            spans.push(Span::styled(
                format!("  {}", err),
                Style::default().fg(CATPPUCCIN_MOCHA.danger),
            ));
        }
        lines.push(Line::from(spans));
    }

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::NONE)),
        chunks[0],
    );

    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            " [enter] menu ",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::styled(" [e] plan ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(" [q] back ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
    ]));
    f.render_widget(help, chunks[1]);
}

// ---------------- HistoryView ----------------

fn handle_history(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let next = match app.list_state.selected() {
                Some(i) if i + 1 < app.runs.len() => i + 1,
                Some(_) => app.runs.len().saturating_sub(1),
                None => 0,
            };
            app.list_state.select(Some(next));
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let prev = match app.list_state.selected() {
                Some(0) | None => 0,
                Some(i) => i - 1,
            };
            app.list_state.select(Some(prev));
        }
        KeyCode::Enter => {
            if let Some(idx) = app.list_state.selected()
                && let Some(run) = app.runs.get(idx)
            {
                app.run = Some(run.clone());
                app.screen = Screen::RunReplay;
            }
        }
        KeyCode::Char('d') => {
            if let Some(idx) = app.list_state.selected() {
                let id = app.runs[idx].id.clone();
                if store::delete(&id).is_ok() {
                    app.runs.remove(idx);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn render_history(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let block = Block::default()
        .title(format!(
            " {} History ({} runs) ",
            icon_set.app,
            app.runs.len()
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.fg_dim));
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    if app.runs.is_empty() {
        f.render_widget(
            Paragraph::new("no runs yet").alignment(Alignment::Center),
            chunks[0],
        );
    } else {
        let items: Vec<ListItem> = app
            .runs
            .iter()
            .map(|r| {
                let status = format!("{:?}", r.status).to_lowercase();
                let mode = format!("{:?}", r.mode).to_lowercase();
                ListItem::new(format!("{}  {}  {}  {}", r.started_at, mode, status, r.id))
            })
            .collect();
        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(CATPPUCCIN_MOCHA.bg)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list, chunks[0], &mut app.list_state);
    }

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" [↑↓] nav ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(
            " [enter] view ",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::styled(" [d] delete ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(" [q] back ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
    ]));
    f.render_widget(help, chunks[1]);
}

// ---------------- RunReplay ----------------

fn handle_replay(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::HistoryView;
        }
        _ => {}
    }
    Ok(())
}

fn render_replay(f: &mut Frame, app: &mut App) {
    let icon_set = icons::current();
    let area = f.area();
    let run = match &app.run {
        Some(r) => r,
        None => {
            f.render_widget(
                Paragraph::new("no run loaded").alignment(Alignment::Center),
                area,
            );
            return;
        }
    };

    let block = Block::default()
        .title(format!(
            " {} Replay: {} — {:?} ",
            icon_set.app, run.id, run.mode
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.fg_dim));
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let mut lines: Vec<Line> = Vec::new();
    for item in &run.items {
        let icon = if item.error.is_some() {
            icon_set.failed
        } else if matches!(item.status, ActionStatus::WillSkip) {
            icon_set.skipped
        } else {
            icon_set.success
        };
        let name = &item.name;
        let dur = item
            .duration_ms
            .map(|d| format!(" ({:.1}s)", d as f64 / 1000.0))
            .unwrap_or_default();
        let attempts = if item.attempts > 1 {
            format!(" ({} attempts)", item.attempts)
        } else {
            String::new()
        };
        lines.push(Line::from(format!(
            "{:<3} {:<24}{}{}",
            icon, name, dur, attempts
        )));
    }

    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::NONE)),
        chunks[0],
    );

    let help = Paragraph::new(Line::from(vec![Span::styled(
        " [q] back ",
        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
    )]));
    f.render_widget(help, chunks[1]);
}

// ---------------- render dispatch ----------------

fn run_item_succeeded(item: &crate::model::RunItem) -> bool {
    item.error.is_none() && !matches!(item.status, ActionStatus::WillSkip)
}

fn render(app: &mut App, f: &mut Frame) {
    let area = f.area();
    f.render_widget(Clear, area);
    match app.screen {
        Screen::MainMenu => render_main_menu(f, app),
        Screen::PlanView => render_plan(f, app),
        Screen::ConfirmView => render_confirm(f, app),
        Screen::RunView => render_run(f, app),
        Screen::ResultView => render_result(f, app),
        Screen::HistoryView => render_history(f, app),
        Screen::RunReplay => render_replay(f, app),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_plan_item(name: &str) -> PlanItem {
        PlanItem {
            id: name.to_string(),
            name: name.to_string(),
            layer: "misc".into(),
            actions: Vec::new(),
            selected: true,
        }
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    fn lines_text(lines: &[Line<'_>]) -> String {
        lines.iter().map(line_text).collect::<Vec<_>>().join("\n")
    }

    fn test_plan_with_items(names: &[&str]) -> Plan {
        Plan {
            id: "test-run".into(),
            mode: PlanMode::Deploy,
            created_at: "2026-01-01T00:00:00Z".into(),
            config_path: PathBuf::from("dotman.yaml"),
            config_hash: "hash".into(),
            host: crate::model::HostInfo {
                hostname: "host".into(),
                os: "macos".into(),
                arch: "aarch64".into(),
                user: "user".into(),
                home: PathBuf::from("/tmp"),
            },
            items: names.iter().map(|name| test_plan_item(name)).collect(),
            auto_install_pkg_manager: false,
        }
    }

    fn test_run_item(name: &str, status: ActionStatus, error: Option<&str>) -> RunItem {
        RunItem {
            id: name.to_string(),
            name: name.to_string(),
            status,
            started_at: Some("2026-01-01T00:00:00Z".into()),
            finished_at: Some("2026-01-01T00:00:01Z".into()),
            duration_ms: Some(1000),
            attempts: 1,
            error: error.map(str::to_string),
            output: Vec::new(),
            actions: vec![RunAction {
                kind: "shell".into(),
                name: name.to_string(),
                status,
                error: error.map(str::to_string),
                output: Vec::new(),
            }],
        }
    }

    fn test_run(status: RunStatus, items: Vec<RunItem>) -> Run {
        Run {
            id: "test-run".into(),
            mode: PlanMode::Deploy,
            started_at: "2026-01-01T00:00:00Z".into(),
            finished_at: Some("2026-01-01T00:00:01Z".into()),
            status,
            config_hash: "hash".into(),
            items,
        }
    }

    #[test]
    fn tui_log_sanitizer_strips_terminal_control_sequences() {
        let line = "fetch \x1b[31mred\x1b[0m\rprogress\tok\x07";
        assert_eq!(sanitize_tui_log_line(line), "fetch redprogress ok");
    }

    #[test]
    fn plan_help_line_fits_narrow_terminal() {
        let line = plan_help_line(78);
        let text = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(line_display_width(&line) <= 78);
        assert!(text.contains("[R]"));
        assert!(text.contains("[Q]"));
        assert!(!text.contains("Info"));
    }

    #[test]
    fn plan_help_line_keeps_full_labels_when_space_allows() {
        let line = plan_help_line(120);
        let text = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(text.contains("Navigate"));
        assert!(text.contains("Review"));
        assert!(text.contains("Back"));
        assert!(!text.contains("Info"));
    }

    #[test]
    fn review_help_line_fits_narrow_terminal() {
        let line = review_help_line(44);
        let text = line_text(&line);

        assert!(line_display_width(&line) <= 44);
        assert!(text.contains("[R]") || text.contains("[Enter/R]"));
        assert!(text.contains("[E/Q]"));
    }

    #[test]
    fn review_help_line_keeps_full_labels_when_space_allows() {
        let line = review_help_line(100);
        let text = line_text(&line);

        assert!(text.contains("Scroll"));
        assert!(text.contains("Page"));
        assert!(text.contains("Run"));
        assert!(text.contains("Edit"));
    }

    #[test]
    fn run_progress_bar_reflects_done_ratio() {
        assert_eq!(run_progress_bar(0, 4, 8), "░░░░░░░░");
        assert_eq!(run_progress_bar(2, 4, 8), "████░░░░");
        assert_eq!(run_progress_bar(4, 4, 8), "████████");
    }

    #[test]
    fn run_help_line_switches_to_stopping() {
        let running = line_text(&run_help_line(40, false, false));
        let stopping = line_text(&run_help_line(40, true, false));
        let finished = line_text(&run_help_line(40, false, true));

        assert!(running.contains("Abort"));
        assert!(stopping.contains("Stopping"));
        assert!(finished.contains("Back"));
        assert!(line_display_width(&run_help_line(8, true, false)) <= 8);
    }

    #[test]
    fn run_title_includes_current_item_and_abort_state() {
        let mut app = App::new(Mode::Deploy);
        app.plan = Some(test_plan_with_items(&["ghostty", "fish"]));
        app.progress = (1, 2);
        app.current_item = Some(1);

        let title = run_title(&app, 80);
        assert!(title.contains("Running"));
        assert!(title.contains("1/2"));
        assert!(title.contains("fish"));

        app.abort_flag = Some(Arc::new(AtomicBool::new(true)));
        let stopping = run_title(&app, 80);
        assert!(stopping.contains("Stopping"));
    }

    #[test]
    fn run_title_keeps_finished_summary() {
        let mut app = App::new(Mode::Deploy);
        app.run = Some(test_run(
            RunStatus::Success,
            vec![
                test_run_item("ghostty", ActionStatus::NoChange, None),
                test_run_item("config", ActionStatus::WillLink, None),
            ],
        ));

        let title = run_title(&app, 100);

        assert!(title.contains("Success"));
        assert!(title.contains("2/2"));
        assert!(title.contains("1 changed"));
        assert!(title.contains("1 no change"));
    }

    #[test]
    fn run_body_lines_group_live_actions() {
        let mut app = App::new(Mode::Deploy);
        let mut item = test_plan_item("ghostty");
        item.actions = vec![Action::Shell {
            command: "echo ghostty".into(),
            description: Some("check ghostty".into()),
            optional: false,
            if_condition: None,
        }];
        app.plan = Some(Plan {
            items: vec![item],
            ..test_plan_with_items(&[])
        });
        app.run_action_statuses = vec![vec![Some(ActionStatus::NoChange)]];

        let done = lines_text(&run_body_lines(&app, 60, 8));
        assert!(done.contains("No Change (1)"));
        assert!(done.contains("no change"));

        app.current_action = Some((0, 0));
        let running = lines_text(&run_body_lines(&app, 60, 8));
        assert!(running.contains("Running (1)"));
        assert!(running.contains("running"));

        app.current_action = None;
        if let Some(plan) = &mut app.plan {
            plan.items[0].selected = false;
        }
        let skipped = lines_text(&run_body_lines(&app, 60, 8));
        assert!(skipped.contains("Skipped (1)"));
        assert!(skipped.contains("skipped"));
    }

    #[test]
    fn run_body_lines_group_finished_actions() {
        let run = test_run(
            RunStatus::Failed,
            vec![
                test_run_item("config", ActionStatus::WillLink, None),
                test_run_item("ghostty", ActionStatus::NoChange, None),
                test_run_item("shell", ActionStatus::WillFail, Some("exit code 1")),
            ],
        );
        let mut app = App::new(Mode::Deploy);
        app.run = Some(run);
        let text = lines_text(&run_body_lines(&app, 80, 12));

        assert!(text.contains("Failed (1)"));
        assert!(text.contains("Changed (1)"));
        assert!(text.contains("No Change (1)"));
        assert!(text.contains("changed"));
        assert!(text.contains("no change"));
        assert!(text.contains("failed"));
    }

    #[test]
    fn sync_finished_run_state_preserves_final_statuses() {
        let run = test_run(
            RunStatus::Success,
            vec![
                test_run_item("ghostty", ActionStatus::NoChange, None),
                test_run_item("config", ActionStatus::WillLink, None),
            ],
        );
        let mut app = App::new(Mode::Deploy);
        app.current_item = Some(0);

        sync_finished_run_state(&mut app, &run);

        assert_eq!(app.progress, (2, 2));
        assert_eq!(app.current_item, None);
        assert_eq!(
            app.run_item_statuses,
            vec![Some(ActionStatus::NoChange), Some(ActionStatus::WillLink)]
        );
        assert_eq!(
            app.run_action_statuses,
            vec![
                vec![Some(ActionStatus::NoChange)],
                vec![Some(ActionStatus::WillLink)]
            ]
        );
    }

    #[test]
    fn review_sudo_check_ignores_already_ok_entries() {
        let icon_set = icons::current();
        let entries = vec![ReviewEntry {
            order: 0,
            item: "ghostty".into(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Success,
            status: "present".into(),
            detail: "sudo pacman -S --needed --noconfirm ghostty".into(),
        }];

        assert!(!review_entries_need_sudo(&entries));
    }

    #[test]
    fn review_sudo_check_detects_pending_sudo_entries() {
        let icon_set = icons::current();
        let entries = vec![ReviewEntry {
            order: 0,
            item: "ghostty".into(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Run,
            status: "missing".into(),
            detail: "sudo pacman -S --needed --noconfirm ghostty".into(),
        }];

        assert!(review_entries_need_sudo(&entries));
    }

    #[test]
    fn review_entry_uses_one_line_when_it_fits() {
        let entry = ReviewEntry {
            order: 0,
            item: "ghostty".into(),
            kind: "install",
            kind_icon: icons::current().action_install,
            severity: ReviewSeverity::Success,
            status: "present".into(),
            detail: "brew install --cask ghostty".into(),
        };

        let lines = review_entry_lines(&entry, 76);
        let text = line_text(&lines[0]);

        assert_eq!(lines.len(), 1);
        assert!(line_display_width(&lines[0]) <= 76);
        assert!(text.contains("present"));
        assert!(text.contains("brew install --cask ghostty"));
    }

    #[test]
    fn review_entry_keeps_two_lines_for_long_details() {
        let entry = ReviewEntry {
            order: 0,
            item: "ghostty".into(),
            kind: "link",
            kind_icon: icons::current().action_link,
            severity: ReviewSeverity::Success,
            status: "linked".into(),
            detail: "~/.config/ghostty -> config/ghostty".into(),
        };

        let lines = review_entry_lines(&entry, 32);

        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|line| line_display_width(line) <= 32));
    }

    #[test]
    fn review_entry_omits_duplicate_detail() {
        let entry = ReviewEntry {
            order: 0,
            item: "Sync fish plugins".into(),
            kind: "shell",
            kind_icon: icons::current().action_shell,
            severity: ReviewSeverity::Warning,
            status: "if ok · optional".into(),
            detail: "Sync fish plugins".into(),
        };

        let lines = review_entry_lines(&entry, 76);
        let text = lines_text(&lines);

        assert_eq!(lines.len(), 1);
        assert_eq!(text.matches("Sync fish plugins").count(), 1);
    }

    #[test]
    fn review_group_entries_sort_by_action_kind_then_plan_order() {
        let icon_set = icons::current();
        let entries = vec![
            ReviewEntry {
                order: 0,
                item: "z-link".into(),
                kind: "link",
                kind_icon: icon_set.action_link,
                severity: ReviewSeverity::Success,
                status: "linked".into(),
                detail: String::new(),
            },
            ReviewEntry {
                order: 1,
                item: "b-install".into(),
                kind: "install",
                kind_icon: icon_set.action_install,
                severity: ReviewSeverity::Success,
                status: "present".into(),
                detail: String::new(),
            },
            ReviewEntry {
                order: 2,
                item: "a-install".into(),
                kind: "install",
                kind_icon: icon_set.action_install,
                severity: ReviewSeverity::Success,
                status: "present".into(),
                detail: String::new(),
            },
            ReviewEntry {
                order: 3,
                item: "x-shell".into(),
                kind: "shell",
                kind_icon: icon_set.action_shell,
                severity: ReviewSeverity::Success,
                status: "run".into(),
                detail: String::new(),
            },
            ReviewEntry {
                order: 4,
                item: "y-create".into(),
                kind: "create",
                kind_icon: icon_set.action_create,
                severity: ReviewSeverity::Success,
                status: "exists".into(),
                detail: String::new(),
            },
        ];

        let sorted = sorted_review_group_entries(&entries, ReviewGroup::AlreadyOk);
        let names = sorted
            .iter()
            .map(|entry| entry.item.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec!["b-install", "a-install", "z-link", "y-create", "x-shell"]
        );
    }

    #[test]
    fn review_body_shows_group_headers() {
        let icon_set = icons::current();
        let entries = vec![
            ReviewEntry {
                order: 0,
                item: "sudo shell".into(),
                kind: "shell",
                kind_icon: icon_set.action_shell,
                severity: ReviewSeverity::Warning,
                status: "run · sudo".into(),
                detail: String::new(),
            },
            ReviewEntry {
                order: 1,
                item: "install".into(),
                kind: "install",
                kind_icon: icon_set.action_install,
                severity: ReviewSeverity::Run,
                status: "missing".into(),
                detail: String::new(),
            },
            ReviewEntry {
                order: 2,
                item: "link".into(),
                kind: "link",
                kind_icon: icon_set.action_link,
                severity: ReviewSeverity::Success,
                status: "linked".into(),
                detail: String::new(),
            },
            ReviewEntry {
                order: 3,
                item: "skip".into(),
                kind: "shell",
                kind_icon: icon_set.action_shell,
                severity: ReviewSeverity::Skip,
                status: "if skip".into(),
                detail: String::new(),
            },
        ];
        let mut scroll = 0;

        let lines = review_body_lines(&entries, 76, 20, &mut scroll);
        let text = lines_text(&lines);

        assert!(text.contains("Attention (1)"));
        assert!(text.contains("Will Run (1)"));
        assert!(text.contains("Already OK (1)"));
        assert!(text.contains("Skipped (1)"));
        assert_eq!(scroll, 0);
    }

    #[test]
    fn review_body_shows_scroll_markers() {
        let icon_set = icons::current();
        let entries = (0..8)
            .map(|idx| ReviewEntry {
                order: idx,
                item: format!("item-{idx}"),
                kind: "install",
                kind_icon: icon_set.action_install,
                severity: ReviewSeverity::Success,
                status: "present".into(),
                detail: String::new(),
            })
            .collect::<Vec<_>>();
        let mut scroll = 2;

        let lines = review_body_lines(&entries, 76, 4, &mut scroll);
        let text = lines_text(&lines);

        assert!(text.contains("2 above"));
        assert!(text.contains("below"));
        assert_eq!(lines.len(), 4);
    }

    #[test]
    fn optional_shell_that_will_run_is_not_attention() {
        let item = test_plan_item("Sync fish plugins");
        let entry = review_shell_entry(
            &item,
            "true",
            Some("Sync fish plugins"),
            true,
            Some("true"),
            icons::current().action_shell,
        );

        assert_eq!(entry.severity, ReviewSeverity::Run);
        assert_eq!(review_group_for(&entry), ReviewGroup::WillRun);
        assert_eq!(entry.status, "if ok · optional");
    }

    #[test]
    fn sudo_shell_stays_attention() {
        let item = test_plan_item("Set default shell");
        let entry = review_shell_entry(
            &item,
            "sudo chsh -s /opt/homebrew/bin/fish",
            Some("Set default shell to fish"),
            false,
            None,
            icons::current().action_shell,
        );

        assert_eq!(entry.severity, ReviewSeverity::Warning);
        assert_eq!(review_group_for(&entry), ReviewGroup::Attention);
        assert!(entry.status.contains("sudo"));
    }

    #[test]
    fn shell_with_false_condition_is_skipped() {
        let item = test_plan_item("Skip shell");
        let entry = review_shell_entry(
            &item,
            "echo skipped",
            None,
            true,
            Some("false"),
            icons::current().action_shell,
        );

        assert_eq!(entry.severity, ReviewSeverity::Skip);
        assert_eq!(review_group_for(&entry), ReviewGroup::Skipped);
        assert_eq!(entry.status, "if skip");
    }
}
