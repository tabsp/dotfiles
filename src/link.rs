use crate::config::DotfilesManifest;
use crate::path::{ensure_parent_dir, expand_home};
use crate::platform::Host;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Conflict {
    Fail,
    Backup,
    Overwrite,
}

pub fn run_link(
    files: &DotfilesManifest,
    host: &Host,
    repo: &Path,
    conflict: Conflict,
    dry_run: bool,
) -> Result<(), String> {
    let actions = plan(files, host, repo, conflict)?;
    if dry_run {
        print_dry_run(&actions);
        if actions
            .iter()
            .any(|action| matches!(action.kind, ActionKind::WouldFail))
        {
            return Err("dry-run: would fail".to_string());
        }
        return Ok(());
    }

    for action in actions {
        apply_action(action)?;
    }
    Ok(())
}

#[derive(Debug)]
struct Action {
    kind: ActionKind,
    source: PathBuf,
    target: PathBuf,
    reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActionKind {
    WouldLink,
    WouldBackup,
    WouldOverwrite,
    WouldFail,
}

fn plan(
    files: &DotfilesManifest,
    host: &Host,
    repo: &Path,
    conflict: Conflict,
) -> Result<Vec<Action>, String> {
    let mut actions = Vec::new();
    for entry in &files.files {
        if !entry.enabled {
            continue;
        }
        if !entry.platforms.is_empty() && !entry.platforms.iter().any(|p| p == host.platform.key()) {
            continue;
        }

        let source = repo.join(&entry.source);
        let target = expand_home(&entry.target)?;
        let reason = if target.exists() || target.is_symlink() {
            if is_expected_symlink(&target, &source) {
                None
            } else {
                Some(describe_conflict(&target, &source))
            }
        } else {
            None
        };

        let kind = match reason {
            None => ActionKind::WouldLink,
            Some(_) => match conflict {
                Conflict::Fail => ActionKind::WouldFail,
                Conflict::Backup => ActionKind::WouldBackup,
                Conflict::Overwrite => ActionKind::WouldOverwrite,
            },
        };

        actions.push(Action {
            kind,
            source,
            target,
            reason,
        });
    }

    Ok(actions)
}

fn apply_action(action: Action) -> Result<(), String> {
    ensure_parent_dir(&action.target)?;
    match action.kind {
        ActionKind::WouldFail => Err(format!("target conflict: {}", action.target.display())),
        ActionKind::WouldBackup => {
            let backup = unique_backup_path(&action.target);
            fs::rename(&action.target, &backup)
                .map_err(|err| format!("failed to backup {}: {err}", action.target.display()))?;
            unix_fs::symlink(&action.source, &action.target)
                .map_err(|err| format!("failed to link {}: {err}", action.target.display()))
        }
        ActionKind::WouldOverwrite => {
            remove_existing(&action.target)?;
            unix_fs::symlink(&action.source, &action.target)
                .map_err(|err| format!("failed to link {}: {err}", action.target.display()))
        }
        ActionKind::WouldLink => {
            if is_expected_symlink(&action.target, &action.source) {
                Ok(())
            } else {
                unix_fs::symlink(&action.source, &action.target)
                    .map_err(|err| format!("failed to link {}: {err}", action.target.display()))
            }
        }
    }
}

fn is_expected_symlink(target: &Path, source: &Path) -> bool {
    fs::read_link(target)
        .map(|actual| actual == source)
        .unwrap_or(false)
}

fn describe_conflict(target: &Path, source: &Path) -> String {
    if let Ok(actual) = fs::read_link(target) {
        return format!(
            "symlink points to {}, expected {}",
            actual.display(),
            source.display()
        );
    }
    if target.is_dir() {
        "target is an existing directory".to_string()
    } else if target.is_file() {
        "target is an existing file".to_string()
    } else {
        "target exists with unsupported file type".to_string()
    }
}

fn remove_existing(path: &Path) -> Result<(), String> {
    if path.is_dir() && !path.is_symlink() {
        fs::remove_dir_all(path)
            .map_err(|err| format!("failed to remove directory {}: {err}", path.display()))
    } else {
        fs::remove_file(path).map_err(|err| format!("failed to remove {}: {err}", path.display()))
    }
}

fn unique_backup_path(target: &Path) -> PathBuf {
    let ts = timestamp();
    let mut candidate = PathBuf::from(format!("{}.backup.{ts}", target.display()));
    let mut counter = 1;
    while candidate.exists() {
        candidate = PathBuf::from(format!("{}.backup.{ts}.{counter}", target.display()));
        counter += 1;
    }
    candidate
}

fn timestamp() -> String {
    let now =
        time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let format = time::macros::format_description!("[year][month][day][hour][minute][second]");
    now.format(&format)
        .unwrap_or_else(|_| "19700101000000".to_string())
}

fn print_dry_run(actions: &[Action]) {
    for wanted in [
        ActionKind::WouldFail,
        ActionKind::WouldOverwrite,
        ActionKind::WouldBackup,
        ActionKind::WouldLink,
    ] {
        for action in actions.iter().filter(|action| action.kind == wanted) {
            println!(
                "{:?}: {} -> {}",
                action.kind,
                action.source.display(),
                action.target.display()
            );
            if let Some(reason) = &action.reason {
                println!("  reason: {reason}");
            }
        }
    }

    if actions
        .iter()
        .any(|action| matches!(action.kind, ActionKind::WouldFail))
    {
        println!("dry-run: would fail");
    } else {
        println!("dry-run: success");
    }
}
