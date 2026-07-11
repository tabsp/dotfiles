//! Profile config management.
//!
//! Manages `~/.config/dotman/config.toml` which stores named profiles.
//! Each profile holds a dotfiles repo URL, branch, checkout path, and config file name.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub const DEFAULT_REPO: &str = "https://github.com/tabsp/dotfiles.git";
pub const DEFAULT_BRANCH: &str = "main";
pub const DEFAULT_PROFILE_NAME: &str = "main";
pub const DEFAULT_CONFIG_FILE: &str = "dotman.yaml";

/// Top-level profile config stored at ~/.config/dotman/config.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

/// A single named profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Git remote URL.
    pub repo: String,

    /// Git branch to track.
    #[serde(default = "default_branch")]
    pub branch: String,

    /// Filesystem path where the repo is checked out (may contain `~`).
    pub path: String,

    /// Name of the config file inside the repo (default: dotman.yaml).
    #[serde(default = "default_config_file_name")]
    pub config: String,

    /// Whether to automatically `git pull` before each plan/deploy.
    #[serde(default = "default_auto_sync")]
    pub auto_sync: bool,
}

// ---- Default helpers ----

fn default_profile_name() -> String {
    DEFAULT_PROFILE_NAME.to_string()
}

fn default_branch() -> String {
    DEFAULT_BRANCH.to_string()
}

fn default_config_file_name() -> String {
    DEFAULT_CONFIG_FILE.to_string()
}

fn default_auto_sync() -> bool {
    true
}

// ---- Default profile ----

/// Build the default profile config (no disk I/O).
pub fn default_config() -> ProfileConfig {
    let mut profiles = HashMap::new();
    profiles.insert(
        DEFAULT_PROFILE_NAME.to_string(),
        Profile {
            repo: DEFAULT_REPO.to_string(),
            branch: DEFAULT_BRANCH.to_string(),
            path: resolve_checkout_path(None, None, DEFAULT_PROFILE_NAME),
            config: DEFAULT_CONFIG_FILE.to_string(),
            auto_sync: true,
        },
    );
    ProfileConfig {
        default_profile: DEFAULT_PROFILE_NAME.to_string(),
        profiles,
    }
}

fn default_checkout_path(profile_name: &str) -> String {
    format!("~/.local/share/dotman/repos/{profile_name}")
}

/// Resolve a checkout path from an explicit init override, an existing profile,
/// or the default location for a new profile.
pub fn resolve_checkout_path(
    explicit_path: Option<&std::path::Path>,
    profile_cfg: Option<&ProfileConfig>,
    profile_name: &str,
) -> String {
    explicit_path
        .map(|path| path.to_string_lossy().into_owned())
        .or_else(|| {
            profile_cfg
                .and_then(|cfg| cfg.profiles.get(profile_name))
                .map(|profile| profile.path.clone())
        })
        .unwrap_or_else(|| default_checkout_path(profile_name))
}

// ---- Profile methods ----

impl Profile {
    /// Expand `~` to $HOME and return the checkout directory.
    pub fn checkout_path(&self) -> PathBuf {
        expand_tilde(&self.path)
    }

    /// Return the full path to the config file inside the checkout.
    pub fn config_path(&self) -> PathBuf {
        self.checkout_path().join(&self.config)
    }

    /// Check whether the checkout directory exists (i.e. repo has been cloned).
    pub fn checkout_exists(&self) -> bool {
        self.checkout_path().exists()
    }
}

// ---- Config file paths ----

/// Return the path to the profile config file (~/.config/dotman/config.toml).
pub fn config_file_path() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".config/dotman/config.toml"))
}

/// Return the config directory (~/.config/dotman).
pub fn config_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".config/dotman"))
}

// ---- Load / Save ----

/// Load profile config from `~/.config/dotman/config.toml`.
///
/// Returns `None` when the file doesn't exist yet (first run).
pub fn load() -> Result<Option<ProfileConfig>> {
    let path = config_file_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let cfg: ProfileConfig =
        toml::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(cfg))
}

/// Save profile config to `~/.config/dotman/config.toml`.
pub fn save(cfg: &ProfileConfig) -> Result<PathBuf> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create config dir {}", dir.display()))?;
    let path = dir.join("config.toml");
    let raw = toml::to_string_pretty(cfg).context("failed to serialize profile config")?;
    std::fs::write(&path, raw).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

/// Get the active profile: loads config, looks up `default_profile`.
///
/// Returns `None` when config doesn't exist or the profile name is missing.
pub fn active_profile() -> Result<Option<Profile>> {
    let Some(cfg) = load()? else {
        return Ok(None);
    };
    Ok(cfg.profiles.get(&cfg.default_profile).cloned())
}

/// Return the active profile config path, or None if not configured.
pub fn active_config_path() -> Result<Option<PathBuf>> {
    Ok(active_profile()?.map(|p| p.config_path()))
}

// ---- Utility ----

/// Replace leading `~/` with the value of $HOME.
pub fn expand_tilde(s: &str) -> PathBuf {
    crate::path::expand_home(s).unwrap_or_else(|_| PathBuf::from(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = default_config();
        assert_eq!(cfg.default_profile, "main");
        let profile = cfg.profiles.get("main").unwrap();
        assert_eq!(profile.repo, DEFAULT_REPO);
        assert_eq!(profile.branch, "main");
        assert_eq!(profile.config, "dotman.yaml");
        assert!(profile.auto_sync);
    }

    #[test]
    fn default_profile_and_init_use_the_same_checkout_path() {
        let cfg = default_config();

        assert_eq!(
            cfg.profiles.get(DEFAULT_PROFILE_NAME).unwrap().path,
            resolve_checkout_path(None, None, DEFAULT_PROFILE_NAME)
        );
    }

    #[test]
    fn profile_expands_checkout_path() {
        let profile = Profile {
            repo: DEFAULT_REPO.to_string(),
            branch: "main".to_string(),
            path: "~/.local/share/dotman/repos/main".to_string(),
            config: "dotman.yaml".to_string(),
            auto_sync: true,
        };
        let cp = profile.checkout_path();
        let home = std::env::var("HOME").unwrap();
        assert!(cp.starts_with(home));
    }

    #[test]
    fn config_file_path_ends_correctly() {
        let path = config_file_path().unwrap();
        assert!(path.ends_with(".config/dotman/config.toml"));
    }

    #[test]
    fn expand_tilde_works() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_tilde("~/foo"), PathBuf::from(&home).join("foo"));
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
    }

    #[test]
    fn profile_config_path_returns_repo_joined_config() {
        let profile = Profile {
            repo: DEFAULT_REPO.to_string(),
            branch: "main".to_string(),
            path: "~/.local/share/dotman/repos/main".to_string(),
            config: "dotman.yaml".to_string(),
            auto_sync: true,
        };
        let cp = profile.config_path();
        assert!(cp.ends_with("dotman.yaml"));
    }

    #[test]
    fn roundtrip_through_toml() {
        let cfg = default_config();
        let raw = toml::to_string_pretty(&cfg).unwrap();
        let parsed: ProfileConfig = toml::from_str(&raw).unwrap();
        assert_eq!(parsed.default_profile, cfg.default_profile);
        assert_eq!(
            parsed.profiles.get("main").unwrap().repo,
            cfg.profiles.get("main").unwrap().repo
        );
    }
}
