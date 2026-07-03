//! TUI: app + all screens in one file.
//!
//! Phase 5+7 minimal: MainMenu, PlanView, RunView, ResultView, HistoryView.

use crate::Mode;
use crate::config;
use crate::icons;
use crate::model::{ActionStatus, Mode as PlanMode, Plan, Run, RunStatus};
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
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::io;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    MainMenu,
    PlanView,
    RunView,
    ResultView,
    HistoryView,
    RunReplay,
}

pub struct App {
    pub screen: Screen,
    pub mode: Mode,
    pub config: Option<config::Config>,
    pub plan: Option<Plan>,
    pub run: Option<Run>,
    pub runs: Vec<Run>,
    pub list_state: ListState,
    pub status_message: String,
    pub should_quit: bool,
    pub dirty: bool,
    // For RunView
    pub spinner_frame: usize,
    pub run_thread: Option<std::thread::JoinHandle<anyhow::Result<Run>>>,
    pub progress: (usize, usize), // (done, total)
    pub current_log: Vec<String>,
    pub run_started: Option<Instant>,
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
            status_message: String::new(),
            should_quit: false,
            dirty: false,
            spinner_frame: 0,
            run_thread: None,
            progress: (0, 0),
            current_log: Vec::new(),
            run_started: None,
        }
    }

    pub fn load_config(&mut self) -> Result<(), String> {
        let path = find_config_path().map_err(|e| e.to_string())?;
        let cfg = config::load(&path).map_err(|e| e.to_string())?;
        self.config = Some(cfg);
        Ok(())
    }

    pub fn build_plan(&mut self) -> Result<(), String> {
        let cfg = self.config.as_ref().ok_or("config not loaded")?;
        let plan_mode = match self.mode {
            Mode::Deploy | Mode::Bootstrap => match self.mode {
                Mode::Deploy => PlanMode::Deploy,
                Mode::Bootstrap => PlanMode::Bootstrap,
                _ => unreachable!(),
            },
            _ => PlanMode::Deploy,
        };
        let plan = plan::build(cfg, plan_mode).map_err(|e| e.to_string())?;
        self.plan = Some(plan);
        self.dirty = false;
        Ok(())
    }

    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % icons::SPINNER_BRAILLE.len();
    }
}

pub fn run(mode: Mode) -> Result<(), String> {
    let mut terminal = setup_terminal().map_err(|e| e.to_string())?;
    let mut app = App::new(mode);
    if let Err(e) = app.load_config() {
        // Defer error to first render; user sees message.
        app.status_message = e;
    }
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

fn find_config_path() -> anyhow::Result<std::path::PathBuf> {
    let candidates = [
        std::path::PathBuf::from("dotman.yaml"),
        std::path::PathBuf::from("dotman.bootstrap.yaml"),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    if let Ok(dir) = std::env::var("DOTFILES_DIR") {
        let p = std::path::PathBuf::from(dir).join("dotman.yaml");
        if p.exists() {
            return Ok(p);
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let p = std::path::PathBuf::from(home)
        .join(".local/share/tabsp-dotfiles")
        .join("dotman.yaml");
    if p.exists() {
        return Ok(p);
    }
    anyhow::bail!("no dotman.yaml found in standard locations")
}

fn run_event_loop<B: ratatui::backend::Backend>(
    app: &mut App,
    terminal: &mut ratatui::Terminal<B>,
) -> Result<()> {
    let mut last_tick = Instant::now();
    loop {
        if app.should_quit {
            return Ok(());
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
            app.build_plan().ok();
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('b') => {
            app.mode = Mode::Bootstrap;
            app.build_plan().ok();
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('p') => {
            app.mode = Mode::Plan;
            app.build_plan().ok();
            app.screen = Screen::PlanView;
        }
        KeyCode::Char('h') => {
            app.runs = store::list().unwrap_or_default();
            app.screen = Screen::HistoryView;
        }
        _ => {}
    }
    Ok(())
}

fn render_main_menu(f: &mut Frame, app: &App) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.fg_dim));
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(Line::from(vec![
        Span::styled("⚙ ", Style::default().fg(CATPPUCCIN_MOCHA.primary)),
        Span::styled(
            "dotman",
            Style::default()
                .fg(CATPPUCCIN_MOCHA.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " — v2 dev env config assistant",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
    ]));
    f.render_widget(title, chunks[0]);

    let options: Vec<ListItem> = [
        "[d]  Deploy".to_string(),
        "[b]  Bootstrap".to_string(),
        "[p]  Plan only".to_string(),
        "[h]  History".to_string(),
        "[q]  Quit".to_string(),
    ]
    .into_iter()
    .map(ListItem::new)
    .collect();
    let list = List::new(options)
        .highlight_style(
            Style::default()
                .fg(CATPPUCCIN_MOCHA.accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, chunks[1], &mut app.list_state.clone());

    let status = if app.status_message.is_empty() {
        "press d/b/p/h/q".to_string()
    } else {
        app.status_message.clone()
    };
    f.render_widget(
        Paragraph::new(status).style(Style::default().fg(CATPPUCCIN_MOCHA.warning)),
        chunks[2],
    );
}

// ---------------- PlanView ----------------

fn handle_plan(app: &mut App, key: KeyCode) -> Result<()> {
    let items = match &app.plan {
        Some(p) => p.items.len(),
        None => 0,
    };
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.screen = Screen::MainMenu;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let next = match app.list_state.selected() {
                Some(i) if i + 1 < items => i + 1,
                Some(_) => items.saturating_sub(1),
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
        KeyCode::Char(' ') => {
            if let Some(plan) = &mut app.plan
                && let Some(idx) = app.list_state.selected()
                && let Some(item) = plan.items.get_mut(idx)
            {
                item.selected = !item.selected;
                app.dirty = true;
            }
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
            start_run(app);
        }
        _ => {}
    }
    Ok(())
}

fn render_plan(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let block = Block::default()
        .title(format!(
            " {} dotman — Plan ({}) ",
            icons::ICON_GEAR,
            if app.dirty {
                "● unsaved"
            } else {
                "○ saved"
            }
        ))
        .borders(Borders::ALL)
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::default().fg(CATPPUCCIN_MOCHA.fg_dim));
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(area);

    let plan = match &app.plan {
        Some(p) => p,
        None => {
            let msg = Paragraph::new("no plan loaded").alignment(Alignment::Center);
            f.render_widget(msg, chunks[0]);
            return;
        }
    };

    // Group by layer.
    let layers = [
        "terminal",
        "shell",
        "multiplexer",
        "software",
        "enhancement",
        "misc",
    ];
    let mut items: Vec<ListItem> = Vec::new();
    for (i, layer) in layers.iter().enumerate() {
        let layer_items: Vec<_> = plan.items.iter().filter(|it| it.layer == *layer).collect();
        if layer_items.is_empty() {
            continue;
        }
        let enabled = layer_items.iter().filter(|it| it.selected).count();
        let total = layer_items.len();
        let header = format!(
            "{} {}. {} ({}/{})",
            icons::ICON_EXPANDED,
            i + 1,
            capitalize(layer),
            enabled,
            total
        );
        items.push(ListItem::new(Line::from(vec![Span::styled(
            header,
            Style::default()
                .fg(CATPPUCCIN_MOCHA.primary)
                .add_modifier(Modifier::BOLD),
        )])));
        for it in layer_items {
            let checkbox = if it.selected {
                Span::styled(
                    icons::ICON_CHECKED,
                    Style::default().fg(CATPPUCCIN_MOCHA.success),
                )
            } else {
                Span::styled(
                    icons::ICON_UNCHECKED,
                    Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
                )
            };
            let mut name = it.name.clone();
            if it.actions.len() > 1 {
                name.push_str(&format!(" (+{} actions)", it.actions.len() - 1));
            }
            let line = Line::from(vec![
                Span::raw("    "),
                checkbox,
                Span::raw(" "),
                Span::styled(name, Style::default().fg(CATPPUCCIN_MOCHA.fg)),
            ]);
            items.push(ListItem::new(line));
        }
    }

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(CATPPUCCIN_MOCHA.bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    f.render_stateful_widget(list, chunks[0], &mut app.list_state);

    let help = Paragraph::new(Line::from(vec![
        Span::styled(" [↑↓] nav ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(
            " [space] toggle ",
            Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
        ),
        Span::styled(" [a] all ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(" [n] none ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(" [r] run ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
        Span::styled(" [q] back ", Style::default().fg(CATPPUCCIN_MOCHA.fg_dim)),
    ]));
    f.render_widget(help, chunks[1]);
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(ch) => ch.to_ascii_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

// ---------------- RunView ----------------

fn handle_run(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.status_message = "abort requested (not yet implemented)".into();
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
    app.run_started = Some(Instant::now());
    app.current_log.clear();
    app.screen = Screen::RunView;

    let handle = std::thread::spawn(move || -> anyhow::Result<Run> {
        let result = crate::execute::execute(&plan, &cfg)?;
        let _ = crate::store::save(&result)?;
        Ok(result)
    });
    app.run_thread = Some(handle);
}

fn render_run(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Try to join the run thread (non-blocking).
    if let Some(handle) = &app.run_thread
        && handle.is_finished()
    {
        let handle = app.run_thread.take().unwrap();
        match handle.join() {
            Ok(Ok(run)) => {
                app.run = Some(run.clone());
                app.progress = (app.progress.0 + 1, app.progress.1);
                app.screen = Screen::ResultView;
            }
            Ok(Err(e)) => {
                app.status_message = format!("run failed: {e}");
                app.screen = Screen::ResultView;
            }
            Err(_) => {
                app.status_message = "run thread panicked".into();
                app.screen = Screen::ResultView;
            }
        }
    }

    let block = Block::default()
        .title(format!(
            " {} Running: {:?} — {}/{} ",
            icons::ICON_RUNNING,
            app.mode,
            app.progress.0,
            app.progress.1
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
            let icon = if !item.selected {
                Span::styled(
                    icons::ICON_SKIP,
                    Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
                )
            } else if i < app.progress.0 {
                Span::styled(
                    icons::ICON_OK,
                    Style::default().fg(CATPPUCCIN_MOCHA.success),
                )
            } else if i == app.progress.0 {
                Span::styled(
                    icons::SPINNER_BRAILLE[app.spinner_frame % icons::SPINNER_BRAILLE.len()],
                    Style::default().fg(CATPPUCCIN_MOCHA.running),
                )
            } else {
                Span::styled(
                    icons::ICON_PENDING,
                    Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
                )
            };
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
        .map(|s| Line::from(s.as_str()))
        .collect();
    f.render_widget(
        Paragraph::new(log_lines)
            .block(
                Block::default()
                    .title(" log ")
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );

    let help = Paragraph::new(Line::from(vec![Span::styled(
        " [q] abort ",
        Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
    )]));
    f.render_widget(help, chunks[2]);
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

    let ok = run
        .items
        .iter()
        .filter(|i| matches!(i.status, ActionStatus::NoChange))
        .count();
    let failed = run.items.iter().filter(|i| i.error.is_some()).count();
    let total = run.items.len();
    let title = format!(
        " {} Run {} — {} ok, {} failed, {} total ",
        icons::ICON_GEAR,
        if matches!(run.status, RunStatus::Success) {
            icons::ICON_OK
        } else {
            icons::ICON_FAIL
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
                icons::ICON_FAIL,
                Style::default().fg(CATPPUCCIN_MOCHA.danger),
            )
        } else if matches!(item.status, ActionStatus::NoChange) {
            Span::styled(
                icons::ICON_OK,
                Style::default().fg(CATPPUCCIN_MOCHA.success),
            )
        } else {
            Span::styled(
                icons::ICON_SKIP,
                Style::default().fg(CATPPUCCIN_MOCHA.fg_dim),
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
    let area = f.area();
    let block = Block::default()
        .title(format!(
            " {} History ({} runs) ",
            icons::ICON_GEAR,
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
            icons::ICON_GEAR,
            run.id,
            run.mode
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
            icons::ICON_FAIL
        } else if matches!(item.status, ActionStatus::NoChange) {
            icons::ICON_OK
        } else {
            icons::ICON_SKIP
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

fn render(app: &mut App, f: &mut Frame) {
    let area = f.area();
    f.render_widget(Clear, area);
    match app.screen {
        Screen::MainMenu => render_main_menu(f, app),
        Screen::PlanView => render_plan(f, app),
        Screen::RunView => render_run(f, app),
        Screen::ResultView => render_result(f, app),
        Screen::HistoryView => render_history(f, app),
        Screen::RunReplay => render_replay(f, app),
    }
}
