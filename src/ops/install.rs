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
    pub tools: Vec<ToolEntry>,
}

/// Embedded tool database (compiled in via include_str!).
pub const TOOL_DB_TOML: &str = include_str!("db.toml");

pub fn load_db() -> Result<ToolDb> {
    let db: ToolDb = toml::from_str(TOOL_DB_TOML).context("failed to parse tool db")?;
    Ok(db)
}

pub fn find<'a>(db: &'a ToolDb, name: &str) -> Option<&'a ToolEntry> {
    db.tools.iter().find(|t| t.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_embedded_db() {
        let db = load_db().unwrap();
        assert!(db.tools.iter().any(|t| t.name == "fish"));
        assert!(db.tools.iter().any(|t| t.name == "tmux"));
    }

    #[test]
    fn find_returns_matching_entry() {
        let db = load_db().unwrap();
        let fish = find(&db, "fish").unwrap();
        assert_eq!(fish.binary, "fish");
        assert_eq!(fish.layer, "shell");
    }

    #[test]
    fn find_returns_none_for_unknown() {
        let db = load_db().unwrap();
        assert!(find(&db, "totally-bogus").is_none());
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
