//! TUI: app + all screens in one file.
//!
//! Phase 5+7 minimal: MainMenu, PlanView, RunView, ResultView, HistoryView.

use crate::cli::Mode;
use crate::config;
use crate::execute::MAX_TUI_OUTPUT_LINES;
use crate::icons;
use crate::model::{
    Action, ActionStatus, Mode as PlanMode, Plan, PlanItem, Run, RunStatus, Selection,
};
use crate::ops::clean;
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
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

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
    pub run_item_statuses: Vec<Option<ActionStatus>>,
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
            run_item_statuses: Vec::new(),
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
    let divider_width = usize::from(chunks[0].width).saturating_sub(title_prefix.chars().count());
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
            let title_content_w = 2 + title_text.chars().count();
            let desc_content_w = 4 + desc.chars().count();
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
        KeyCode::Char('i') => show_plan_info(app, &rows),
        KeyCode::Char('r') => {
            if matches!(app.mode, Mode::Plan) {
                app.status_message = "plan mode is read-only; choose deploy to run".into();
                app.status_is_focus_info = false;
            } else if selected_item_count(app.plan.as_ref()) == 0 {
                app.status_message = "nothing selected".into();
                app.status_is_focus_info = false;
            } else {
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
    let divider_width = usize::from(chunks[0].width).saturating_sub(status_prefix.chars().count());
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
        Line::from(vec![
            keycap("↑↓"),
            hint(" Navigate  "),
            keycap("Space"),
            hint(" Toggle  "),
            keycap("Enter"),
            hint(" Expand  "),
            keycap("1-6"),
            hint(" Jump  "),
            keycap("I"),
            hint(" Info  "),
            keycap("S"),
            hint(" Save  "),
            keycap("R"),
            hint(" Review  "),
            keycap("Q"),
            hint(" Back"),
        ]),
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
    let right_width = right.chars().count();
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

fn show_plan_info(app: &mut App, rows: &[PlanRow]) {
    let Some(plan) = &app.plan else {
        app.status_message = "no plan loaded".into();
        app.status_is_focus_info = false;
        return;
    };
    let Some(row_idx) = app.list_state.selected() else {
        app.status_message = "nothing focused".into();
        app.status_is_focus_info = false;
        return;
    };

    match rows.get(row_idx) {
        Some(PlanRow::Header {
            layer,
            enabled,
            total,
            ..
        }) => {
            let state = if app.collapsed_layers.contains(layer) {
                "collapsed"
            } else {
                "expanded"
            };
            app.status_message =
                format!("{}: {enabled}/{total} selected, {state}", capitalize(layer));
            app.status_is_focus_info = true;
        }
        Some(PlanRow::Item(item_idx)) => {
            if let Some(item) = plan.items.get(*item_idx) {
                app.status_message = plan_item_info(item);
                app.status_is_focus_info = true;
            }
        }
        Some(PlanRow::InlineItems(item_indices)) => {
            let col = app.grid_col.min(item_indices.len().saturating_sub(1));
            if let Some(item_idx) = item_indices.get(col)
                && let Some(item) = plan.items.get(*item_idx)
            {
                app.status_message = plan_item_info(item);
                app.status_is_focus_info = true;
            }
        }
        Some(PlanRow::Divider) | None => {
            clear_focus_info(app);
        }
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

fn fit_to_width(value: &str, width: usize) -> String {
    let len = value.chars().count();
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
    let mut out = value.chars().take(width - 3).collect::<String>();
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
        KeyCode::Enter | KeyCode::Char('r') => {
            // Pre-cache sudo credentials before executing if the plan needs them.
            if let Some(plan) = &app.plan
                && plan.needs_sudo()
            {
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

fn render_confirm(f: &mut Frame, app: &App) {
    let icon_set = icons::current();
    let area = f.area();
    let block = Block::default()
        .title(format!(
            " {} Review changes — {:?} ",
            icon_set.warning, app.mode
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.warning));
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
    let risk_lines = review_lines(plan);
    let risk_count = risk_lines.len();

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
            Span::raw(if risk_count == 0 {
                "no file-changing link/clean actions need attention".to_string()
            } else {
                format!("{risk_count} file-changing items below")
            }),
        ]),
    ];
    f.render_widget(Paragraph::new(summary), chunks[0]);

    let body = if risk_lines.is_empty() {
        vec![Line::from(
            "No selected link or clean actions require review.",
        )]
    } else {
        risk_lines
    };
    f.render_widget(
        Paragraph::new(body)
            .block(
                Block::default()
                    .title(" changes ")
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );

    let help = Paragraph::new(Line::from(vec![
        Span::styled(
            " [enter/r] run ",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::styled(
            " [e/q] edit plan ",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    ]));
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

fn review_lines(plan: &Plan) -> Vec<Line<'static>> {
    let icon_set = icons::current();
    let config_dir = plan
        .config_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let mut lines = Vec::new();
    for item in plan.items.iter().filter(|item| item.selected) {
        for action in &item.actions {
            match action {
                Action::Link { target, source } => {
                    let settings = LinkSettings {
                        create: true,
                        relative: true,
                        backup: true,
                        relink: false,
                    };
                    let status = match link::plan_link(config_dir, target, source, settings) {
                        Ok(link_plan) => describe_link_action(&link_plan.action),
                        Err(e) => format!("inspect failed: {e}"),
                    };
                    lines.push(review_line(
                        &item.name,
                        icon_set.warning,
                        &status,
                        &format!("{} -> {}", target.display(), source.display()),
                    ));
                }
                Action::Clean { target, force } => {
                    let status = match clean::plan_clean(target, *force) {
                        Ok(clean_action) => format!("{clean_action:?}"),
                        Err(e) => format!("inspect failed: {e}"),
                    };
                    lines.push(review_line(
                        &item.name,
                        icon_set.warning,
                        &status,
                        &target.display().to_string(),
                    ));
                }
                _ => {}
            }
        }
    }
    lines
}

fn describe_link_action(action: &LinkAction) -> String {
    match action {
        LinkAction::Skip => "skip: already linked".into(),
        LinkAction::Link => "link: create symlink".into(),
        LinkAction::Backup(path) => format!("backup then link: {}", path.display()),
        LinkAction::Relink => "relink: replace wrong symlink".into(),
        LinkAction::Fail(reason) => format!("fail: {reason}"),
    }
}

fn review_line(item: &str, icon: &'static str, status: &str, detail: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(icon, Style::default().fg(CATPPUCCIN_MOCHA.warning)),
        Span::raw(" "),
        Span::styled(item.to_string(), Style::default().fg(CATPPUCCIN_MOCHA.fg)),
        Span::raw(" — "),
        Span::styled(
            status.to_string(),
            Style::default().fg(CATPPUCCIN_MOCHA.accent),
        ),
        Span::raw("  "),
        Span::styled(
            detail.to_string(),
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    ])
}

// ---------------- RunView ----------------

fn handle_run(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            if let Some(flag) = &app.abort_flag {
                flag.store(true, Ordering::SeqCst);
                app.status_message = "abort requested; waiting for current action".into();
                push_log(app, "abort requested; waiting for current action", None);
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
    app.run_started = Some(Instant::now());
    app.current_log.clear();
    app.run_item_statuses = vec![None; plan.items.len()];
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
    let icon_set = icons::current();
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
                app.progress.0 = app.progress.1;
                app.abort_flag = None;
                app.run_events = None;
                app.screen = Screen::ResultView;
            }
            Ok(Err(e)) => {
                app.status_message = format!("run failed: {e}");
                app.abort_flag = None;
                app.run_events = None;
                app.screen = Screen::ResultView;
            }
            Err(_) => {
                app.status_message = "run thread panicked".into();
                app.abort_flag = None;
                app.run_events = None;
                app.screen = Screen::ResultView;
            }
        }
    }

    let block = Block::default()
        .title(format!(
            " {} Running: {:?} — {}/{} ",
            icon_set.running, app.mode, app.progress.0, app.progress.1
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.running));
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(40),
            Constraint::Length(1),
        ])
        .split(area);

    // Steps list.
    let step_lines: Vec<Line> = if let Some(plan) = &app.plan {
        let mut lines = Vec::new();
        for (i, item) in plan.items.iter().enumerate() {
            let icon = run_step_icon(app, i, item);
            let name = if item.selected {
                item.name.clone()
            } else {
                format!("{} (skipped)", item.name)
            };
            lines.push(Line::from(vec![icon, Span::raw(" "), Span::raw(name)]));
        }
        lines
    } else {
        vec![Line::from("loading...")]
    };
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

    let help = Paragraph::new(Line::from(vec![Span::styled(
        " [q] abort ",
        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
    )]));
    f.render_widget(help, chunks[2]);
}

fn drain_run_events(app: &mut App) {
    let Some(rx) = app.run_events.take() else {
        return;
    };
    while let Ok(event) = rx.try_recv() {
        match event {
            crate::execute::ExecuteEvent::ItemStarted { index, name } => {
                app.current_item = Some(index);
                push_log(app, &format!("started {name}"), None);
            }
            crate::execute::ExecuteEvent::ActionStarted { item, action } => {
                push_log(app, &format!("{item}: {action}"), None);
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
        text: line.to_string(),
        fg,
    });
    // Cap per-step TUI log at MAX_TUI_OUTPUT_LINES (1000).
    if app.current_log.len() > MAX_TUI_OUTPUT_LINES {
        let drop_count = app.current_log.len() - MAX_TUI_OUTPUT_LINES;
        app.current_log.drain(0..drop_count);
    }
}

fn run_step_icon(app: &App, index: usize, item: &PlanItem) -> Span<'static> {
    let icon_set = icons::current();
    if !item.selected {
        return Span::styled(
            icon_set.skipped,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        );
    }

    if Some(index) == app.current_item {
        return Span::styled(
            icons::SPINNER_BRAILLE[app.spinner_frame % icons::SPINNER_BRAILLE.len()],
            Style::default().fg(CATPPUCCIN_MOCHA.running),
        );
    }

    match app.run_item_statuses.get(index).and_then(|status| *status) {
        Some(ActionStatus::WillFail) => Span::styled(
            icon_set.failed,
            Style::default().fg(CATPPUCCIN_MOCHA.danger),
        ),
        Some(ActionStatus::WillSkip) => Span::styled(
            icon_set.skipped,
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
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
