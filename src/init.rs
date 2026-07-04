//! Init orchestration: auto-init, config resolution, git bootstrap, repo clone.
//!
//! Shared logic used by both TUI and headless modes. The flow:
//!
//! 1. resolve_config() -> finds or initializes a profile, returns ConfigSource
//! 2. sync_profile() -> git pull on the active profile checkout
//! 3. resolve_and_sync() -> both steps combined, only syncs for profile sources

use crate::bootstrap;
use crate::cli;
use crate::profile;
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Where the config was resolved from. Determines whether a repo sync is needed.
#[derive(Debug, Clone)]
pub enum ConfigSource {
    /// Config loaded directly (--config flag or cwd dotman.yaml). No profile sync needed.
    Direct(PathBuf),
    /// Config loaded from a profile. Sync (git pull) is expected before deploy.
    Profile(PathBuf),
}

impl ConfigSource {
    /// Return the filesystem path to dotman.yaml.
    pub fn path(&self) -> PathBuf {
        match self {
            ConfigSource::Direct(p) | ConfigSource::Profile(p) => p.clone(),
        }
    }
}

/// Determine which dotman.yaml to use, triggering auto-init if needed.
///
/// Priority:
/// 1. `--config <path>`                                    -> Direct
/// 2. `./dotman.yaml` exists in cwd                        -> Direct
/// 3. `~/.config/dotman/config.toml` exists + valid         -> Profile
/// 4. otherwise                                             -> auto-init -> Profile
pub fn resolve_config(cli: &cli::Cli) -> Result<ConfigSource, String> {
    // 1. Explicit --config
    if let Some(config) = &cli.config {
        if config.exists() {
            return Ok(ConfigSource::Direct(config.clone()));
        }
        return Err(format!("config file not found: {}", config.display()));
    }

    // 2. Current directory dotman.yaml
    if Path::new("dotman.yaml").exists() {
        return Ok(ConfigSource::Direct(PathBuf::from("dotman.yaml")));
    }

    // 3. --no-init -> fail
    if cli.no_init {
        return Err("no config found and --no-init is set".into());
    }

    // 4. Try active profile
    if let Some(p) = profile::active_profile().map_err(|e| e.to_string())? {
        let cp = p.config_path();
        if cp.exists() {
            return Ok(ConfigSource::Profile(cp));
        }
        // Checkout missing -- profile exists but not cloned yet.
        clone_profile_with_git_policy(&p, cli)
            .map_err(|e| format!("failed to clone profile repo: {e}"))?;
        if cp.exists() {
            return Ok(ConfigSource::Profile(cp));
        }
        return Err(format!(
            "profile checkout exists but {} not found inside",
            profile::DEFAULT_CONFIG_FILE
        ));
    }

    // 5. Auto-init (first run)
    auto_init(cli).map_err(|e| format!("auto-init failed: {e}"))?;

    // Now config should exist
    match profile::active_config_path() {
        Ok(Some(p)) if p.exists() => Ok(ConfigSource::Profile(p)),
        Ok(Some(p)) => Err(format!(
            "init completed but config not found at {}",
            p.display()
        )),
        Ok(None) => Err("init completed but no active profile".into()),
        Err(e) => Err(format!("failed to read profile after init: {e}")),
    }
}

/// Run profile sync (git pull) on the active profile checkout.
///
/// Respects `auto_sync` — if the profile has `auto_sync = false`, sync is skipped.
/// Returns an error if git pull fails, so deploy is blocked on sync failures.
pub fn sync_profile() -> Result<(), String> {
    let p = match profile::active_profile() {
        Ok(Some(p)) => p,
        Ok(None) => return Ok(()),
        Err(e) => return Err(format!("failed to read profile: {e}")),
    };
    if !p.auto_sync {
        return Ok(());
    }
    if !p.checkout_exists() {
        return Ok(());
    }
    let checkout = p.checkout_path();
    let git_dir = checkout.join(".git");
    if !git_dir.exists() {
        return Ok(());
    }

    let output = Command::new("git")
        .args([
            "-C",
            &checkout.to_string_lossy(),
            "pull",
            "--ff-only",
            "--rebase=false",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("failed to run git pull: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "git pull failed for profile checkout at {}: {}",
            checkout.display(),
            stderr.trim()
        ));
    }
    Ok(())
}

/// Combined: resolve config, then sync the profile (only for profile sources).
pub fn resolve_and_sync(cli: &cli::Cli) -> Result<PathBuf, String> {
    let source = resolve_config(cli)?;
    // Only sync when the config came from a profile, not direct paths.
    if matches!(source, ConfigSource::Profile(_)) {
        sync_profile()?;
    }
    Ok(source.path())
}

/// Clone or reuse a dotfiles checkout, verifying remote matches if reusing.
///
/// Returns the checkout path, ready to have a profile written for it.
/// Handles: git exists check, existing checkout remote match, git pull, fresh clone.
pub fn clone_or_reuse_checkout(
    repo: &str,
    branch: &str,
    path_str: &str,
) -> anyhow::Result<PathBuf> {
    let checkout = profile::expand_tilde(path_str);
    if checkout.exists() {
        let git_dir = checkout.join(".git");
        if !git_dir.exists() {
            anyhow::bail!(
                "path exists but is not a git repository: {}",
                checkout.display()
            );
        }
        let remote =
            git_remote_url(&checkout).context("failed to get remote URL for existing checkout")?;
        let expected = repo.trim_end_matches(".git");
        if remote != expected {
            anyhow::bail!(
                "existing checkout remote '{remote}' does not match target repo '{expected}'"
            );
        }
        eprintln!("Reusing existing checkout at {}", checkout.display());
        let status = Command::new("git")
            .args([
                "-C",
                &checkout.to_string_lossy(),
                "pull",
                "--ff-only",
                "--rebase=false",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .status()
            .context("failed to git pull existing checkout")?;
        if !status.success() {
            anyhow::bail!("failed to sync existing checkout at {}", checkout.display());
        }
    } else {
        do_clone(&checkout, repo, branch)?;
    }
    Ok(checkout)
}

/// Run the full init sequence: ensure git, clone repo, write profile config.
///
/// Called when no existing config is found. Behavior depends on flags:
/// - `--headless`: use defaults, fail if git missing (unless `--bootstrap-git`)
/// - `--headless --bootstrap-git`: auto-install git
/// - default (interactive): prompt user for confirmation
pub fn auto_init(cli: &cli::Cli) -> anyhow::Result<()> {
    let repo = profile::DEFAULT_REPO;
    let branch = profile::DEFAULT_BRANCH;
    let profile_name = profile::DEFAULT_PROFILE_NAME;
    let path = format!("~/.local/share/dotman/repos/{profile_name}");

    if !cli.headless {
        eprintln!("dotman has not been initialized yet.");
        eprintln!();
        eprintln!("Default configuration:");
        eprintln!("  profile = {profile_name}");
        eprintln!("  repo    = {repo}");
        eprintln!("  branch  = {branch}");
        eprintln!("  path    = {path}");
        eprintln!();
        eprint!("Initialize dotman now? [Y/n] ");

        let mut input = String::new();
        let _ = std::io::stdin().read_line(&mut input);
        let answer = input.trim().to_lowercase();
        if !answer.is_empty() && answer != "y" && answer != "yes" {
            anyhow::bail!("initialization cancelled by user");
        }
    }

    ensure_git(cli)?;
    let checkout = clone_or_reuse_checkout(repo, branch, &path)?;

    let config_path = checkout.join(profile::DEFAULT_CONFIG_FILE);
    validate_config(&config_path, cli.headless)?;

    update_or_add_profile(profile_name, repo, branch, &path)
        .context("failed to save profile config")?;

    eprintln!("dotman initialized: profile '{profile_name}' -> {repo}");
    Ok(())
}

/// Check git availability according to policy flags, then install or fail.
pub fn ensure_git(cli: &cli::Cli) -> anyhow::Result<()> {
    if bootstrap::git_installed() {
        return Ok(());
    }

    if cli.headless && !cli.bootstrap_git {
        anyhow::bail!(
            "git is not installed. Run with --bootstrap-git to auto-install, or install git manually."
        );
    }

    if cli.bootstrap_git {
        eprintln!("git not found. Attempting auto-install...");
        match bootstrap::auto_install_git()? {
            true => {
                eprintln!("git installed successfully.");
                return Ok(());
            }
            false => {
                eprintln!("Could not auto-install git on this platform.");
                bootstrap::print_git_help();
                anyhow::bail!("git is required. Please install git and try again.");
            }
        }
    }

    // Interactive mode: prompt
    eprintln!();
    eprintln!("git was not found.");
    eprintln!();
    eprintln!("dotman needs git to clone:");
    eprintln!("  {}", profile::DEFAULT_REPO);
    eprintln!();
    eprint!("Install git now? [Y/n] ");

    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    let answer = input.trim().to_lowercase();
    if !answer.is_empty() && answer != "y" && answer != "yes" {
        bootstrap::print_git_help();
        anyhow::bail!("git is required. Please install git and try again.");
    }

    match bootstrap::auto_install_git()? {
        true => {
            eprintln!("git installed successfully.");
            Ok(())
        }
        false => {
            bootstrap::print_git_help();
            anyhow::bail!("could not auto-install git. Please install git and try again.");
        }
    }
}

/// Clone a profile's repo using git policy from cli flags.
///
/// This is used when a profile is already configured but the checkout is missing.
/// Respects --bootstrap-git for headless mode.
pub fn clone_profile_with_git_policy(p: &profile::Profile, cli: &cli::Cli) -> anyhow::Result<()> {
    // Ensure git, respecting the cli flags for bootstrap policy.
    if !bootstrap::git_installed() {
        if cli.headless && !cli.bootstrap_git {
            anyhow::bail!(
                "git is not installed. Run with --bootstrap-git to auto-install, or install git manually."
            );
        }
        ensure_git(cli)?;
    }

    clone_or_reuse_checkout(&p.repo, &p.branch, &p.path)?;
    Ok(())
}

/// Get the remote origin URL from a git repo at `checkout`.
fn git_remote_url(checkout: &std::path::Path) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args([
            "-C",
            &checkout.to_string_lossy(),
            "remote",
            "get-url",
            "origin",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .context("failed to get git remote URL")?;

    if !output.status.success() {
        anyhow::bail!("no git remote 'origin' configured");
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(url.trim_end_matches(".git").to_string())
}

/// Core clone operation. Assumes git is available and checkout does not exist.
fn do_clone(checkout: &std::path::Path, repo: &str, branch: &str) -> anyhow::Result<()> {
    let parent = checkout
        .parent()
        .ok_or_else(|| anyhow::anyhow!("invalid checkout path: {}", checkout.display()))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))?;

    eprintln!("Cloning {repo} into {}...", checkout.display());
    let status = Command::new("git")
        .args(["clone", "--branch", branch, repo])
        .arg(checkout)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .context("failed to run git clone")?;

    if !status.success() {
        anyhow::bail!("git clone failed for {repo} (branch: {branch})");
    }
    Ok(())
}

/// Validate that dotman.yaml exists in the checkout and can be parsed into a plan.
///
/// Called by both auto-init and explicit init before saving the profile.
/// Returns an error if the config is missing or invalid.
pub fn validate_config(config_path: &std::path::Path, headless: bool) -> anyhow::Result<()> {
    if !config_path.exists() {
        anyhow::bail!(
            "{} not found in cloned repo — add a dotman.yaml and retry",
            config_path.display()
        );
    }
    let cfg = crate::config::load(config_path)
        .with_context(|| format!("failed to parse {}", config_path.display()))?;
    // Verify the config produces a valid plan.
    crate::plan::build(&cfg, crate::model::Mode::Deploy)
        .context("failed to build plan from config")?;

    let _ = headless; // reserved for future headless-specific validation
    Ok(())
}

/// Add or update a profile in the config, preserving existing profiles.
fn update_or_add_profile(
    profile_name: &str,
    repo: &str,
    branch: &str,
    path: &str,
) -> anyhow::Result<()> {
    let mut cfg = profile::load()?.unwrap_or_else(profile::default_config);
    cfg.default_profile = profile_name.to_string();
    cfg.profiles.insert(
        profile_name.to_string(),
        profile::Profile {
            repo: repo.to_string(),
            branch: branch.to_string(),
            path: path.to_string(),
            config: profile::DEFAULT_CONFIG_FILE.to_string(),
            auto_sync: true,
        },
    );
    profile::save(&cfg)?;
    Ok(())
}

// ---- Doctor ----

/// Validate system prerequisites and return a list of diagnostics.
pub struct Diagnostic {
    pub ok: bool,
    pub message: String,
}

pub fn run_doctor() -> Vec<Diagnostic> {
    let mut results = Vec::new();

    if bootstrap::git_installed() {
        results.push(Diagnostic {
            ok: true,
            message: "git is installed".into(),
        });
    } else {
        results.push(Diagnostic {
            ok: false,
            message: "git is not installed".into(),
        });
    }

    match profile::config_file_path() {
        Ok(p) if p.exists() => results.push(Diagnostic {
            ok: true,
            message: format!("profile config found at {}", p.display()),
        }),
        Ok(p) => results.push(Diagnostic {
            ok: false,
            message: format!("profile config not found at {}", p.display()),
        }),
        Err(e) => results.push(Diagnostic {
            ok: false,
            message: format!("cannot check profile config: {e}"),
        }),
    }

    match profile::active_profile() {
        Ok(Some(p)) => {
            let cp = p.config_path();
            if cp.exists() {
                results.push(Diagnostic {
                    ok: true,
                    message: format!("active profile points to config: {}", cp.display()),
                });
            } else {
                results.push(Diagnostic {
                    ok: false,
                    message: format!(
                        "active profile configured but config not found at {}",
                        cp.display()
                    ),
                });
            }
        }
        Ok(None) => {
            results.push(Diagnostic {
                ok: false,
                message: "no active profile configured".into(),
            });
        }
        Err(e) => {
            results.push(Diagnostic {
                ok: false,
                message: format!("failed to read profile: {e}"),
            });
        }
    }

    let brew = std::process::Command::new("which")
        .arg("brew")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    results.push(Diagnostic {
        ok: true,
        message: if brew {
            "Homebrew available".into()
        } else {
            "Homebrew not available".into()
        },
    });

    results
}

// ---- Status ----

pub fn show_status() -> Result<(), String> {
    println!("dotman status");
    println!("-------------");

    match profile::active_profile() {
        Ok(Some(p)) => {
            let name = profile::load()
                .ok()
                .flatten()
                .map(|cfg| cfg.default_profile)
                .unwrap_or_else(|| "main".to_string());
            println!("  profile:     {name} (active)");
            println!("  repo:        {}", p.repo);
            println!("  branch:      {}", p.branch);
            println!("  checkout:    {}", p.checkout_path().display());
            println!("  config:      {}", p.config_path().display());
            println!(
                "  checkout ok: {}",
                if p.checkout_exists() { "yes" } else { "no" }
            );
            println!(
                "  config ok:   {}",
                if p.config_path().exists() {
                    "yes"
                } else {
                    "no"
                }
            );
        }
        Ok(None) => {
            println!("  No active profile configured.");
            println!("  Run `dotman init` to set up your dotfiles.");
        }
        Err(e) => {
            println!("  Error reading profile: {e}");
        }
    }

    Ok(())
}
