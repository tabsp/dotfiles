use crate::config::{self, Installer};
use crate::path::expand_home;
use crate::path::paths_match;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::status;

#[derive(Debug, Serialize, Clone)]
struct DiffOutput {
    installed_tools: Vec<ToolDiffEntry>,
    linked_dotfiles: Vec<DotfileDiffEntry>,
    backups: Vec<StaleEntry>,
    staging_leftovers: Vec<StaleEntry>,
    source_checkout: Option<SourceCheckoutDiffEntry>,
    summary: DiffSummary,
}

#[derive(Debug, Serialize, Clone)]
struct ToolDiffEntry {
    name: String,
    path: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_version: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct DotfileDiffEntry {
    name: String,
    path: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual_target: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct StaleEntry {
    path: String,
    status: String,
}

#[derive(Debug, Serialize, Clone)]
struct SourceCheckoutDiffEntry {
    path: String,
    status: String,
}

#[derive(Debug, Serialize, Default, Clone)]
struct DiffSummary {
    ok: usize,
    missing: usize,
    drifted: usize,
    stale: usize,
    wrong_target: usize,
    version_unknown: usize,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    narrow: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    reconcile_commands: Vec<String>,
}

pub fn run_diff(json: bool, narrow: bool, reconcile: bool) -> Result<(), String> {
    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;

    if !repo.join("deps.toml").exists() || !repo.join("dotfiles.toml").exists() {
        return Err(
            "not in a dotfiles repo. The release installer clones the repo to \
             ~/.local/share/dotman/dotfiles — run dotman diff from there."
                .to_string(),
        );
    }

    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = crate::platform::detect_host()?;

    let tool_diffs = diff_tools(&deps.deps, &host);
    let dotfile_diffs = diff_dotfiles(&files, &repo);
    let (backups, staging_leftovers) = status::collect_backups_and_staging()?;
    let source_checkout = status::collect_source_checkout();

    let backup_entries: Vec<StaleEntry> = backups
        .into_iter()
        .map(|b| StaleEntry {
            path: b.path,
            status: "stale".to_string(),
        })
        .collect();

    let staging_entries: Vec<StaleEntry> = staging_leftovers
        .into_iter()
        .map(|s| StaleEntry {
            path: s.path,
            status: "stale".to_string(),
        })
        .collect();

    let source_entry = source_checkout.map(|c| SourceCheckoutDiffEntry {
        path: c.path,
        status: if c.is_git_repo {
            "ok".to_string()
        } else {
            "not_git".to_string()
        },
    });

    let mut summary = DiffSummary {
        narrow,
        ..Default::default()
    };
    for t in &tool_diffs {
        match t.status.as_str() {
            "ok" => summary.ok += 1,
            "missing" => summary.missing += 1,
            "drifted" => summary.drifted += 1,
            "version_unknown" => summary.version_unknown += 1,
            _ => {}
        }
    }
    for d in &dotfile_diffs {
        match d.status.as_str() {
            "ok" => summary.ok += 1,
            "missing" => summary.missing += 1,
            "wrong_target" => summary.wrong_target += 1,
            _ => {}
        }
    }
    summary.stale = backup_entries.len() + staging_entries.len();

    let has_issues = summary.missing > 0
        || summary.drifted > 0
        || summary.stale > 0
        || summary.wrong_target > 0
        || summary.version_unknown > 0;

    // Generate reconcile commands if requested
    let reconcile_cmds = if reconcile {
        let temp = DiffOutput {
            installed_tools: tool_diffs.clone(),
            linked_dotfiles: dotfile_diffs.clone(),
            backups: backup_entries.clone(),
            staging_leftovers: staging_entries.clone(),
            source_checkout: source_entry.clone(),
            summary: DiffSummary::default(),
        };
        generate_reconcile_commands(&temp)
    } else {
        Vec::new()
    };
    summary.reconcile_commands = reconcile_cmds.clone();

    if json {
        let output = DiffOutput {
            installed_tools: tool_diffs,
            linked_dotfiles: dotfile_diffs,
            backups: backup_entries,
            staging_leftovers: staging_entries,
            source_checkout: source_entry,
            summary,
        };
        let json_str =
            serde_json::to_string_pretty(&output).map_err(|e| format!("JSON error: {e}"))?;
        println!("{json_str}");
    } else {
        print_human_diff(
            &tool_diffs,
            &dotfile_diffs,
            &backup_entries,
            &staging_entries,
            &source_entry,
            &summary,
            narrow,
        );
    }

    // Print reconcile commands if requested
    if reconcile && !json {
        println!();
        println!("==> Reconcile commands (advisory — review before running)");
        if reconcile_cmds.is_empty() {
            println!("  Nothing to reconcile.");
        } else {
            for cmd in &reconcile_cmds {
                println!("  # {}", cmd);
            }
        }
    }

    if has_issues {
        std::process::exit(1);
    }

    Ok(())
}

fn diff_tools(
    deps: &BTreeMap<String, crate::config::Dependency>,
    host: &crate::platform::Host,
) -> Vec<ToolDiffEntry> {
    let mut results: Vec<ToolDiffEntry> = Vec::new();

    for (name, dep) in deps {
        let entries = dep.entries_for_host(host);
        for entry in &entries {
            match entry.installer {
                Installer::DownloadBinary | Installer::OfficialScript => {
                    if let Some(diff) = diff_one_tool(name, dep, entry) {
                        results.push(diff);
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

fn param_str(entry: &crate::config::InstallEntry, key: &str) -> Option<String> {
    entry.params.get(key)?.as_str().map(|s| s.to_string())
}

fn diff_one_tool(
    name: &str,
    dep: &crate::config::Dependency,
    entry: &crate::config::InstallEntry,
) -> Option<ToolDiffEntry> {
    let install_to = param_str(entry, "install_to")?;
    let install_to_path = expand_home(&install_to).ok()?;

    if !install_to_path.exists() {
        return Some(ToolDiffEntry {
            name: name.to_string(),
            path: install_to_path.display().to_string(),
            status: "missing".to_string(),
            installed_version: None,
            expected_version: None,
        });
    }

    let expected_version = if entry.version != "latest" && !entry.version.is_empty() {
        Some(entry.version.clone())
    } else {
        None
    };

    // Check version if version_check is configured
    if let Some(check) = &dep.version_check {
        let command_path =
            crate::path::which(&dep.command).unwrap_or_else(|| install_to_path.clone());

        match crate::doctor::read_version(&command_path, check) {
            Ok(installed) => {
                if let Some(ref expected) = expected_version
                    && installed != *expected
                {
                    return Some(ToolDiffEntry {
                        name: name.to_string(),
                        path: install_to_path.display().to_string(),
                        status: "drifted".to_string(),
                        installed_version: Some(installed),
                        expected_version: Some(expected.clone()),
                    });
                }
            }
            Err(_) => {
                return Some(ToolDiffEntry {
                    name: name.to_string(),
                    path: install_to_path.display().to_string(),
                    status: "version_unknown".to_string(),
                    installed_version: None,
                    expected_version: expected_version.clone(),
                });
            }
        }
    }

    Some(ToolDiffEntry {
        name: name.to_string(),
        path: install_to_path.display().to_string(),
        status: "ok".to_string(),
        installed_version: expected_version.clone(),
        expected_version,
    })
}

fn diff_dotfiles(files: &crate::config::DotfilesManifest, repo: &Path) -> Vec<DotfileDiffEntry> {
    let mut results: Vec<DotfileDiffEntry> = Vec::new();

    for file in &files.files {
        if !file.enabled {
            continue;
        }
        let target = match expand_home(&file.target) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let expected = repo.join(&file.source);
        let name = file
            .source
            .strip_prefix("config/")
            .unwrap_or(&file.source)
            .to_string();

        match fs::read_link(&target) {
            Ok(actual) if paths_match(&actual, &expected) => {
                results.push(DotfileDiffEntry {
                    name,
                    path: target.display().to_string(),
                    status: "ok".to_string(),
                    actual_target: None,
                });
            }
            Ok(actual) => {
                results.push(DotfileDiffEntry {
                    name,
                    path: target.display().to_string(),
                    status: "wrong_target".to_string(),
                    actual_target: Some(actual.display().to_string()),
                });
            }
            Err(_) => {
                results.push(DotfileDiffEntry {
                    name,
                    path: target.display().to_string(),
                    status: "missing".to_string(),
                    actual_target: None,
                });
            }
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

fn print_human_diff(
    tools: &[ToolDiffEntry],
    dotfiles: &[DotfileDiffEntry],
    backup_entries: &[StaleEntry],
    staging_entries: &[StaleEntry],
    source_entry: &Option<SourceCheckoutDiffEntry>,
    summary: &DiffSummary,
    narrow: bool,
) {
    println!("==> Installed tools");
    let display_tools: Vec<&ToolDiffEntry> = if narrow {
        tools.iter().filter(|t| t.status != "ok").collect()
    } else {
        tools.iter().collect()
    };
    if display_tools.is_empty() {
        println!("  (none)");
    } else {
        for t in &display_tools {
            match t.status.as_str() {
                "drifted" => {
                    println!(
                        "  {:<12} {:<30} drifted (installed {}, expected {})",
                        t.name,
                        t.path,
                        t.installed_version.as_deref().unwrap_or("?"),
                        t.expected_version.as_deref().unwrap_or("?")
                    );
                }
                "version_unknown" => {
                    println!("  {:<12} {:<30} version_unknown", t.name, t.path);
                }
                _ => {
                    println!("  {:<12} {:<30} {}", t.name, t.path, t.status);
                }
            }
        }
    }

    println!();
    println!("==> Linked dotfiles");
    let display_dotfiles: Vec<&DotfileDiffEntry> = if narrow {
        dotfiles.iter().filter(|d| d.status != "ok").collect()
    } else {
        dotfiles.iter().collect()
    };
    if display_dotfiles.is_empty() {
        println!("  (none)");
    } else {
        for d in &display_dotfiles {
            match d.status.as_str() {
                "wrong_target" => {
                    println!(
                        "  {:<12} {:<30} wrong_target (-> {})",
                        d.name,
                        d.path,
                        d.actual_target.as_deref().unwrap_or("?")
                    );
                }
                _ => {
                    println!("  {:<12} {:<30} {}", d.name, d.path, d.status);
                }
            }
        }
    }

    println!();
    println!("==> Backups");
    if backup_entries.is_empty() {
        println!("  (none)");
    } else {
        for b in backup_entries {
            println!("  {:<40} {}", b.path, b.status);
        }
    }

    println!();
    println!("==> Staging leftovers");
    if staging_entries.is_empty() {
        println!("  (none)");
    } else {
        for s in staging_entries {
            println!("  {:<40} {}", s.path, s.status);
        }
    }

    println!();
    println!("==> Source checkout");
    if let Some(c) = source_entry {
        println!("  {}       {}", c.path, c.status);
    } else {
        println!("  (none)");
    }

    println!();
    println!(
        "{} ok, {} missing, {} drifted, {} wrong_target, {} version_unknown, {} stale",
        summary.ok,
        summary.missing,
        summary.drifted,
        summary.wrong_target,
        summary.version_unknown,
        summary.stale
    );
}

fn generate_reconcile_commands(output: &DiffOutput) -> Vec<String> {
    let mut cmds: Vec<String> = Vec::new();

    let has_actionable = output
        .installed_tools
        .iter()
        .any(|t| t.status == "missing" || t.status == "drifted");
    if has_actionable {
        cmds.push("dotman bootstrap".to_string());
    }

    let dotfile_names: Vec<&str> = output
        .linked_dotfiles
        .iter()
        .filter(|d| d.status == "missing" || d.status == "wrong_target")
        .map(|d| d.name.as_str())
        .collect();
    if !dotfile_names.is_empty() {
        cmds.push(format!("dotman link --force {}", dotfile_names.join(" ")));
    }

    let has_stale = !output.backups.is_empty() || !output.staging_leftovers.is_empty();
    if has_stale {
        cmds.push("dotman cleanup".to_string());
    }

    cmds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_output_json_serializes() {
        let output = DiffOutput {
            installed_tools: vec![ToolDiffEntry {
                name: "bat".to_string(),
                path: "/tmp/bat".to_string(),
                status: "ok".to_string(),
                installed_version: None,
                expected_version: None,
            }],
            linked_dotfiles: vec![],
            backups: vec![],
            staging_leftovers: vec![],
            source_checkout: None,
            summary: DiffSummary {
                ok: 1,
                ..Default::default()
            },
        };
        let json = serde_json::to_string_pretty(&output).expect("serialize");
        assert!(json.contains("bat"));
        assert!(json.contains("\"ok\""));
    }

    #[test]
    fn diff_output_drifted_includes_versions() {
        let output = DiffOutput {
            installed_tools: vec![ToolDiffEntry {
                name: "delta".to_string(),
                path: "/tmp/delta".to_string(),
                status: "drifted".to_string(),
                installed_version: Some("0.18.2".to_string()),
                expected_version: Some("0.19.0".to_string()),
            }],
            linked_dotfiles: vec![],
            backups: vec![],
            staging_leftovers: vec![],
            source_checkout: None,
            summary: DiffSummary {
                drifted: 1,
                ..Default::default()
            },
        };
        let json = serde_json::to_string_pretty(&output).expect("serialize");
        assert!(json.contains("installed_version"));
        assert!(json.contains("expected_version"));
        assert!(json.contains("0.18.2"));
    }

    #[test]
    fn wrong_target_includes_actual_target() {
        let entry = DotfileDiffEntry {
            name: "nvim".to_string(),
            path: "/tmp/nvim".to_string(),
            status: "wrong_target".to_string(),
            actual_target: Some("/wrong/path".to_string()),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains("actual_target"));
        assert!(json.contains("/wrong/path"));
    }

    #[test]
    fn missing_tool_has_no_versions() {
        let entry = ToolDiffEntry {
            name: "zoxide".to_string(),
            path: "/tmp/zoxide".to_string(),
            status: "missing".to_string(),
            installed_version: None,
            expected_version: None,
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(!json.contains("installed_version"));
        assert!(!json.contains("expected_version"));
    }

    #[test]
    fn version_unknown_tool_has_expected_version() {
        let entry = ToolDiffEntry {
            name: "tool".to_string(),
            path: "/tmp/tool".to_string(),
            status: "version_unknown".to_string(),
            installed_version: None,
            expected_version: Some("1.0.0".to_string()),
        };
        let json = serde_json::to_string(&entry).expect("serialize");
        assert!(json.contains("version_unknown"));
    }

    #[test]
    fn diff_dotfiles_detects_missing() {
        use std::fs;
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = tmp.path();
        fs::create_dir_all(repo.join("config")).expect("create config");
        fs::write(repo.join("deps.toml"), "").expect("write deps");

        let dotfiles_toml = r#"
[[files]]
source = "config/nvim"
target = "~/.nonexistent-dotman-test-dir/nvim"
"#;
        fs::write(repo.join("dotfiles.toml"), dotfiles_toml).expect("write");

        let files = config::load_dotfiles(Path::new(&repo.join("dotfiles.toml"))).expect("load");
        let results = diff_dotfiles(&files, repo);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "missing");
    }

    #[test]
    fn reconcile_generates_bootstrap_for_missing_tools() {
        let output = DiffOutput {
            installed_tools: vec![ToolDiffEntry {
                name: "zoxide".to_string(),
                path: "/tmp/zoxide".to_string(),
                status: "missing".to_string(),
                installed_version: None,
                expected_version: Some("0.1.0".to_string()),
            }],
            linked_dotfiles: vec![],
            backups: vec![],
            staging_leftovers: vec![],
            source_checkout: None,
            summary: DiffSummary::default(),
        };
        let cmds = generate_reconcile_commands(&output);
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].contains("bootstrap"));
    }

    #[test]
    fn reconcile_generates_link_force_for_missing_dotfiles() {
        let output = DiffOutput {
            installed_tools: vec![],
            linked_dotfiles: vec![
                DotfileDiffEntry {
                    name: "nvim".to_string(),
                    path: "~/.config/nvim".to_string(),
                    status: "missing".to_string(),
                    actual_target: None,
                },
                DotfileDiffEntry {
                    name: "fish".to_string(),
                    path: "~/.config/fish".to_string(),
                    status: "wrong_target".to_string(),
                    actual_target: Some("/wrong/path".to_string()),
                },
            ],
            backups: vec![],
            staging_leftovers: vec![],
            source_checkout: None,
            summary: DiffSummary::default(),
        };
        let cmds = generate_reconcile_commands(&output);
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].contains("link --force"));
        assert!(cmds[0].contains("nvim"));
        assert!(cmds[0].contains("fish"));
    }

    #[test]
    fn reconcile_empty_when_all_ok() {
        let output = DiffOutput {
            installed_tools: vec![ToolDiffEntry {
                name: "bat".to_string(),
                path: "/tmp/bat".to_string(),
                status: "ok".to_string(),
                installed_version: None,
                expected_version: None,
            }],
            linked_dotfiles: vec![],
            backups: vec![],
            staging_leftovers: vec![],
            source_checkout: None,
            summary: DiffSummary::default(),
        };
        let cmds = generate_reconcile_commands(&output);
        assert!(cmds.is_empty());
    }

    #[test]
    fn reconcile_includes_cleanup_for_stale() {
        let output = DiffOutput {
            installed_tools: vec![],
            linked_dotfiles: vec![],
            backups: vec![StaleEntry {
                path: "/tmp/bat.dotman-backup".to_string(),
                status: "stale".to_string(),
            }],
            staging_leftovers: vec![],
            source_checkout: None,
            summary: DiffSummary::default(),
        };
        let cmds = generate_reconcile_commands(&output);
        assert_eq!(cmds.len(), 1);
        assert!(cmds[0].contains("cleanup"));
    }

    #[test]
    fn reconcile_batches_multiple_categories() {
        let output = DiffOutput {
            installed_tools: vec![ToolDiffEntry {
                name: "fd".to_string(),
                path: "/tmp/fd".to_string(),
                status: "drifted".to_string(),
                installed_version: Some("1.0".to_string()),
                expected_version: Some("2.0".to_string()),
            }],
            linked_dotfiles: vec![DotfileDiffEntry {
                name: "wezterm".to_string(),
                path: "~/.config/wezterm".to_string(),
                status: "missing".to_string(),
                actual_target: None,
            }],
            backups: vec![StaleEntry {
                path: "/tmp/old.dotman-backup".to_string(),
                status: "stale".to_string(),
            }],
            staging_leftovers: vec![],
            source_checkout: None,
            summary: DiffSummary::default(),
        };
        let cmds = generate_reconcile_commands(&output);
        assert_eq!(cmds.len(), 3);
        assert!(cmds.iter().any(|c| c.contains("bootstrap")));
        assert!(cmds.iter().any(|c| c.contains("link --force")));
        assert!(cmds.iter().any(|c| c.contains("cleanup")));
    }
}
