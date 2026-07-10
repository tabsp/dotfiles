//! Plan: pure function from config + filesystem state to Plan.
//!
//! Converts Config to Plan with layer grouping, auto-attachment of links
//! to install items, and smart defaults.

use crate::config::Config;
use crate::model::{Action, ActionStatus, Plan, PlanItem};
use crate::model::{HostInfo, Mode};
use crate::ops::shell;
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

/// Strategy for each layer.
pub fn layer_strategy(layer: &str) -> LayerStrategy {
    match layer {
        "terminal" | "shell" => LayerStrategy::PickOne,
        "multiplexer" | "software" | "enhancement" => LayerStrategy::All,
        _ => LayerStrategy::All,
    }
}

pub fn build(config: &Config, mode: Mode) -> Result<Plan> {
    let config_path = config.path.clone();
    let config_hash = hash_file(&config_path).unwrap_or_default();
    let id = Ulid::new().to_string();
    let tool_db = crate::ops::install::load_db()?;

    let mut items: Vec<PlanItem> = Vec::new();
    let mut used_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Step 0b: auto-install package manager (runs before install items)
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

    // Step 1: install items -> PlanItems, layer from tool db metadata.
    for tool in &config.install {
        let layer =
            crate::ops::install::tool_layer(&tool_db, tool).unwrap_or_else(|| "software".into());
        let id = unique_id(tool, &mut used_ids);
        items.push(PlanItem {
            id,
            name: tool.clone(),
            layer,
            actions: vec![Action::Install {
                pkg_mgr: "auto".into(),
                binary: tool.clone(),
                source: format!("install {tool}"),
            }],
            selected: false,
        });
    }

    // Step 2: links -> auto-attach to install or standalone
    for link in &config.links {
        let owner = find_owner(&link.target, &mut items);
        let action = Action::Link {
            target: link.target.clone(),
            source: link.source.clone(),
            backup: link.backup.unwrap_or(true),
            relink: link.relink.unwrap_or(false),
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
    if let Some(default_shell) = &config.default_shell {
        let name = format!("Set default shell to {default_shell}");
        let action = Action::Shell {
            command: default_shell_command(default_shell),
            description: Some(name.clone()),
            optional: false,
            if_condition: Some(default_shell_condition(default_shell)),
        };
        if let Some(item) = items.iter_mut().find(|item| item.name == *default_shell) {
            item.actions.push(action);
        } else {
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

    // Step 5: shell -> misc
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

    // Step 6: clean -> misc
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

    // Step 7: apply first-run smart defaults (selection)
    apply_smart_defaults(&mut items);

    // Step 7a: auto apt-get update when the resolved pkg mgr is apt AND
    // there are selected install items (not just shell/links).
    //
    // Docker images and stale machines often have out-of-date cache;
    // without this, `apt install` fails with "Unable to locate package".
    //
    // Uses the same resolution as the execution path (resolve + default
    // fallback) so we don't miss "apt" when no explicit pkg mgr is configured
    // but the distro defaults to apt.
    let pkg_mgr = resolve_pkg_mgr_name(&config.package_managers)
        .unwrap_or_else(crate::package_managers::default_pkg_mgr_name);
    if pkg_mgr == "apt" {
        let has_selected_install = items.iter().any(|item| {
            item.selected
                && item
                    .actions
                    .iter()
                    .any(|a| matches!(a, Action::Install { .. }))
        });
        if has_selected_install {
            let id = unique_id("auto-apt-update", &mut used_ids);
            items.insert(
                0,
                PlanItem {
                    id,
                    name: "auto apt update".into(),
                    layer: "misc".into(),
                    actions: vec![Action::Shell {
                        command: "sudo apt-get update -qq".into(),
                        description: Some("Update apt package cache".into()),
                        optional: false,
                        if_condition: None,
                    }],
                    selected: true,
                },
            );
        }
    }

    // Step 8: compute host info
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
        auto_install_pkg_manager: config.auto_install_pkg_manager,
    })
}

/// Find an owner PlanItem whose install name matches the target path.
fn find_owner<'a>(target: &Path, items: &'a mut [PlanItem]) -> Option<&'a mut PlanItem> {
    let target_str = target.to_string_lossy();
    for item in items.iter_mut() {
        if item.layer == "misc" {
            continue;
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

fn default_shell_condition(shell: &str) -> String {
    let shell = sh_single_quote(shell);
    format!(
        "shell_path=$(command -v {shell}) || exit 1; current_shell=$(getent passwd \"$USER\" 2>/dev/null | awk -F: '{{print $7}}'); if [ -z \"$current_shell\" ] && command -v dscl >/dev/null 2>&1; then current_shell=$(dscl . -read \"/Users/$USER\" UserShell 2>/dev/null | awk '{{print $2}}'); fi; [ \"$current_shell\" != \"$shell_path\" ]"
    )
}

fn default_shell_command(shell: &str) -> String {
    let shell = sh_single_quote(shell);
    format!(
        "shell_path=$(command -v {shell}) || exit 1; current_shell=$(getent passwd \"$USER\" 2>/dev/null | awk -F: '{{print $7}}'); if [ -z \"$current_shell\" ] && command -v dscl >/dev/null 2>&1; then current_shell=$(dscl . -read \"/Users/$USER\" UserShell 2>/dev/null | awk '{{print $2}}'); fi; if [ \"$current_shell\" = \"$shell_path\" ]; then echo \"default shell already $shell_path\"; exit 0; fi; if ! grep -Fxq \"$shell_path\" /etc/shells; then echo \"$shell_path\" | sudo tee -a /etc/shells >/dev/null; fi; sudo chsh -s \"$shell_path\" \"$USER\""
    )
}

fn sh_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn hash_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    Ok(format!("{digest:x}"))
}

/// Smart defaults for selection:
/// - PickOne layers: first item selected, rest unselected
/// - All layers: all selected
/// - Misc: all selected
fn apply_smart_defaults(items: &mut [PlanItem]) {
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

impl Plan {
    /// Returns `true` if any selected action in this plan may require sudo.
    ///
    /// Used to decide whether to pre-cache sudo credentials before execution.
    /// This is a static check — it does NOT evaluate shell if_condition guards.
    /// Covers:
    /// 1. Shell commands containing the word "sudo" (Linux installs, bootstrap).
    /// 2. Any selected Install action on Linux (db.toml commands all use sudo).
    pub fn needs_sudo(&self) -> bool {
        self.items.iter().filter(|item| item.selected).any(|item| {
            item.actions.iter().any(|action| match action {
                Action::Shell { command, .. } => shell::command_contains_sudo(command),
                Action::Install { .. } => {
                    crate::package_managers::detect_os() == crate::package_managers::Os::Linux
                }
                _ => false,
            })
        })
    }

    /// Sync auto-generated steps so they stay in lockstep with user selections.
    ///
    /// "auto apt update" only makes sense when there is at least one selected
    /// Install action. This is called after user selections are applied so that
    /// a saved selection that turned off all install items doesn't leave an
    /// orphaned apt update step that unexpectedly asks for sudo.
    pub fn sync_auto_steps(&mut self) {
        let has_selected_install = self.items.iter().any(|item| {
            item.selected
                && item
                    .actions
                    .iter()
                    .any(|a| matches!(a, Action::Install { .. }))
        });

        for item in &mut self.items {
            if item.name == "auto apt update" {
                item.selected = has_selected_install;
            }
        }
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
            default_shell: None,
            clean: vec![],
            auto_install_pkg_manager: false,
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
        assert_eq!(fish.actions.len(), 2);
        assert!(matches!(fish.actions[0], Action::Install { .. }));
        assert!(matches!(fish.actions[1], Action::Link { .. }));
    }

    #[test]
    fn plan_preserves_link_backup_and_relink_settings() {
        let cfg = Config {
            install: vec![],
            links: vec![
                LinkEntry {
                    target: PathBuf::from("/tmp/home/a"),
                    source: PathBuf::from("a"),
                    backup: Some(false),
                    relink: Some(true),
                },
                LinkEntry {
                    target: PathBuf::from("/tmp/home/b"),
                    source: PathBuf::from("b"),
                    backup: None,
                    relink: None,
                },
            ],
            ..sample_config()
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();

        let links = plan
            .items
            .iter()
            .flat_map(|item| item.actions.iter())
            .collect::<Vec<_>>();
        assert!(matches!(
            links.first(),
            Some(Action::Link {
                backup: false,
                relink: true,
                ..
            })
        ));
        assert!(matches!(
            links.get(1),
            Some(Action::Link {
                backup: true,
                relink: false,
                ..
            })
        ));
    }

    #[test]
    fn plan_attaches_default_shell_to_matching_install() {
        let cfg = Config {
            default_shell: Some("fish".into()),
            ..sample_config()
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let fish = plan.items.iter().find(|i| i.name == "fish").unwrap();
        assert!(fish.actions.iter().any(|a| matches!(
            a,
            Action::Shell {
                description,
                if_condition,
                ..
            } if description.as_deref() == Some("Set default shell to fish")
                && if_condition.is_some()
        )));
    }

    #[test]
    fn default_shell_command_resolves_shell_from_path() {
        let command = default_shell_command("fish");
        assert!(command.contains("command -v 'fish'"));
        assert!(command.contains("/etc/shells"));
        assert!(command.contains("sudo chsh -s \"$shell_path\" \"$USER\""));
    }

    #[test]
    fn plan_auto_attaches_prefix_match() {
        let cfg = sample_config();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let tmux = plan.items.iter().find(|i| i.name == "tmux").unwrap();
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
        let fish = plan.items.iter().find(|i| i.name == "fish").unwrap();
        assert!(fish.selected);
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
        assert!(shell.selected);
    }

    #[test]
    fn auto_install_pkg_manager_adds_shell_action_when_brew() {
        let mut cfg = sample_config();
        cfg.auto_install_pkg_manager = true;
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let pkg = plan
            .items
            .iter()
            .find(|i| i.name == "auto-install package manager");
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
        cfg.package_managers = PackageManagerConfig::default();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        let pkg = plan
            .items
            .iter()
            .find(|i| i.name == "auto-install package manager");
        assert!(pkg.is_none());
    }

    #[test]
    fn needs_sudo_false_when_brew_auto_install_no_sudo_in_cmd() {
        // auto_install_pkg_manager with brew does NOT force needs_sudo()
        // because the brew install script doesn't contain "sudo" and Install
        // actions on macOS don't require sudo.
        let mut cfg = sample_config();
        cfg.auto_install_pkg_manager = true;
        let plan = build(&cfg, Mode::Deploy).unwrap();
        if crate::package_managers::detect_os() == crate::package_managers::Os::Mac {
            // On macOS: brew script has no sudo, install actions don't need sudo
            assert!(!plan.needs_sudo());
        } else {
            // On Linux: Install actions imply sudo
            assert!(plan.needs_sudo());
        }
    }

    #[test]
    fn needs_sudo_true_for_shell_cmd_with_sudo() {
        let cfg = Config {
            shell: vec![ShellEntry {
                command: "sudo systemctl restart foo".into(),
                description: None,
                optional: false,
                if_condition: None,
            }],
            ..sample_config()
        };
        let plan = build(&cfg, Mode::Deploy).unwrap();
        assert!(plan.needs_sudo());
    }

    #[test]
    fn needs_sudo_false_when_no_sudo_anywhere() {
        let cfg = sample_config();
        let plan = build(&cfg, Mode::Deploy).unwrap();
        // No shell with sudo, no install actions on macOS.
        if crate::package_managers::detect_os() == crate::package_managers::Os::Mac {
            assert!(!plan.needs_sudo());
        } else {
            assert!(plan.needs_sudo()); // Linux install actions imply sudo
        }
    }
}
