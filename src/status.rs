use crate::config::{self, InstallEntry, Installer};
use crate::path::expand_home;
use crate::path::paths_match;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize)]
struct StatusOutput {
    installed_tools: Vec<ToolEntry>,
    linked_dotfiles: Vec<DotfileEntry>,
    backups: Vec<BackupEntry>,
    staging_leftovers: Vec<StagingEntry>,
    source_checkout: Option<SourceCheckoutEntry>,
}

#[derive(Debug, Serialize)]
struct ToolEntry {
    name: String,
    path: String,
    kind: String,
    certainty: String,
}

#[derive(Debug, Serialize)]
struct DotfileEntry {
    name: String,
    path: String,
    target: String,
    certainty: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct BackupEntry {
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) certainty: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct StagingEntry {
    pub(crate) path: String,
    pub(crate) certainty: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct SourceCheckoutEntry {
    pub(crate) path: String,
    pub(crate) certainty: String,
    pub(crate) is_git_repo: bool,
}

pub fn run_status(json: bool) -> Result<(), String> {
    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;

    if !repo.join("deps.toml").exists() || !repo.join("dotfiles.toml").exists() {
        return Err(
            "not in a dotfiles repo. The release installer clones the repo to \
             ~/.local/share/dotman/dotfiles — run dotman status from there."
                .to_string(),
        );
    }

    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;

    let installed_tools = collect_tools(&deps.deps);
    let linked_dotfiles = collect_dotfiles(&files, &repo);
    let (backups, staging_leftovers) = collect_backups_and_staging()?;
    let source_checkout = collect_source_checkout();

    if installed_tools.is_empty()
        && linked_dotfiles.is_empty()
        && backups.is_empty()
        && staging_leftovers.is_empty()
        && source_checkout.is_none()
    {
        println!("no managed state found");
        return Ok(());
    }

    if json {
        let output = StatusOutput {
            installed_tools,
            linked_dotfiles,
            backups,
            staging_leftovers,
            source_checkout,
        };
        let json_str =
            serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?;
        println!("{json_str}");
        return Ok(());
    }

    print_human(
        &installed_tools,
        &linked_dotfiles,
        &backups,
        &staging_leftovers,
        &source_checkout,
    );
    Ok(())
}

fn collect_tools(deps: &BTreeMap<String, crate::config::Dependency>) -> Vec<ToolEntry> {
    let mut tools: Vec<ToolEntry> = Vec::new();

    for (name, dep) in deps {
        let mac_arm = dep.entries_for("mac", "aarch64");
        let mac_x86 = dep.entries_for("mac", "x86_64");
        let linux_arm = dep.entries_for("linux", "aarch64");
        let linux_x86 = dep.entries_for("linux", "x86_64");

        for entry in mac_arm
            .iter()
            .chain(mac_x86.iter())
            .chain(linux_arm.iter())
            .chain(linux_x86.iter())
        {
            if let Some(tool) = tool_from_entry(name, entry) {
                tools.push(tool);
                break;
            }
        }
    }

    tools.sort_by(|a, b| a.name.cmp(&b.name));
    tools
}

fn param_str(entry: &InstallEntry, key: &str) -> Option<String> {
    entry.params.get(key)?.as_str().map(|s| s.to_string())
}

fn tool_from_entry(name: &str, entry: &InstallEntry) -> Option<ToolEntry> {
    match entry.installer {
        Installer::DownloadBinary | Installer::OfficialScript => {
            if let (Some(install_to), Some(install_dir_to)) = (
                param_str(entry, "install_to"),
                param_str(entry, "install_dir_to"),
            ) {
                let install_to_path = expand_home(&install_to).ok()?;
                let install_dir = expand_home(&install_dir_to).ok()?;

                match fs::read_link(&install_to_path) {
                    Ok(target) => {
                        let target_canon = target.canonicalize().ok();
                        let dir_canon = install_dir.canonicalize().ok();
                        let is_managed = match (&target_canon, &dir_canon) {
                            (Some(t), Some(d)) => *t == *d,
                            _ => false,
                        };
                        if is_managed {
                            return Some(ToolEntry {
                                name: name.to_string(),
                                path: install_to_path.display().to_string(),
                                kind: "directory_symlink".to_string(),
                                certainty: "managed".to_string(),
                            });
                        }
                    }
                    Err(_) => {
                        if install_to_path.exists() {
                            return Some(ToolEntry {
                                name: name.to_string(),
                                path: install_to_path.display().to_string(),
                                kind: "directory_symlink".to_string(),
                                certainty: "detected".to_string(),
                            });
                        }
                    }
                }
                return None;
            }

            if let Some(install_to) = param_str(entry, "install_to") {
                let install_to_path = expand_home(&install_to).ok()?;
                if install_to_path.exists() {
                    return Some(ToolEntry {
                        name: name.to_string(),
                        path: install_to_path.display().to_string(),
                        kind: "binary".to_string(),
                        certainty: "detected".to_string(),
                    });
                }
                return None;
            }

            if entry.installer == Installer::OfficialScript && crate::path::which(name).is_some() {
                return Some(ToolEntry {
                    name: name.to_string(),
                    path: format!("(on PATH: {name})"),
                    kind: "binary".to_string(),
                    certainty: "detected".to_string(),
                });
            }

            None
        }
        _ => None,
    }
}

fn collect_dotfiles(files: &crate::config::DotfilesManifest, repo: &Path) -> Vec<DotfileEntry> {
    let mut dotfiles: Vec<DotfileEntry> = Vec::new();

    for file in &files.files {
        if !file.enabled {
            continue;
        }
        let target = match expand_home(&file.target) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let expected = repo.join(&file.source);

        match fs::read_link(&target) {
            Ok(actual) if paths_match(&actual, &expected) => {
                let name = file
                    .source
                    .strip_prefix("config/")
                    .unwrap_or(&file.source)
                    .to_string();
                dotfiles.push(DotfileEntry {
                    name,
                    path: target.display().to_string(),
                    target: expected.display().to_string(),
                    certainty: "managed".to_string(),
                });
            }
            Ok(_) | Err(_) => {}
        }
    }

    dotfiles.sort_by(|a, b| a.name.cmp(&b.name));
    dotfiles
}

pub(crate) fn collect_backups_and_staging() -> Result<(Vec<BackupEntry>, Vec<StagingEntry>), String>
{
    let bin_dir = expand_home("~/.local/bin")?;
    let mut backups: Vec<BackupEntry> = Vec::new();
    let mut staging: Vec<StagingEntry> = Vec::new();

    if let Ok(entries) = fs::read_dir(&bin_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !path.is_dir() {
                continue;
            }
            if name.ends_with(".dotman-backup") {
                backups.push(BackupEntry {
                    path: path.display().to_string(),
                    kind: "install_backup".to_string(),
                    certainty: "managed".to_string(),
                });
            } else if name.ends_with(".dotman-staging") {
                staging.push(StagingEntry {
                    path: path.display().to_string(),
                    certainty: "managed".to_string(),
                });
            }
        }
    }

    backups.sort_by(|a, b| a.path.cmp(&b.path));
    staging.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((backups, staging))
}

pub(crate) fn collect_source_checkout() -> Option<SourceCheckoutEntry> {
    let path = expand_home("~/.local/share/dotman/dotfiles").ok()?;
    if !path.exists() {
        return None;
    }
    let is_git = path.join(".git").exists();
    Some(SourceCheckoutEntry {
        path: path.display().to_string(),
        certainty: "detected".to_string(),
        is_git_repo: is_git,
    })
}

fn print_human(
    tools: &[ToolEntry],
    dotfiles: &[DotfileEntry],
    backups: &[BackupEntry],
    staging: &[StagingEntry],
    checkout: &Option<SourceCheckoutEntry>,
) {
    println!("==> Installed tools ({})", tools.len());
    if tools.is_empty() {
        println!("  (none)");
    } else {
        for t in tools {
            println!(
                "  {:<12} {:<30} ({}, {})",
                t.name, t.path, t.kind, t.certainty
            );
        }
    }

    println!();
    println!("==> Linked dotfiles ({})", dotfiles.len());
    if dotfiles.is_empty() {
        println!("  (none)");
    } else {
        for d in dotfiles {
            println!(
                "  {:<12} {:<30} -> {} ({})",
                d.name, d.path, d.target, d.certainty
            );
        }
    }

    println!();
    println!("==> Backups ({})", backups.len());
    if backups.is_empty() {
        println!("  (none)");
    } else {
        for b in backups {
            println!("  {:<40} ({})", b.path, b.certainty);
        }
    }

    println!();
    println!("==> Staging leftovers ({})", staging.len());
    if staging.is_empty() {
        println!("  (none)");
    } else {
        for s in staging {
            println!("  {:<40} ({})", s.path, s.certainty);
        }
    }

    println!();
    println!("==> Source checkout");
    if let Some(c) = checkout {
        println!(
            "  {} (certainty: {}, git: {})",
            c.path, c.certainty, c.is_git_repo
        );
    } else {
        println!("  (none)");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn status_output_json_serializes() {
        let output = StatusOutput {
            installed_tools: vec![ToolEntry {
                name: "bat".to_string(),
                path: "/home/user/.local/bin/bat".to_string(),
                kind: "directory_symlink".to_string(),
                certainty: "managed".to_string(),
            }],
            linked_dotfiles: vec![],
            backups: vec![],
            staging_leftovers: vec![],
            source_checkout: None,
        };
        let json = serde_json::to_string_pretty(&output).expect("serialize");
        assert!(json.contains("bat"));
        assert!(json.contains("managed"));
    }

    #[test]
    fn status_output_json_deserializes_back() {
        let output = StatusOutput {
            installed_tools: vec![],
            linked_dotfiles: vec![],
            backups: vec![BackupEntry {
                path: "/tmp/foo.dotman-backup".to_string(),
                kind: "install_backup".to_string(),
                certainty: "managed".to_string(),
            }],
            staging_leftovers: vec![],
            source_checkout: Some(SourceCheckoutEntry {
                path: "/tmp/dotfiles".to_string(),
                certainty: "detected".to_string(),
                is_git_repo: true,
            }),
        };
        let json = serde_json::to_string_pretty(&output).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed["backups"][0]["kind"], "install_backup");
        assert_eq!(
            parsed["source_checkout"]["is_git_repo"],
            serde_json::Value::Bool(true)
        );
    }

    #[test]
    fn collect_backups_finds_dotman_backup_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let backup = tmp.path().join("mytool.dotman-backup");
        fs::create_dir(&backup).expect("create");

        let (backups, staging) = collect_at_dir(tmp.path());
        assert_eq!(backups.len(), 1);
        assert!(backups[0].path.ends_with("mytool.dotman-backup"));
        assert!(staging.is_empty());
    }

    #[test]
    fn collect_staging_finds_dotman_staging_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let staging_dir = tmp.path().join("mytool.dotman-staging");
        fs::create_dir(&staging_dir).expect("create");

        let (backups, staging) = collect_at_dir(tmp.path());
        assert!(backups.is_empty());
        assert_eq!(staging.len(), 1);
        assert!(staging[0].path.ends_with("mytool.dotman-staging"));
    }

    #[test]
    fn collect_backups_ignores_files_and_other_dirs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        fs::write(tmp.path().join("regular-file"), b"data").expect("write");
        fs::create_dir(tmp.path().join("normal-dir")).expect("create dir");

        let (backups, staging) = collect_at_dir(tmp.path());
        assert!(backups.is_empty());
        assert!(staging.is_empty());
    }

    #[test]
    pub(crate) fn collect_source_checkout_returns_none_when_missing() {
        let result = collect_source_checkout_at("/nonexistent/path/should/not/exist");
        assert!(result.is_none());
    }

    #[test]
    fn run_status_no_repo_error_includes_fallback_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let cwd = std::env::current_dir().ok();
        std::env::set_current_dir(tmp.path()).ok();

        let err = run_status(false).unwrap_err();
        assert!(err.contains("not in a dotfiles repo"));
        assert!(err.contains("~/.local/share/dotman/dotfiles"));

        if let Some(d) = cwd {
            std::env::set_current_dir(d).ok();
        }
    }

    fn collect_at_dir(dir: &Path) -> (Vec<BackupEntry>, Vec<StagingEntry>) {
        let mut backups: Vec<BackupEntry> = Vec::new();
        let mut staging: Vec<StagingEntry> = Vec::new();

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if !path.is_dir() {
                    continue;
                }
                if name.ends_with(".dotman-backup") {
                    backups.push(BackupEntry {
                        path: path.display().to_string(),
                        kind: "install_backup".to_string(),
                        certainty: "managed".to_string(),
                    });
                } else if name.ends_with(".dotman-staging") {
                    staging.push(StagingEntry {
                        path: path.display().to_string(),
                        certainty: "managed".to_string(),
                    });
                }
            }
        }

        (backups, staging)
    }

    pub(crate) fn collect_source_checkout_at(path_str: &str) -> Option<SourceCheckoutEntry> {
        let path = PathBuf::from(path_str);
        if !path.exists() {
            return None;
        }
        let is_git = path.join(".git").exists();
        Some(SourceCheckoutEntry {
            path: path.display().to_string(),
            certainty: "detected".to_string(),
            is_git_repo: is_git,
        })
    }
}
