use crate::config::{InstallEntry, Installer};
use crate::path::which;
use crate::platform::{Host, Platform, distro_supported};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

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
                let install_to = crate::path::expand_home(path)?;
                Ok(matches!(
                    existing_install_state(&install_to)?,
                    ExistingInstall::Installed
                ))
            } else {
                Ok(which(command).is_some())
            }
        }
        Installer::DownloadBinary => {
            let install_to = crate::path::expand_home(required_string(entry, "install_to")?)?;
            Ok(matches!(
                existing_install_state(&install_to)?,
                ExistingInstall::Installed
            ))
        }
    }
}

pub fn install_missing(command: &str, entry: &InstallEntry, host: &Host) -> Result<(), String> {
    if is_installed(command, entry)? {
        return Ok(());
    }

    match entry.installer {
        Installer::System => Err(format!("missing system command: {command}")),
        Installer::Brew => run("brew", &["install", package(entry)?]),
        Installer::Cask => run("brew", &["install", "--cask", package(entry)?]),
        Installer::Apt => run("sudo", &["apt-get", "install", "-y", package(entry)?]),
        Installer::RepoPackage => install_repo_package(entry, host),
        Installer::OfficialScript => install_official_script(command, entry),
        Installer::DownloadBinary => install_download_binary(entry),
    }
}

fn package(entry: &InstallEntry) -> Result<&str, String> {
    string_param(entry, "package").ok_or_else(|| "missing package param".to_string())
}

fn string_param<'a>(entry: &'a InstallEntry, key: &str) -> Option<&'a str> {
    entry.params.get(key)?.as_str()
}

fn required_string<'a>(entry: &'a InstallEntry, key: &str) -> Result<&'a str, String> {
    string_param(entry, key).ok_or_else(|| format!("missing {key} param"))
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

pub fn cleanup_temp(temp: tempfile::TempDir, succeeded: bool) -> Result<(), String> {
    match temp.close() {
        Ok(()) => Ok(()),
        Err(err) if succeeded => {
            eprintln!("warn: failed to remove temporary directory: {err}");
            Ok(())
        }
        Err(err) => Err(format!("failed to remove temporary directory: {err}")),
    }
}

fn install_download_binary(entry: &InstallEntry) -> Result<(), String> {
    let url = required_string(entry, "url")?;
    let sha256 = required_string(entry, "sha256")?;
    let archive_kind = crate::archive::parse_archive_kind(required_string(entry, "archive_kind")?)?;
    let binary_path = required_string(entry, "binary_path")?;
    let install_to = crate::path::expand_home(required_string(entry, "install_to")?)?;

    match existing_install_state(&install_to)? {
        ExistingInstall::Installed => return Ok(()),
        ExistingInstall::Invalid(reason) => {
            return Err(format!(
                "download_binary invalid install_to={} reason={reason}",
                install_to.display()
            ));
        }
        ExistingInstall::Missing => {}
    }

    let temp = tempfile::tempdir().map_err(|err| format!("failed to create temp dir: {err}"))?;
    let result = (|| {
        let downloaded = crate::http::download_https(url, true)?;
        verify_sha256(&downloaded.bytes, sha256)?;
        let payload = crate::archive::unpack(&downloaded.bytes, &archive_kind, temp.path())?;
        let binary =
            resolve_binary_path(temp.path(), payload.as_deref(), &archive_kind, binary_path)?;
        install_binary(&binary, &install_to)?;
        Ok(())
    })();
    cleanup_temp(temp, result.is_ok())?;
    result
}

fn install_official_script(command: &str, entry: &InstallEntry) -> Result<(), String> {
    let script_url = required_string(entry, "script_url")?;
    let args = string_array_param(entry, "args")?;
    let install_to = if let Some(path) = string_param(entry, "install_to") {
        Some(crate::path::expand_home(path)?)
    } else {
        None
    };

    if let Some(install_to) = &install_to {
        match existing_install_state(install_to)? {
            ExistingInstall::Installed => return Ok(()),
            ExistingInstall::Invalid(reason) => {
                return Err(format!(
                    "official_script invalid install_to={} reason={reason}",
                    install_to.display()
                ));
            }
            ExistingInstall::Missing => {}
        }
    } else if which(command).is_some() {
        return Ok(());
    }

    let temp = tempfile::tempdir().map_err(|err| format!("failed to create temp dir: {err}"))?;
    let script_path = temp.path().join("install.sh");
    let result = (|| {
        let downloaded = crate::http::download_https(script_url, false)?;
        fs::write(&script_path, downloaded.bytes)
            .map_err(|err| format!("failed to write temporary install script: {err}"))?;

        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script_path)
                .map_err(|err| format!("failed to read script permissions: {err}"))?
                .permissions();
            perms.set_mode(0o700);
            fs::set_permissions(&script_path, perms)
                .map_err(|err| format!("failed to set script executable bit: {err}"))?;
        }

        let mut child = Command::new(&script_path);
        child.args(args.iter().map(String::as_str));
        let output = child
            .output()
            .map_err(|err| format!("failed to execute official_script: {err}"))?;
        if !output.status.success() {
            let mut message = format!("official_script exited {}", output.status);
            if let Some(context) = crate::process::failure_context(&output) {
                message.push('\n');
                message.push_str(&context);
            }
            return Err(message);
        }

        if let Some(install_to) = &install_to {
            match existing_install_state(install_to)? {
                ExistingInstall::Installed => {}
                ExistingInstall::Invalid(reason) => {
                    return Err(format!(
                        "official_script completed but install_to is invalid: {} ({reason})",
                        install_to.display()
                    ));
                }
                ExistingInstall::Missing => {
                    return Err(format!(
                        "official_script completed but install_to is missing: {}",
                        install_to.display()
                    ));
                }
            }
        } else if which(command).is_none() {
            return Err(format!(
                "official_script completed but command not found in PATH: {command}"
            ));
        }
        Ok(())
    })();
    cleanup_temp(temp, result.is_ok())?;
    result
}

enum ExistingInstall {
    Installed,
    Invalid(String),
    Missing,
}

fn existing_install_state(path: &Path) -> Result<ExistingInstall, String> {
    if !path.exists() {
        return Ok(ExistingInstall::Missing);
    }
    let metadata = fs::symlink_metadata(path).map_err(|err| {
        format!(
            "failed to read existing install target {}: {err}",
            path.display()
        )
    })?;
    if !metadata.is_file() {
        return Ok(ExistingInstall::Invalid(
            "target exists but is not a regular file".to_string(),
        ));
    }
    #[cfg(unix)]
    {
        if metadata.permissions().mode() & 0o111 == 0 {
            return Ok(ExistingInstall::Invalid(
                "target exists but is not executable".to_string(),
            ));
        }
    }
    Ok(ExistingInstall::Installed)
}

fn verify_sha256(bytes: &[u8], expected_hex: &str) -> Result<(), String> {
    let digest = Sha256::digest(bytes);
    let actual = format!("{:x}", digest);
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(format!(
            "sha256 mismatch: expected {expected_hex}, got {actual}"
        ))
    }
}

fn string_array_param(entry: &InstallEntry, key: &str) -> Result<Vec<String>, String> {
    let Some(value) = entry.params.get(key) else {
        return Ok(Vec::new());
    };
    let Some(array) = value.as_array() else {
        return Err(format!("{key} must be an array of strings"));
    };
    let mut out = Vec::with_capacity(array.len());
    for item in array {
        let Some(text) = item.as_str() else {
            return Err(format!("{key} must be an array of strings"));
        };
        out.push(text.to_string());
    }
    Ok(out)
}

fn resolve_binary_path(
    temp_dir: &Path,
    raw_payload: Option<&Path>,
    archive_kind: &crate::archive::ArchiveKind,
    binary_path: &str,
) -> Result<PathBuf, String> {
    match archive_kind {
        crate::archive::ArchiveKind::Raw => raw_payload
            .map(Path::to_path_buf)
            .ok_or_else(|| "raw archive payload missing".to_string()),
        _ => {
            let path = temp_dir.join(binary_path);
            if path.exists() {
                Ok(path)
            } else {
                Err(format!("binary_path not found after unpack: {binary_path}"))
            }
        }
    }
}

fn install_binary(source: &Path, install_to: &Path) -> Result<(), String> {
    if let Some(parent) = install_to.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create install directory {}: {err}",
                parent.display()
            )
        })?;
    }
    fs::copy(source, install_to).map_err(|err| {
        format!(
            "failed to copy binary {} -> {}: {err}",
            source.display(),
            install_to.display()
        )
    })?;
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(install_to)
            .map_err(|err| {
                format!(
                    "failed to read permissions on {}: {err}",
                    install_to.display()
                )
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(install_to, perms)
            .map_err(|err| format!("failed to set executable permissions: {err}"))?;
    }
    Ok(())
}

fn install_repo_package(entry: &InstallEntry, host: &Host) -> Result<(), String> {
    if host.platform != Platform::Linux || !distro_supported(host) {
        return Err("repo_package supports Ubuntu/Debian Linux only".to_string());
    }

    let package = required_string(entry, "package")?;
    let repo_url = required_string(entry, "repo_url")?;
    let repo_key_url = required_string(entry, "repo_key_url")?;
    let repo_channel = required_string(entry, "repo_channel")?;
    let repo_components = string_array_param(entry, "repo_components")?;
    if repo_components.is_empty() {
        return Err("repo_components must be a non-empty string array".to_string());
    }

    let temp = tempfile::tempdir().map_err(|err| format!("failed to create temp dir: {err}"))?;
    let result = (|| {
        let key_download = crate::http::download_https(repo_key_url, false)?;
        let key_asc_path = temp.path().join("repo-key.asc");
        fs::write(&key_asc_path, &key_download.bytes)
            .map_err(|err| format!("failed to write temporary key file: {err}"))?;

        let keyring_name = format!("{package}-dotman.gpg");
        let keyring_dest = format!("/usr/share/keyrings/{keyring_name}");
        let temp_key_gpg = temp.path().join("repo-key.gpg");
        let temp_key_gpg_s = temp_key_gpg.to_string_lossy().to_string();
        if is_armored_pgp_key(&key_download.bytes) {
            let key_asc_s = key_asc_path.to_string_lossy().to_string();
            let dearmor_output = crate::process::run_capture(
                "gpg",
                &["--dearmor", "-o", &temp_key_gpg_s, &key_asc_s],
            )?;
            if !dearmor_output.status.success() {
                let mut message = format!("gpg --dearmor exited {}", dearmor_output.status);
                if let Some(context) = crate::process::failure_context(&dearmor_output) {
                    message.push('\n');
                    message.push_str(&context);
                }
                return Err(message);
            }
        } else {
            fs::write(&temp_key_gpg, &key_download.bytes)
                .map_err(|err| format!("failed to write temporary keyring file: {err}"))?;
        }

        let key_bytes = fs::read(&temp_key_gpg)
            .map_err(|err| format!("failed to read temporary keyring file: {err}"))?;
        install_if_different(&temp_key_gpg_s, &keyring_dest, &key_bytes)?;

        let sources_path = format!("/etc/apt/sources.list.d/{package}-dotman.list");
        let source_line = format!(
            "deb [signed-by={}] {} {} {}\n",
            keyring_dest,
            repo_url,
            repo_channel,
            repo_components.join(" ")
        );
        let temp_sources = temp.path().join(format!("{package}.list"));
        {
            let mut file = fs::File::create(&temp_sources)
                .map_err(|err| format!("failed to create temporary sources file: {err}"))?;
            file.write_all(source_line.as_bytes())
                .map_err(|err| format!("failed to write temporary sources file: {err}"))?;
        }
        let temp_sources_s = temp_sources.to_string_lossy().to_string();
        install_if_different(&temp_sources_s, &sources_path, source_line.as_bytes())?;

        run_capture_checked("sudo", &["apt-get", "update"])?;
        run_capture_checked("sudo", &["apt-get", "install", "-y", package])?;
        Ok(())
    })();
    cleanup_temp(temp, result.is_ok())?;
    result
}

fn run_capture_checked(command: &str, args: &[&str]) -> Result<(), String> {
    let output = crate::process::run_capture(command, args)?;
    if output.status.success() {
        return Ok(());
    }
    let mut message = format!("{command} exited {}", output.status);
    if let Some(context) = crate::process::failure_context(&output) {
        message.push('\n');
        message.push_str(&context);
    }
    Err(message)
}

fn is_armored_pgp_key(bytes: &[u8]) -> bool {
    bytes.starts_with(b"-----BEGIN PGP")
}

fn install_if_different(src: &str, dest: &str, desired_bytes: &[u8]) -> Result<(), String> {
    let current = Command::new("sudo")
        .args(["cat", dest])
        .output()
        .map_err(|err| format!("failed to read existing {dest} via sudo: {err}"))?;
    if current.status.success() && current.stdout == desired_bytes {
        return Ok(());
    }
    run_capture_checked("sudo", &["install", "-m", "0644", src, dest])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, Host, Platform};

    fn fake_entry_with(params: &[(&str, toml::Value)]) -> InstallEntry {
        let mut map = std::collections::BTreeMap::new();
        for (k, v) in params {
            map.insert((*k).to_string(), v.clone());
        }
        InstallEntry {
            installer: Installer::DownloadBinary,
            version: "1.0.0".to_string(),
            source: Some("https://example.invalid".to_string()),
            params: map,
        }
    }

    fn host_mac() -> Host {
        Host {
            platform: Platform::Mac,
            arch: Arch::Arm64,
            distro: None,
        }
    }

    fn host_linux_ubuntu() -> Host {
        Host {
            platform: Platform::Linux,
            arch: Arch::X86_64,
            distro: Some("ubuntu".to_string()),
        }
    }

    #[test]
    fn verify_sha256_accepts_expected_hash() {
        let bytes = b"hello-world";
        let expected = format!("{:x}", Sha256::digest(bytes));
        assert!(verify_sha256(bytes, &expected).is_ok());
    }

    #[test]
    fn verify_sha256_rejects_mismatch() {
        let err = verify_sha256(b"hello-world", "deadbeef").expect_err("must fail");
        assert!(err.contains("sha256 mismatch"));
    }

    #[test]
    fn existing_install_state_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("missing-bin");
        let state = existing_install_state(&path).expect("state");
        assert!(matches!(state, ExistingInstall::Missing));
    }

    #[test]
    fn existing_install_state_invalid_non_executable_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("bin");
        fs::write(&path, b"#!/bin/sh\necho hi\n").expect("write file");
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&path).expect("meta").permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&path, perms).expect("chmod");
        }
        let state = existing_install_state(&path).expect("state");
        assert!(matches!(state, ExistingInstall::Invalid(_)));
    }

    #[test]
    fn existing_install_state_installed_executable_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("bin");
        fs::write(&path, b"#!/bin/sh\necho hi\n").expect("write file");
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&path).expect("meta").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).expect("chmod");
        }
        let state = existing_install_state(&path).expect("state");
        assert!(matches!(state, ExistingInstall::Installed));
    }

    #[test]
    fn resolve_binary_path_non_raw_requires_existing_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let kind = crate::archive::ArchiveKind::TarGz;
        let err =
            resolve_binary_path(temp.path(), None, &kind, "nvim/bin/nvim").expect_err("must fail");
        assert!(err.contains("binary_path not found"));
    }

    #[test]
    fn string_array_param_parses_string_list() {
        let entry = fake_entry_with(&[(
            "args",
            toml::Value::Array(vec![
                toml::Value::String("--yes".to_string()),
                toml::Value::String("--verbose".to_string()),
            ]),
        )]);
        let args = string_array_param(&entry, "args").expect("args");
        assert_eq!(args, vec!["--yes".to_string(), "--verbose".to_string()]);
    }

    #[test]
    fn string_array_param_rejects_non_string_items() {
        let entry = fake_entry_with(&[("args", toml::Value::Array(vec![toml::Value::Integer(1)]))]);
        let err = string_array_param(&entry, "args").expect_err("must fail");
        assert!(err.contains("must be an array of strings"));
    }

    #[test]
    fn official_script_install_to_existing_non_executable_fails_before_network() {
        let temp = tempfile::tempdir().expect("tempdir");
        let install_to = temp.path().join("tool");
        fs::write(&install_to, b"content").expect("write");
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&install_to).expect("meta").permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&install_to, perms).expect("chmod");
        }
        let entry = InstallEntry {
            installer: Installer::OfficialScript,
            version: "latest".to_string(),
            source: Some("https://example.invalid".to_string()),
            params: {
                let mut map = std::collections::BTreeMap::new();
                map.insert(
                    "script_url".to_string(),
                    toml::Value::String("https://example.invalid/install.sh".to_string()),
                );
                map.insert(
                    "install_to".to_string(),
                    toml::Value::String(install_to.to_string_lossy().to_string()),
                );
                map
            },
        };

        let err =
            install_missing("non-existent-command", &entry, &host_mac()).expect_err("must fail");
        assert!(err.contains("official_script invalid install_to"));
    }

    #[test]
    fn download_binary_install_to_existing_non_executable_fails_before_network() {
        let temp = tempfile::tempdir().expect("tempdir");
        let install_to = temp.path().join("tool");
        fs::write(&install_to, b"content").expect("write");
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&install_to).expect("meta").permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&install_to, perms).expect("chmod");
        }
        let entry = InstallEntry {
            installer: Installer::DownloadBinary,
            version: "1.0.0".to_string(),
            source: Some("https://example.invalid".to_string()),
            params: {
                let mut map = std::collections::BTreeMap::new();
                map.insert(
                    "url".to_string(),
                    toml::Value::String("https://example.invalid/tool.tar.gz".to_string()),
                );
                map.insert(
                    "sha256".to_string(),
                    toml::Value::String("deadbeef".to_string()),
                );
                map.insert(
                    "archive_kind".to_string(),
                    toml::Value::String("tar.gz".to_string()),
                );
                map.insert(
                    "binary_path".to_string(),
                    toml::Value::String("tool/bin/tool".to_string()),
                );
                map.insert(
                    "install_to".to_string(),
                    toml::Value::String(install_to.to_string_lossy().to_string()),
                );
                map
            },
        };

        let err =
            install_missing("non-existent-command", &entry, &host_mac()).expect_err("must fail");
        assert!(err.contains("download_binary invalid install_to"));
    }

    #[test]
    fn repo_package_rejects_non_linux_or_unsupported_distro() {
        let entry = InstallEntry {
            installer: Installer::RepoPackage,
            version: "1.0.0".to_string(),
            source: Some("https://example.invalid".to_string()),
            params: {
                let mut map = std::collections::BTreeMap::new();
                map.insert(
                    "package".to_string(),
                    toml::Value::String("tool".to_string()),
                );
                map.insert(
                    "repo_url".to_string(),
                    toml::Value::String("https://example.invalid/repo".to_string()),
                );
                map.insert(
                    "repo_key_url".to_string(),
                    toml::Value::String("https://example.invalid/key.asc".to_string()),
                );
                map.insert(
                    "repo_channel".to_string(),
                    toml::Value::String("stable".to_string()),
                );
                map.insert(
                    "repo_components".to_string(),
                    toml::Value::Array(vec![toml::Value::String("main".to_string())]),
                );
                map
            },
        };

        let err_mac = install_repo_package(&entry, &host_mac()).expect_err("must fail on mac");
        assert!(err_mac.contains("Ubuntu/Debian Linux only"));

        let host_unsupported = Host {
            platform: Platform::Linux,
            arch: Arch::X86_64,
            distro: Some("fedora".to_string()),
        };
        let err_distro =
            install_repo_package(&entry, &host_unsupported).expect_err("must fail on distro");
        assert!(err_distro.contains("Ubuntu/Debian Linux only"));
    }

    #[test]
    fn repo_package_missing_params_fail_before_command_execution() {
        let entry = InstallEntry {
            installer: Installer::RepoPackage,
            version: "1.0.0".to_string(),
            source: Some("https://example.invalid".to_string()),
            params: std::collections::BTreeMap::new(),
        };
        let err = install_missing("tool", &entry, &host_linux_ubuntu()).expect_err("must fail");
        assert!(err.contains("missing package param"));
    }
}
