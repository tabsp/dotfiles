use super::*;
use crate::model::{Action, HostInfo, MAX_HISTORY_OUTPUT_LINES, Mode, Plan, PlanItem, RunStatus};
use crate::plan::build;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn test_host() -> HostInfo {
    HostInfo {
        hostname: "host".into(),
        os: "test".into(),
        arch: "test".into(),
        user: "user".into(),
        home: PathBuf::from("/tmp"),
    }
}

fn test_plan(actions: Vec<Action>) -> Plan {
    Plan {
        id: "plan-id".into(),
        mode: Mode::Deploy,
        created_at: "2026-01-01T00:00:00Z".into(),
        config_path: PathBuf::from("/tmp/dotman.yaml"),
        config_hash: "hash".into(),
        host: test_host(),
        items: vec![PlanItem {
            id: "item".into(),
            name: "item".into(),
            layer: "misc".into(),
            actions,
            selected: true,
        }],
        auto_install_pkg_manager: false,
    }
}

fn test_install_spec(name: &str, pkg_mgr: &str) -> crate::ops::install::InstallSpec {
    let db = crate::ops::install::load_db().unwrap();
    crate::ops::install::resolve_install(&db, name, pkg_mgr).unwrap()
}

fn test_config(path: PathBuf) -> Config {
    Config {
        path,
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec![],
        links: vec![],
        create: vec![],
        shell: vec![],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    }
}

#[test]
fn execute_empty_plan_runs_no_actions() {
    let cfg = Config {
        path: PathBuf::from("/tmp/dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec![],
        links: vec![],
        create: vec![],
        shell: vec![],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = build(&cfg, Mode::Deploy).unwrap();
    let run = execute(&plan, &cfg).unwrap();
    assert_eq!(run.status, RunStatus::Success);
    assert_eq!(run.items.len(), 0);
}

#[test]
fn same_plan_generates_unique_run_ids() {
    let cfg = test_config(PathBuf::from("/tmp/dotman.yaml"));
    let plan = test_plan(vec![Action::Shell {
        command: "true".into(),
        description: Some("ok".into()),
        optional: false,
        if_condition: None,
    }]);

    let first = execute(&plan, &cfg).unwrap();
    let second = execute(&plan, &cfg).unwrap();

    assert_ne!(first.id, second.id);
    assert_eq!(first.plan_id.as_deref(), Some("plan-id"));
    assert_eq!(second.plan_id.as_deref(), Some("plan-id"));
}

#[test]
fn link_relink_setting_is_used_by_execute() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let old_source = dir.path().join("old");
    let target = dir.path().join("target");
    std::fs::write(&source, "new").unwrap();
    std::fs::write(&old_source, "old").unwrap();
    std::os::unix::fs::symlink(&old_source, &target).unwrap();
    let cfg = test_config(dir.path().join("dotman.yaml"));
    let plan = test_plan(vec![Action::Link {
        target: target.clone(),
        source: source.clone(),
        backup: false,
        relink: true,
    }]);

    let run = execute(&plan, &cfg).unwrap();

    assert_eq!(run.status, RunStatus::Success);
    assert_eq!(run.items[0].actions[0].status, ActionStatus::WillLink);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
}

#[test]
fn link_without_backup_or_relink_fails_but_returns_partial_run() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let target = dir.path().join("target");
    std::fs::write(&source, "new").unwrap();
    std::fs::write(&target, "conflict").unwrap();
    let cfg = test_config(dir.path().join("dotman.yaml"));
    let plan = test_plan(vec![Action::Link {
        target,
        source,
        backup: false,
        relink: false,
    }]);

    let run = execute(&plan, &cfg).unwrap();

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.items[0].actions[0].status, ActionStatus::WillFail);
    assert!(run.items[0].actions[0].error.is_some());
}

#[test]
fn action_outputs_remain_assigned_after_item_output_truncation() {
    let first = format!(
        "for i in $(seq 1 {}); do echo first-$i; done",
        MAX_HISTORY_OUTPUT_LINES + 20
    );
    let second = "echo second-action".to_string();
    let cfg = test_config(PathBuf::from("/tmp/dotman.yaml"));
    let plan = test_plan(vec![
        Action::Shell {
            command: first,
            description: Some("first".into()),
            optional: false,
            if_condition: None,
        },
        Action::Shell {
            command: second,
            description: Some("second".into()),
            optional: false,
            if_condition: None,
        },
    ]);

    let run = execute(&plan, &cfg).unwrap();

    let actions = &run.items[0].actions;
    assert_eq!(actions.len(), 2);
    assert!(
        actions[0]
            .output
            .iter()
            .all(|line| !line.line.contains("second-action"))
    );
    assert!(
        actions[1]
            .output
            .iter()
            .any(|line| line.line.contains("second-action"))
    );
}

#[test]
fn execute_with_events_can_abort_before_running() {
    let cfg = Config {
        path: PathBuf::from("/tmp/dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec!["fish".into()],
        links: vec![],
        create: vec![],
        shell: vec![],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = build(&cfg, Mode::Deploy).unwrap();
    let mut saw_abort = false;
    let run = execute_with_events(
        &plan,
        &cfg,
        |event| {
            if matches!(event, ExecuteEvent::Aborted) {
                saw_abort = true;
            }
        },
        || true,
    )
    .unwrap();

    assert!(saw_abort);
    assert_eq!(run.status, RunStatus::Aborted);
    assert_eq!(run.items.len(), plan.items.len());
    for (run_item, plan_item) in run.items.iter().zip(&plan.items) {
        if plan_item.selected {
            assert_eq!(run_item.status, ActionStatus::NotRun);
            assert!(
                run_item
                    .error
                    .as_deref()
                    .is_some_and(|e| e.contains("aborted"))
            );
        } else {
            assert_eq!(run_item.status, ActionStatus::WillSkip);
        }
    }
}

#[test]
fn install_skips_when_binary_is_present() {
    fn deny_sudo(_: &str) -> bool {
        false
    }

    let mut events = Vec::new();
    let mut sudo_auth = deny_sudo;
    let spec = test_install_spec("sh", &crate::package_managers::default_pkg_mgr_name());
    let (status, err, attempts, output) = run_install_streaming(
        &spec,
        DEFAULT_INSTALL_RETRIES,
        "sh",
        &mut |event| events.push(event),
        &|| false,
        &mut sudo_auth,
    )
    .unwrap();

    assert_eq!(status, ActionStatus::NoChange);
    assert!(err.is_none());
    assert_eq!(attempts, 0);
    assert!(
        output
            .iter()
            .any(|line| line.line.contains("already installed: sh"))
    );
    assert!(events.iter().any(|event| {
        matches!(event, ExecuteEvent::ActionMessage { message, .. } if message.contains("already installed: sh"))
    }));
}

#[test]
fn install_executes_command_captured_in_spec() {
    fn allow_sudo(_: &str) -> bool {
        true
    }

    let mut spec = test_install_spec("dotman-test-snapshot-command", "brew");
    spec.command = Some("printf 'snapshot-command\\n'".into());
    spec.error = None;
    let mut events = Vec::new();
    let mut sudo_auth = allow_sudo;

    let (status, err, attempts, output) = run_install_streaming(
        &spec,
        0,
        "snapshot install",
        &mut |event| events.push(event),
        &|| false,
        &mut sudo_auth,
    )
    .unwrap();

    assert_eq!(status, ActionStatus::WillInstall);
    assert!(err.is_none());
    assert_eq!(attempts, 1);
    assert!(output.iter().any(|line| line.line == "snapshot-command"));
}

#[test]
fn streaming_shell_command_emits_output_events() {
    let cfg = Config {
        path: PathBuf::from("/tmp/dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec![],
        links: vec![],
        create: vec![],
        shell: vec![crate::config::ShellEntry {
            command: "echo hello".into(),
            description: Some("test shell".into()),
            optional: false,
            if_condition: None,
        }],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = build(&cfg, Mode::Deploy).unwrap();
    let mut outputs: Vec<String> = Vec::new();
    let run = execute_with_events(
        &plan,
        &cfg,
        |event| {
            if let ExecuteEvent::Output { line, .. } = &event {
                outputs.push(line.clone());
            }
        },
        || false,
    )
    .unwrap();

    assert_eq!(run.status, RunStatus::Success);
    assert!(outputs.iter().any(|l| l.contains("hello")));
}

#[test]
fn shell_command_runs_when_config_path_has_no_parent() {
    let dir = tempfile::tempdir().unwrap();
    let original_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let cfg = Config {
        path: PathBuf::from("dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec![],
        links: vec![],
        create: vec![],
        shell: vec![crate::config::ShellEntry {
            command: "echo relative-config-path".into(),
            description: Some("relative config path shell".into()),
            optional: false,
            if_condition: None,
        }],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = build(&cfg, Mode::Deploy).unwrap();
    let run = execute(&plan, &cfg).unwrap();

    std::env::set_current_dir(original_cwd).unwrap();
    assert_eq!(run.status, RunStatus::Success);
    assert!(run.items.iter().any(|item| {
        item.output
            .iter()
            .any(|line| line.line.contains("relative-config-path"))
    }));
}

#[test]
fn link_emits_action_message() {
    let dir = tempfile::tempdir().unwrap();
    // Create source file.
    let src = dir.path().join("test_src");
    std::fs::write(&src, "content").unwrap();
    let target = dir.path().join("test_link");

    let cfg = Config {
        path: dir.path().join("dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec![],
        links: vec![crate::config::LinkEntry {
            target: target.clone(),
            source: src.clone(),
            backup: None,
            relink: None,
        }],
        create: vec![],
        shell: vec![],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = build(&cfg, Mode::Deploy).unwrap();
    let mut messages: Vec<String> = Vec::new();
    let run = execute_with_events(
        &plan,
        &cfg,
        |event| {
            if let ExecuteEvent::ActionMessage { message, .. } = &event {
                messages.push(message.clone());
            }
        },
        || false,
    )
    .unwrap();

    assert_eq!(run.status, RunStatus::Success);
    assert_eq!(run.items[0].status, ActionStatus::WillLink);
    assert!(messages.iter().any(|m| m.contains("linked")));
}

#[test]
fn create_existing_directory_reports_no_change() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("already-there");
    std::fs::create_dir_all(&target).unwrap();

    let cfg = Config {
        path: dir.path().join("dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec![],
        links: vec![],
        create: vec![target.clone()],
        shell: vec![],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = build(&cfg, Mode::Deploy).unwrap();
    let run = execute(&plan, &cfg).unwrap();

    assert_eq!(run.status, RunStatus::Success);
    assert_eq!(run.items[0].status, ActionStatus::NoChange);
    assert!(
        run.items[0]
            .output
            .iter()
            .any(|line| line.line.contains("exists"))
    );
}

#[test]
fn clean_actions_execute_skip_symlink_and_backup_remove_paths() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing");
    let source = dir.path().join("source");
    let symlink = dir.path().join("symlink");
    let forced = dir.path().join("forced.txt");
    std::fs::write(&source, "source").unwrap();
    std::os::unix::fs::symlink(&source, &symlink).unwrap();
    std::fs::write(&forced, "remove me").unwrap();

    let cfg = test_config(dir.path().join("dotman.yaml"));
    let plan = test_plan(vec![
        Action::Clean {
            target: missing.clone(),
            force: false,
        },
        Action::Clean {
            target: symlink.clone(),
            force: false,
        },
        Action::Clean {
            target: forced.clone(),
            force: true,
        },
    ]);

    let run = execute(&plan, &cfg).unwrap();

    assert_eq!(run.status, RunStatus::Success);
    assert_eq!(run.items[0].actions.len(), 3);
    assert_eq!(run.items[0].actions[0].status, ActionStatus::NoChange);
    assert_eq!(run.items[0].actions[1].status, ActionStatus::WillClean);
    assert_eq!(
        run.items[0].actions[2].status,
        ActionStatus::WillBackupRemove
    );
    assert!(!missing.exists());
    assert!(!symlink.is_symlink());
    assert!(!forced.exists());
    assert!(
        dir.path()
            .read_dir()
            .unwrap()
            .filter_map(|entry| entry.ok())
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with("forced.backup."))
    );
}

#[test]
fn condition_execution_error_fails_action_without_running_command() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("must-not-run");
    let later = dir.path().join("later-action");
    let cfg = test_config(dir.path().join("dotman.yaml"));
    let plan = test_plan(vec![
        Action::Shell {
            command: format!("touch {}", marker.display()),
            description: Some("guarded command".into()),
            optional: false,
            if_condition: Some("dotman-condition-command-that-does-not-exist-52f1".into()),
        },
        Action::Create {
            target: later.clone(),
        },
    ]);
    let mut errors = Vec::new();

    let run = execute_with_events(
        &plan,
        &cfg,
        |event| {
            if let ExecuteEvent::ActionError { message, .. } = event {
                errors.push(message);
            }
        },
        || false,
    )
    .unwrap();

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.items[0].actions[0].status, ActionStatus::WillFail);
    assert!(
        run.items[0].actions[0]
            .error
            .as_deref()
            .is_some_and(|error| error.contains("condition failed to execute"))
    );
    assert_eq!(run.items[0].actions[1].status, ActionStatus::NotRun);
    assert!(!marker.exists());
    assert!(!later.exists());
    assert!(
        errors
            .iter()
            .any(|message| message.contains("condition error"))
    );
}

#[test]
fn abort_between_actions_preserves_completed_and_not_run_states() {
    use std::cell::Cell;

    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first");
    let second = dir.path().join("second");
    let later_item_target = dir.path().join("later-item");
    let cfg = test_config(dir.path().join("dotman.yaml"));
    let mut plan = test_plan(vec![
        Action::Create {
            target: first.clone(),
        },
        Action::Create {
            target: second.clone(),
        },
    ]);
    plan.items.push(PlanItem {
        id: "later-item".into(),
        name: "later item".into(),
        layer: "misc".into(),
        actions: vec![Action::Create {
            target: later_item_target.clone(),
        }],
        selected: true,
    });
    let checks = Cell::new(0);
    let abort_events = Cell::new(0);

    let run = execute_with_events(
        &plan,
        &cfg,
        |event| {
            if matches!(event, ExecuteEvent::Aborted) {
                abort_events.set(abort_events.get() + 1);
            }
        },
        || {
            let current = checks.get();
            checks.set(current + 1);
            current >= 2
        },
    )
    .unwrap();

    assert_eq!(run.status, RunStatus::Aborted);
    assert_eq!(abort_events.get(), 1);
    assert_eq!(run.items.len(), 2);
    assert_eq!(run.items[0].status, ActionStatus::Aborted);
    assert_eq!(run.items[0].actions[0].status, ActionStatus::WillCreate);
    assert_eq!(run.items[0].actions[1].status, ActionStatus::NotRun);
    assert_eq!(run.items[1].status, ActionStatus::NotRun);
    assert_eq!(run.items[1].actions[0].status, ActionStatus::NotRun);
    assert!(first.is_dir());
    assert!(!second.exists());
    assert!(!later_item_target.exists());
}

#[test]
fn failed_action_stops_remaining_actions_in_item() {
    let dir = tempfile::tempdir().unwrap();
    let create_target = dir.path().join("should-not-exist");
    let cfg = Config {
        path: dir.path().join("dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec![],
        links: vec![],
        create: vec![],
        shell: vec![],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = Plan {
        id: "test-run".into(),
        mode: Mode::Deploy,
        created_at: "2026-01-01T00:00:00Z".into(),
        config_path: cfg.path.clone(),
        config_hash: "hash".into(),
        host: HostInfo {
            hostname: "test".into(),
            os: "Linux".into(),
            arch: "x86_64".into(),
            user: "test".into(),
            home: dir.path().to_path_buf(),
        },
        items: vec![PlanItem {
            id: "failing-item".into(),
            name: "failing item".into(),
            layer: "misc".into(),
            selected: true,
            actions: vec![
                Action::Shell {
                    command: "false".into(),
                    description: None,
                    optional: false,
                    if_condition: None,
                },
                Action::Create {
                    target: create_target.clone(),
                },
            ],
        }],
        auto_install_pkg_manager: false,
    };

    let run = execute(&plan, &cfg).unwrap();

    assert_eq!(run.status, RunStatus::Failed);
    assert!(!create_target.exists());
}

#[test]
fn failed_item_does_not_stop_later_items() {
    let dir = tempfile::tempdir().unwrap();
    let later_target = dir.path().join("later-item-ran");
    let cfg = test_config(dir.path().join("dotman.yaml"));
    let plan = Plan {
        id: "multi-item".into(),
        mode: Mode::Deploy,
        created_at: "2026-01-01T00:00:00Z".into(),
        config_path: cfg.path.clone(),
        config_hash: "hash".into(),
        host: test_host(),
        items: vec![
            PlanItem {
                id: "failing".into(),
                name: "failing".into(),
                layer: "misc".into(),
                selected: true,
                actions: vec![Action::Shell {
                    command: "false".into(),
                    description: Some("fail".into()),
                    optional: false,
                    if_condition: None,
                }],
            },
            PlanItem {
                id: "later".into(),
                name: "later".into(),
                layer: "misc".into(),
                selected: true,
                actions: vec![Action::Create {
                    target: later_target.clone(),
                }],
            },
        ],
        auto_install_pkg_manager: false,
    };

    let run = execute(&plan, &cfg).unwrap();

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.items.len(), 2);
    assert_eq!(run.items[0].status, ActionStatus::WillFail);
    assert_eq!(run.items[1].status, ActionStatus::WillCreate);
    assert!(later_target.is_dir());
}

#[test]
fn optional_shell_failure_allows_the_next_action_to_run() {
    let dir = tempfile::tempdir().unwrap();
    let later_target = dir.path().join("created-after-optional-failure");
    let cfg = test_config(dir.path().join("dotman.yaml"));
    let plan = test_plan(vec![
        Action::Shell {
            command: "false".into(),
            description: Some("optional failure".into()),
            optional: true,
            if_condition: None,
        },
        Action::Create {
            target: later_target.clone(),
        },
    ]);

    let run = execute(&plan, &cfg).unwrap();

    assert_eq!(run.status, RunStatus::Success);
    assert_eq!(run.items[0].actions.len(), 2);
    assert_eq!(run.items[0].actions[0].status, ActionStatus::NoChange);
    assert_eq!(run.items[0].actions[1].status, ActionStatus::WillCreate);
    assert!(later_target.is_dir());
}

#[test]
fn execute_emits_lifecycle_events_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = test_config(dir.path().join("dotman.yaml"));
    let plan = test_plan(vec![Action::Create {
        target: dir.path().join("created"),
    }]);
    let mut lifecycle = Vec::new();

    let run = execute_with_events(
        &plan,
        &cfg,
        |event| match event {
            ExecuteEvent::ItemStarted { .. } => lifecycle.push("item-started"),
            ExecuteEvent::ActionStarted { .. } => lifecycle.push("action-started"),
            ExecuteEvent::ActionMessage { .. } => lifecycle.push("action-message"),
            ExecuteEvent::ActionFinished { .. } => lifecycle.push("action-finished"),
            ExecuteEvent::ItemFinished { .. } => lifecycle.push("item-finished"),
            ExecuteEvent::Output { .. }
            | ExecuteEvent::ActionError { .. }
            | ExecuteEvent::SudoPrompt { .. }
            | ExecuteEvent::Aborted => {}
        },
        || false,
    )
    .unwrap();

    assert_eq!(run.status, RunStatus::Success);
    assert_eq!(
        lifecycle,
        [
            "item-started",
            "action-started",
            "action-message",
            "action-finished",
            "item-finished",
        ]
    );
}

#[test]
fn abort_preserves_current_item_in_run() {
    // When a shell action is aborted, the current RunItem should still
    // appear in run.items with its output and error.
    let cfg = Config {
        path: std::path::PathBuf::from("/tmp/dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec![],
        links: vec![],
        create: vec![],
        shell: vec![crate::config::ShellEntry {
            command: "echo started && sleep 10".into(),
            description: Some("long shell".into()),
            optional: false,
            if_condition: None,
        }],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = build(&cfg, Mode::Deploy).unwrap();
    let abort_flag = Arc::new(AtomicBool::new(false));
    let emit_flag = Arc::clone(&abort_flag);
    let check_flag = Arc::clone(&abort_flag);
    let run = execute_with_events(
        &plan,
        &cfg,
        move |event| {
            if matches!(event, ExecuteEvent::Output { line, .. } if line.contains("started"))
                && !emit_flag.load(Ordering::SeqCst)
            {
                emit_flag.store(true, Ordering::SeqCst);
            }
        },
        move || check_flag.load(Ordering::SeqCst),
    )
    .unwrap();

    assert_eq!(run.status, RunStatus::Aborted);
    // The shell item must be present in the run.
    let shell_item = run.items.iter().find(|i| i.name != "fish").unwrap();
    assert!(shell_item.error.is_some());
    assert!(
        shell_item.output.iter().any(|l| l.line.contains("started")),
        "output should contain 'started' line"
    );
}

#[test]
fn install_abort_sets_run_status_and_emits_abort_event() {
    use std::cell::Cell;

    let cfg = Config {
        path: PathBuf::from("/tmp/dotman.yaml"),
        package_managers: crate::config::PackageManagerConfig::default(),
        install: vec!["dotman-binary-that-does-not-exist".into()],
        links: vec![],
        create: vec![],
        shell: vec![],
        default_shell: None,
        clean: vec![],
        auto_install_pkg_manager: false,
    };
    let plan = build(&cfg, Mode::Deploy).unwrap();
    let abort_checks = Cell::new(0);
    let abort_events = Cell::new(0);

    let run = execute_with_events(
        &plan,
        &cfg,
        |event| {
            if matches!(event, ExecuteEvent::Aborted) {
                abort_events.set(abort_events.get() + 1);
            }
        },
        || {
            let check = abort_checks.get();
            abort_checks.set(check + 1);
            check > 1
        },
    )
    .unwrap();

    assert_eq!(run.status, RunStatus::Aborted);
    assert_eq!(run.items[0].status, ActionStatus::Aborted);
    assert_eq!(run.items[0].actions[0].status, ActionStatus::Aborted);
    assert_eq!(abort_events.get(), 1);
}

#[test]
fn history_output_respects_500_line_cap() {
    use crate::model::{OutputLine, OutputStream};
    // Build a vec with 600 lines, push them through push_output_line,
    // and verify only the last 500 are retained.
    let mut output: Vec<OutputLine> = Vec::new();
    for i in 0..600 {
        push_output_line(&mut output, OutputStream::Stdout, &format!("line {i}"));
    }
    assert_eq!(output.len(), 500);
    assert_eq!(output.first().unwrap().line, "line 100");
    assert_eq!(output.last().unwrap().line, "line 599");
}

#[test]
fn cap_output_after_extend() {
    use crate::model::{OutputLine, OutputStream};
    // Simulate the install retry pattern: push 1 action line,
    // extend 500 output lines, push another action line — must stay at 500.
    let mut all_output: Vec<OutputLine> = Vec::new();
    push_output_line(&mut all_output, OutputStream::Action, "attempt 1/3");
    for i in 0..500 {
        push_output_line(&mut all_output, OutputStream::Stdout, &format!("line {i}"));
    }
    assert_eq!(all_output.len(), 500);
    // Now "extend" another batch (simulating retry output)
    let mut retry_output = Vec::new();
    for i in 0..100 {
        retry_output.push(OutputLine {
            stream: OutputStream::Stdout,
            line: format!("retry {i}"),
        });
    }
    all_output.extend(retry_output);
    cap_output_len(&mut all_output);
    push_output_line(&mut all_output, OutputStream::Action, "final");
    assert_eq!(all_output.len(), 500);
    assert_eq!(all_output.last().unwrap().line, "final");
}

#[test]
fn shell_quote_handles_single_quotes() {
    assert_eq!(shell_quote("hello"), "'hello'");
    assert_eq!(shell_quote("it's working"), "'it'\\''s working'");
    assert_eq!(shell_quote(""), "''");
}

#[test]
fn font_command_starts_with_mkdir_not_quoted_command() {
    // shell_quote must be called on individual arguments, not entire
    // commands — otherwise the shell tries to execute a binary named
    // "mkdir -p /path" instead of running mkdir with arguments.
    let source_url = "https://example.com/font.zip";
    let home = "/home/user";
    let name = "test-font";
    let fonts_dir = format!("{home}/.local/share/fonts");
    let zip = format!("{fonts_dir}/{name}.zip");

    let cmd = format!(
        "mkdir -p {} && curl -fsSL {} -o {} && unzip -o -q {} -d {} && rm -f {}",
        shell_quote(&fonts_dir),
        shell_quote(source_url),
        shell_quote(&zip),
        shell_quote(&zip),
        shell_quote(&fonts_dir),
        shell_quote(&zip),
    );

    // Must start with mkdir, not a quoted string containing mkdir.
    assert!(
        cmd.starts_with("mkdir"),
        "font command should start with 'mkdir', got: {cmd}"
    );
    // Must contain the quoted paths.
    assert!(cmd.contains("'/home/user/.local/share/fonts'"));
    // Must not quote the entire mkdir command.
    assert!(!cmd.contains("'mkdir"));
}
