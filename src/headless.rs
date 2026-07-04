use crate::config;
use crate::execute;
use crate::model::{Mode as PlanMode, Run};
use crate::plan;
use crate::store;
use std::path::PathBuf;

use crate::Mode;

pub fn run(mode: Mode) -> Result<(), String> {
    let config_path = find_config_path().map_err(|e| e.to_string())?;
    let cfg = config::load(&config_path).map_err(|e| e.to_string())?;

    match mode {
        Mode::Plan => {
            let plan = plan::build(&cfg, PlanMode::Deploy).map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&plan).map_err(|e| e.to_string())?;
            println!("{json}");
            Ok(())
        }
        Mode::History => run_history(),
        Mode::Run(_) => Err("dotman run <id> requires TUI".into()),
        Mode::Menu | Mode::Deploy | Mode::Bootstrap => run_full(&cfg, mode),
    }
}

fn run_full(cfg: &config::Config, mode: Mode) -> Result<(), String> {
    let plan_mode = match mode {
        Mode::Menu | Mode::Deploy => PlanMode::Deploy,
        Mode::Bootstrap => PlanMode::Bootstrap,
        _ => unreachable!(),
    };
    let plan = plan::build(cfg, plan_mode).map_err(|e| e.to_string())?;
    let run = execute::execute(&plan, cfg).map_err(|e| e.to_string())?;
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

fn find_config_path() -> anyhow::Result<PathBuf> {
    // Order: cwd/dotman.yaml, DOTFILES_DIR/dotman.yaml, ~/.local/share/tabsp-dotfiles/dotman.yaml.
    let candidates = [
        PathBuf::from("dotman.yaml"),
        PathBuf::from("dotman.bootstrap.yaml"),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    if let Ok(dir) = std::env::var("DOTFILES_DIR") {
        let p = PathBuf::from(dir).join("dotman.yaml");
        if p.exists() {
            return Ok(p);
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let p = PathBuf::from(home)
        .join(".local/share/tabsp-dotfiles")
        .join("dotman.yaml");
    if p.exists() {
        return Ok(p);
    }
    anyhow::bail!("no dotman.yaml found in any standard location")
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
            plan_id: "test".into(),
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
            }],
        };
        let saved = std::path::Path::new("/tmp/test-run.json");
        // Should not panic — just prints to stdout
        print_summary(&run, saved);
    }
}
