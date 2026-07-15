use crate::theme::CATPPUCCIN_MOCHA;
use crate::tui::{App, LogFilter, NoticeKind, RunThreadResult, Screen, run_help_line};
use anyhow::Result;
use crossterm::event::{KeyCode, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Instant;

const RUN_EVENT_CHANNEL_CAPACITY: usize = 1024;
const MAX_RUN_EVENTS_PER_FRAME: usize = 256;
const MOUSE_SCROLL_LINES: isize = 3;

pub(super) mod events;
pub(super) mod log;
pub(super) mod view;

pub(super) use events::drain_run_events;
use events::{apply_run_thread_result, clear_active_run_state, drain_all_run_events};
use log::{
    clamp_log_scroll, log_bottom_scroll, log_scroll_offset, push_log, run_log_title,
    scroll_run_log, scroll_run_log_lines, toggle_current_log_group, visible_log_lines,
};
pub(super) use view::{
    final_run_summary, run_item_status_label, run_status_label, run_status_style,
};
use view::{
    run_body_lines, run_is_aborting, run_is_terminal, run_log_panel_height, run_status_line,
    run_title_line, selected_run_action_total,
};

// ---------------- RunView ----------------

pub(super) fn handle_run(app: &mut App, key: KeyCode) -> Result<()> {
    match key {
        KeyCode::PageUp => scroll_run_log(app, -1),
        KeyCode::PageDown => scroll_run_log(app, 1),
        KeyCode::Up => scroll_run_log_lines(app, -1),
        KeyCode::Down => scroll_run_log_lines(app, 1),
        KeyCode::Char('k') => scroll_run_log_lines(app, -1),
        KeyCode::Char('j') => scroll_run_log_lines(app, 1),
        KeyCode::Left => {
            app.log_filter = app.log_filter.previous();
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, app.log_viewport_height.max(1));
        }
        KeyCode::Right => {
            app.log_filter = app.log_filter.next();
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, app.log_viewport_height.max(1));
        }
        KeyCode::Home => {
            app.log_follow = false;
            app.log_scroll = 0;
        }
        KeyCode::End | KeyCode::Char('f') | KeyCode::Char('F') => {
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, app.log_viewport_height.max(1));
        }
        KeyCode::Tab => {
            app.log_filter = app.log_filter.next();
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, app.log_viewport_height.max(1));
        }
        KeyCode::Char('c') => {
            app.log_filter = LogFilter::Current;
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, app.log_viewport_height.max(1));
        }
        KeyCode::Char('e') => {
            app.log_filter = LogFilter::Errors;
            app.log_follow = true;
            app.log_scroll = log_bottom_scroll(app, app.log_viewport_height.max(1));
        }
        KeyCode::Enter => toggle_current_log_group(app),
        KeyCode::Char('q') | KeyCode::Esc => {
            if let Some(flag) = &app.abort_flag {
                flag.store(true, Ordering::SeqCst);
                app.status_message = "abort requested; waiting for current action".into();
                app.status_kind = NoticeKind::Warning;
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

pub(super) fn handle_run_mouse(app: &mut App, kind: MouseEventKind) {
    match kind {
        MouseEventKind::ScrollUp => scroll_run_log_lines(app, -MOUSE_SCROLL_LINES),
        MouseEventKind::ScrollDown => scroll_run_log_lines(app, MOUSE_SCROLL_LINES),
        _ => {}
    }
}

pub(super) fn jump_run_top(app: &mut App) {
    app.log_follow = false;
    app.log_scroll = 0;
}

pub(super) fn jump_run_bottom(app: &mut App) {
    app.log_follow = true;
    app.log_scroll = log_bottom_scroll(app, app.log_viewport_height.max(1));
}

pub(super) fn start_run(app: &mut App) {
    if app.plan.is_none() || app.config.is_none() {
        return;
    }
    let plan = app.plan.clone().unwrap();
    let cfg = app.config.clone().unwrap();
    let total = selected_run_action_total(&plan);
    app.progress = (0, total);
    app.current_item = None;
    app.last_item_index = None;
    app.current_action = None;
    app.run_started = Some(Instant::now());
    app.current_log.clear();
    app.log_scroll = 0;
    app.log_follow = true;
    app.log_dropped_count = 0;
    app.log_group = None;
    app.active_log_group = None;
    app.log_filter = LogFilter::All;
    app.collapsed_log_groups.clear();
    app.run_error = None;
    app.run_save_warning = None;
    app.run = None;
    app.run_item_statuses = vec![None; plan.items.len()];
    app.run_action_statuses = plan
        .items
        .iter()
        .map(|item| vec![None; item.actions.len()])
        .collect();
    app.screen = Screen::RunView;

    let (tx, rx) = mpsc::sync_channel(RUN_EVENT_CHANNEL_CAPACITY);
    let sudo_tx = tx.clone();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let thread_abort_flag = Arc::clone(&abort_flag);
    let handle = std::thread::spawn(move || -> RunThreadResult {
        let result = match crate::execute::execute_with_events_and_sudo(
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
        ) {
            Ok(run) => run,
            Err(error) => {
                return RunThreadResult {
                    run: None,
                    error: Some(error.to_string()),
                    save_warning: None,
                };
            }
        };
        let save_warning = crate::store::save(&result)
            .err()
            .map(|error| format!("history save failed: {error}"));
        RunThreadResult {
            run: Some(result),
            error: None,
            save_warning,
        }
    });
    app.run_thread = Some(handle);
    app.run_events = Some(rx);
    app.abort_flag = Some(abort_flag);
}

pub(super) fn render_run(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Try to join the run thread (non-blocking).
    if let Some(handle) = &app.run_thread
        && handle.is_finished()
    {
        drain_all_run_events(app);
        let handle = app.run_thread.take().unwrap();
        match handle.join() {
            Ok(result) => {
                apply_run_thread_result(app, result);
                app.abort_flag = None;
                app.run_events = None;
            }
            Err(_) => {
                app.run_error = Some("run thread panicked".into());
                clear_active_run_state(app);
                push_log(app, "run thread panicked", Some(CATPPUCCIN_MOCHA.danger));
                app.abort_flag = None;
                app.run_events = None;
            }
        }
    }

    let aborting = run_is_aborting(app);
    let finished = run_is_terminal(app);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(run_log_panel_height(area.height)),
            Constraint::Length(1),
        ])
        .split(area);

    f.render_widget(
        Paragraph::new(run_title_line(app, usize::from(chunks[0].width))),
        chunks[0],
    );

    f.render_widget(
        Paragraph::new(run_status_line(app, usize::from(chunks[1].width))),
        chunks[1],
    );

    let step_lines = run_body_lines(app, usize::from(chunks[2].width), chunks[2].height as usize);
    f.render_widget(
        Paragraph::new(step_lines).block(Block::default().borders(Borders::NONE)),
        chunks[2],
    );

    let log_viewport_height = chunks[3].height.saturating_sub(2).max(1) as usize;
    app.log_viewport_height = log_viewport_height;
    clamp_log_scroll(app, log_viewport_height);
    let log_lines = visible_log_lines(app, log_viewport_height);
    let log_scroll = log_scroll_offset(app, log_viewport_height);
    f.render_widget(
        Paragraph::new(log_lines)
            .block(
                Block::default()
                    .title(run_log_title(app))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Plain),
            )
            .scroll((log_scroll, 0)),
        chunks[3],
    );

    let help = Paragraph::new(run_help_line(
        usize::from(chunks[4].width),
        aborting,
        finished,
        app.log_follow,
    ));
    f.render_widget(help, chunks[4]);
}
