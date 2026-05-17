use std::fs;
use std::path::PathBuf;

use crate::path::expand_home;

pub fn run_cleanup(execute: bool) -> Result<(), String> {
    let bin_dir = expand_home("~/.local/bin")?;

    let entries = fs::read_dir(&bin_dir)
        .map_err(|err| format!("failed to read {}: {err}", bin_dir.display()))?;

    let mut stale: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("read_dir entry error: {err}"))?;
        let path = entry.path();
        if path.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && (name.ends_with(".dotman-backup") || name.ends_with(".dotman-staging"))
        {
            stale.push(path);
        }
    }

    if stale.is_empty() {
        println!("nothing to clean up");
        return Ok(());
    }

    for path in &stale {
        if execute {
            fs::remove_dir_all(path)
                .map_err(|err| format!("failed to remove {}: {err}", path.display()))?;
            println!("removed {}", path.display());
        } else {
            println!("would remove {}", path.display());
        }
    }

    if !execute {
        println!("run with --execute to remove");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn cleanup_empty_dir_finds_nothing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let result = run_cleanup_at(tmp.path(), false);
        assert!(result.is_ok());
    }

    #[test]
    fn cleanup_finds_stale_backup_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let backup = tmp.path().join("mytool.dotman-backup");
        fs::create_dir(&backup).expect("create backup");
        let stale = find_stale_dirs(tmp.path());
        assert_eq!(stale.len(), 1);
        assert!(stale[0].ends_with("mytool.dotman-backup"));
    }

    #[test]
    fn cleanup_finds_stale_staging_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let staging = tmp.path().join("mytool.dotman-staging");
        fs::create_dir(&staging).expect("create staging");
        let stale = find_stale_dirs(tmp.path());
        assert_eq!(stale.len(), 1);
        assert!(stale[0].ends_with("mytool.dotman-staging"));
    }

    #[test]
    fn cleanup_ignores_regular_files_and_other_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::write(tmp.path().join("regular-file"), b"data").expect("write");
        fs::create_dir(tmp.path().join("normal-dir")).expect("create dir");
        let stale = find_stale_dirs(tmp.path());
        assert!(stale.is_empty());
    }

    fn find_stale_dirs(dir: &Path) -> Vec<PathBuf> {
        let mut stale = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && (name.ends_with(".dotman-backup") || name.ends_with(".dotman-staging"))
                {
                    stale.push(path);
                }
            }
        }
        stale
    }

    fn run_cleanup_at(dir: &Path, execute: bool) -> Result<(), String> {
        let stale = find_stale_dirs(dir);
        if stale.is_empty() {
            return Ok(());
        }
        if execute {
            for path in &stale {
                fs::remove_dir_all(path).map_err(|e| format!("{e}"))?;
            }
        }
        Ok(())
    }
}
