//! Platform detection and package manager decision.
//!
//! Phase 8 will implement: OS detection, install brew/pacman/dnf automatically.

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

pub fn pkg_mgr_for(_os: Os) -> Result<String> {
    anyhow::bail!("package manager selection not yet implemented (Phase 8)")
}

pub fn dotman_data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine data local dir"))?
        .join("dotman");
    Ok(dir)
}
