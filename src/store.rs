//! Store: read/write run logs and per-machine selection state.
//!
//! Phase 4: save/load/list runs, ULID naming, latest.json symlink, dir creation.

use crate::model::Run;
use crate::model::RunId;
use crate::model::Selection;
use crate::package_managers::dotman_data_dir;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::path::PathBuf;
use ulid::Ulid;

#[derive(Debug, Clone, Default)]
pub struct RunList {
    pub runs: Vec<Run>,
    pub warnings: Vec<String>,
}

pub fn runs_dir() -> Result<PathBuf> {
    let dir = dotman_data_dir()?.join("runs");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create runs dir {}", dir.display()))?;
    Ok(dir)
}

/// Return the per-machine selection state path.
pub fn selection_path() -> Result<PathBuf> {
    Ok(dotman_data_dir()?.join("state.toml"))
}

pub fn scoped_selection_path(config_path: &Path, config_hash: &str) -> Result<PathBuf> {
    let canonical =
        std::fs::canonicalize(config_path).unwrap_or_else(|_| config_path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    hasher.update(b"\0");
    hasher.update(config_hash.as_bytes());
    let id = format!("{:x}", hasher.finalize());
    Ok(dotman_data_dir()?
        .join("selection")
        .join(format!("{id}.toml")))
}

/// Load per-machine selection state, returning defaults when no state exists yet.
pub fn load_selection() -> Result<Selection> {
    let path = selection_path()?;
    if !path.exists() {
        return Ok(Selection::default());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read selection state {}", path.display()))?;
    toml::from_str(&raw)
        .with_context(|| format!("failed to parse selection state {}", path.display()))
}

pub fn load_selection_scoped(config_path: &Path, config_hash: &str) -> Result<Selection> {
    let scoped = scoped_selection_path(config_path, config_hash)?;
    if scoped.exists() {
        let raw = std::fs::read_to_string(&scoped)
            .with_context(|| format!("failed to read selection state {}", scoped.display()))?;
        return toml::from_str(&raw)
            .with_context(|| format!("failed to parse selection state {}", scoped.display()));
    }
    load_selection()
}

/// Save per-machine selection state.
pub fn save_selection(selection: &Selection) -> Result<PathBuf> {
    let path = selection_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create state dir {}", parent.display()))?;
    }
    let raw = toml::to_string_pretty(selection).context("failed to serialize selection state")?;
    atomic_write(&path, raw.as_bytes())
        .with_context(|| format!("failed to write selection state {}", path.display()))?;
    Ok(path)
}

pub fn save_selection_scoped(
    config_path: &Path,
    config_hash: &str,
    selection: &Selection,
) -> Result<PathBuf> {
    let path = scoped_selection_path(config_path, config_hash)?;
    let raw = toml::to_string_pretty(selection).context("failed to serialize selection state")?;
    atomic_write(&path, raw.as_bytes())
        .with_context(|| format!("failed to write selection state {}", path.display()))?;
    Ok(path)
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
    atomic_write(&path, json.as_bytes())
        .with_context(|| format!("failed to write run log {}", path.display()))?;

    // Update latest.json symlink.
    let latest = latest_link()?;
    let _ = std::fs::remove_file(&latest);
    if let Err(e) = std::os::unix::fs::symlink(&path, &latest) {
        tracing::warn!("failed to update latest.json symlink: {e}");
    }

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
    Ok(list_detailed()?.runs)
}

pub fn list_detailed() -> Result<RunList> {
    let dir = runs_dir()?;
    let mut runs: Vec<Run> = Vec::new();
    let mut warnings = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .with_context(|| format!("failed to read runs dir {}", dir.display()))?
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(format!("failed to read run entry: {error}"));
                continue;
            }
        };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if path.file_name().and_then(|s| s.to_str()) == Some("latest.json") {
            continue;
        }
        match std::fs::read_to_string(&path) {
            Ok(json) => match serde_json::from_str::<Run>(&json) {
                Ok(run) => runs.push(run),
                Err(error) => warnings.push(format!(
                    "failed to parse run log {}: {error}",
                    path.display()
                )),
            },
            Err(error) => warnings.push(format!(
                "failed to read run log {}: {error}",
                path.display()
            )),
        }
    }
    runs.sort_by_key(|run| std::cmp::Reverse(run_sort_key(run)));
    Ok(RunList { runs, warnings })
}

pub fn delete(id: &RunId) -> Result<()> {
    let path = path_for(id)?;
    std::fs::remove_file(&path)
        .with_context(|| format!("failed to delete run log {}", path.display()))?;
    refresh_latest_link()?;
    Ok(())
}

pub fn new_run_id() -> String {
    Ulid::new().to_string()
}

fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create dir {}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("tmp")
    ));
    std::fs::write(&tmp, bytes)
        .with_context(|| format!("failed to write temp file {}", tmp.display()))?;
    if let Err(error) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(error)
            .with_context(|| format!("failed to rename {} to {}", tmp.display(), path.display()));
    }
    Ok(())
}

fn refresh_latest_link() -> Result<()> {
    let latest = latest_link()?;
    if let Err(error) = std::fs::remove_file(&latest)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!("failed to remove stale latest.json: {error}");
    }
    let Some(run) = list_detailed()?.runs.into_iter().next() else {
        return Ok(());
    };
    let path = path_for(&run.id)?;
    if let Err(e) = std::os::unix::fs::symlink(&path, &latest) {
        tracing::warn!("failed to update latest.json symlink: {e}");
    }
    Ok(())
}

fn run_sort_key(run: &Run) -> String {
    if run.id.len() == 26 && run.id.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        run.id.clone()
    } else {
        run.started_at.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ActionStatus;
    use crate::model::{Mode, RunItem, RunStatus};
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn sample_run(id: &str) -> Run {
        Run {
            id: id.to_string(),
            plan_id: None,
            mode: Mode::Deploy,
            started_at: format!("epoch:{}", 1_000_000),
            finished_at: Some(format!("epoch:{}", 1_000_010)),
            status: RunStatus::Success,
            config_hash: "abc".into(),
            config_path: None,
            host: None,
            items: vec![RunItem {
                id: "1".into(),
                name: "fish".into(),
                status: ActionStatus::NoChange,
                started_at: Some("epoch:1".into()),
                finished_at: Some("epoch:2".into()),
                duration_ms: Some(1000),
                attempts: 1,
                error: None,
                output: vec![],
                actions: vec![],
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

    #[test]
    fn selection_roundtrips_through_toml() {
        let mut selection = Selection::default();
        selection.items.insert("fish".into(), true);
        selection.items.insert("ghostty".into(), false);

        let raw = toml::to_string_pretty(&selection).unwrap();
        let parsed: Selection = toml::from_str(&raw).unwrap();

        assert_eq!(parsed.items.get("fish"), Some(&true));
        assert_eq!(parsed.items.get("ghostty"), Some(&false));
    }

    #[test]
    fn list_reports_invalid_json_warnings() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvGuard::new(&["XDG_DATA_HOME", "HOME"]);
        unsafe {
            std::env::set_var("XDG_DATA_HOME", dir.path());
            std::env::set_var("HOME", dir.path());
        }
        let runs = runs_dir().unwrap();
        std::fs::write(runs.join("bad.json"), "{not-json").unwrap();
        save(&sample_run("01K00000000000000000000000")).unwrap();

        let report = list_detailed().unwrap();

        assert_eq!(report.runs.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].contains("failed to parse run log"));
    }

    #[test]
    fn deleting_latest_refreshes_latest_link() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvGuard::new(&["XDG_DATA_HOME", "HOME"]);
        unsafe {
            std::env::set_var("XDG_DATA_HOME", dir.path());
            std::env::set_var("HOME", dir.path());
        }
        let newer = sample_run("01K00000000000000000000002");
        let older = sample_run("01K00000000000000000000001");
        save(&older).unwrap();
        save(&newer).unwrap();

        delete(&newer.id).unwrap();

        let latest = latest_link().unwrap();
        assert_eq!(
            std::fs::read_link(latest).unwrap(),
            path_for(&older.id).unwrap()
        );
        delete(&older.id).unwrap();
        assert!(!latest_link().unwrap().exists());
    }

    #[test]
    fn scoped_selection_isolated_by_config_identity() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let dir = tempfile::tempdir().unwrap();
        let _env = EnvGuard::new(&["XDG_DATA_HOME", "HOME"]);
        unsafe {
            std::env::set_var("XDG_DATA_HOME", dir.path());
            std::env::set_var("HOME", dir.path());
        }
        let config_a = dir.path().join("a").join("dotman.yaml");
        let config_b = dir.path().join("b").join("dotman.yaml");
        std::fs::create_dir_all(config_a.parent().unwrap()).unwrap();
        std::fs::create_dir_all(config_b.parent().unwrap()).unwrap();
        std::fs::write(&config_a, "").unwrap();
        std::fs::write(&config_b, "").unwrap();
        let mut selection = Selection::default();
        selection.items.insert("fish".into(), true);

        save_selection_scoped(&config_a, "hash-a", &selection).unwrap();

        assert_eq!(
            load_selection_scoped(&config_a, "hash-a")
                .unwrap()
                .items
                .get("fish"),
            Some(&true)
        );
        assert!(
            load_selection_scoped(&config_b, "hash-b")
                .unwrap()
                .items
                .is_empty()
        );
    }

    struct EnvGuard {
        values: Vec<(&'static str, Option<std::ffi::OsString>)>,
    }

    impl EnvGuard {
        fn new(keys: &[&'static str]) -> Self {
            Self {
                values: keys
                    .iter()
                    .map(|key| (*key, std::env::var_os(key)))
                    .collect(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.values.drain(..) {
                restore_env(key, value);
            }
        }
    }

    fn restore_env(key: &str, old: Option<std::ffi::OsString>) {
        unsafe {
            if let Some(value) = old {
                std::env::set_var(key, value);
            } else {
                std::env::remove_var(key);
            }
        }
    }
}
