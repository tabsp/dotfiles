//! Platform detection and package manager decision.

use crate::config::PackageManagerConfig;
use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Mac,
    Linux,
    Unknown,
}

pub fn detect_os() -> Os {
    if cfg!(target_os = "macos") {
        Os::Mac
    } else {
        Os::Linux
    }
}

/// Resolve the configured package manager name for the current OS.
pub fn resolve_pkg_mgr_name(cfg: &PackageManagerConfig) -> Option<String> {
    match detect_os() {
        Os::Mac => cfg.macos.clone(),
        Os::Linux => detect_distro_pkg_mgr(cfg),
        Os::Unknown => None,
    }
}

fn detect_distro_pkg_mgr(cfg: &PackageManagerConfig) -> Option<String> {
    let contents = std::fs::read_to_string("/etc/os-release").ok()?;
    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("ID=") {
            let id = rest.trim().trim_matches('"');
            return match id {
                "ubuntu" => cfg.ubuntu.clone(),
                "debian" => cfg.debian.clone(),
                "arch" => cfg.arch.clone(),
                "fedora" => cfg.fedora.clone(),
                _ => None,
            };
        }
    }
    None
}

pub fn dotman_data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine data local dir"))?
        .join("dotman");
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PackageManagerConfig;

    #[test]
    fn detect_os_is_mac_or_linux() {
        let os = detect_os();
        assert!(os == Os::Mac || os == Os::Linux);
    }

    #[test]
    fn resolve_pkg_mgr_name_returns_macos_config() {
        let cfg = PackageManagerConfig {
            macos: Some("brew".into()),
            ..Default::default()
        };
        // On macOS this returns Some("brew"), on Linux None (no /etc/os-release reads "macos").
        let result = resolve_pkg_mgr_name(&cfg);
        if cfg!(target_os = "macos") {
            assert_eq!(result, Some("brew".into()));
        }
        // On Linux, no assertion — depends on /etc/os-release
    }

    #[test]
    fn dotman_data_dir_ends_with_dotman() {
        let dir = dotman_data_dir().unwrap();
        assert!(dir.ends_with("dotman"));
    }

    #[test]
    fn os_debug_roundtrip() {
        assert_eq!(format!("{:?}", Os::Mac), "Mac");
        assert_eq!(format!("{:?}", Os::Linux), "Linux");
        assert_eq!(format!("{:?}", Os::Unknown), "Unknown");
    }
}
