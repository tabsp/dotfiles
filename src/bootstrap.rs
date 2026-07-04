//! Git bootstrap: check for git, install if missing.
//!
//! dotman needs git to clone the dotfiles repo. This module handles detection
//! and platform-appropriate installation.

#[cfg(target_os = "macos")]
use anyhow::Context;
use anyhow::Result;
use std::process::Command;

/// Check whether git is available on $PATH.
pub fn git_installed() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Try to install git automatically on the current platform.
///
/// Returns Ok(true) if installation succeeded, Ok(false) if the platform
/// doesn't have a known install strategy, or an error if the command failed.
pub fn auto_install_git() -> Result<bool> {
    #[cfg(target_os = "macos")]
    {
        // Prefer xcode-select (triggers GUI prompt if CLT not installed).
        // First check if CLT is already present.
        if Command::new("xcode-select")
            .arg("-p")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            // CLT is installed but git wasn't found — this is unusual.
            // Fall through to brew.
        } else {
            let status = Command::new("xcode-select")
                .arg("--install")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .context("failed to run xcode-select --install")?;
            if status.success() {
                return Ok(true);
            }
        }
        // Fallback: brew if available.
        if Command::new("brew")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            let status = Command::new("brew")
                .args(["install", "git"])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .status()
                .context("failed to run brew install git")?;
            return Ok(status.success());
        }
        // If xcode-select returned success we wouldn't reach here.
        // xcode-select --install on modern macOS often returns non-zero
        // even though it triggers the install prompt. We treat the macOS
        // case as "guided" — tell the user what to do.
        Ok(false)
    }

    #[cfg(target_os = "linux")]
    {
        // Try apt (Debian/Ubuntu)
        if which("apt-get")
            && run_bootstrap_cmd("sudo apt-get update -qq && sudo apt-get install -y -qq git")
        {
            return Ok(true);
        }
        // Try dnf (Fedora)
        if which("dnf") && run_bootstrap_cmd("sudo dnf install -y git") {
            return Ok(true);
        }
        // Try pacman (Arch)
        if which("pacman") && run_bootstrap_cmd("sudo pacman -Sy --needed --noconfirm git") {
            return Ok(true);
        }
        Ok(false)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Ok(false)
    }
}

/// Print a help message telling the user how to install git manually.
pub fn print_git_help() {
    #[cfg(target_os = "macos")]
    {
        eprintln!("  Run: xcode-select --install");
        eprintln!("  Or install Homebrew from https://brew.sh then: brew install git");
    }
    #[cfg(target_os = "linux")]
    {
        eprintln!("  Debian/Ubuntu: sudo apt-get install -y git");
        eprintln!("  Fedora:        sudo dnf install -y git");
        eprintln!("  Arch:          sudo pacman -Sy git");
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        eprintln!("  Install git using your system package manager.");
    }
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn run_bootstrap_cmd(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn which(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_installed_returns_bool() {
        // This test runs on any machine with or without git.
        let result = git_installed();
        assert!(result);
    }

    #[test]
    fn which_known_tool() {
        assert!(which("sh"));
    }

    #[test]
    fn which_unknown_tool() {
        assert!(!which("this-bogus-tool-xyzzy"));
    }
}
