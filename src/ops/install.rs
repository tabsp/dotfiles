//! Install operation + tool database.
//!
//! Phase 3: real implementations (brew/pacman/dnf, font handling, retry).

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct InstallCommand {
    pub command: String,
    #[serde(default)]
    pub os: Vec<String>,
}

impl InstallCommand {
    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn supports_os(&self, os_name: &str) -> bool {
        self.os.is_empty() || self.os.iter().any(|allowed| allowed == os_name)
    }
}

/// One entry in the tool database.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolEntry {
    pub name: String,
    pub binary: String,
    pub layer: String,
    #[serde(default)]
    pub kind: String, // "pkg" (default) or "font"
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub platforms: BTreeMap<String, InstallCommand>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolDb {
    #[serde(default)]
    pub default_platforms: Vec<String>,
    #[serde(default)]
    pub templates: BTreeMap<String, InstallCommandTemplate>,
    #[serde(default)]
    pub tools: BTreeMap<String, ToolRule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallCommandTemplate {
    pub command: String,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub os: Vec<String>,
}

impl InstallCommandTemplate {
    fn render(&self, package: &str) -> InstallCommand {
        InstallCommand {
            command: self.command.replace("{package}", package),
            os: self.os.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ToolRule {
    #[serde(default)]
    pub binary: Option<String>,
    #[serde(default)]
    pub layer: Option<String>,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub packages: BTreeMap<String, String>,
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
}

/// Embedded tool database (compiled in via include_str!).
pub const TOOL_DB_TOML: &str = include_str!("db.toml");

pub fn load_db() -> Result<ToolDb> {
    let db: ToolDb = toml::from_str(TOOL_DB_TOML).context("failed to parse tool db")?;
    Ok(db)
}

pub fn find(db: &ToolDb, name: &str) -> Option<ToolEntry> {
    if name.trim().is_empty() {
        return None;
    }

    let rule = db.tools.get(name);
    let binary = rule
        .and_then(|r| r.binary.clone())
        .unwrap_or_else(|| name.to_string());
    let layer = rule
        .and_then(|r| r.layer.clone())
        .unwrap_or_else(|| "software".to_string());
    let kind = rule.map(|r| r.kind.clone()).unwrap_or_default();
    let source_url = rule.map(|r| r.source_url.clone()).unwrap_or_default();
    let platforms = expand_platforms(db, name, rule)?;

    Some(ToolEntry {
        name: name.to_string(),
        binary,
        layer,
        kind,
        source_url,
        platforms,
    })
}

pub fn tool_layer(db: &ToolDb, name: &str) -> Option<String> {
    db.tools.get(name).and_then(|rule| rule.layer.clone())
}

fn expand_platforms(
    db: &ToolDb,
    name: &str,
    rule: Option<&ToolRule>,
) -> Option<BTreeMap<String, InstallCommand>> {
    let platform_keys = rule
        .and_then(|r| r.platforms.clone())
        .unwrap_or_else(|| db.default_platforms.clone());
    let mut platforms = BTreeMap::new();

    for platform_key in platform_keys {
        let template = db.templates.get(&platform_key)?;
        let package = package_for(name, &platform_key, rule);
        let install_key = template
            .platform
            .clone()
            .unwrap_or_else(|| platform_key.clone());
        platforms.insert(install_key, template.render(&package));
    }

    Some(platforms)
}

fn package_for(name: &str, platform_key: &str, rule: Option<&ToolRule>) -> String {
    rule.and_then(|r| r.packages.get(platform_key).cloned())
        .or_else(|| rule.and_then(|r| r.package.clone()))
        .unwrap_or_else(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_embedded_db() {
        let db = load_db().unwrap();
        assert!(db.templates.contains_key("brew"));
        assert!(db.tools.contains_key("fish"));
    }

    #[test]
    fn find_returns_matching_entry() {
        let db = load_db().unwrap();
        let neovim = find(&db, "neovim").unwrap();
        assert_eq!(neovim.binary, "nvim");
        assert_eq!(neovim.layer, "software");
        assert_eq!(
            neovim.platforms.get("brew").unwrap().command(),
            "brew install neovim"
        );
    }

    #[test]
    fn find_synthesizes_unknown_tool_from_defaults() {
        let db = load_db().unwrap();
        let tool = find(&db, "totally-bogus").unwrap();
        assert_eq!(tool.binary, "totally-bogus");
        assert_eq!(tool.layer, "software");
        assert_eq!(
            tool.platforms.get("apt").unwrap().command(),
            "sudo apt install -y totally-bogus"
        );
    }

    #[test]
    fn find_applies_platform_package_overrides() {
        let db = load_db().unwrap();
        let fd = find(&db, "fd").unwrap();
        assert_eq!(
            fd.platforms.get("apt").unwrap().command(),
            "sudo apt install -y fd-find"
        );
        assert_eq!(
            fd.platforms.get("brew").unwrap().command(),
            "brew install fd"
        );
    }

    #[test]
    fn find_maps_brew_cask_template_to_brew_platform() {
        let db = load_db().unwrap();
        let ghostty = find(&db, "ghostty").unwrap();
        assert_eq!(
            ghostty.platforms.get("brew").unwrap().command(),
            "brew install --cask ghostty"
        );
        assert!(!ghostty.platforms.get("brew").unwrap().supports_os("linux"));
        assert_eq!(
            ghostty.platforms.get("pacman").unwrap().command(),
            "sudo pacman -S --needed --noconfirm ghostty"
        );
        let ubuntu_cmd = ghostty.platforms.get("ubuntu").unwrap().command();
        assert!(ubuntu_cmd.starts_with("sudo -n apt-get update -qq && "));
        assert!(ubuntu_cmd.contains("sudo -n env"));
        assert!(ubuntu_cmd.contains("DEBIAN_FRONTEND=noninteractive"));
        assert!(ubuntu_cmd.contains("NEEDRESTART_MODE=a"));
        assert!(ghostty.platforms.contains_key("fedora"));
    }

    #[test]
    fn install_command_without_os_supports_all_os() {
        let cmd = InstallCommand {
            command: "brew install fish".into(),
            os: vec![],
        };
        assert_eq!(cmd.command(), "brew install fish");
        assert!(cmd.supports_os("macos"));
        assert!(cmd.supports_os("linux"));
    }

    #[test]
    fn install_command_respects_os() {
        let cmd = InstallCommand {
            command: "brew install --cask ghostty".into(),
            os: vec!["macos".into()],
        };
        assert_eq!(cmd.command(), "brew install --cask ghostty");
        assert!(cmd.supports_os("macos"));
        assert!(!cmd.supports_os("linux"));
    }
}
