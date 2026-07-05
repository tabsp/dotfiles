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
    println!("  status: {:?}", run.status);
    if let Some(finished) = &run.finished_at {
        println!("  finished: {finished}");
    }
    println!("  items: {}", run.items.len());
    println!();
    for item in &run.items {
        let status = format!("{:?}", item.status).to_lowercase();
        let error = item
            .error
            .as_deref()
            .map(|e| format!(" ({e})"))
            .unwrap_or_default();
        println!("  {:<24} {:<14}{}", item.name, status, error);
    }
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
            ExecuteEvent::ItemStarted { name, .. } => {
                println!("[dotman] started {name}");
            }
            ExecuteEvent::ItemFinished { name, status, .. } => {
                println!("[dotman] finished {name}: {status:?}");
            }
            ExecuteEvent::Aborted => {
                println!("[dotman] run aborted");
            }
            ExecuteEvent::ActionStarted { .. } => {
                // Headless: action starts are implicit from the output that follows.
            }
        },
        || false,
    )
    .map_err(|e| e.to_string())?;

    let saved = store::save(&run).map_err(|e| e.to_string())?;
    print_summary(&run, &saved);
    match run.status {
        crate::model::RunStatus::Success => Ok(()),
        _ => Err(format!("run finished with status {:?}", run.status)),
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
        let status = format!("{:?}", run.status).to_lowercase();
        let mode = format!("{:?}", run.mode).to_lowercase();
        println!(
            "{:<26}  {:<10}  {:<8}  {}",
            run.id, mode, status, run.started_at
        );
    }
    Ok(())
}

fn print_summary(run: &Run, saved: &std::path::Path) {
    let ok = run
        .items
        .iter()
        .filter(|i| matches!(i.status, crate::model::ActionStatus::NoChange))
        .count();
    let installed = run
        .items
        .iter()
        .filter(|i| matches!(i.status, crate::model::ActionStatus::WillInstall))
        .count();
    let failed = run.items.iter().filter(|i| i.error.is_some()).count();
    println!(
        "run {} finished: {} ok, {} installed, {} failed",
        run.id, ok, installed, failed,
    );
    println!("log: {}", saved.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_summary_does_not_panic() {
        let run = Run {
            id: "test".into(),
            mode: crate::model::Mode::Deploy,
            started_at: "2026-01-01T00:00:00Z".into(),
            finished_at: Some("2026-01-01T00:01:00Z".into()),
            status: crate::model::RunStatus::Success,
            config_hash: "abc".into(),
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
            }],
        };
        let saved = std::path::Path::new("/tmp/test-run.json");
        print_summary(&run, saved);
    }
}
