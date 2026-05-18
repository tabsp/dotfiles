#![allow(dead_code)]

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
pub struct DepsManifest {
    #[serde(default)]
    pub schema_version: Option<u32>,
    pub deps: BTreeMap<String, Dependency>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Dependency {
    pub command: String,
    #[serde(default)]
    pub version_check: Option<VersionCheck>,
    #[serde(default)]
    pub default: Option<InstallEntry>,
    #[serde(default)]
    pub mac: BTreeMap<String, InstallEntry>,
    #[serde(default)]
    pub linux: BTreeMap<String, InstallEntry>,
}

impl Dependency {
    fn merge_default<'a>(&'a self, entry: Option<&'a InstallEntry>) -> Option<InstallEntry> {
        let default = self.default.as_ref()?;
        let entry = entry?;
        let mut merged_params = default.params.clone();
        for (k, v) in &entry.params {
            merged_params.insert(k.clone(), v.clone());
        }
        Some(InstallEntry {
            installer: entry.installer,
            version: entry.version.clone(),
            source: entry.source.clone().or(default.source.clone()),
            distros: entry.distros.clone().or(default.distros.clone()),
            params: merged_params,
        })
    }

    pub fn entries_for(&self, platform: &str, arch: &str) -> Vec<InstallEntry> {
        let map = match platform {
            "mac" => &self.mac,
            "linux" => &self.linux,
            _ => return Vec::new(),
        };
        if let Some(entry) = map.get(arch) {
            vec![entry.clone()]
        } else if self.default.is_some() {
            vec![self.default.clone().unwrap()]
        } else {
            Vec::new()
        }
    }

    pub fn entries_for_host(&self, host: &crate::platform::Host) -> Vec<InstallEntry> {
        self.entries_for(host.platform.key(), host.arch.key())
            .into_iter()
            .filter(|entry| entry.matches_distro(host))
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct VersionCheck {
    #[serde(default = "default_version_args")]
    pub args: Vec<String>,
    pub regex: String,
    #[serde(default = "default_version_stream")]
    pub stream: VersionStream,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum VersionStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, Deserialize)]
pub struct InstallEntry {
    pub installer: Installer,
    pub version: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub distros: Option<Vec<String>>,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
}

impl InstallEntry {
    pub fn matches_distro(&self, host: &crate::platform::Host) -> bool {
        let Some(distros) = &self.distros else {
            return true;
        };
        if host.platform != crate::platform::Platform::Linux || distros.is_empty() {
            return false;
        }
        let Some(distro) = host.distro.as_deref() else {
            return false;
        };
        distros.iter().any(|item| item == distro)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Installer {
    System,
    Brew,
    Cask,
    Apt,
    RepoPackage,
    Ppa,
    OfficialScript,
    DownloadBinary,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DotfilesManifest {
    #[serde(default)]
    pub schema_version: Option<u32>,
    pub files: Vec<FileEntry>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FileEntry {
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub kind: Option<FileKind>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    File,
    Dir,
}

fn validate_schema_version(version: Option<u32>, path: &Path) -> Result<u32, String> {
    match version {
        None => Ok(1),
        Some(1) => Ok(1),
        Some(v) if v > 1 => Err(format!(
            "{}: manifest requires schema version {v} but dotman supports up to 1. Upgrade dotman or use an older manifest.",
            path.display()
        )),
        Some(v) => Err(format!(
            "{}: manifest schema version {v} is not supported (minimum: 1).",
            path.display()
        )),
    }
}

pub fn load_deps(path: &Path) -> Result<DepsManifest, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let manifest: DepsManifest =
        toml::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    validate_schema_version(manifest.schema_version, path)?;
    Ok(manifest)
}

pub fn load_dotfiles(path: &Path) -> Result<DotfilesManifest, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let manifest: DotfilesManifest =
        toml::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    validate_schema_version(manifest.schema_version, path)?;
    Ok(manifest)
}

fn default_enabled() -> bool {
    true
}

fn default_version_args() -> Vec<String> {
    vec!["--version".to_string()]
}

fn default_version_stream() -> VersionStream {
    VersionStream::Stdout
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, Host, Platform};

    fn entry_with_distros(distros: Option<Vec<&str>>) -> InstallEntry {
        InstallEntry {
            installer: Installer::Ppa,
            version: "latest".to_string(),
            source: None,
            distros: distros.map(|items| items.into_iter().map(str::to_string).collect()),
            params: BTreeMap::new(),
        }
    }

    fn linux_host(distro: &str) -> Host {
        Host {
            platform: Platform::Linux,
            arch: Arch::X86_64,
            distro: Some(distro.to_string()),
        }
    }

    #[test]
    fn entry_without_distros_matches_any_distro() {
        assert!(entry_with_distros(None).matches_distro(&linux_host("debian")));
    }

    #[test]
    fn entry_with_matching_distro_matches() {
        assert!(entry_with_distros(Some(vec!["ubuntu"])).matches_distro(&linux_host("ubuntu")));
    }

    #[test]
    fn entry_with_non_matching_distro_does_not_match() {
        assert!(!entry_with_distros(Some(vec!["ubuntu"])).matches_distro(&linux_host("debian")));
    }

    #[test]
    fn empty_distros_matches_no_distro() {
        assert!(!entry_with_distros(Some(vec![])).matches_distro(&linux_host("ubuntu")));
    }

    #[test]
    fn entries_for_host_filters_by_distro() {
        let mut linux = BTreeMap::new();
        linux.insert(
            "x86_64".to_string(),
            entry_with_distros(Some(vec!["ubuntu"])),
        );
        let dep = Dependency {
            command: "fish".to_string(),
            version_check: None,
            default: None,
            mac: BTreeMap::new(),
            linux,
        };

        assert_eq!(dep.entries_for_host(&linux_host("debian")).len(), 0);
        assert_eq!(dep.entries_for_host(&linux_host("ubuntu")).len(), 1);
    }

    fn basic_entry() -> InstallEntry {
        InstallEntry {
            installer: Installer::DownloadBinary,
            version: "1.0.0".to_string(),
            source: None,
            distros: None,
            params: BTreeMap::new(),
        }
    }

    #[test]
    fn entries_for_returns_arch_entry_when_present() {
        let mut mac = BTreeMap::new();
        mac.insert("arm64".to_string(), basic_entry());
        let dep = Dependency {
            command: "test".to_string(),
            version_check: None,
            default: None,
            mac,
            linux: BTreeMap::new(),
        };
        let entries = dep.entries_for("mac", "arm64");
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn entries_for_returns_default_when_no_arch_entry() {
        let dep = Dependency {
            command: "test".to_string(),
            version_check: None,
            default: Some(basic_entry()),
            mac: BTreeMap::new(),
            linux: BTreeMap::new(),
        };
        let entries = dep.entries_for("mac", "arm64");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version, "1.0.0");
    }

    #[test]
    fn entries_for_returns_empty_when_no_default_and_no_arch() {
        let dep = Dependency {
            command: "test".to_string(),
            version_check: None,
            default: None,
            mac: BTreeMap::new(),
            linux: BTreeMap::new(),
        };
        let entries = dep.entries_for("mac", "arm64");
        assert!(entries.is_empty());
    }

    #[test]
    fn entries_for_returns_empty_for_unknown_platform() {
        let dep = Dependency {
            command: "test".to_string(),
            version_check: None,
            default: Some(basic_entry()),
            mac: BTreeMap::new(),
            linux: BTreeMap::new(),
        };
        let entries = dep.entries_for("windows", "x86_64");
        assert!(entries.is_empty());
    }

    #[test]
    fn manifest_with_schema_version_1_parses() {
        let toml_str = r#"
schema_version = 1

[deps.bat]
command = "bat"
version_check = { args = ["--version"], regex = "bat ([0-9.]+)" }

[bat.default]
installer = "download-binary"
version = "0.24.0"
source = "https://example.com/bat-{version}-{target}.tar.gz"
params = { install_to = "bat", install_dir_to = "~/.local/bin" }
"#;
        let manifest: DepsManifest = toml::from_str(toml_str).expect("parse");
        assert_eq!(manifest.schema_version, Some(1));
        assert!(manifest.deps.contains_key("bat"));
    }

    #[test]
    fn manifest_without_schema_version_defaults_to_none() {
        let toml_str = r#"
[deps.fd]
command = "fd"
"#;
        let manifest: DepsManifest = toml::from_str(toml_str).expect("parse");
        assert_eq!(manifest.schema_version, None);
    }

    #[test]
    fn validate_schema_version_accepts_none() {
        let result = validate_schema_version(None, Path::new("deps.toml"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn validate_schema_version_accepts_1() {
        let result = validate_schema_version(Some(1), Path::new("deps.toml"));
        assert!(result.is_ok());
    }

    #[test]
    fn validate_schema_version_rejects_99() {
        let result = validate_schema_version(Some(99), Path::new("deps.toml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("supports up to 1"));
    }

    #[test]
    fn validate_schema_version_rejects_0() {
        let result = validate_schema_version(Some(0), Path::new("deps.toml"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not supported"));
    }

    #[test]
    fn schema_version_99_manifest_fails_to_load() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let deps_path = tmp.path().join("deps.toml");
        std::fs::write(
            &deps_path,
            r#"schema_version = 99

[deps.bat]
command = "bat"
"#,
        )
        .expect("write");
        let result = load_deps(&deps_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("supports up to 1"));
    }

    #[test]
    fn schema_version_1_manifest_loads() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let deps_path = tmp.path().join("deps.toml");
        std::fs::write(
            &deps_path,
            r#"schema_version = 1

[deps.bat]
command = "bat"
"#,
        )
        .expect("write");
        let result = load_deps(&deps_path);
        assert!(result.is_ok());
    }
}
