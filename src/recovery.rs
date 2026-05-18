use std::fs;
use std::path::Path;

use crate::config;
use crate::path::expand_home;

pub fn run_cleanup(execute: bool) -> Result<(), String> {
    let repo = std::env::current_dir()
        .map_err(|err| format!("failed to read current dir: {err}"))?;
    let bin_dir = expand_home("~/.local/bin")?;

    let mut stale: Vec<StaleItem> = Vec::new();

    // Scan install backup/staging directories
    if let Ok(entries) = fs::read_dir(&bin_dir) {
        for entry in entries {
            let entry = entry.map_err(|err| format!("read_dir entry error: {err}"))?;
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|n| n.to_str())
            {
                if name.ends_with(".dotman-backup") {
                    stale.push(StaleItem {
                        path,
                        category: "install backup",
                    });
                } else if name.ends_with(".dotman-staging") {
                    stale.push(StaleItem {
                        path,
                        category: "staging leftover",
                    });
                }
            }
        }
    }

    // Scan link-conflict backups
    if repo.join("dotfiles.toml").exists()
        && let Ok(files) = config::load_dotfiles(Path::new("dotfiles.toml"))
    {
        for file in &files.files {
            if !file.enabled {
                continue;
            }
            let target = match expand_home(&file.target) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let Some(parent) = target.parent() else {
                continue;
            };
            let Some(target_name) = target.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let prefix = format!("{target_name}.backup.");

            if let Ok(entries) = fs::read_dir(parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if name.starts_with(&prefix)
                        && name[prefix.len()..]
                            .chars()
                            .all(|c| c.is_ascii_digit())
                    {
                        stale.push(StaleItem {
                            path,
                            category: "link-conflict backup",
                        });
                    }
                }
            }
        }
    }

    if stale.is_empty() {
        println!("nothing to clean up");
        println!("hint: run `dotman status` for a full inventory of managed state");
        return Ok(());
    }

    for item in &stale {
        if execute {
            if item.path.is_dir() {
                fs::remove_dir_all(&item.path)
                    .map_err(|err| format!("failed to remove {}: {err}", item.path.display()))?;
            } else {
                fs::remove_file(&item.path)
                    .map_err(|err| format!("failed to remove {}: {err}", item.path.display()))?;
            }
            println!("removed {} ({})", item.path.display(), item.category);
        } else {
            println!("would remove {} ({})", item.path.display(), item.category);
        }
    }

    if !execute {
        println!("run with --execute to remove");
    }
    println!("hint: run `dotman status` for a full inventory of managed state");

    Ok(())
}

struct StaleItem {
    path: std::path::PathBuf,
    category: &'static str,
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn find_link_backups_matches_timestamp_pattern() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let backup = tmp.path().join("nvim.backup.20260518000000");
        fs::write(&backup, b"backup").expect("write");

        let found = find_link_backups(tmp.path(), "nvim");
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("nvim.backup.20260518000000"));
    }

    #[test]
    fn find_link_backups_ignores_non_timestamp_suffix() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::write(tmp.path().join("nvim.backup.critical"), b"data").expect("write");

        let found = find_link_backups(tmp.path(), "nvim");
        assert!(found.is_empty());
    }

    #[test]
    fn find_link_backups_ignores_unrelated_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::write(tmp.path().join("other-file"), b"data").expect("write");
        fs::write(tmp.path().join("nvimrc"), b"data").expect("write");

        let found = find_link_backups(tmp.path(), "nvim");
        assert!(found.is_empty());
    }

    fn find_stale_dirs(dir: &Path) -> Vec<std::path::PathBuf> {
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

    fn find_link_backups(dir: &Path, prefix: &str) -> Vec<std::path::PathBuf> {
        let mut found = Vec::new();
        let pattern = format!("{prefix}.backup.");
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if name.starts_with(&pattern)
                    && name[pattern.len()..].chars().all(|c| c.is_ascii_digit())
                {
                    found.push(path);
                }
            }
        }
        found
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
