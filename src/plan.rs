//! Plan: pure function from config + filesystem state to Plan.
//!
//! Phase 2: drift detection (B level), layer assignment, tool grouping.

use crate::config::Config;
use crate::model::{Action, ActionStatus, Plan, PlanItem};
use crate::model::{HostInfo, Mode};
use crate::package_managers::{detect_os, resolve_pkg_mgr_name};
use anyhow::Result;
use std::path::Path;
use ulid::Ulid;

/// Layer assignment + selection strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerStrategy {
    PickOne,
    All,
}

/// Returns the layer name for a given tool name.
///
/// "Misc" tools (link/create not paired to any tool) are not routed through here.
pub fn tool_layer(tool: &str) -> Option<&'static str> {
    match tool {
        // Layer 1: Terminal
        "ghostty" | "kitty" | "alacritty" | "wezterm" => Some("terminal"),

        // Layer 2: Shell
        "fish" | "zsh" | "nushell" | "bash" => Some("shell"),

        // Layer 3: Multiplexer
        "tmux" | "zellij" | "herdr" => Some("multiplexer"),

        // Layer 4: Software
        "neovim" | "nvim" | "lazygit" | "btop" | "fastfetch" | "yazi" | "dua" | "jq" | "yq" => {
            Some("software")
        }

        // Layer 5: Enhancement (incl. fonts, fzf, ripgrep, etc.)
        "ripgrep"
        | "fd"
        | "bat"
        | "eza"
        | "fzf"
        | "starship"
        | "tealdeer"
        | "tldr"
        | "font-maple-mono-nf-cn" => Some("enhancement"),

        _ => None,
    }
}

/// Strategy for each layer.
pub fn layer_strategy(layer: &str) -> LayerStrategy {
    match layer {
        "terminal" | "shell" | "multiplexer" => LayerStrategy::PickOne,
        "software" | "enhancement" => LayerStrategy::All,
        _ => LayerStrategy::All,
    }
}

pub fn build(config: &Config, mode: Mode) -> Result<Plan> {
    let config_path = config.path.clone();
    let config_hash = hash_file(&config_path).unwrap_or_default();
    let id = Ulid::new().to_string();

    // PlanItems keyed by step id (lowercase tool name with disambiguation).
    let mut items: Vec<PlanItem> = Vec::new();
    let mut used_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Step 0a: auto-clone repo (runs first)
    if let Some(repo) = &config.auto_clone_repo {
        let id = unique_id("auto-clone-repo", &mut used_ids);
        let name = repo
            .url
            .rsplit('/')
            .next()
            .unwrap_or(&repo.url)
            .trim_end_matches(".git")
            .to_string();
        let mut cmd = format!("git clone {}", repo.url);
        if let Some(branch) = &repo.branch {
            cmd.push_str(&format!(" --branch {branch}"));
        }
        // Expand leading `~/` to $HOME so the shell command works on all POSIX shells.
        let target_str = repo.target.to_string_lossy();
        let expanded = if let Some(rest) = target_str.strip_prefix("~/") {
            format!("$HOME/{rest}")
        } else {
            target_str.to_string()
        };
        cmd.push_str(&format!(" {expanded}"));
        let guard = format!("! test -d {expanded}");
        items.push(PlanItem {
            id,
            name: format!("clone {name}"),
            layer: "misc".into(),
            actions: vec![Action::Shell {
                command: cmd,
                description: Some("Auto-clone dotfiles repository".into()),
                optional: false,
                if_condition: Some(guard),
            }],
            selected: true,
        });
    }

    // Step 0b: auto-install package manager
    if config.auto_install_pkg_manager
        && let Some(install_cmd) = auto_pkg_mgr_install_cmd(&config.package_managers)
    {
        let id = unique_id("auto-pkg-mgr", &mut used_ids);
        items.push(PlanItem {
            id,
            name: "auto-install package manager".into(),
            layer: "misc".into(),
            actions: vec![Action::Shell {
                command: install_cmd.cmd,
                description: Some("Auto-install package manager".into()),
                optional: false,
                if_condition: Some(install_cmd.guard),
            }],
            selected: true,
        });
    }

    // Step 1: install items -> PlanItems, layer from tool_layer()
    for tool in &config.install {
        let layer = tool_layer(tool).unwrap_or("software").to_string();
        let id = unique_id(tool, &mut used_ids);
        items.push(PlanItem {
            id,
            name: tool.clone(),
            layer,
            actions: vec![Action::Install {
                pkg_mgr: "auto".into(),
                binary: tool.clone(),
                source: format!("install {tool}"), // overridden in Phase 3
            }],
            selected: false, // filled in by apply_defaults
        });
    }

    // Step 2: links -> auto-attach to install or standalone
    for link in &config.links {
        let owner = find_owner(&link.target, &mut items);
        let action = Action::Link {
            target: link.target.clone(),
            source: link.source.clone(),
        };
        if let Some(item) = owner {
            item.actions.push(action);
        } else {
            let name = link.target.display().to_string();
            let id = unique_id(&name, &mut used_ids);
            items.push(PlanItem {
                id,
                name,
                layer: "misc".into(),
                actions: vec![action],
                selected: false,
            });
        }
    }

    // Step 3: create -> auto-attach
    for target in &config.create {
        let owner = find_owner(target, &mut items);
        let action = Action::Create {
            target: target.clone(),
        };
        if let Some(item) = owner {
            item.actions.push(action);
        } else {
            let name = target.display().to_string();
            let id = unique_id(&name, &mut used_ids);
            items.push(PlanItem {
                id,
                name,
                layer: "misc".into(),
                actions: vec![action],
                selected: false,
            });
        }
    }

    // Step 4: shell -> misc
    for shell in &config.shell {
        let name = shell
            .description
            .clone()
            .unwrap_or_else(|| shell.command.clone());
        let id = unique_id(&name, &mut used_ids);
        items.push(PlanItem {
            id,
            name,
            layer: "misc".into(),
            actions: vec![Action::Shell {
                command: shell.command.clone(),
                description: shell.description.clone(),
                optional: shell.optional,
                if_condition: shell.if_condition.clone(),
            }],
            selected: false,
        });
    }

    // Step 5: clean -> misc
    for clean in &config.clean {
        let name = clean.target.display().to_string();
        let id = unique_id(&name, &mut used_ids);
        items.push(PlanItem {
            id,
            name,
            layer: "misc".into(),
            actions: vec![Action::Clean {
                target: clean.target.clone(),
                force: clean.force,
            }],
            selected: false,
        });
    }

    // Step 6: apply first-run smart defaults (selection)
    apply_smart_defaults(&mut items);

    // Step 7: compute host info
    let host = HostInfo {
        hostname: hostname(),
        os: format!("{:?}", detect_os()),
        arch: std::env::consts::ARCH.into(),
        user: std::env::var("USER").unwrap_or_default(),
        home: dirs::home_dir().unwrap_or_default(),
    };

    Ok(Plan {
        id,
        mode,
        created_at: now_iso(),
        config_path,
        config_hash,
        host,
        items,
    })
}

/// Find an owner PlanItem whose install name matches the target path.
/// Uses path component + prefix matching.
fn find_owner<'a>(target: &Path, items: &'a mut [PlanItem]) -> Option<&'a mut PlanItem> {
    // First, look for an install whose name is a path component of the target.
    let target_str = target.to_string_lossy();
    for item in items.iter_mut() {
        if item.layer == "misc" {
            continue; // only attach to install items
        }
        if let Some(Action::Install { binary, .. }) = item.actions.first() {
            for owner_name in owner_names(binary) {
                if target_str.contains(&format!("/{owner_name}/"))
                    || target_str.ends_with(&format!("/{owner_name}"))
                    || target_str.contains(&format!("/{owner_name}."))
                    || target_str.contains(&format!("/.{owner_name}."))
                    || target_str.ends_with(&format!("/.{owner_name}"))
                {
                    return Some(item);
                }
                // Prefix match: e.g. tmux-status matches tmux
                if let Some(stripped) = target_str
                    .rsplit('/')
                    .next()
                    .and_then(|base| base.strip_prefix(&format!("{owner_name}-")))
                    && !stripped.is_empty()
                {
                    return Some(item);
                }
            }
        }
    }
    None
}

fn owner_names(tool: &str) -> Vec<&str> {
    match tool {
        "neovim" => vec!["neovim", "nvim"],
        "ripgrep" => vec!["ripgrep", "rg"],
        "tealdeer" => vec!["tealdeer", "tldr"],
        "font-maple-mono-nf-cn" => vec!["font-maple-mono-nf-cn", "maple-mono"],
        _ => vec![tool],
    }
}

/// Check drift for a link target.
fn check_link_status(target: &Path, source: &Path) -> ActionStatus {
    if !target.exists() && !target.is_symlink() {
        return ActionStatus::WillLink;
    }
    if let Ok(actual) = std::fs::read_link(target) {
        if paths_match(&actual, target, source) {
            ActionStatus::NoChange
        } else {
            ActionStatus::WillBackupLink
        }
    } else {
        // target exists, not a symlink -> conflict
        ActionStatus::WillBackupLink
    }
}

fn paths_match(link_target: &Path, link_path: &Path, expected: &Path) -> bool {
    let abs = if link_target.is_absolute() {
        link_target.to_path_buf()
    } else {
        link_path
            .parent()
            .map(|p| p.join(link_target))
            .unwrap_or_else(|| link_target.to_path_buf())
    };
    if let (Ok(a), Ok(e)) = (std::fs::canonicalize(&abs), std::fs::canonicalize(expected)) {
        return a == e;
    }
    abs == expected
}

/// Smart defaults for selection:
/// - PickOne layers: first item selected, rest unselected
/// - All layers: all selected
/// - Misc: all selected
fn apply_smart_defaults(items: &mut [PlanItem]) {
    // For each pick-one layer, mark first as selected, rest unselected
    let mut seen_pick_one: std::collections::HashMap<String, bool> =
        std::collections::HashMap::new();
    for item in items.iter_mut() {
        let strategy = if item.layer == "misc" {
            LayerStrategy::All
        } else {
            layer_strategy(&item.layer)
        };
        match strategy {
            LayerStrategy::All => {
                item.selected = true;
            }
            LayerStrategy::PickOne => {
                let first = !seen_pick_one.contains_key(&item.layer);
                if first {
                    item.selected = true;
                    seen_pick_one.insert(item.layer.clone(), true);
                } else {
                    item.selected = false;
                }
            }
        }
    }
}

fn unique_id(name: &str, used: &mut std::collections::HashSet<String>) -> String {
    let base = sanitize_id(name);
    let mut id = base.clone();
    let mut n = 2;
    while used.contains(&id) {
        id = format!("{base}-{n}");
        n += 1;
    }
    used.insert(id.clone());
    id
}

fn sanitize_id(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn hash_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    // Lowercase hex without external crate.
    Ok(format!("{digest:x}"))
}

/// Map configured package manager name to an auto-install shell command and guard.
/// Returns `None` when there is no known auto-install procedure for this platform.
struct AutoInstallCmd {
    cmd: String,
    guard: String,
}

fn auto_pkg_mgr_install_cmd(
    pkg_mgrs: &crate::config::PackageManagerConfig,
) -> Option<AutoInstallCmd> {
    let pkg_mgr = resolve_pkg_mgr_name(pkg_mgrs)?;
    match pkg_mgr.as_str() {
        "brew" => Some(AutoInstallCmd {
            cmd: r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#.into(),
            guard: "! command -v brew".into(),
        }),
        _ => None,
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|s| s.trim().to_string()))
        .unwrap_or_default()
}

fn now_iso() -> String {
    time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::new())
}

// PartialOrd on ActionStatus so we can keep the "worst" status.
impl PartialOrd for ActionStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ActionStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn rank(s: &ActionStatus) -> u8 {
            match s {
                ActionStatus::NoChange => 0,
                ActionStatus::WillRun => 1,
                ActionStatus::WillSkip => 1,
                ActionStatus::WillLink => 2,
                ActionStatus::WillCreate => 2,
                ActionStatus::WillInstall => 2,
                ActionStatus::WillClean => 2,
                ActionStatus::WillBackupLink => 3,
                ActionStatus::WillBackupRemove => 3,
                ActionStatus::WillFail => 4,
            }
        }
        rank(self).cmp(&rank(other))
    }
}

impl PlanItem {
    /// Pick a representative status (currently the worst among actions).
    pub fn primary_status(&self) -> ActionStatus {
        self.actions
            .iter()
            .map(status_for_action)
            .max()
            .unwrap_or(ActionStatus::WillRun)
    }
}

fn status_for_action(a: &Action) -> ActionStatus {
    match a {
        Action::Install { .. } => ActionStatus::WillInstall,
        Action::Link { .. } => ActionStatus::WillLink,
        Action::Create { .. } => ActionStatus::WillCreate,
        Action::Shell { .. } => ActionStatus::WillRun,
        Action::Clean { .. } => ActionStatus::WillClean,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, LinkEntry, PackageManagerConfig, ShellEntry};
    use std::path::PathBuf;

    fn sample_config() -> Config {
        Config {
            path: PathBuf::from("/tmp/dotman.yaml"),
            package_managers: PackageManagerConfig {
                macos: Some("brew".into()),
                ..Default::default()
            },
            install: vec!["fish".into(), "tmux".into(), "nvim".into()],
            links: vec![
                LinkEntry {
                    target: PathBuf::from("/tmp/home/.config/fish"),
                    source: PathBuf::from("config/fish"),
                    backup: None,
                    relink: None,
                },
                LinkEntry {
                    target: PathBuf::from("/tmp/home/.local/bin/tmux-status"),
                    source: PathBuf::from("bin/tmux-status"),
                    backup: None,
                    relink: None,
                },
                LinkEntry {
                    target: PathBuf::from("/tmp/home/.tmux.conf"),
                    source: PathBuf::from("config/tmux.conf"),
                    backup: None,
                    relink: None,
                },
                LinkEntry {
                    target: PathBuf::from("/tmp/home/some/random"),
                    source: PathBuf::from("config/random"),
                    backup: None,
                    relink: None,
                },
                LinkEntry {
                    target: PathBuf::from("/tmp/home/.config/nvim"),
                    source: PathBuf::from("config/nvim"),
                    backup: None,
                    relink: None,
                },
            ],
            create: vec![],
            shell: vec![ShellEntry {
                command: "fisher update".into(),
                description: Some("Sync fish plugins".into()),
                optional: true,
                if_condition: None,
            }],
            clean: vec![],
            auto_install_pkg_manager: false,
            auto_clone_repo: None,
        }
    }

    #[test]
    fn plan_assigns_layers_correctly() {
        let cfg = sample_config();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let fish = plan.items.iter().find(|i| i.name == "fish").unwrap();
        assert_eq!(fish.layer, "shell");
        let tmux = plan.items.iter().find(|i| i.name == "tmux").unwrap();
        assert_eq!(tmux.layer, "multiplexer");
    }

    #[test]
    fn plan_auto_attaches_link_to_install() {
        let cfg = sample_config();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let fish = plan.items.iter().find(|i| i.name == "fish").unwrap();
        // fish should have install + link
        assert_eq!(fish.actions.len(), 2);
        assert!(matches!(fish.actions[0], Action::Install { .. }));
        assert!(matches!(fish.actions[1], Action::Link { .. }));
    }

    #[test]
    fn plan_auto_attaches_prefix_match() {
        let cfg = sample_config();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let tmux = plan.items.iter().find(|i| i.name == "tmux").unwrap();
        // tmux-status should attach to tmux
        assert_eq!(tmux.actions.len(), 3);
        assert!(tmux
            .actions
            .iter()
            .any(|a| matches!(a, Action::Link { target, .. } if target == &PathBuf::from("/tmp/home/.tmux.conf"))));
    }

    #[test]
    fn plan_auto_attaches_binary_alias_match() {
        let cfg = Config {
            install: vec!["neovim".into()],
            ..sample_config()
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let neovim = plan.items.iter().find(|i| i.name == "neovim").unwrap();
        assert!(neovim
            .actions
            .iter()
            .any(|a| matches!(a, Action::Link { target, .. } if target == &PathBuf::from("/tmp/home/.config/nvim"))));
    }

    #[test]
    fn plan_orphan_goes_to_misc() {
        let cfg = sample_config();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let random = plan
            .items
            .iter()
            .find(|i| i.name.contains("random"))
            .unwrap();
        assert_eq!(random.layer, "misc");
    }

    #[test]
    fn smart_default_pick_one_layer() {
        let cfg = sample_config();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        // terminal layer not present, so check shell
        let fish = plan.items.iter().find(|i| i.name == "fish").unwrap();
        assert!(fish.selected);
        // shell layer has only 1 item, so it's the first
    }

    #[test]
    fn shell_step_goes_to_misc() {
        let cfg = sample_config();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let shell = plan
            .items
            .iter()
            .find(|i| {
                i.layer == "misc" && i.actions.iter().any(|a| matches!(a, Action::Shell { .. }))
            })
            .unwrap();
        assert!(shell.selected); // misc default on
    }

    #[test]
    fn auto_clone_repo_adds_shell_action() {
        let mut cfg = sample_config();
        cfg.auto_clone_repo = Some(crate::config::CloneRepo {
            url: "https://github.com/user/dotfiles.git".into(),
            target: PathBuf::from("/tmp/test-dotfiles"),
            branch: None,
        });
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let clone = plan
            .items
            .iter()
            .find(|i| i.name.starts_with("clone "))
            .unwrap();
        assert!(clone.selected);
        assert!(matches!(
            clone.actions[0],
            Action::Shell { ref command, .. } if command.contains("git clone")
        ));
        assert_eq!(clone.layer, "misc");
    }

    #[test]
    fn auto_clone_repo_uses_branch_when_set() {
        let mut cfg = sample_config();
        cfg.auto_clone_repo = Some(crate::config::CloneRepo {
            url: "https://github.com/user/dotfiles.git".into(),
            target: PathBuf::from("/tmp/test-dotfiles"),
            branch: Some("main".into()),
        });
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let clone = plan
            .items
            .iter()
            .find(|i| i.name.starts_with("clone "))
            .unwrap();
        let cmd = match &clone.actions[0] {
            Action::Shell { command, .. } => command,
            _ => panic!("expected Shell action"),
        };
        assert!(cmd.contains(" --branch main"));
    }

    #[test]
    fn auto_install_pkg_manager_adds_shell_action_when_brew() {
        let mut cfg = sample_config();
        cfg.auto_install_pkg_manager = true;
        cfg.package_managers.macos = Some("brew".into());
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let pkg = plan
            .items
            .iter()
            .find(|i| i.name == "auto-install package manager");
        // On macOS brew is recognized; on Linux with no distro match it's None.
        if cfg!(target_os = "macos") {
            let pkg = pkg.expect("auto-install item should exist on macOS with brew configured");
            assert!(pkg.selected);
            assert!(matches!(
                pkg.actions[0],
                Action::Shell { ref if_condition, .. } if if_condition.as_deref() == Some("! command -v brew")
            ));
        }
    }

    #[test]
    fn auto_install_no_action_when_pkg_mgr_unknown() {
        let mut cfg = sample_config();
        cfg.auto_install_pkg_manager = true;
        cfg.package_managers = PackageManagerConfig::default(); // no pkg mgr configured
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let pkg = plan
            .items
            .iter()
            .find(|i| i.name == "auto-install package manager");
        // No recognized pkg mgr → no plan item added
        assert!(pkg.is_none());
    }
}
