#![allow(dead_code)]

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct DepsManifest {
    pub deps: BTreeMap<String, Dependency>,
}

#[derive(Debug, Deserialize)]
pub struct Dependency {
    pub command: String,
    #[serde(default)]
    pub version_check: Option<VersionCheck>,
    #[serde(default)]
    pub mac: BTreeMap<String, InstallEntry>,
    #[serde(default)]
    pub linux: BTreeMap<String, InstallEntry>,
}

impl Dependency {
    pub fn entries_for(&self, platform: &str, arch: &str) -> Vec<&InstallEntry> {
        match platform {
            "mac" => self.mac.get(arch).into_iter().collect(),
            "linux" => self.linux.get(arch).into_iter().collect(),
            _ => Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct InstallEntry {
    pub installer: Installer,
    pub version: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Installer {
    System,
    Brew,
    Cask,
    Apt,
    RepoPackage,
    OfficialScript,
    DownloadBinary,
}

#[derive(Debug, Deserialize)]
pub struct DotfilesManifest {
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Deserialize)]
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

pub fn load_deps(path: &Path) -> Result<DepsManifest, String> {
    let raw =
        fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

pub fn load_dotfiles(path: &Path) -> Result<DotfilesManifest, String> {
    let raw =
        fs::read_to_string(path).map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    toml::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
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
