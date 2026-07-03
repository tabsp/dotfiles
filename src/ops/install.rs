//! Install operation + tool database.
//!
//! Phase 3: real implementations (brew/pacman/dnf, font handling, retry).

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};

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
    pub platforms: BTreeMap<String, String>,
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

pub fn is_installed(binary: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {binary} >/dev/null 2>&1"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn install(tool: &ToolEntry, pkg_mgr: &str) -> Result<InstallOutput> {
    if tool.kind == "font" {
        return install_font(tool);
    }
    let cmd = tool
        .platforms
        .get(pkg_mgr)
        .with_context(|| format!("no install command for {pkg_mgr}"))?;
    run_install_cmd(cmd)
}

#[derive(Debug, Clone)]
pub struct InstallOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

fn run_install_cmd(cmd: &str) -> Result<InstallOutput> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run: {cmd}"))?;
    Ok(InstallOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

fn install_font(tool: &ToolEntry) -> Result<InstallOutput> {
    if tool.source_url.is_empty() {
        anyhow::bail!("font {} missing source_url", tool.name);
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    let fonts_dir = Path::new(&home).join(".local/share/fonts");
    std::fs::create_dir_all(&fonts_dir).context("failed to create fonts dir")?;

    // Skip if already installed: a directory matching the tool's binary name.
    if fonts_dir.join(&tool.binary).exists() {
        return Ok(InstallOutput {
            stdout: "font already installed".into(),
            stderr: String::new(),
            exit_code: 0,
        });
    }

    // Best-effort download + unzip. (Phase 3 minimal; Phase 5+ TUI streaming.)
    let zip = fonts_dir.join(format!("{}.zip", tool.name));
    let status = Command::new("curl")
        .arg("-fsSL")
        .arg(&tool.source_url)
        .arg("-o")
        .arg(&zip)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to download font from {}", tool.source_url))?;
    if !status.success() {
        anyhow::bail!("font download failed");
    }
    let _ = Command::new("unzip")
        .arg("-o")
        .arg("-q")
        .arg(&zip)
        .arg("-d")
        .arg(&fonts_dir)
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to unzip {}", zip.display()))?;
    let _ = std::fs::remove_file(&zip);
    Ok(InstallOutput {
        stdout: format!("font installed to {}", fonts_dir.display()),
        stderr: String::new(),
        exit_code: 0,
    })
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
}
