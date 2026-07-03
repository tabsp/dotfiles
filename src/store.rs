//! Store: read/write run logs to ~/.local/share/dotman/runs/.
//!
//! Phase 4: save/load/list runs, ULID naming, latest.json symlink, dir creation.

use crate::model::Run;
use crate::model::RunId;
use crate::package_managers::dotman_data_dir;
use anyhow::{Context, Result};
use std::path::PathBuf;
use ulid::Ulid;

pub fn runs_dir() -> Result<PathBuf> {
    let dir = dotman_data_dir()?.join("runs");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create runs dir {}", dir.display()))?;
    Ok(dir)
}

pub fn path_for(id: &RunId) -> Result<PathBuf> {
    let dir = runs_dir()?;
    Ok(dir.join(format!("{id}.json")))
}

pub fn latest_link() -> Result<PathBuf> {
    let dir = runs_dir()?;
    Ok(dir.join("latest.json"))
}

pub fn save(run: &Run) -> Result<PathBuf> {
    let path = path_for(&run.id)?;
    let json = serde_json::to_string_pretty(run).context("failed to serialize run")?;
    std::fs::write(&path, json)
        .with_context(|| format!("failed to write run log {}", path.display()))?;

    // Update latest.json symlink.
    let latest = latest_link()?;
    let _ = std::fs::remove_file(&latest);
    let _ = std::os::unix::fs::symlink(&path, &latest);

    Ok(path)
}

pub fn load(id: &RunId) -> Result<Run> {
    let path = path_for(id)?;
    let json = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read run log {}", path.display()))?;
    let run: Run = serde_json::from_str(&json)
        .with_context(|| format!("failed to parse run log {}", path.display()))?;
    Ok(run)
}

pub fn list() -> Result<Vec<Run>> {
    let dir = runs_dir()?;
    let mut runs: Vec<Run> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("failed to read runs dir {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) == Some("latest.json") {
            continue;
        }
        if let Ok(json) = std::fs::read_to_string(&path)
            && let Ok(run) = serde_json::from_str::<Run>(&json)
        {
            runs.push(run);
        }
    }
    runs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(runs)
}

pub fn delete(id: &RunId) -> Result<()> {
    let path = path_for(id)?;
    std::fs::remove_file(&path)
        .with_context(|| format!("failed to delete run log {}", path.display()))?;
    Ok(())
}

pub fn new_run_id() -> String {
    Ulid::new().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ActionStatus;
    use crate::model::{Mode, RunItem, RunStatus};

    fn sample_run(id: &str) -> Run {
        Run {
            id: id.to_string(),
            plan_id: id.to_string(),
            mode: Mode::Deploy,
            started_at: format!("epoch:{}", 1_000_000),
            finished_at: Some(format!("epoch:{}", 1_000_010)),
            status: RunStatus::Success,
            config_hash: "abc".into(),
            items: vec![RunItem {
                id: "1".into(),
                name: "fish".into(),
                status: ActionStatus::NoChange,
                started_at: Some("epoch:1".into()),
                finished_at: Some("epoch:2".into()),
                duration_ms: Some(1000),
                attempts: 1,
                error: None,
            }],
        }
    }

    #[test]
    fn run_roundtrips_through_serde() {
        let id = new_run_id();
        let run = sample_run(&id);
        let json = serde_json::to_string_pretty(&run).unwrap();
        let parsed: Run = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, run.id);
        assert_eq!(parsed.items.len(), 1);
    }

    #[test]
    fn new_run_id_is_ulid_format() {
        let id = new_run_id();
        assert_eq!(id.len(), 26);
    }
}
