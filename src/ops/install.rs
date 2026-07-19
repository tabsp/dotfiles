//! Install operation + tool database.
//!
//! Phase 3: real implementations (brew/pacman/dnf, font handling, retry).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallPresence {
    Present,
    Missing,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolEntry {
    pub name: String,
    pub binary: String,
    pub layer: String,
    #[serde(default)]
    pub kind: String, // "pkg" (default) or "font"
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub font_family: String,
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

impl ToolDb {
    pub fn validate(&self) -> Result<()> {
        if self.default_platforms.is_empty() {
            anyhow::bail!("tool db has no default_platforms");
        }

        for platform in &self.default_platforms {
            let template = self.templates.get(platform).with_context(|| {
                format!("default platform '{platform}' references an unknown template")
            })?;
            if !template.command.contains("{package}") {
                anyhow::bail!("default platform template '{platform}' must contain '{{package}}'");
            }
        }

        for (name, template) in &self.templates {
            if name.trim().is_empty() {
                anyhow::bail!("tool db contains an empty template name");
            }
            if template.command.trim().is_empty() {
                anyhow::bail!("template '{name}' has an empty command");
            }
            if template
                .platform
                .as_deref()
                .is_some_and(|platform| platform.trim().is_empty())
            {
                anyhow::bail!("template '{name}' has an empty platform");
            }
            if template.os.iter().any(|os| os.trim().is_empty()) {
                anyhow::bail!("template '{name}' contains an empty os value");
            }
        }

        for (name, rule) in &self.tools {
            if name.trim().is_empty() {
                anyhow::bail!("tool db contains an empty tool name");
            }
            if rule
                .binary
                .as_deref()
                .is_some_and(|binary| binary.trim().is_empty())
            {
                anyhow::bail!("tool '{name}' has an empty binary");
            }
            if rule
                .layer
                .as_deref()
                .is_some_and(|layer| layer.trim().is_empty())
            {
                anyhow::bail!("tool '{name}' has an empty layer");
            }
            if !rule.kind.is_empty() && rule.kind != "font" {
                anyhow::bail!("tool '{name}' has unsupported kind '{}'", rule.kind);
            }
            if rule.kind == "font" && rule.font_family.trim().is_empty() {
                anyhow::bail!("font tool '{name}' is missing font_family");
            }
            if rule
                .package
                .as_deref()
                .is_some_and(|package| package.trim().is_empty())
                || rule
                    .packages
                    .values()
                    .any(|package| package.trim().is_empty())
            {
                anyhow::bail!("tool '{name}' contains an empty package name");
            }

            for platform in rule.packages.keys() {
                if !self.templates.contains_key(platform) {
                    anyhow::bail!(
                        "tool '{name}' has a package override for unknown template '{platform}'"
                    );
                }
            }

            let platforms = rule.platforms.as_ref().unwrap_or(&self.default_platforms);
            let has_font_fallback = rule.kind == "font" && !rule.source_url.is_empty();
            if platforms.is_empty() && !has_font_fallback {
                anyhow::bail!("tool '{name}' has no install platforms");
            }

            let mut install_keys = BTreeSet::new();
            for platform in platforms {
                let template = self.templates.get(platform).with_context(|| {
                    format!("tool '{name}' references unknown template '{platform}'")
                })?;
                let install_key = template.platform.as_deref().unwrap_or(platform);
                if !install_keys.insert(install_key) {
                    anyhow::bail!(
                        "tool '{name}' maps multiple templates to install platform '{install_key}'"
                    );
                }
            }
        }

        Ok(())
    }
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
    pub font_family: String,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub packages: BTreeMap<String, String>,
    #[serde(default)]
    pub platforms: Option<Vec<String>>,
}

/// Fully resolved install data captured when a Plan is built.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstallSpec {
    pub entry: ToolEntry,
    pub pkg_mgr: String,
    pub command: Option<String>,
    pub error: Option<String>,
}

/// Embedded tool database (compiled in via include_str!).
pub const TOOL_DB_TOML: &str = include_str!("db.toml");

pub fn load_db() -> Result<ToolDb> {
    let db: ToolDb = toml::from_str(TOOL_DB_TOML).context("failed to parse tool db")?;
    db.validate().context("invalid tool db")?;
    Ok(db)
}

pub fn resolve_install(db: &ToolDb, name: &str, pkg_mgr: &str) -> Result<InstallSpec> {
    let entry = find(db, name).with_context(|| format!("cannot resolve tool '{name}'"))?;
    let (command, error) = resolve_command(&entry, pkg_mgr);
    Ok(InstallSpec {
        entry,
        pkg_mgr: pkg_mgr.to_string(),
        command,
        error,
    })
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
    let font_family = rule.map(|r| r.font_family.clone()).unwrap_or_default();
    let platforms = expand_platforms(db, name, rule)?;

    Some(ToolEntry {
        name: name.to_string(),
        binary,
        layer,
        kind,
        source_url,
        font_family,
        platforms,
    })
}

pub fn tool_layer(db: &ToolDb, name: &str) -> Option<String> {
    db.tools.get(name).and_then(|rule| rule.layer.clone())
}

pub fn command_for_current_platform(entry: &ToolEntry, pkg_mgr: &str) -> Option<String> {
    resolve_command(entry, pkg_mgr).0
}

fn resolve_command(entry: &ToolEntry, pkg_mgr: &str) -> (Option<String>, Option<String>) {
    let distro = crate::package_managers::distro_id();
    let mut candidates = vec![pkg_mgr.to_string()];
    if let Some(distro) = &distro
        && !candidates.iter().any(|candidate| candidate == distro)
    {
        candidates.push(distro.clone());
    }
    let os = crate::package_managers::os_name().to_string();
    if !candidates.iter().any(|candidate| candidate == &os) {
        candidates.push(os);
    }

    let mut saw_unsupported = false;
    for candidate in candidates {
        if let Some(command) = entry.platforms.get(&candidate) {
            if command_supports_current_platform(command, distro.as_deref()) {
                return (Some(command.command().to_string()), None);
            }
            saw_unsupported = true;
        }
    }

    if entry.kind == "font" {
        let error = entry
            .source_url
            .is_empty()
            .then(|| format!("font {} missing source_url", entry.name));
        return (None, error);
    }
    let error = if saw_unsupported {
        format!(
            "{} is not supported for {pkg_mgr} on {}",
            entry.name,
            crate::package_managers::os_name()
        )
    } else {
        format!("no install command for {pkg_mgr}")
    };
    (None, Some(error))
}

pub fn detect_presence(entry: &ToolEntry, install_command: Option<&str>) -> InstallPresence {
    detect_presence_with_probe(entry, install_command, &RealPresenceProbe)
}

trait PresenceProbe {
    fn command_available(&self, binary: &str) -> bool;
    fn fontconfig_reports_family(&self, family: &str) -> bool;
    fn font_dir_contains(&self, dir: &Path, token: &str) -> bool;
    fn path_exists(&self, path: &Path) -> bool;
}

struct RealPresenceProbe;

impl PresenceProbe for RealPresenceProbe {
    fn command_available(&self, binary: &str) -> bool {
        command_available(binary)
    }

    fn fontconfig_reports_family(&self, family: &str) -> bool {
        fontconfig_reports_family(family)
    }

    fn font_dir_contains(&self, dir: &Path, token: &str) -> bool {
        font_dir_contains(dir, token)
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

fn detect_presence_with_probe(
    entry: &ToolEntry,
    install_command: Option<&str>,
    probe: &impl PresenceProbe,
) -> InstallPresence {
    if entry.kind == "font" {
        return detect_font_presence(entry, probe);
    }

    if crate::package_managers::os_name() == "macos"
        && let Some(command) = install_command
        && let Some(package) = cask_package_from_command(command)
    {
        return detect_macos_cask_presence(&package, probe);
    }

    if entry.binary.trim().is_empty() {
        InstallPresence::Unknown
    } else if probe.command_available(&entry.binary) {
        InstallPresence::Present
    } else {
        InstallPresence::Missing
    }
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

fn command_supports_current_platform(command: &InstallCommand, distro: Option<&str>) -> bool {
    command.supports_os(crate::package_managers::os_name())
        || distro.is_some_and(|distro| command.supports_os(distro))
}

fn command_available(binary: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!(
            "command -v {} >/dev/null 2>&1",
            shell_quote(binary)
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .is_some_and(|status| status.success())
}

fn detect_macos_cask_presence(package: &str, probe: &impl PresenceProbe) -> InstallPresence {
    let app_bundle = app_bundle_name(package);
    let mut app_paths = vec![
        PathBuf::from("/Applications").join(&app_bundle),
        PathBuf::from("/opt/homebrew/Caskroom").join(package),
        PathBuf::from("/usr/local/Caskroom").join(package),
    ];
    if let Some(home) = dirs::home_dir() {
        app_paths.push(home.join("Applications").join(&app_bundle));
    }

    if app_paths.iter().any(|path| probe.path_exists(path)) {
        InstallPresence::Present
    } else {
        InstallPresence::Missing
    }
}

fn detect_font_presence(entry: &ToolEntry, probe: &impl PresenceProbe) -> InstallPresence {
    if !entry.font_family.trim().is_empty()
        && probe.command_available("fc-list")
        && probe.fontconfig_reports_family(&entry.font_family)
    {
        return InstallPresence::Present;
    }

    let token = font_match_token(entry);
    if token.is_empty() {
        return InstallPresence::Unknown;
    }

    let mut dirs = vec![PathBuf::from("/Library/Fonts")];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Library/Fonts"));
        dirs.push(home.join(".local/share/fonts"));
    }

    if dirs.iter().any(|dir| probe.font_dir_contains(dir, &token)) {
        InstallPresence::Present
    } else {
        InstallPresence::Missing
    }
}

fn fontconfig_reports_family(family: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("fc-list | grep -qi {}", shell_quote(family)))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .is_some_and(|status| status.success())
}

fn font_dir_contains(dir: &Path, token: &str) -> bool {
    std::fs::read_dir(dir).ok().is_some_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .replace([' ', '-', '_'], "")
                .contains(token)
        })
    })
}

fn cask_package_from_command(command: &str) -> Option<String> {
    let parts = command.split_whitespace().collect::<Vec<_>>();
    if parts.first() != Some(&"brew")
        || parts.get(1) != Some(&"install")
        || !parts.contains(&"--cask")
    {
        return None;
    }
    parts
        .iter()
        .skip(2)
        .rev()
        .find(|part| !part.starts_with('-'))
        .map(|part| (*part).to_string())
}

fn app_bundle_name(package: &str) -> String {
    let name = package
        .split(['-', '_', '.'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<String>();
    format!("{name}.app")
}

fn font_match_token(entry: &ToolEntry) -> String {
    let raw = if entry.font_family.trim().is_empty() {
        entry.name.trim_start_matches("font-")
    } else {
        &entry.font_family
    };
    raw.to_ascii_lowercase().replace([' ', '-', '_'], "")
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[derive(Default)]
    struct FakePresenceProbe {
        commands: BTreeSet<String>,
        font_families: BTreeSet<String>,
        font_tokens: BTreeSet<String>,
        paths: BTreeSet<PathBuf>,
    }

    impl PresenceProbe for FakePresenceProbe {
        fn command_available(&self, binary: &str) -> bool {
            self.commands.contains(binary)
        }

        fn fontconfig_reports_family(&self, family: &str) -> bool {
            self.font_families.contains(family)
        }

        fn font_dir_contains(&self, _dir: &Path, token: &str) -> bool {
            self.font_tokens.contains(token)
        }

        fn path_exists(&self, path: &Path) -> bool {
            self.paths.contains(path)
        }
    }

    fn tool_entry(name: &str, binary: &str, kind: &str, font_family: &str) -> ToolEntry {
        ToolEntry {
            name: name.into(),
            binary: binary.into(),
            layer: "software".into(),
            kind: kind.into(),
            source_url: String::new(),
            font_family: font_family.into(),
            platforms: BTreeMap::new(),
        }
    }

    #[test]
    fn loads_embedded_db() {
        let db = load_db().unwrap();
        assert!(db.templates.contains_key("brew"));
        assert!(db.tools.contains_key("fish"));
    }

    #[test]
    fn validation_rejects_unknown_tool_template() {
        let db: ToolDb = toml::from_str(
            r#"
default_platforms = ["brew"]

[templates.brew]
command = "brew install {package}"

[tools.fish]
platforms = ["missing"]
"#,
        )
        .unwrap();

        let error = db.validate().unwrap_err().to_string();
        assert!(error.contains("tool 'fish' references unknown template 'missing'"));
    }

    #[test]
    fn validation_rejects_duplicate_install_platforms() {
        let db: ToolDb = toml::from_str(
            r#"
default_platforms = ["brew"]

[templates.brew]
command = "brew install {package}"

[templates.brew_cask]
command = "brew install --cask {package}"
platform = "brew"

[tools.ghostty]
platforms = ["brew", "brew_cask"]
"#,
        )
        .unwrap();

        let error = db.validate().unwrap_err().to_string();
        assert!(error.contains("maps multiple templates to install platform 'brew'"));
    }

    #[test]
    fn validation_rejects_default_template_without_package_placeholder() {
        let db: ToolDb = toml::from_str(
            r#"
default_platforms = ["brew"]

[templates.brew]
command = "brew install fish"
"#,
        )
        .unwrap();

        let error = db.validate().unwrap_err().to_string();
        assert!(error.contains("must contain '{package}'"));
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
    fn find_tree_sitter_cli_uses_cli_binary() {
        let db = load_db().unwrap();
        let tree_sitter = find(&db, "tree-sitter-cli").unwrap();
        assert_eq!(tree_sitter.binary, "tree-sitter");
        assert_eq!(tree_sitter.layer, "enhancement");
        assert_eq!(
            tree_sitter.platforms.get("brew").unwrap().command(),
            "brew install tree-sitter-cli"
        );
        assert_eq!(
            tree_sitter.platforms.get("pacman").unwrap().command(),
            "sudo pacman -S --needed --noconfirm tree-sitter-cli"
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

    #[test]
    fn detects_present_cli_binary() {
        let entry = tool_entry("sh", "sh", "", "");
        assert_eq!(detect_presence(&entry, None), InstallPresence::Present);
    }

    #[test]
    fn detects_missing_cli_binary() {
        let entry = tool_entry(
            "definitely-not-installed-dotman-test",
            "definitely-not-installed-dotman-test",
            "",
            "",
        );
        assert_eq!(detect_presence(&entry, None), InstallPresence::Missing);
    }

    #[test]
    fn detects_unknown_when_binary_is_empty() {
        let entry = tool_entry("metadata-only", "", "", "");
        let probe = FakePresenceProbe::default();

        assert_eq!(
            detect_presence_with_probe(&entry, None, &probe),
            InstallPresence::Unknown
        );
    }

    #[test]
    fn detects_linux_ghostty_from_binary_probe() {
        let db = load_db().unwrap();
        let entry = find(&db, "ghostty").unwrap();
        let mut probe = FakePresenceProbe::default();
        probe.commands.insert("ghostty".into());

        assert_eq!(
            detect_presence_with_probe(
                &entry,
                Some("sudo pacman -S --needed --noconfirm ghostty"),
                &probe
            ),
            InstallPresence::Present
        );
    }

    #[test]
    fn detects_font_from_fontconfig_probe() {
        let entry = tool_entry(
            "font-maple-mono-nf-cn",
            "font-maple-mono-nf-cn",
            "font",
            "Maple Mono",
        );
        let mut probe = FakePresenceProbe::default();
        probe.commands.insert("fc-list".into());
        probe.font_families.insert("Maple Mono".into());

        assert_eq!(
            detect_presence_with_probe(&entry, None, &probe),
            InstallPresence::Present
        );
    }

    #[test]
    fn detects_font_from_font_dir_probe() {
        let entry = tool_entry(
            "font-maple-mono-nf-cn",
            "font-maple-mono-nf-cn",
            "font",
            "Maple Mono",
        );
        let mut probe = FakePresenceProbe::default();
        probe.font_tokens.insert("maplemono".into());

        assert_eq!(
            detect_presence_with_probe(&entry, None, &probe),
            InstallPresence::Present
        );
    }

    #[test]
    fn parses_brew_cask_package() {
        assert_eq!(
            cask_package_from_command("brew install --cask ghostty"),
            Some("ghostty".into())
        );
        assert_eq!(cask_package_from_command("brew install fish"), None);
    }

    #[test]
    fn derives_simple_app_bundle_name() {
        assert_eq!(app_bundle_name("ghostty"), "Ghostty.app");
        assert_eq!(
            app_bundle_name("visual-studio-code"),
            "VisualStudioCode.app"
        );
    }

    #[test]
    fn builds_font_match_token_from_family() {
        let entry = ToolEntry {
            name: "font-maple-mono-nf-cn".into(),
            binary: "font-maple-mono-nf-cn".into(),
            layer: "enhancement".into(),
            kind: "font".into(),
            source_url: String::new(),
            font_family: "Maple Mono".into(),
            platforms: BTreeMap::new(),
        };
        assert_eq!(font_match_token(&entry), "maplemono");
    }
}
