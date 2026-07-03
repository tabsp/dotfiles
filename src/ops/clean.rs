//! Clean operation: symlink-only by default, force: true for files/dirs.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanAction {
    Skip,
    RemoveSymlink,
    BackupAndRemove(PathBuf),
}

pub fn plan_clean(target: &Path, force: bool) -> Result<CleanAction> {
    if !target.exists() && !target.is_symlink() {
        return Ok(CleanAction::Skip);
    }

    if target.is_symlink() {
        return Ok(CleanAction::RemoveSymlink);
    }

    if !force {
        return Ok(CleanAction::Skip);
    }

    // force + non-symlink -> backup + remove.
    let backup = super::link::unique_backup_path(target);
    Ok(CleanAction::BackupAndRemove(backup))
}

pub fn apply_clean(action: CleanAction, target: &Path) -> Result<()> {
    match action {
        CleanAction::Skip => Ok(()),
        CleanAction::RemoveSymlink => std::fs::remove_file(target)
            .with_context(|| format!("failed to remove symlink {}", target.display())),
        CleanAction::BackupAndRemove(backup) => {
            std::fs::rename(target, &backup).with_context(|| {
                format!(
                    "failed to backup {} to {}",
                    target.display(),
                    backup.display()
                )
            })?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_when_target_missing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nope");
        let action = plan_clean(&target, false).unwrap();
        assert_eq!(action, CleanAction::Skip);
    }

    #[test]
    fn remove_symlink_by_default() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("link");
        std::os::unix::fs::symlink("/tmp/x", &target).unwrap();
        let action = plan_clean(&target, false).unwrap();
        assert_eq!(action, CleanAction::RemoveSymlink);
        apply_clean(action, &target).unwrap();
        assert!(!target.exists());
    }

    #[test]
    fn skip_file_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("file");
        std::fs::write(&target, "x").unwrap();
        let action = plan_clean(&target, false).unwrap();
        assert_eq!(action, CleanAction::Skip);
    }

    #[test]
    fn backup_and_remove_file_with_force() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("file");
        std::fs::write(&target, "x").unwrap();
        let action = plan_clean(&target, true).unwrap();
        match action {
            CleanAction::BackupAndRemove(backup) => {
                apply_clean(CleanAction::BackupAndRemove(backup.clone()), &target).unwrap();
                assert!(!target.exists());
                assert!(backup.exists());
            }
            _ => panic!("expected BackupAndRemove"),
        }
    }
}
