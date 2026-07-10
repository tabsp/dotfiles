//! Headless runner (--headless mode, no TUI).
//!
//! Non-interactive execution: uses safe defaults, fails on ambiguity.
//! Suitable for scripts, CI, new machine automation, and remote bootstrap.

use crate::cli::Mode;
use crate::config;
use crate::execute;
use crate::execute::ExecuteEvent;
use crate::model::{Mode as PlanMode, OutputStream, Run};
use crate::ops::shell;
use crate::plan;
use crate::store;
use std::path::PathBuf;

/// Run with a resolved config path (for plan/deploy commands).
pub fn run_with_mode(config_path: PathBuf, mode: Mode) -> Result<(), String> {
    let cfg = config::load(&config_path).map_err(|e| e.to_string())?;

    match mode {
        Mode::Plan => {
            let plan = plan::build(&cfg, PlanMode::Deploy).map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?;
            println!("{json}");
            Ok(())
        }
        Mode::Deploy => run_full(&cfg),
        Mode::Menu => Err(
            "headless mode requires an explicit subcommand (deploy, plan, sync, status, doctor). \
             run `dotman --help` for available commands"
                .into(),
        ),
        _ => Err(format!("mode {mode:?} requires TUI")),
    }
}

/// Run without a config (for history/run commands that don't need config).
pub fn run_no_config(mode: Mode) -> Result<(), String> {
    match mode {
        Mode::History => run_history(),
        Mode::Run(id) => run_run(&id),
        _ => Err(format!("mode {mode:?} requires config")),
    }
}

fn run_run(id: &str) -> Result<(), String> {
    let run = store::load(&id.to_string()).map_err(|e| format!("failed to load run {id}: {e}"))?;
    print_run_detail(&run);
    Ok(())
}

fn print_run_detail(run: &crate::model::Run) {
    println!("Run: {}", run.id);
    println!("  mode: {:?}", run.mode);
    println!("  started: {}", run.started_at);
    println!("  status: {}", run.status.result_label());
    if let Some(finished) = &run.finished_at {
        println!("  finished: {finished}");
    }
    println!("  items: {}", run.items.len());
    println!();
    for row in run_detail_rows(run) {
        let error = row.error.map(|e| format!(" ({e})")).unwrap_or_default();
        println!("  {:<36} {:<14}{}", row.name, row.status, error);
    }
}

struct RunDetailRow<'a> {
    name: String,
    status: &'static str,
    error: Option<&'a str>,
}

fn run_detail_rows(run: &Run) -> Vec<RunDetailRow<'_>> {
    let mut rows = Vec::new();
    for item in &run.items {
        if item.actions.is_empty() {
            rows.push(RunDetailRow {
                name: item.name.clone(),
                status: item.status.result_label(),
                error: item.error.as_deref(),
            });
            continue;
        }
        rows.extend(item.actions.iter().map(|action| RunDetailRow {
            name: format!("{} / {}", item.name, action.name),
            status: action.status.result_label(),
            error: action.error.as_deref(),
        }));
    }
    rows
}

fn run_full(cfg: &config::Config) -> Result<(), String> {
    let mut plan = plan::build(cfg, PlanMode::Deploy).map_err(|e| e.to_string())?;
    plan.sync_auto_steps();

    // Pre-cache sudo credentials if any action needs them.
    if plan.needs_sudo() {
        eprintln!("[dotman] sudo required — enter password to continue:");
        if !shell::pre_cache_sudo().unwrap_or(false) {
            return Err("sudo authentication failed".into());
        }
    }

    let run = execute::execute_with_events(
        &plan,
        cfg,
        |event| match event {
            ExecuteEvent::Output { item, stream, line } => {
                let prefix = match stream {
                    OutputStream::Stdout => "stdout",
                    OutputStream::Stderr => "stderr",
                    OutputStream::Action => "action",
                };
                println!("[{item} {prefix}] {line}");
            }
            ExecuteEvent::ActionMessage { item, message } => {
                println!("[{item} action] {message}");
            }
            ExecuteEvent::ActionError { item, message } => {
                eprintln!("[{item} error] {message}");
            }
            ExecuteEvent::ItemStarted { name, .. } => {
                println!("[dotman] started {name}");
            }
            ExecuteEvent::ItemFinished { name, status, .. } => {
                println!("[dotman] finished {name}: {}", status.result_label());
            }
            ExecuteEvent::Aborted => {
                println!("[dotman] run aborted");
            }
            ExecuteEvent::SudoPrompt { response, .. } => {
                let ok = shell::pre_cache_sudo().unwrap_or(false);
                let _ = response.send(ok);
            }
            ExecuteEvent::ActionStarted { .. } => {
                // Headless: action starts are implicit from the output that follows.
            }
            ExecuteEvent::ActionFinished { .. } => {
                // Headless: final action status is summarized by the item result.
            }
        },
        || false,
    )
    .map_err(|e| e.to_string())?;

    print_summary(&run);
    let saved = store::save(&run).map_err(|error| history_save_error(&run, &error))?;
    println!("log: {}", saved.display());
    match run.status {
        crate::model::RunStatus::Success => Ok(()),
        _ => Err(format!(
            "run finished with status {}",
            run.status.result_label()
        )),
    }
}

fn run_history() -> Result<(), String> {
    let runs = store::list().map_err(|e| e.to_string())?;
    if runs.is_empty() {
        println!("no runs yet");
        return Ok(());
    }
    println!("{:<26}  {:<10}  {:<8}  STARTED", "ID", "MODE", "STATUS");
    for run in runs {
        let status = run.status.result_label();
        let mode = format!("{:?}", run.mode).to_lowercase();
        println!(
            "{:<26}  {:<10}  {:<8}  {}",
            run.id, mode, status, run.started_at
        );
    }
    Ok(())
}

fn print_summary(run: &Run) {
    let summary = crate::model::RunSummary::from_run(run);
    println!("run {} finished: {}", run.id, summary.display());
}

fn history_save_error(run: &Run, error: &impl std::fmt::Display) -> String {
    format!(
        "run {} finished with status {}, but history was not saved: {error}; check disk space and dotman data directory permissions",
        run.id,
        run.status.result_label()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_summary_does_not_panic() {
        let run = Run {
            id: "test".into(),
            plan_id: None,
            mode: crate::model::Mode::Deploy,
            started_at: "2026-01-01T00:00:00Z".into(),
            finished_at: Some("2026-01-01T00:01:00Z".into()),
            status: crate::model::RunStatus::Success,
            config_hash: "abc".into(),
            config_path: None,
            host: None,
            items: vec![crate::model::RunItem {
                id: "1".into(),
                name: "fish".into(),
                status: crate::model::ActionStatus::NoChange,
                started_at: None,
                finished_at: None,
                duration_ms: Some(100),
                attempts: 1,
                error: None,
                output: vec![],
                actions: vec![],
            }],
        };
        print_summary(&run);
    }

    #[test]
    fn history_save_error_preserves_run_result_and_next_step() {
        let mut run = test_run();
        run.status = crate::model::RunStatus::Failed;

        let message = history_save_error(&run, &"disk full");

        assert!(message.contains("finished with status failed"));
        assert!(message.contains("history was not saved: disk full"));
        assert!(message.contains("check disk space"));
    }

    #[test]
    fn run_detail_uses_action_results_and_legacy_item_fallback() {
        let mut run = test_run();
        run.items = vec![crate::model::RunItem {
            id: "multi".into(),
            name: "multi".into(),
            status: crate::model::ActionStatus::WillFail,
            started_at: None,
            finished_at: None,
            duration_ms: None,
            attempts: 1,
            error: Some("item error".into()),
            output: vec![],
            actions: vec![
                crate::model::RunAction {
                    kind: "create".into(),
                    name: "create dir".into(),
                    status: crate::model::ActionStatus::WillCreate,
                    error: None,
                    output: vec![],
                },
                crate::model::RunAction {
                    kind: "shell".into(),
                    name: "configure".into(),
                    status: crate::model::ActionStatus::WillFail,
                    error: Some("exit code 1".into()),
                    output: vec![],
                },
            ],
        }];

        let rows = run_detail_rows(&run);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "multi / create dir");
        assert_eq!(rows[0].status, "changed");
        assert_eq!(rows[0].error, None);
        assert_eq!(rows[1].status, "failed");
        assert_eq!(rows[1].error, Some("exit code 1"));

        run.items[0].actions.clear();
        let legacy = run_detail_rows(&run);
        assert_eq!(legacy.len(), 1);
        assert_eq!(legacy[0].name, "multi");
        assert_eq!(legacy[0].error, Some("item error"));
    }

    fn test_run() -> Run {
        Run {
            id: "test".into(),
            plan_id: None,
            mode: crate::model::Mode::Deploy,
            started_at: "2026-01-01T00:00:00Z".into(),
            finished_at: Some("2026-01-01T00:01:00Z".into()),
            status: crate::model::RunStatus::Success,
            config_hash: "abc".into(),
            config_path: None,
            host: None,
            items: vec![],
        }
    }
}
