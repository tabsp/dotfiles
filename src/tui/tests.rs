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
