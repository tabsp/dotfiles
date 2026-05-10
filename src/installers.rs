use crate::config::{InstallEntry, Installer};
use crate::path::which;
use std::process::Stdio;
use std::process::Command;

pub fn is_installed(command: &str, entry: &InstallEntry) -> Result<bool, String> {
    match entry.installer {
        Installer::System => Ok(which(command).is_some()),
        Installer::Brew => package_command("brew", &["list", "--formula", package(entry)?]),
        Installer::Cask => package_command("brew", &["list", "--cask", package(entry)?]),
        Installer::Apt | Installer::RepoPackage => {
            package_command("dpkg", &["-s", package(entry)?])
        }
        Installer::OfficialScript => {
            if let Some(path) = string_param(entry, "install_to") {
                Ok(crate::path::expand_home(path)?.exists())
            } else {
                Ok(which(command).is_some())
            }
        }
        Installer::DownloadBinary => {
            let path = string_param(entry, "install_to")
                .ok_or_else(|| "download_binary missing install_to".to_string())?;
            Ok(crate::path::expand_home(path)?.exists())
        }
    }
}

pub fn install_missing(command: &str, entry: &InstallEntry) -> Result<(), String> {
    if is_installed(command, entry)? {
        return Ok(());
    }

    match entry.installer {
        Installer::System => Err(format!("missing system command: {command}")),
        Installer::Brew => run("brew", &["install", package(entry)?]),
        Installer::Cask => run("brew", &["install", "--cask", package(entry)?]),
        Installer::Apt => run("sudo", &["apt-get", "install", "-y", package(entry)?]),
        Installer::RepoPackage => {
            Err("repo_package installer execution is deferred from the first runnable slice"
                .to_string())
        }
        Installer::OfficialScript => {
            Err("official_script installer execution is deferred from the first runnable slice"
                .to_string())
        }
        Installer::DownloadBinary => {
            Err("download_binary installer execution is deferred from the first runnable slice"
                .to_string())
        }
    }
}

fn package(entry: &InstallEntry) -> Result<&str, String> {
    string_param(entry, "package").ok_or_else(|| "missing package param".to_string())
}

fn string_param<'a>(entry: &'a InstallEntry, key: &str) -> Option<&'a str> {
    entry.params.get(key)?.as_str()
}

fn package_command(command: &str, args: &[&str]) -> Result<bool, String> {
    let status = Command::new(command)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("failed to run {command}: {err}"))?;
    Ok(status.success())
}

fn run(command: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|err| format!("failed to run {command}: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command} exited {status}"))
    }
}
