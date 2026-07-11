use clap::Parser;
use dotman::cli::{Cli, Command, ProfileAction};
use dotman::config;
use dotman::init;
use dotman::model;
use dotman::plan;
use dotman::profile;
use dotman::*;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    init_tracing();

    let cli = Cli::parse();

    match &cli.command {
        None => run_tui_or_headless(&cli, Mode::Menu),
        Some(Command::Deploy) => run_tui_or_headless(&cli, Mode::Deploy),
        Some(Command::Plan) => run_tui_or_headless(&cli, Mode::Plan),
        Some(Command::Sync) => run_sync(&cli),
        Some(Command::Status) => run_status(),
        Some(Command::Doctor) => run_doctor(),
        Some(Command::Init {
            repo,
            branch,
            profile,
            path,
        }) => run_init(
            &cli,
            repo.as_deref(),
            branch.as_deref(),
            profile.as_deref(),
            path.as_deref(),
        ),
        Some(Command::Profile { action }) => run_profile(action),
        Some(Command::History) => run_tui_or_headless(&cli, Mode::History),
        Some(Command::Run { id }) => run_tui_or_headless(&cli, Mode::Run(id.clone())),
        Some(Command::NewLink { target, source }) => add_link(target, source),
    }
}

fn run_tui_or_headless(cli: &Cli, mode: Mode) -> Result<(), String> {
    // Menu in headless/non-TTY: show a brief status hint instead of deploying.
    if matches!(mode, Mode::Menu) && !is_tty() {
        eprintln!("headless mode requires an explicit subcommand (deploy, plan, status, etc.)");
        eprintln!("run `dotman --help` for available commands");
        return Ok(());
    }

    let needs_config = matches!(mode, Mode::Deploy | Mode::Plan | Mode::Menu);

    if needs_config {
        let config_path = init::resolve_and_sync(cli)?;
        if cli.headless {
            headless::run_with_mode(config_path, mode)
        } else if is_tty() {
            tui::run_with_config(config_path, mode)
        } else {
            eprintln!("warning: not a TTY, falling back to headless mode");
            headless::run_with_mode(config_path, mode)
        }
    } else {
        if cli.headless {
            headless::run_no_config(mode)
        } else if is_tty() {
            tui::run_no_config(mode)
        } else {
            eprintln!("warning: not a TTY, falling back to headless mode");
            headless::run_no_config(mode)
        }
    }
}

fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("dotman=info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

// ---- Command implementations ----

fn run_sync(cli: &Cli) -> Result<(), String> {
    let source = init::resolve_config(cli)?;
    match source {
        init::ConfigSource::Profile(config_path) => {
            init::sync_profile()?;
            println!("sync complete ({})", config_path.display());
            Ok(())
        }
        init::ConfigSource::Direct(config_path) => {
            println!(
                "config loaded directly from {} — no profile to sync",
                config_path.display()
            );
            Ok(())
        }
    }
}

fn run_status() -> Result<(), String> {
    init::show_status()
}

fn run_doctor() -> Result<(), String> {
    let diagnostics = init::run_doctor();
    let mut all_ok = true;
    for d in &diagnostics {
        let icon = if d.ok { "✓" } else { "✗" };
        println!(" {icon} {}", d.message);
        if !d.ok {
            all_ok = false;
        }
    }
    println!();
    if all_ok {
        println!("All checks passed.");
        Ok(())
    } else {
        Err("some checks failed — see above".into())
    }
}

fn run_init(
    cli: &Cli,
    repo: Option<&str>,
    branch: Option<&str>,
    profile_name: Option<&str>,
    path: Option<&std::path::Path>,
) -> Result<(), String> {
    let repo = repo.unwrap_or(profile::DEFAULT_REPO);
    let branch = branch.unwrap_or(profile::DEFAULT_BRANCH);
    let profile_name = profile_name.unwrap_or(profile::DEFAULT_PROFILE_NAME);
    let mut profile_cfg = profile::load().map_err(|e| e.to_string())?;
    let path_str = profile::resolve_checkout_path(path, profile_cfg.as_ref(), profile_name);

    init::ensure_git(cli).map_err(|e| e.to_string())?;
    let checkout =
        init::clone_or_reuse_checkout(repo, branch, &path_str).map_err(|e| e.to_string())?;

    let config_path = checkout.join(profile::DEFAULT_CONFIG_FILE);
    init::validate_config(&config_path, cli.headless).map_err(|e| e.to_string())?;
    let cfg = config::load(&config_path)
        .map_err(|e| format!("failed to parse {}: {e}", config_path.display()))?;
    let plan = plan::build(&cfg, model::Mode::Deploy).map_err(|e| e.to_string())?;

    // Only save profile config AFTER clone + config validation both succeed.
    let mut profile_cfg = profile_cfg.take().unwrap_or_else(profile::default_config);
    profile_cfg.default_profile = profile_name.to_string();
    profile_cfg.profiles.insert(
        profile_name.to_string(),
        profile::Profile {
            repo: repo.to_string(),
            branch: branch.to_string(),
            path: path_str,
            config: profile::DEFAULT_CONFIG_FILE.to_string(),
            auto_sync: true,
        },
    );
    profile::save(&profile_cfg).map_err(|e| format!("failed to save profile config: {e}"))?;

    let items = plan.items.len();
    let selected = plan.items.iter().filter(|i| i.selected).count();
    println!("dotman initialized: profile '{profile_name}' -> {repo}");
    println!("Config: {}", config_path.display());
    println!("plan: {selected}/{items} steps ready to deploy");
    Ok(())
}

fn run_profile(action: &ProfileAction) -> Result<(), String> {
    match action {
        ProfileAction::List => {
            let cfg = profile::load().map_err(|e| e.to_string())?;
            match cfg {
                Some(cfg) => {
                    println!("default profile: {}", cfg.default_profile);
                    println!();
                    for (name, p) in &cfg.profiles {
                        let marker = if *name == cfg.default_profile {
                            " (active)"
                        } else {
                            ""
                        };
                        println!("  {name}{marker}");
                        println!("    repo:   {}", p.repo);
                        println!("    branch: {}", p.branch);
                        println!("    path:   {}", p.path);
                    }
                }
                None => {
                    println!("No profiles configured.");
                    println!("Run `dotman init` to set up your dotfiles.");
                }
            }
            Ok(())
        }
        ProfileAction::Add { name, repo } => {
            let mut cfg = profile::load()
                .map_err(|e| e.to_string())?
                .unwrap_or_else(profile::default_config);
            if cfg.profiles.contains_key(name) {
                return Err(format!("profile '{name}' already exists"));
            }
            let path = profile::resolve_checkout_path(None, Some(&cfg), name);
            cfg.profiles.insert(
                name.clone(),
                profile::Profile {
                    repo: repo.clone(),
                    branch: profile::DEFAULT_BRANCH.to_string(),
                    path,
                    config: profile::DEFAULT_CONFIG_FILE.to_string(),
                    auto_sync: true,
                },
            );
            profile::save(&cfg).map_err(|e| format!("failed to save profile: {e}"))?;
            println!("profile '{name}' added: {repo}");
            Ok(())
        }
        ProfileAction::Remove { name } => {
            let mut cfg = profile::load()
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "no profiles configured".to_string())?;
            if !cfg.profiles.contains_key(name) {
                return Err(format!("profile '{name}' not found"));
            }
            cfg.profiles.remove(name);
            if cfg.default_profile == *name {
                cfg.default_profile = cfg
                    .profiles
                    .keys()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| profile::DEFAULT_PROFILE_NAME.to_string());
            }
            profile::save(&cfg).map_err(|e| format!("failed to save profile: {e}"))?;
            println!(
                "profile '{name}' removed (re-run `dotman init` with a different profile to re-clone)"
            );
            Ok(())
        }
    }
}

fn add_link(target: &str, source: &str) -> Result<(), String> {
    let path = std::path::Path::new("dotman.yaml");
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let updated = insert_link_entry(&raw, target, source)?;
    std::fs::write(path, updated)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    println!("added link: {target} -> {source}");
    Ok(())
}

fn insert_link_entry(raw: &str, target: &str, source: &str) -> Result<String, String> {
    use serde_yaml::{Mapping, Value};

    let mut doc: Value =
        serde_yaml::from_str(raw).map_err(|e| format!("failed to parse dotman.yaml: {e}"))?;

    let mapping = doc
        .as_mapping_mut()
        .ok_or_else(|| "dotman.yaml is not a mapping".to_string())?;

    let target_val = Value::String(target.to_string());
    let source_val = Value::String(source.to_string());

    match mapping.get_mut("links") {
        Some(Value::Mapping(links)) => {
            links.insert(target_val, source_val);
        }
        Some(Value::Sequence(seq)) => {
            // Convert sequence entries into a map, preserving simple {target, source} items.
            let mut new_links = Mapping::new();
            for item in seq.iter() {
                let item_map = item.as_mapping().ok_or_else(|| {
                    "links: list item is not a mapping; cannot convert to map form".to_string()
                })?;
                let item_target = item_map
                    .get("target")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "links: list item missing 'target' field".to_string())?;
                let item_source = item_map
                    .get("source")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "links: list item missing 'source' field".to_string())?;
                // Reject entries with extra fields that can't round-trip through map format.
                let extra_keys: Vec<&str> = item_map
                    .keys()
                    .filter_map(|k| k.as_str())
                    .filter(|k| *k != "target" && *k != "source")
                    .collect();
                if !extra_keys.is_empty() {
                    return Err(format!(
                        "links: list item '{}' has extra fields ({}) that cannot be preserved \
                         in map format; edit dotman.yaml manually to add this link",
                        item_target,
                        extra_keys.join(", ")
                    ));
                }
                new_links.insert(
                    Value::String(item_target.to_string()),
                    Value::String(item_source.to_string()),
                );
            }
            new_links.insert(target_val, source_val);
            mapping.insert(
                Value::String("links".to_string()),
                Value::Mapping(new_links),
            );
        }
        Some(Value::Null) | None => {
            let mut new_links = Mapping::new();
            new_links.insert(target_val, source_val);
            mapping.insert(
                Value::String("links".to_string()),
                Value::Mapping(new_links),
            );
        }
        Some(_) => {
            return Err("links: section has unexpected format".into());
        }
    }

    serde_yaml::to_string(&doc).map_err(|e| format!("failed to serialize dotman.yaml: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_link_entry_adds_to_links_section() {
        let raw = "install: []\nlinks:\n  ~/.config/fish: config/fish\ncreate: []\n";
        let updated = insert_link_entry(raw, "~/.tmux.conf", "config/tmux.conf").unwrap();
        // Verify round-trip: parsed YAML has both links.
        let doc: serde_yaml::Value = serde_yaml::from_str(&updated).unwrap();
        let links = doc["links"].as_mapping().unwrap();
        assert_eq!(
            links.get("~/.config/fish").unwrap().as_str().unwrap(),
            "config/fish"
        );
        assert_eq!(
            links.get("~/.tmux.conf").unwrap().as_str().unwrap(),
            "config/tmux.conf"
        );
    }

    #[test]
    fn insert_link_entry_replaces_existing_target() {
        let raw = "links:\n  ~/.tmux.conf: old\n";
        let updated = insert_link_entry(raw, "~/.tmux.conf", "config/tmux.conf").unwrap();
        let doc: serde_yaml::Value = serde_yaml::from_str(&updated).unwrap();
        let links = doc["links"].as_mapping().unwrap();
        assert_eq!(
            links.get("~/.tmux.conf").unwrap().as_str().unwrap(),
            "config/tmux.conf"
        );
    }

    #[test]
    fn insert_link_entry_handles_sequence_links_format() {
        // Links as a sequence (list form): convert to map form, preserving all entries.
        let raw = "install: []\nlinks:\n  - target: ~/.config/fish\n    source: config/fish\n";
        let updated = insert_link_entry(raw, "~/.tmux.conf", "config/tmux.conf").unwrap();
        let doc: serde_yaml::Value = serde_yaml::from_str(&updated).unwrap();
        let links = doc["links"].as_mapping().unwrap();
        assert_eq!(
            links.get("~/.config/fish").unwrap().as_str().unwrap(),
            "config/fish"
        );
        assert_eq!(
            links.get("~/.tmux.conf").unwrap().as_str().unwrap(),
            "config/tmux.conf"
        );
    }

    #[test]
    fn insert_link_entry_rejects_backup_field_in_sequence() {
        // List entry with backup field cannot be represented in map form → error.
        let raw = "links:\n  - target: ~/.config/fish\n    source: config/fish\n    backup: true\n";
        let result = insert_link_entry(raw, "~/.tmux.conf", "config/tmux.conf");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("backup"));
    }
}
