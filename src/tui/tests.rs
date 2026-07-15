use super::*;
use crate::execute::MAX_TUI_OUTPUT_LINES;
use crate::tui::history::{history_content_area, replay_help_line, replay_lines};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

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

fn test_shell_action(name: &str) -> Action {
    Action::Shell {
        command: format!("echo {name}"),
        description: Some(name.into()),
        optional: false,
        if_condition: None,
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
        plan_id: None,
        mode: PlanMode::Deploy,
        started_at: "2026-01-01T00:00:00Z".into(),
        finished_at: Some("2026-01-01T00:00:01Z".into()),
        status,
        config_hash: "hash".into(),
        config_path: None,
        host: None,
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
    let line = plan_help_line(78, false);
    let text = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(line_display_width(&line) <= 78);
    assert!(text.contains("[r]"));
    assert!(text.contains("[q]"));
    assert!(!text.contains("Info"));
}

#[test]
fn plan_help_line_keeps_full_labels_when_space_allows() {
    let line = plan_help_line(120, false);
    let text = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(text.contains("Navigate"));
    assert!(text.contains("[Space]"));
    assert!(text.contains("Review"));
    assert!(text.contains("Back"));
    assert!(!text.contains("Info"));
}

#[test]
fn plan_only_help_is_read_only_without_run_entry() {
    let line = plan_help_line(120, true);
    let text = line_text(&line);

    assert!(text.contains("read-only"));
    assert!(!text.contains("Review"));
    assert!(!text.contains("[R]"));
}

#[test]
fn review_help_line_fits_narrow_terminal() {
    let line = review_help_line(30);
    let text = line_text(&line);

    assert!(line_display_width(&line) <= 30);
    assert!(text.contains("[r]"));
    assert!(text.contains("[q]"));
    assert!(!text.contains("Enter/R"));
    assert!(!text.contains("E/Q"));
}

#[test]
fn review_help_line_keeps_full_labels_when_space_allows() {
    let line = review_help_line(100);
    let text = line_text(&line);

    assert!(text.contains("Scroll"));
    assert!(text.contains("[r]"));
    assert!(!text.contains("Page"));
    assert!(text.contains("Run"));
    assert!(text.contains("Back"));
}

#[test]
fn replay_help_lists_only_the_primary_toggle_key() {
    let text = line_text(&replay_help_line(100));

    assert!(text.contains("[Space]"));
    assert!(!text.contains("Enter"));
    assert!(text.contains("Toggle"));
}

#[test]
fn run_progress_bar_reflects_done_ratio() {
    assert_eq!(run_progress_bar(0, 4, 8), "░░░░░░░░");
    assert_eq!(run_progress_bar(2, 4, 8), "████░░░░");
    assert_eq!(run_progress_bar(4, 4, 8), "████████");
}

#[test]
fn selected_run_action_total_excludes_unselected_items() {
    let mut selected = test_plan_item("selected");
    selected.actions = vec![
        Action::Shell {
            command: "echo one".into(),
            description: None,
            optional: false,
            if_condition: None,
        },
        Action::Shell {
            command: "echo two".into(),
            description: None,
            optional: false,
            if_condition: None,
        },
    ];
    let mut unselected = test_plan_item("unselected");
    unselected.selected = false;
    unselected.actions = vec![Action::Shell {
        command: "echo skip".into(),
        description: None,
        optional: false,
        if_condition: None,
    }];
    let plan = Plan {
        items: vec![selected, unselected],
        ..test_plan_with_items(&[])
    };

    assert_eq!(selected_run_action_total(&plan), 2);
}

#[test]
fn run_progress_advances_on_action_finished() {
    let mut app = App::new(Mode::Deploy);
    app.progress = (0, 2);
    app.run_action_statuses = vec![vec![None, None]];
    let (tx, rx) = mpsc::channel();
    app.run_events = Some(rx);
    tx.send(crate::execute::ExecuteEvent::ActionFinished {
        item_index: 0,
        action_index: 1,
        item: "item".into(),
        action: "action".into(),
        status: ActionStatus::NoChange,
    })
    .unwrap();

    drain_run_events(&mut app);

    assert_eq!(app.progress, (1, 2));
    assert_eq!(app.run_action_statuses[0][1], Some(ActionStatus::NoChange));
}

#[test]
fn run_event_lifecycle_updates_active_and_final_state() {
    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["config"]));
    app.progress = (0, 1);
    app.run_item_statuses = vec![None];
    app.run_action_statuses = vec![vec![None]];
    let (tx, rx) = mpsc::channel();
    app.run_events = Some(rx);

    tx.send(crate::execute::ExecuteEvent::ItemStarted {
        index: 0,
        name: "config".into(),
    })
    .unwrap();
    tx.send(crate::execute::ExecuteEvent::ActionStarted {
        item_index: 0,
        action_index: 0,
        item: "config".into(),
        action: "shell".into(),
    })
    .unwrap();
    tx.send(crate::execute::ExecuteEvent::Output {
        item: "config".into(),
        stream: OutputStream::Stdout,
        line: "working".into(),
    })
    .unwrap();
    tx.send(crate::execute::ExecuteEvent::ActionFinished {
        item_index: 0,
        action_index: 0,
        item: "config".into(),
        action: "shell".into(),
        status: ActionStatus::Executed,
    })
    .unwrap();
    tx.send(crate::execute::ExecuteEvent::ItemFinished {
        index: 0,
        name: "config".into(),
        status: ActionStatus::Executed,
    })
    .unwrap();

    drain_all_run_events(&mut app);

    assert_eq!(app.progress, (1, 1));
    assert_eq!(app.current_item, None);
    assert_eq!(app.current_action, None);
    assert_eq!(app.last_item_index, Some(0));
    assert_eq!(app.run_item_statuses, vec![Some(ActionStatus::Executed)]);
    assert_eq!(
        app.run_action_statuses,
        vec![vec![Some(ActionStatus::Executed)]]
    );
    let log = app
        .current_log
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    assert!(log.contains(&"working"));
    assert!(log.contains(&"finished shell: ran"));
    assert!(log.contains(&"finished: ran"));
}

#[test]
fn run_help_line_switches_to_stopping() {
    let running = line_text(&run_help_line(100, false, false, true));
    let stopping = line_text(&run_help_line(40, true, false, true));
    let finished = line_text(&run_help_line(100, false, true, true));
    let paused = line_text(&run_help_line(80, false, false, false));

    assert!(running.contains("Abort"));
    assert!(stopping.contains("Stopping"));
    assert!(finished.contains("Back"));
    assert!(paused.contains("Follow"));
    assert!(line_display_width(&run_help_line(8, true, false, true)) <= 8);
}

#[test]
fn run_help_uses_terminal_state_for_errors_and_running() {
    let mut app = App::new(Mode::Deploy);
    app.run_error = Some("run failed: command error".into());
    let error_help = line_text(&run_help_line(100, false, run_is_terminal(&app), true));
    assert!(error_help.contains("Back"));
    assert!(!error_help.contains("Abort"));

    let running_help = line_text(&run_help_line(
        100,
        false,
        run_is_terminal(&App::new(Mode::Deploy)),
        true,
    ));
    assert!(running_help.contains("Abort"));

    let mut aborting = App::new(Mode::Deploy);
    aborting.abort_flag = Some(Arc::new(AtomicBool::new(true)));
    let stopping = line_text(&run_help_line(
        100,
        run_is_aborting(&aborting),
        run_is_terminal(&aborting),
        true,
    ));
    assert!(stopping.contains("Stopping"));

    let mut saved_with_warning = App::new(Mode::Deploy);
    saved_with_warning.run = Some(test_run(RunStatus::Success, Vec::new()));
    saved_with_warning.run_save_warning = Some("history save failed".into());
    let warning_help = line_text(&run_help_line(
        100,
        false,
        run_is_terminal(&saved_with_warning),
        true,
    ));
    assert!(warning_help.contains("Back"));
    assert!(!warning_help.contains("Abort"));
}

#[test]
fn run_title_keeps_current_item_out_of_header() {
    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["ghostty", "fish"]));
    app.progress = (1, 2);
    app.current_item = Some(1);

    let title = run_title(&app, 80);
    assert!(title.contains("Running"));
    assert!(title.contains("1/2"));
    assert!(!title.contains("fish"));

    let status = line_text(&run_status_line(&app, 80));
    assert!(status.contains("fish"));

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
    let status = line_text(&run_status_line(&app, 100));

    assert!(title.contains("Success"));
    assert!(title.contains("2/2"));
    assert!(status.contains("1 changed"));
    assert!(status.contains("1 no change"));
}

#[test]
fn run_body_lines_keep_live_actions_in_plan_order() {
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
    assert!(!done.contains("No Change (1)"));
    assert!(done.contains("no change"));

    app.current_action = Some((0, 0));
    let running = lines_text(&run_body_lines(&app, 60, 8));
    assert!(!running.contains("Running (1)"));
    assert!(running.contains("running"));

    app.current_action = None;
    if let Some(plan) = &mut app.plan {
        plan.items[0].selected = false;
    }
    let skipped = lines_text(&run_body_lines(&app, 60, 8));
    assert!(!skipped.contains("Skipped (1)"));
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
fn run_error_state_does_not_render_as_running() {
    let mut app = App::new(Mode::Deploy);
    app.progress = (1, 3);
    app.run_error = Some("run failed: command error".into());

    let title = run_title(&app, 80);
    let status = line_text(&run_status_line(&app, 80));

    assert!(title.contains("Failed"));
    assert!(!title.contains("Running"));
    assert!(status.contains("command error"));
}

#[test]
fn run_save_warning_keeps_finished_run_result_visible() {
    let run = test_run(
        RunStatus::Success,
        vec![test_run_item("ghostty", ActionStatus::NoChange, None)],
    );
    let mut app = App::new(Mode::Deploy);
    apply_run_thread_result(
        &mut app,
        RunThreadResult {
            run: Some(run),
            error: None,
            save_warning: Some("history save failed: disk full".into()),
        },
    );

    assert!(finished_run_for_view(&app).is_some());
    assert!(run_title(&app, 80).contains("Success"));
    assert!(
        line_text(&run_status_line(&app, 80)).contains("0 ran, 0 changed, 1 no change, 0 failed")
    );
    let status = line_text(&run_status_line(&app, 160));
    assert!(status.contains("0 ran, 0 changed, 1 no change, 0 failed"));
    assert!(status.contains("warning: history save failed: disk full"));
}

#[test]
fn page_selection_states_are_independent_and_clamped() {
    let mut app = App::new(Mode::Menu);
    app.menu_state.select(Some(3));
    app.plan_state.select(Some(9));
    app.history_state.select(Some(4));
    app.runs = vec![test_run(RunStatus::Success, vec![])];

    history::clamp_menu_selection(&mut app);
    history::clamp_history_selection(&mut app);

    assert_eq!(app.menu_state.selected(), Some(3));
    assert_eq!(app.history_state.selected(), Some(0));
    assert_eq!(app.plan_state.selected(), Some(9));
}

#[test]
fn plan_focus_restores_same_item_after_column_change() {
    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["one", "two", "three", "four"]));
    app.plan_columns = 1;
    let rows = build_plan_rows(
        app.plan.as_ref().unwrap(),
        &app.collapsed_layers,
        app.plan_columns,
    );
    let row = rows
        .iter()
        .position(|row| matches!(row, PlanRow::Item(2)))
        .unwrap();
    select_plan_row(&mut app.plan_state, row, true);
    let focus = current_plan_focus(&app);

    app.plan_columns = 3;
    restore_plan_focus(&mut app, focus);

    assert_eq!(focused_plan_item_info(&app).as_deref(), Some("three"));
}

#[test]
fn plan_focus_info_does_not_overwrite_errors_or_warnings() {
    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["one"]));
    plan::select_first_plan_row(
        &mut app.plan_state,
        app.plan.as_ref(),
        &app.collapsed_layers,
        app.plan_columns,
    );
    app.status_message = "selection save failed: disk full".into();
    app.status_kind = NoticeKind::Error;
    app.status_is_focus_info = false;

    update_plan_focus_info(&mut app);
    assert_eq!(app.status_message, "selection save failed: disk full");

    app.status_message = "nothing selected".into();
    app.status_kind = NoticeKind::Warning;
    update_plan_focus_info(&mut app);
    assert_eq!(app.status_message, "nothing selected");
}

#[test]
fn plan_vertical_navigation_stops_at_boundaries() {
    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["one", "two"]));
    let rows = build_plan_rows(
        app.plan.as_ref().unwrap(),
        &app.collapsed_layers,
        app.plan_columns,
    );
    let first = rows.iter().position(is_selectable_plan_row).unwrap();
    let last = rows.iter().rposition(is_selectable_plan_row).unwrap();

    select_plan_row(&mut app.plan_state, first, true);
    handle_plan(&mut app, KeyCode::Char('k')).unwrap();
    assert_eq!(app.plan_state.selected(), Some(first));

    select_plan_row(&mut app.plan_state, last, true);
    handle_plan(&mut app, KeyCode::Char('j')).unwrap();
    assert_eq!(app.plan_state.selected(), Some(last));
}

#[test]
fn plan_bulk_selection_only_marks_actual_changes_dirty() {
    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["one", "two"]));

    handle_plan(&mut app, KeyCode::Char('a')).unwrap();
    assert!(!app.dirty);

    handle_plan(&mut app, KeyCode::Char('n')).unwrap();
    assert!(app.dirty);
    assert!(
        app.plan
            .as_ref()
            .unwrap()
            .items
            .iter()
            .all(|item| !item.selected)
    );

    app.dirty = false;
    handle_plan(&mut app, KeyCode::Char('n')).unwrap();
    assert!(!app.dirty);
}

#[test]
fn plan_item_layer_grid_and_jump_interactions_preserve_focus() {
    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["one", "two", "three", "four"]));
    app.plan_columns = 3;

    handle_plan(&mut app, KeyCode::Enter).unwrap();
    assert!(app.collapsed_layers.contains("misc"));
    handle_plan(&mut app, KeyCode::Char(' ')).unwrap();
    assert!(!app.collapsed_layers.contains("misc"));

    let rows = build_plan_rows(
        app.plan.as_ref().unwrap(),
        &app.collapsed_layers,
        app.plan_columns,
    );
    let first_grid_row = rows
        .iter()
        .position(|row| matches!(row, PlanRow::InlineItems(_)))
        .unwrap();
    select_plan_row(&mut app.plan_state, first_grid_row, true);

    handle_plan(&mut app, KeyCode::Right).unwrap();
    assert_eq!(app.grid_col, 1);
    handle_plan(&mut app, KeyCode::Char(' ')).unwrap();
    assert!(!app.plan.as_ref().unwrap().items[1].selected);
    assert!(app.dirty);
    assert!(app.status_message.contains("two"));

    handle_plan(&mut app, KeyCode::Char('h')).unwrap();
    handle_plan(&mut app, KeyCode::Left).unwrap();
    assert_eq!(app.grid_col, 0);
    handle_plan(&mut app, KeyCode::Char('l')).unwrap();
    assert_eq!(app.grid_col, 1);

    handle_plan(&mut app, KeyCode::Char('6')).unwrap();
    assert!(app.collapsed_layers.contains("misc"));
    handle_plan(&mut app, KeyCode::Char('6')).unwrap();
    assert!(!app.collapsed_layers.contains("misc"));

    jump_plan_bottom(&mut app);
    let bottom = app.plan_state.selected().unwrap();
    handle_plan(&mut app, KeyCode::PageUp).unwrap();
    assert!(app.plan_state.selected().unwrap() <= bottom);
    handle_plan(&mut app, KeyCode::PageDown).unwrap();
    jump_plan_top(&mut app);
    assert_eq!(
        current_plan_focus(&app),
        Some(PlanFocus::Layer("misc".into()))
    );
}

#[test]
fn plan_run_and_dirty_exit_guards_preserve_user_intent() {
    let mut read_only = App::new(Mode::Plan);
    read_only.plan = Some(test_plan_with_items(&["one"]));
    handle_plan(&mut read_only, KeyCode::Char('r')).unwrap();
    assert_eq!(read_only.screen, Screen::MainMenu);
    assert_eq!(
        read_only.status_message,
        "plan mode is read-only; choose deploy to run"
    );

    let mut empty_selection = App::new(Mode::Deploy);
    let mut plan = test_plan_with_items(&["one"]);
    plan.items[0].selected = false;
    empty_selection.plan = Some(plan);
    handle_plan(&mut empty_selection, KeyCode::Char('r')).unwrap();
    assert_eq!(empty_selection.status_message, "nothing selected");
    assert_eq!(empty_selection.status_kind, NoticeKind::Warning);

    let mut dirty = App::new(Mode::Deploy);
    dirty.plan = Some(test_plan_with_items(&["one"]));
    dirty.dirty = true;
    handle_plan(&mut dirty, KeyCode::Esc).unwrap();
    assert!(dirty.plan_exit_pending);
    assert!(dirty.status_message.contains("Unsaved selection changes"));
    handle_plan(&mut dirty, KeyCode::Char('x')).unwrap();
    assert!(dirty.plan_exit_pending);
    handle_plan(&mut dirty, KeyCode::Esc).unwrap();
    assert!(!dirty.plan_exit_pending);
    assert!(dirty.status_message.is_empty());
    assert_eq!(dirty.screen, Screen::MainMenu);
}

#[test]
fn plan_render_handles_missing_compact_and_wide_layouts() {
    let backend = ratatui::backend::TestBackend::new(50, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let mut missing = App::new(Mode::Deploy);
    terminal
        .draw(|frame| render_plan(frame, &mut missing))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("no plan loaded"));

    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["one", "two", "three", "four"]));
    app.dirty = true;
    app.status_message = "selection changed".into();
    terminal.draw(|frame| render_plan(frame, &mut app)).unwrap();
    let compact = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(compact.contains("unsaved"));
    assert_eq!(app.plan_columns, 1);

    let backend = ratatui::backend::TestBackend::new(140, 18);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|frame| render_plan(frame, &mut app)).unwrap();
    let wide = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(wide.contains("4 selected"));
    assert!(wide.contains("selection changed"));
    assert_eq!(app.plan_columns, GRID_COLUMNS);
}

#[test]
fn history_selection_handles_empty_first_and_last_delete_positions() {
    let mut app = App::new(Mode::History);
    history::clamp_history_selection(&mut app);
    assert_eq!(app.history_state.selected(), None);

    app.runs = vec![
        test_run(
            RunStatus::Success,
            vec![test_run_item("one", ActionStatus::NoChange, None)],
        ),
        test_run(
            RunStatus::Success,
            vec![test_run_item("two", ActionStatus::NoChange, None)],
        ),
    ];
    app.history_state.select(Some(0));
    app.runs.remove(0);
    history::clamp_history_selection(&mut app);
    assert_eq!(app.history_state.selected(), Some(0));

    app.history_state.select(Some(9));
    app.runs.clear();
    history::clamp_history_selection(&mut app);
    assert_eq!(app.history_state.selected(), None);
}

#[test]
fn successful_history_reload_clears_stale_page_notice() {
    let mut app = App::new(Mode::History);
    app.status_message = "deleted run old-id".into();
    app.status_kind = NoticeKind::Success;

    app::apply_history_warnings(&mut app, &[]);

    assert!(app.status_message.is_empty());
    assert_eq!(app.status_kind, NoticeKind::Info);

    app::apply_history_warnings(
        &mut app,
        &["failed to parse first run".into(), "second warning".into()],
    );
    assert_eq!(app.status_message, "failed to parse first run (+1 more)");
    assert_eq!(app.status_kind, NoticeKind::Warning);
}

#[test]
fn history_notice_reserves_a_row_above_content() {
    let area = Rect::new(2, 4, 80, 10);

    assert_eq!(history_content_area(area, false), area);
    assert_eq!(history_content_area(area, true), Rect::new(2, 5, 80, 9));
    assert_eq!(
        history_content_area(Rect::new(0, 0, 20, 0), true),
        Rect::new(0, 1, 20, 0)
    );
}

#[test]
fn home_and_end_use_consistent_global_navigation() {
    let mut app = App::new(Mode::History);
    app.screen = Screen::HistoryView;
    app.runs = vec![
        test_run(RunStatus::Success, vec![]),
        test_run(RunStatus::Failed, vec![]),
        test_run(RunStatus::Aborted, vec![]),
    ];
    app.history_state.select(Some(1));

    handle_key(&mut app, KeyCode::Home).unwrap();
    assert_eq!(app.history_state.selected(), Some(0));
    handle_key(&mut app, KeyCode::End).unwrap();
    assert_eq!(app.history_state.selected(), Some(2));

    app.screen = Screen::MainMenu;
    app.menu_state.select(Some(2));
    handle_key(&mut app, KeyCode::Home).unwrap();
    assert_eq!(app.menu_state.selected(), Some(0));
    handle_key(&mut app, KeyCode::End).unwrap();
    assert_eq!(app.menu_state.selected(), Some(3));
}

#[test]
fn current_run_item_name_uses_explicit_item_state_not_progress() {
    let mut app = App::new(Mode::Deploy);
    let mut first = test_plan_item("first");
    first.actions = vec![test_shell_action("one"), test_shell_action("two")];
    let second = test_plan_item("second");
    app.plan = Some(Plan {
        items: vec![first, second],
        ..test_plan_with_items(&[])
    });
    app.progress = (2, 3);

    assert_eq!(current_run_item_name(&app), None);
    app.last_item_index = Some(0);
    assert_eq!(current_run_item_name(&app), Some("first"));
}

#[test]
fn finished_run_progress_keeps_not_run_actions_in_total() {
    let mut item = test_run_item("multi", ActionStatus::WillFail, Some("exit code 1"));
    item.actions = vec![
        RunAction {
            kind: "shell".into(),
            name: "first".into(),
            status: ActionStatus::WillFail,
            error: Some("exit code 1".into()),
            output: vec![],
        },
        RunAction {
            kind: "shell".into(),
            name: "second".into(),
            status: ActionStatus::NotRun,
            error: Some("not run after previous failure".into()),
            output: vec![],
        },
    ];
    let run = test_run(RunStatus::Failed, vec![item]);
    let mut app = App::new(Mode::Deploy);
    app.progress = (1, 2);
    app.run = Some(run.clone());

    sync_finished_run_state(&mut app, &run);

    assert_eq!(run_action_total(&run), 2);
    assert_eq!(run_executed_action_total(&run), 1);
    assert_eq!(app.progress, (1, 2));
    assert!(final_run_summary(&run).contains("1 not run"));
    assert!(lines_text(&run_body_lines(&app, 80, 8)).contains("Not Run (1)"));
}

#[test]
fn finished_run_progress_counts_condition_skips_as_resolved_actions() {
    let run = test_run(
        RunStatus::Success,
        vec![test_run_item("guarded", ActionStatus::WillSkip, None)],
    );
    let mut app = App::new(Mode::Deploy);
    app.progress = (1, 1);

    sync_finished_run_state(&mut app, &run);

    assert_eq!(run_action_total(&run), 1);
    assert_eq!(run_executed_action_total(&run), 1);
    assert_eq!(app.progress, (1, 1));
    assert_eq!(
        final_run_summary(&run),
        "0 ran, 0 changed, 0 no change, 0 failed, 1 skipped"
    );
}

#[test]
fn final_summary_counts_actions_instead_of_only_the_item_terminal_status() {
    let mut item = test_run_item("multi", ActionStatus::NoChange, None);
    item.actions = vec![
        RunAction {
            kind: "install".into(),
            name: "install tool".into(),
            status: ActionStatus::WillInstall,
            error: None,
            output: vec![],
        },
        RunAction {
            kind: "shell".into(),
            name: "optional setup".into(),
            status: ActionStatus::NoChange,
            error: None,
            output: vec![],
        },
    ];
    let run = test_run(RunStatus::Success, vec![item]);

    assert_eq!(
        final_run_summary(&run),
        "0 ran, 1 changed, 1 no change, 0 failed"
    );
}

#[test]
fn live_spinner_uses_current_spinner_frame() {
    let mut app = App::new(Mode::Deploy);
    let mut item = test_plan_item("ghostty");
    item.actions = vec![test_shell_action("check")];
    app.plan = Some(Plan {
        items: vec![item],
        ..test_plan_with_items(&[])
    });
    app.current_action = Some((0, 0));
    app.spinner_frame = 0;
    let first = lines_text(&run_body_lines(&app, 80, 4));
    app.tick();
    let second = lines_text(&run_body_lines(&app, 80, 4));

    assert_ne!(first, second);
}

#[test]
fn run_status_colors_are_shared_and_semantically_distinct() {
    assert_eq!(
        run_status_color(RunStatus::Running),
        CATPPUCCIN_MOCHA.running
    );
    assert_eq!(
        run_status_color(RunStatus::Success),
        CATPPUCCIN_MOCHA.success
    );
    assert_eq!(run_status_color(RunStatus::Failed), CATPPUCCIN_MOCHA.danger);
    assert_eq!(
        run_status_color(RunStatus::Aborted),
        CATPPUCCIN_MOCHA.warning
    );
    assert_ne!(
        run_status_color(RunStatus::Running),
        run_status_color(RunStatus::Aborted)
    );
}

#[test]
fn executed_shells_are_ran_not_changed() {
    assert_eq!(
        run_group_for_status(Some(ActionStatus::Executed), false),
        RunGroup::Ran
    );
    assert_eq!(
        run_group_for_status(Some(ActionStatus::WillLink), false),
        RunGroup::Changed
    );
    let ran_header = line_text(&run_group_header_line(RunGroup::Ran, 2, 80));
    assert!(ran_header.contains("Ran (2)"));
}

#[test]
fn log_follow_manual_scroll_and_resume_work() {
    let mut app = App::new(Mode::Deploy);
    app.log_viewport_height = 5;
    for idx in 0..20 {
        push_log(&mut app, &format!("line {idx}"), None);
    }
    assert!(app.log_follow);
    let bottom = log_scroll_offset(&app, 5);
    assert!(bottom > 0);

    scroll_run_log(&mut app, -1);
    assert!(!app.log_follow);
    assert_eq!(app.log_scroll, usize::from(bottom) - 5);
    let paused_offset = log_scroll_offset(&app, 5);
    assert_eq!(paused_offset as usize, app.log_scroll);

    handle_run(&mut app, KeyCode::Char('f')).unwrap();
    assert!(app.log_follow);
    assert_eq!(log_scroll_offset(&app, 5), bottom);
}

#[test]
fn log_home_end_and_page_keys_use_absolute_scroll() {
    let mut app = App::new(Mode::Deploy);
    app.log_viewport_height = 5;
    for idx in 0..30 {
        push_log(&mut app, &format!("line {idx}"), None);
    }

    handle_run(&mut app, KeyCode::Home).unwrap();
    assert!(!app.log_follow);
    assert_eq!(log_scroll_offset(&app, 5), 0);

    handle_run(&mut app, KeyCode::PageDown).unwrap();
    assert!(!app.log_follow);
    assert_eq!(log_scroll_offset(&app, 5), 5);

    let after_down = app.log_scroll;
    handle_run(&mut app, KeyCode::PageUp).unwrap();
    assert_eq!(app.log_scroll, after_down - 5);

    handle_run(&mut app, KeyCode::PageUp).unwrap();
    assert_eq!(app.log_scroll, 0);

    handle_run(&mut app, KeyCode::End).unwrap();
    assert!(app.log_follow);
    assert_eq!(
        log_scroll_offset(&app, 5),
        log_bottom_scroll(&app, 5) as u16
    );

    handle_run(&mut app, KeyCode::Home).unwrap();
    handle_run(&mut app, KeyCode::Char('f')).unwrap();
    assert!(app.log_follow);
    assert_eq!(
        log_scroll_offset(&app, 5),
        log_bottom_scroll(&app, 5) as u16
    );
}

#[test]
fn first_page_up_uses_current_log_viewport_height() {
    for viewport_height in [5usize, 9] {
        let mut app = App::new(Mode::Deploy);
        app.log_viewport_height = viewport_height;
        for idx in 0..30 {
            push_log(&mut app, &format!("line {idx}"), None);
        }
        let bottom = log_bottom_scroll(&app, viewport_height);

        handle_run(&mut app, KeyCode::PageUp).unwrap();

        assert!(!app.log_follow);
        assert_eq!(app.log_scroll, bottom - viewport_height);
    }
}

#[test]
fn page_down_clamps_to_bottom_after_manual_scroll() {
    let mut app = App::new(Mode::Deploy);
    app.log_viewport_height = 7;
    for idx in 0..18 {
        push_log(&mut app, &format!("line {idx}"), None);
    }
    let bottom = log_bottom_scroll(&app, 7);

    handle_run(&mut app, KeyCode::Home).unwrap();
    handle_run(&mut app, KeyCode::PageDown).unwrap();
    assert_eq!(app.log_scroll, 7);

    handle_run(&mut app, KeyCode::PageDown).unwrap();
    assert_eq!(app.log_scroll, bottom);

    handle_run(&mut app, KeyCode::PageDown).unwrap();
    assert_eq!(app.log_scroll, bottom);
}

#[test]
fn vim_log_keys_scroll_one_line_and_pause_follow() {
    let mut app = App::new(Mode::Deploy);
    app.log_viewport_height = 5;
    for index in 0..20 {
        push_log(&mut app, &format!("line {index}"), None);
    }

    handle_run(&mut app, KeyCode::Char('k')).unwrap();
    assert!(!app.log_follow);
    let after_up = app.log_scroll;
    handle_run(&mut app, KeyCode::Char('j')).unwrap();
    assert_eq!(app.log_scroll, after_up + 1);
}

#[test]
fn run_arrow_keys_scroll_and_switch_filters() {
    let mut app = App::new(Mode::Deploy);
    app.log_viewport_height = 5;
    for index in 0..20 {
        push_log(&mut app, &format!("line {index}"), None);
    }

    handle_run(&mut app, KeyCode::Up).unwrap();
    assert!(!app.log_follow);
    let after_up = app.log_scroll;
    handle_run(&mut app, KeyCode::Down).unwrap();
    assert_eq!(app.log_scroll, after_up + 1);

    assert_eq!(app.log_filter, LogFilter::All);
    handle_run(&mut app, KeyCode::Left).unwrap();
    assert_eq!(app.log_filter, LogFilter::Errors);
    handle_run(&mut app, KeyCode::Right).unwrap();
    assert_eq!(app.log_filter, LogFilter::All);
}

#[test]
fn run_shortcuts_preserve_filters_folding_navigation_and_exit_states() {
    let mut app = App::new(Mode::Deploy);
    app.log_viewport_height = 3;
    push_log_group(&mut app, "group");
    push_log_indented(&mut app, "failed", None, 1, LogKind::Stderr);
    for index in 0..8 {
        push_log(&mut app, &format!("line {index}"), None);
    }

    handle_run(&mut app, KeyCode::Tab).unwrap();
    assert_eq!(app.log_filter, LogFilter::Current);
    handle_run(&mut app, KeyCode::Char('e')).unwrap();
    assert_eq!(app.log_filter, LogFilter::Errors);
    handle_run(&mut app, KeyCode::Char('c')).unwrap();
    assert_eq!(app.log_filter, LogFilter::Current);

    app.log_filter = LogFilter::All;
    app.log_scroll = 0;
    handle_run(&mut app, KeyCode::Enter).unwrap();
    assert!(app.collapsed_log_groups.contains("group"));

    jump_run_top(&mut app);
    assert!(!app.log_follow);
    assert_eq!(app.log_scroll, 0);
    jump_run_bottom(&mut app);
    assert!(app.log_follow);

    handle_run_mouse(&mut app, crossterm::event::MouseEventKind::ScrollUp);
    assert!(!app.log_follow);
    handle_run_mouse(&mut app, crossterm::event::MouseEventKind::ScrollDown);
    handle_run_mouse(&mut app, crossterm::event::MouseEventKind::Moved);

    handle_run(&mut app, KeyCode::Char('F')).unwrap();
    assert!(app.log_follow);

    app.plan = Some(test_plan_with_items(&["fish"]));
    handle_run(&mut app, KeyCode::Esc).unwrap();
    assert_eq!(app.screen, Screen::PlanView);

    app.plan = None;
    app.screen = Screen::RunView;
    handle_run(&mut app, KeyCode::Char('q')).unwrap();
    assert_eq!(app.screen, Screen::MainMenu);

    let abort_flag = Arc::new(AtomicBool::new(false));
    app.abort_flag = Some(Arc::clone(&abort_flag));
    handle_run(&mut app, KeyCode::Char('q')).unwrap();
    assert!(abort_flag.load(Ordering::SeqCst));
    assert_eq!(app.status_kind, NoticeKind::Warning);
    assert!(app.status_message.contains("abort requested"));

    handle_run(&mut app, KeyCode::Null).unwrap();
}

#[test]
fn render_run_joins_finished_worker_and_displays_result() {
    let backend = ratatui::backend::TestBackend::new(80, 20);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let mut app = App::new(Mode::Deploy);
    app.screen = Screen::RunView;
    app.progress = (0, 1);
    app.run_thread = Some(std::thread::spawn(|| RunThreadResult {
        run: Some(test_run(
            RunStatus::Success,
            vec![test_run_item("fish", ActionStatus::Executed, None)],
        )),
        error: None,
        save_warning: Some("history save warning".into()),
    }));
    while !app.run_thread.as_ref().unwrap().is_finished() {
        std::thread::yield_now();
    }

    terminal.draw(|frame| render_run(frame, &mut app)).unwrap();

    assert!(app.run_thread.is_none());
    assert!(app.run_events.is_none());
    assert!(app.abort_flag.is_none());
    assert_eq!(
        app.run.as_ref().map(|run| run.status),
        Some(RunStatus::Success)
    );
    assert_eq!(
        app.run_save_warning.as_deref(),
        Some("history save warning")
    );
    assert!(app.log_viewport_height > 0);
}

#[test]
fn render_run_reports_worker_panics() {
    let backend = ratatui::backend::TestBackend::new(60, 12);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let mut app = App::new(Mode::Deploy);
    app.run_thread = Some(std::thread::spawn(|| -> RunThreadResult {
        panic!("test worker panic")
    }));
    while !app.run_thread.as_ref().unwrap().is_finished() {
        std::thread::yield_now();
    }

    terminal.draw(|frame| render_run(frame, &mut app)).unwrap();

    assert_eq!(app.run_error.as_deref(), Some("run thread panicked"));
    assert!(app.run_thread.is_none());
    assert!(
        app.current_log
            .iter()
            .any(|line| line.text.contains("run thread panicked"))
    );
}

#[test]
fn run_view_helpers_preserve_status_action_and_overflow_semantics() {
    let icon_set = icons::current();
    let statuses = [
        (
            ActionStatus::WillFail,
            icon_set.failed,
            CATPPUCCIN_MOCHA.danger,
        ),
        (
            ActionStatus::Aborted,
            icon_set.warning,
            CATPPUCCIN_MOCHA.warning,
        ),
        (
            ActionStatus::WillSkip,
            icon_set.skipped,
            CATPPUCCIN_MOCHA.fg_dim,
        ),
        (
            ActionStatus::NotRun,
            icon_set.skipped,
            CATPPUCCIN_MOCHA.fg_dim,
        ),
        (
            ActionStatus::NoChange,
            icon_set.info,
            CATPPUCCIN_MOCHA.text_muted,
        ),
        (
            ActionStatus::Executed,
            icon_set.success,
            CATPPUCCIN_MOCHA.success,
        ),
        (
            ActionStatus::WillInstall,
            icon_set.success,
            CATPPUCCIN_MOCHA.success,
        ),
    ];
    for (status, expected_icon, expected_color) in statuses {
        let icon = run_status_icon(Some(status));
        assert_eq!(icon.content.as_ref(), expected_icon);
        assert_eq!(icon.style.fg, Some(expected_color));
        assert_eq!(
            run_status_style(Some(status), false).fg,
            Some(expected_color)
        );
        assert_eq!(
            run_status_label_for_view(Some(status), false),
            status.result_label()
        );
    }
    let pending = run_status_icon(None);
    assert_eq!(pending.content.as_ref(), icon_set.pending);
    assert_eq!(pending.style.fg, Some(CATPPUCCIN_MOCHA.fg_dim));
    assert_eq!(
        run_status_style(None, false).fg,
        Some(CATPPUCCIN_MOCHA.fg_dim)
    );
    assert_eq!(
        run_status_style(None, true).fg,
        Some(CATPPUCCIN_MOCHA.running)
    );
    assert_eq!(run_status_label_for_view(None, false), "pending");
    assert_eq!(run_status_label_for_view(None, true), "running");

    for (group, expected_label) in [
        (RunGroup::Failed, "Failed (1)"),
        (RunGroup::Aborted, "Aborted (1)"),
        (RunGroup::Running, "Running (1)"),
        (RunGroup::Ran, "Ran (1)"),
        (RunGroup::Changed, "Changed (1)"),
        (RunGroup::NoChange, "No Change (1)"),
        (RunGroup::NotRun, "Not Run (1)"),
        (RunGroup::Skipped, "Skipped (1)"),
        (RunGroup::Pending, "Pending (1)"),
    ] {
        assert!(line_text(&run_group_header_line(group, 1, 40)).contains(expected_label));
    }

    let actions = [
        Action::Install {
            pkg_mgr: "brew".into(),
            binary: "fish".into(),
            source: "brew install fish".into(),
        },
        Action::Link {
            target: PathBuf::from("target"),
            source: PathBuf::from("source"),
            backup: false,
            relink: false,
        },
        Action::Create {
            target: PathBuf::from("dir"),
        },
        test_shell_action("shell"),
        Action::Clean {
            target: PathBuf::from("old"),
            force: false,
        },
    ];
    for (action, (expected_kind, expected_icon)) in actions.iter().zip([
        ("install", icon_set.action_install),
        ("link", icon_set.action_link),
        ("create", icon_set.action_create),
        ("shell", icon_set.action_shell),
        ("clean", icon_set.action_clean),
    ]) {
        assert_eq!(action_kind_for_view(action), expected_kind);
        assert_eq!(run_action_kind_icon(expected_kind), expected_icon);
    }
    assert_eq!(run_action_kind_icon("unknown"), icon_set.info);

    let make_lines = || {
        (0..12)
            .map(|index| RunDisplayLine {
                group: if index % 2 == 0 {
                    RunGroup::Failed
                } else {
                    RunGroup::Changed
                },
                line: Line::from(format!("line {index}")),
                active: index == 7,
            })
            .collect::<Vec<_>>()
    };
    let grouped = grouped_run_lines(make_lines(), 30, 4);
    assert_eq!(grouped.len(), 4);
    assert!(lines_text(&grouped).contains("above"));
    assert!(lines_text(&grouped).contains("below"));
    let ordered = ordered_run_lines(make_lines(), 30, 4);
    assert_eq!(ordered.len(), 4);
    assert!(lines_text(&ordered).contains("above"));
    assert!(lines_text(&ordered).contains("below"));
}

#[test]
fn live_run_lines_preserve_empty_unselected_and_action_kind_states() {
    let mut empty = test_plan_item("empty");
    empty.selected = false;
    let mut actions = test_plan_item("actions");
    actions.actions = vec![
        Action::Install {
            pkg_mgr: "brew".into(),
            binary: "fish".into(),
            source: "brew install fish".into(),
        },
        Action::Link {
            target: PathBuf::from("target"),
            source: PathBuf::from("source"),
            backup: false,
            relink: false,
        },
        Action::Create {
            target: PathBuf::from("dir"),
        },
        test_shell_action("shell"),
        Action::Clean {
            target: PathBuf::from("old"),
            force: false,
        },
    ];
    let mut plan = test_plan_with_items(&[]);
    plan.items = vec![empty, actions];
    let mut app = App::new(Mode::Deploy);
    app.current_action = Some((1, 2));
    app.run_item_statuses = vec![None, None];
    app.run_action_statuses = vec![vec![], vec![Some(ActionStatus::NoChange); 5]];

    let lines = live_run_display_lines(&app, &plan, 80);
    assert_eq!(lines.len(), 6);
    assert_eq!(lines.iter().filter(|line| line.active).count(), 1);
    assert_eq!(lines[0].group, RunGroup::Skipped);
    assert_eq!(lines[3].group, RunGroup::Running);
    assert!(
        lines[1..]
            .iter()
            .enumerate()
            .all(|(index, line)| index == 2 || line.group == RunGroup::NoChange)
    );
    let rendered = lines
        .iter()
        .map(|line| line_text(&line.line))
        .collect::<Vec<_>>()
        .join("\n");
    for expected in ["install fish", "target", "dir", "shell", "old"] {
        assert!(
            rendered.contains(expected),
            "missing action text: {expected}"
        );
    }
    assert_eq!(
        lines_text(&run_body_lines(&App::new(Mode::Deploy), 40, 3)),
        "loading..."
    );
    assert_eq!(run_log_panel_height(2), 0);
    assert_eq!(run_log_panel_height(30), 10);
}

#[test]
fn replay_selected_row_has_focus_background() {
    let mut app = App::new(Mode::History);
    app.run = Some(test_run(
        RunStatus::Success,
        vec![test_run_item("fish", ActionStatus::NoChange, None)],
    ));
    app.replay_state.select(Some(0));

    let lines = replay_lines(&app, 80);

    assert_eq!(lines[0].spans[0].style.bg, Some(focus_bg()));
    assert_eq!(lines[0].spans[1].style.bg, Some(focus_bg()));
    assert_eq!(lines[0].spans[3].style.bg, Some(focus_bg()));
}

#[test]
fn replay_does_not_attach_item_error_to_a_successful_action() {
    let mut item = test_run_item("multi", ActionStatus::WillFail, Some("second failed"));
    item.actions = vec![
        RunAction {
            kind: "shell".into(),
            name: "first".into(),
            status: ActionStatus::Executed,
            error: None,
            output: vec![],
        },
        RunAction {
            kind: "shell".into(),
            name: "second".into(),
            status: ActionStatus::WillFail,
            error: Some("second failed".into()),
            output: vec![],
        },
    ];
    let mut app = App::new(Mode::History);
    app.run = Some(test_run(RunStatus::Failed, vec![item]));
    app.replay_state.select(Some(0));
    app.replay_expanded.insert("0:0".into());

    let text = lines_text(&replay_lines(&app, 80));

    assert!(!text.contains("error: second failed"));
}

#[test]
fn log_filter_current_errors_and_fold_work() {
    let mut app = App::new(Mode::Deploy);
    push_log_group(&mut app, "fish");
    push_log_indented(&mut app, "installing plugins", None, 1, LogKind::Stdout);
    push_log_group(&mut app, "neovim");
    push_log_indented(&mut app, "downloading package", None, 1, LogKind::Stdout);
    push_log_indented(
        &mut app,
        "failed to extract archive",
        Some(CATPPUCCIN_MOCHA.danger),
        1,
        LogKind::Stderr,
    );

    app.log_filter = LogFilter::Current;
    let current = lines_text(&visible_log_lines(&app, 10));
    assert!(current.contains("neovim"));
    assert!(current.contains("downloading package"));
    assert!(!current.contains("fish"));

    app.log_filter = LogFilter::Errors;
    let errors = lines_text(&visible_log_lines(&app, 10));
    assert!(errors.contains("neovim"));
    assert!(errors.contains("failed to extract"));
    assert!(!errors.contains("downloading package"));

    app.log_filter = LogFilter::All;
    toggle_current_log_group(&mut app);
    let folded = lines_text(&visible_log_lines(&app, 10));
    assert!(folded.contains("neovim"));
    assert!(folded.contains("collapsed"));
    assert!(!folded.contains("downloading package"));
}

#[test]
fn log_truncation_keeps_group_header_for_child_lines() {
    let mut app = App::new(Mode::Deploy);
    push_log_group(&mut app, "big-action");
    for idx in 0..(MAX_TUI_OUTPUT_LINES + 10) {
        push_log_indented(&mut app, &format!("line {idx}"), None, 1, LogKind::Stdout);
    }

    assert_eq!(
        app.current_log.first().map(|line| line.kind),
        Some(LogKind::Header)
    );
    assert_eq!(
        app.current_log.first().map(|line| line.text.as_str()),
        Some("big-action")
    );
    assert_eq!(app.current_log.len(), MAX_TUI_OUTPUT_LINES);
    assert_eq!(
        app.current_log.last().map(|line| line.text.as_str()),
        Some("line 1009")
    );
    assert_eq!(app.log_dropped_count, 11);
}

#[test]
fn action_output_stays_in_current_action_group() {
    let mut app = App::new(Mode::Deploy);
    let (tx, rx) = mpsc::channel();
    app.run_events = Some(rx);
    tx.send(crate::execute::ExecuteEvent::ActionStarted {
        item_index: 0,
        action_index: 0,
        item: "neovim".into(),
        action: "install plugins".into(),
    })
    .unwrap();
    tx.send(crate::execute::ExecuteEvent::Output {
        item: "neovim".into(),
        stream: crate::model::OutputStream::Stdout,
        line: "downloading".into(),
    })
    .unwrap();
    tx.send(crate::execute::ExecuteEvent::ActionMessage {
        item: "neovim".into(),
        message: "extracting".into(),
    })
    .unwrap();

    drain_run_events(&mut app);

    assert_eq!(
        app.current_log
            .iter()
            .filter(|line| line.kind == LogKind::Header)
            .count(),
        1
    );
    assert!(
        app.current_log
            .iter()
            .filter(|line| line.kind != LogKind::Header)
            .all(|line| line.group.as_deref() == Some("neovim / install plugins"))
    );

    app.log_filter = LogFilter::Current;
    let current = lines_text(&visible_log_lines(&app, 10));
    assert!(current.contains("neovim / install plugins"));
    assert!(current.contains("downloading"));
    assert!(current.contains("extracting"));
}

#[test]
fn completed_run_drains_events_beyond_the_per_frame_limit() {
    let mut app = App::new(Mode::Deploy);
    let (tx, rx) = mpsc::channel();
    app.run_events = Some(rx);
    for index in 0..300 {
        tx.send(crate::execute::ExecuteEvent::Output {
            item: "verbose".into(),
            stream: crate::model::OutputStream::Stdout,
            line: format!("line-{index}"),
        })
        .unwrap();
    }
    drop(tx);

    drain_all_run_events(&mut app);

    assert!(app.current_log.iter().any(|line| line.text == "line-299"));
    assert!(app.run_events.is_some());
}

#[test]
fn errors_filter_keeps_headers_even_when_group_is_collapsed() {
    let mut app = App::new(Mode::Deploy);
    push_log_group(&mut app, "fish / install");
    push_log_indented(&mut app, "ordinary stdout", None, 1, LogKind::Stdout);
    push_log_group(&mut app, "neovim / install");
    push_log_indented(&mut app, "ordinary stdout", None, 1, LogKind::Stdout);
    push_log_indented(
        &mut app,
        "failed to extract archive",
        Some(CATPPUCCIN_MOCHA.danger),
        1,
        LogKind::Stderr,
    );
    app.collapsed_log_groups.insert("neovim / install".into());
    app.log_filter = LogFilter::Errors;

    let errors = lines_text(&visible_log_lines(&app, 10));
    assert!(errors.contains("neovim / install"));
    assert!(errors.contains("failed to extract archive"));
    assert!(!errors.contains("fish / install"));
    assert!(!errors.contains("ordinary stdout"));
}

#[test]
fn compact_layout_and_empty_log_have_visible_hints() {
    assert_eq!(layout_density(60, 18), LayoutDensity::Compact);
    assert_eq!(layout_density(100, 30), LayoutDensity::Normal);

    let app = App::new(Mode::Deploy);
    let text = lines_text(&visible_log_lines(&app, 5));
    assert!(text.contains("log is empty"));
    assert_eq!(run_body_lines(&app, 20, 0).len(), 0);
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
fn review_renders_sudo_failure_message() {
    let backend = ratatui::backend::TestBackend::new(80, 12);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["sudo"]));
    app.review_entries = Vec::new();
    app.status_message = "sudo authentication failed".into();

    terminal
        .draw(|frame| render_confirm(frame, &mut app))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let rendered = buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(rendered.contains("sudo authentication failed"));
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
fn review_enter_obeys_the_danger_confirmation_gate() {
    let icon_set = icons::current();
    let mut app = App::new(Mode::Deploy);
    app.screen = Screen::ConfirmView;
    app.review_entries = vec![ReviewEntry {
        order: 0,
        item: "replace config".into(),
        kind: "link",
        kind_icon: icon_set.action_link,
        severity: ReviewSeverity::Danger,
        status: "would overwrite target".into(),
        detail: String::new(),
    }];

    handle_confirm(&mut app, KeyCode::Enter).unwrap();

    assert_eq!(app.screen, Screen::ConfirmView);
    assert_eq!(
        app.status_message,
        "plan has danger items; press ! to confirm before running"
    );
    assert_eq!(app.status_kind, NoticeKind::Error);
}

#[test]
fn review_background_check_success_and_panic_reach_terminal_states() {
    let mut app = App::new(Mode::Deploy);
    start_review(&mut app);
    assert!(app.review_thread.is_none());

    app.plan = Some(test_plan_with_items(&["empty"]));
    start_review(&mut app);
    assert_eq!(app.screen, Screen::ConfirmView);
    assert_eq!(app.status_message, "checking action conditions...");
    assert!(app.review_thread.is_some());
    while !app.review_thread.as_ref().unwrap().is_finished() {
        std::thread::yield_now();
    }
    assert!(poll_review_thread(&mut app));
    assert!(app.review_thread.is_none());
    assert!(app.review_entries.is_empty());
    assert!(app.status_message.is_empty());
    assert!(!poll_review_thread(&mut app));

    app.review_entries.push(ReviewEntry {
        order: 0,
        item: "stale".into(),
        kind: "shell",
        kind_icon: icons::current().action_shell,
        severity: ReviewSeverity::Run,
        status: "run".into(),
        detail: String::new(),
    });
    app.review_thread = Some(std::thread::spawn(|| -> Vec<ReviewEntry> {
        panic!("review test panic")
    }));
    while !app.review_thread.as_ref().unwrap().is_finished() {
        std::thread::yield_now();
    }
    assert!(poll_review_thread(&mut app));
    assert!(app.review_entries.is_empty());
    assert_eq!(app.status_message, "review condition check panicked");
    assert_eq!(app.status_kind, NoticeKind::Error);
}

#[test]
fn review_navigation_running_guard_confirmation_and_back_are_consistent() {
    let icon_set = icons::current();
    let mut app = App::new(Mode::Deploy);
    app.screen = Screen::ConfirmView;
    app.plan = Some(test_plan_with_items(&["danger"]));
    app.review_entries = vec![ReviewEntry {
        order: 0,
        item: "replace config".into(),
        kind: "link",
        kind_icon: icon_set.action_link,
        severity: ReviewSeverity::Danger,
        status: "would overwrite target".into(),
        detail: String::new(),
    }];

    handle_confirm(&mut app, KeyCode::Down).unwrap();
    handle_confirm(&mut app, KeyCode::Char('j')).unwrap();
    assert_eq!(app.review_scroll, 2);
    handle_confirm(&mut app, KeyCode::Up).unwrap();
    handle_confirm(&mut app, KeyCode::Char('k')).unwrap();
    assert_eq!(app.review_scroll, 0);
    handle_confirm(&mut app, KeyCode::PageDown).unwrap();
    assert_eq!(app.review_scroll, 8);
    handle_confirm(&mut app, KeyCode::PageUp).unwrap();
    assert_eq!(app.review_scroll, 0);
    handle_confirm(&mut app, KeyCode::End).unwrap();
    assert_eq!(app.review_scroll, usize::MAX);
    handle_confirm(&mut app, KeyCode::Home).unwrap();
    assert_eq!(app.review_scroll, 0);
    jump_review_bottom(&mut app);
    assert_eq!(app.review_scroll, usize::MAX);
    jump_review_top(&mut app);
    assert_eq!(app.review_scroll, 0);

    let (release_tx, release_rx) = mpsc::channel();
    app.review_thread = Some(std::thread::spawn(move || {
        release_rx.recv().unwrap();
        Vec::new()
    }));
    handle_confirm(&mut app, KeyCode::Enter).unwrap();
    assert_eq!(app.status_message, "review checks are still running");
    release_tx.send(()).unwrap();
    app.review_thread.take().unwrap().join().unwrap();

    handle_confirm(&mut app, KeyCode::Char('!')).unwrap();
    assert!(app.review_danger_confirmed);
    assert_eq!(app.status_message, "danger confirmed; press r to run once");
    handle_confirm(&mut app, KeyCode::Char('r')).unwrap();
    assert!(!app.review_danger_confirmed);
    assert_eq!(app.screen, Screen::ConfirmView);

    handle_confirm(&mut app, KeyCode::Esc).unwrap();
    assert_eq!(app.screen, Screen::PlanView);
    app.screen = Screen::ConfirmView;
    handle_confirm(&mut app, KeyCode::Char('q')).unwrap();
    assert_eq!(app.screen, Screen::PlanView);
    handle_confirm(&mut app, KeyCode::Null).unwrap();
}

#[test]
fn review_render_handles_missing_plan_and_compact_and_normal_summaries() {
    let backend = ratatui::backend::TestBackend::new(50, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let mut missing = App::new(Mode::Deploy);
    terminal
        .draw(|frame| render_confirm(frame, &mut missing))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("no plan loaded"));

    let mut app = App::new(Mode::Deploy);
    app.plan = Some(test_plan_with_items(&["one", "two"]));
    app.plan.as_mut().unwrap().items[1].selected = false;
    app.review_entries = vec![ReviewEntry {
        order: 0,
        item: "one".into(),
        kind: "shell",
        kind_icon: icons::current().action_shell,
        severity: ReviewSeverity::Warning,
        status: "run · sudo".into(),
        detail: "sudo true".into(),
    }];
    terminal
        .draw(|frame| render_confirm(frame, &mut app))
        .unwrap();
    let compact = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(compact.contains("Selected 1"));
    assert!(compact.contains("1 attention"));

    let backend = ratatui::backend::TestBackend::new(100, 24);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    app.status_message = "danger confirmation required".into();
    app.status_kind = NoticeKind::Warning;
    terminal
        .draw(|frame| render_confirm(frame, &mut app))
        .unwrap();
    let normal = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(normal.contains("Selected: 1 steps"));
    assert!(normal.contains("Skipped: 1 steps"));
    assert!(normal.contains("danger confirmation required"));
}

#[test]
fn review_filesystem_previews_preserve_action_severity_and_status() {
    let dir = tempfile::tempdir().unwrap();
    let item = test_plan_item("filesystem");
    let icon_set = icons::current();

    let missing_dir = dir.path().join("missing-dir");
    let create = review_create_entry(&item, &missing_dir, icon_set.action_create);
    assert_eq!(create.severity, ReviewSeverity::Run);
    assert_eq!(create.status, "create");
    std::fs::create_dir(&missing_dir).unwrap();
    let exists = review_create_entry(&item, &missing_dir, icon_set.action_create);
    assert_eq!(exists.severity, ReviewSeverity::Success);
    assert_eq!(exists.status, "exists");

    let source = dir.path().join("source");
    let target = dir.path().join("target");
    std::fs::write(&source, "content").unwrap();
    let link = review_link_entry(
        &item,
        dir.path(),
        &target,
        &source,
        false,
        false,
        icon_set.action_link,
    );
    assert_eq!(link.severity, ReviewSeverity::Run);
    assert_eq!(link.status, "link");
    std::os::unix::fs::symlink(&source, &target).unwrap();
    let linked = review_link_entry(
        &item,
        dir.path(),
        &target,
        &source,
        false,
        false,
        icon_set.action_link,
    );
    assert_eq!(linked.severity, ReviewSeverity::Success);
    assert_eq!(linked.status, "linked");

    let absent = review_clean_entry(
        &item,
        &dir.path().join("absent"),
        false,
        icon_set.action_clean,
    );
    assert_eq!(absent.severity, ReviewSeverity::Skip);
    assert_eq!(absent.status, "skip");
    let clean_link = review_clean_entry(&item, &target, false, icon_set.action_clean);
    assert_eq!(clean_link.severity, ReviewSeverity::Warning);
    assert_eq!(clean_link.status, "remove symlink");
}

#[test]
fn review_severity_helpers_map_to_exact_groups_icons_and_colors() {
    let icon_set = icons::current();
    let cases = [
        (
            ReviewSeverity::Success,
            ReviewGroup::AlreadyOk,
            icon_set.success,
            CATPPUCCIN_MOCHA.success,
        ),
        (
            ReviewSeverity::Skip,
            ReviewGroup::Skipped,
            icon_set.skipped,
            CATPPUCCIN_MOCHA.skip,
        ),
        (
            ReviewSeverity::Run,
            ReviewGroup::WillRun,
            icon_set.running,
            CATPPUCCIN_MOCHA.running,
        ),
        (
            ReviewSeverity::Warning,
            ReviewGroup::Attention,
            icon_set.warning,
            CATPPUCCIN_MOCHA.warning,
        ),
        (
            ReviewSeverity::Danger,
            ReviewGroup::Attention,
            icon_set.failed,
            CATPPUCCIN_MOCHA.danger,
        ),
    ];
    for (severity, group, icon, color) in cases {
        let entry = ReviewEntry {
            order: 0,
            item: "item".into(),
            kind: "unknown",
            kind_icon: icon_set.info,
            severity,
            status: "status".into(),
            detail: String::new(),
        };
        assert_eq!(review_group_for(&entry), group);
        assert_eq!(review_status_icon(icon_set, severity), icon);
        assert_eq!(review_status_style(severity).fg, Some(color));
    }
    assert_eq!(review_kind_rank("install"), 0);
    assert_eq!(review_kind_rank("link"), 1);
    assert_eq!(review_kind_rank("create"), 2);
    assert_eq!(review_kind_rank("shell"), 3);
    assert_eq!(review_kind_rank("clean"), 4);
    assert_eq!(review_kind_rank("unknown"), usize::MAX);
    assert_eq!(selected_item_count(None), 0);
    assert_eq!(selected_action_count(None), 0);
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
        Path::new("."),
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
        Path::new("."),
        icons::current().action_shell,
    );

    assert_eq!(entry.severity, ReviewSeverity::Warning);
    assert_eq!(review_group_for(&entry), ReviewGroup::Attention);
    assert!(entry.status.contains("sudo"));
}

#[test]
fn sudo_does_not_downgrade_a_condition_error() {
    let item = test_plan_item("Broken guarded command");
    let entry = review_shell_entry(
        &item,
        "sudo true",
        None,
        false,
        Some("dotman-command-that-does-not-exist"),
        Path::new("."),
        icons::current().action_shell,
    );

    assert_eq!(entry.severity, ReviewSeverity::Danger);
    assert_eq!(review_group_for(&entry), ReviewGroup::Attention);
    assert!(entry.status.contains("if error:"));
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
        Path::new("."),
        icons::current().action_shell,
    );

    assert_eq!(entry.severity, ReviewSeverity::Skip);
    assert_eq!(review_group_for(&entry), ReviewGroup::Skipped);
    assert_eq!(entry.status, "if skip");
}

#[test]
fn review_relative_config_path_uses_current_directory_for_conditions() {
    let mut plan = test_plan_with_items(&["Skip shell"]);
    plan.items[0].actions.push(Action::Shell {
        command: "echo skipped".into(),
        description: None,
        optional: true,
        if_condition: Some("false".into()),
    });

    let entries = review_entries(&plan, None);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].severity, ReviewSeverity::Skip);
    assert_eq!(entries[0].status, "if skip");
}
