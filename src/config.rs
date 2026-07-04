//! YAML config loading.
//!
//! Parses dotman.yaml / dotman.bootstrap.yaml into Config.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawConfig {
    #[serde(default)]
    pub package_managers: RawPackageManagers,

    #[serde(default)]
    pub install: Vec<String>,

    #[serde(default)]
    pub links: RawLinks,

    #[serde(default)]
    pub create: Vec<PathBuf>,

    #[serde(default)]
    pub shell: Vec<RawShell>,

    #[serde(default)]
    pub clean: Vec<RawClean>,

    #[serde(default)]
    pub auto_install_pkg_manager: bool,

    #[serde(default)]
    pub auto_clone_repo: Option<RawCloneRepo>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawPackageManagers {
    #[serde(default)]
    pub macos: Option<String>,
    #[serde(default)]
    pub ubuntu: Option<String>,
    #[serde(default)]
    pub debian: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
    #[serde(default)]
    pub fedora: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawLink {
    pub target: PathBuf,
    pub source: PathBuf,
    #[serde(default)]
    pub backup: Option<bool>,
    #[serde(default)]
    pub relink: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum RawLinks {
    List(Vec<RawLink>),
    Map(BTreeMap<PathBuf, PathBuf>),
}

impl Default for RawLinks {
    fn default() -> Self {
        Self::List(Vec::new())
    }
}

impl RawLinks {
    fn into_vec(self) -> Vec<RawLink> {
        match self {
            Self::List(links) => links,
            Self::Map(links) => links
                .into_iter()
                .map(|(target, source)| RawLink {
                    target,
                    source,
                    backup: None,
                    relink: None,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawShell {
    pub command: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(rename = "if", default)]
    pub if_condition: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawClean {
    pub target: PathBuf,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawCloneRepo {
    pub url: String,
    pub target: PathBuf,
    #[serde(default)]
    pub branch: Option<String>,
}

/// Normalized config used by the rest of dotman.
#[derive(Debug, Clone)]
pub struct Config {
    pub path: PathBuf,
    pub package_managers: PackageManagerConfig,
    pub install: Vec<String>,
    pub links: Vec<LinkEntry>,
    pub create: Vec<PathBuf>,
    pub shell: Vec<ShellEntry>,
    pub clean: Vec<CleanEntry>,
    pub auto_install_pkg_manager: bool,
    pub auto_clone_repo: Option<CloneRepo>,
}

#[derive(Debug, Clone, Default)]
pub struct PackageManagerConfig {
    pub macos: Option<String>,
    pub ubuntu: Option<String>,
    pub debian: Option<String>,
    pub arch: Option<String>,
    pub fedora: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LinkEntry {
    pub target: PathBuf,
    pub source: PathBuf,
    pub backup: Option<bool>,
    pub relink: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ShellEntry {
    pub command: String,
    pub description: Option<String>,
    pub optional: bool,
    pub if_condition: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CleanEntry {
    pub target: PathBuf,
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct CloneRepo {
    pub url: String,
    pub target: PathBuf,
    pub branch: Option<String>,
}

pub fn load(path: &Path) -> Result<Config> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let raw: RawConfig = serde_yaml::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    let config = normalize(raw, path);
    Ok(config)
}

fn normalize(raw: RawConfig, path: &Path) -> Config {
    Config {
        path: path.to_path_buf(),
        package_managers: PackageManagerConfig {
            macos: raw.package_managers.macos,
            ubuntu: raw.package_managers.ubuntu,
            debian: raw.package_managers.debian,
            arch: raw.package_managers.arch,
            fedora: raw.package_managers.fedora,
        },
        install: raw.install,
        links: raw
            .links
            .into_vec()
            .into_iter()
            .map(|l| LinkEntry {
                target: l.target,
                source: l.source,
                backup: l.backup,
                relink: l.relink,
            })
            .collect(),
        create: raw.create,
        shell: raw
            .shell
            .into_iter()
            .map(|s| ShellEntry {
                command: s.command,
                description: s.description,
                optional: s.optional,
                if_condition: s.if_condition,
            })
            .collect(),
        clean: raw
            .clean
            .into_iter()
            .map(|c| CleanEntry {
                target: c.target,
                force: c.force,
            })
            .collect(),
        auto_install_pkg_manager: raw.auto_install_pkg_manager,
        auto_clone_repo: raw.auto_clone_repo.map(|c| CloneRepo {
            url: c.url,
            target: c.target,
            branch: c.branch,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_config() {
        let yaml = r#"
install: [nvim, fish]
links:
  - target: ~/.config/fish
    source: config/fish
"#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.install, vec!["nvim", "fish"]);
        let links = raw.links.into_vec();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target.to_string_lossy(), "~/.config/fish");
    }

    #[test]
    fn parses_link_map_shorthand() {
        let yaml = r#"
links:
  ~/.config/fish: config/fish
  ~/.tmux.conf: config/tmux.conf
"#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        let links = raw.links.into_vec();
        assert_eq!(links.len(), 2);
        assert!(links.iter().any(|link| {
            link.target.to_string_lossy() == "~/.config/fish"
                && link.source.to_string_lossy() == "config/fish"
        }));
        assert!(links.iter().any(|link| {
            link.target.to_string_lossy() == "~/.tmux.conf"
                && link.source.to_string_lossy() == "config/tmux.conf"
        }));
    }

    #[test]
    fn parses_package_managers() {
        let yaml = r#"
package_managers:
  macos: brew
  ubuntu: brew
  arch: pacman
install: []
"#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.package_managers.macos.as_deref(), Some("brew"));
        assert_eq!(raw.package_managers.arch.as_deref(), Some("pacman"));
        assert_eq!(raw.package_managers.ubuntu.as_deref(), Some("brew"));
    }

    #[test]
    fn parses_shell_with_optional_and_if() {
        let yaml = r#"
shell:
  - command: fisher update
    description: Sync fish plugins
    optional: true
    if: command -v fisher
"#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.shell.len(), 1);
        assert_eq!(raw.shell[0].command, "fisher update");
        assert!(raw.shell[0].optional);
        assert_eq!(
            raw.shell[0].if_condition.as_deref(),
            Some("command -v fisher")
        );
    }

    #[test]
    fn parses_clean_with_force() {
        let yaml = r#"
clean:
  - target: ~/.config/old-fish
  - target: ~/.config/old-stuff
    force: true
"#;
        let raw: RawConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(raw.clean.len(), 2);
        assert!(!raw.clean[0].force);
        assert!(raw.clean[1].force);
    }

    #[test]
    fn normalizes_to_config() {
        let raw = RawConfig {
            install: vec!["fish".into()],
            ..Default::default()
        };
        let cfg = normalize(raw, Path::new("/tmp/dotman.yaml"));
        assert_eq!(cfg.install, vec!["fish"]);
        assert!(!cfg.auto_install_pkg_manager);
    }

    #[test]
    fn load_from_temp_file_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dotman.yaml");
        std::fs::write(
            &path,
            r#"
package_managers:
  macos: brew
  arch: pacman
install: [fish, tmux]
links:
  - target: ~/.config/fish
    source: config/fish
shell:
  - command: fisher update
    optional: true
"#,
        )
        .unwrap();
        let cfg = load(&path).expect("load");
        assert_eq!(cfg.install, vec!["fish", "tmux"]);
        assert_eq!(cfg.links.len(), 1);
        assert_eq!(cfg.shell.len(), 1);
        assert!(cfg.shell[0].optional);
        assert_eq!(cfg.package_managers.macos.as_deref(), Some("brew"));
    }
}
