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
use std::time::{SystemTime, UNIX_EPOCH};

pub fn is_installed(command: &str, entry: &InstallEntry) -> Result<bool, String> {
    match entry.installer {
        Installer::System => Ok(which(command).is_some()),
        Installer::Brew => package_command("brew", &["list", "--formula", package(entry)?]),
        Installer::Cask => package_command("brew", &["list", "--cask", package(entry)?]),
        Installer::Apt | Installer::RepoPackage | Installer::Ppa => {
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
            if let Some(path) = string_param(entry, "install_dir_to") {
                return archive_dir_install_state(&install_to, &crate::path::expand_home(path)?);
            }
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
        Installer::Ppa => install_ppa(entry, host),
        Installer::OfficialScript => install_official_script(command, entry),
        Installer::DownloadBinary => install_download_binary(entry),
    }
}

fn package(entry: &InstallEntry) -> Result<&str, String> {
    string_param(entry, "package").ok_or_else(|| "missing package param".to_string())
}

fn bootstrap_package(entry: &InstallEntry) -> &str {
    string_param(entry, "bootstrap_package").unwrap_or("software-properties-common")
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
    let install_dir_from = string_param(entry, "install_dir_from");
    let install_dir_to = string_param(entry, "install_dir_to")
        .map(crate::path::expand_home)
        .transpose()?;

    if install_dir_from.is_none() {
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
    }

    let temp = tempfile::tempdir().map_err(|err| format!("failed to create temp dir: {err}"))?;
    let result = (|| {
        let downloaded = crate::http::download_https(url, true)?;
        verify_sha256(&downloaded.bytes, sha256)?;
        let payload = crate::archive::unpack(&downloaded.bytes, &archive_kind, temp.path())?;
        let binary =
            resolve_binary_path(temp.path(), payload.as_deref(), &archive_kind, binary_path)?;
        if let (Some(install_dir_from), Some(install_dir_to)) = (install_dir_from, &install_dir_to)
        {
            install_archive_dir(
                temp.path(),
                install_dir_from,
                install_dir_to,
                binary_path,
                &install_to,
            )?;
        } else {
            install_binary(&binary, &install_to)?;
        }
        Ok(())
    })();
    cleanup_temp(temp, result.is_ok())?;
    result
}

fn install_official_script(command: &str, entry: &InstallEntry) -> Result<(), String> {
    let script_url = required_string(entry, "script_url")?;
    let args = expand_script_args(string_array_param(entry, "args")?)?;
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

fn expand_script_args(args: Vec<String>) -> Result<Vec<String>, String> {
    args.iter().map(|arg| expand_script_arg(arg)).collect()
}

fn expand_script_arg(arg: &str) -> Result<String, String> {
    if arg.starts_with("~/") {
        return crate::path::expand_home(arg).map(|path| path.to_string_lossy().into_owned());
    }
    Ok(arg.to_string())
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

fn archive_dir_install_state(install_to: &Path, install_dir_to: &Path) -> Result<bool, String> {
    if !install_dir_to.is_dir() {
        return Ok(false);
    }
    let metadata = match fs::metadata(install_to) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => {
            return Err(format!(
                "failed to read existing install target {}: {err}",
                install_to.display()
            ));
        }
    };
    if !metadata.is_file() {
        return Ok(false);
    }
    #[cfg(unix)]
    {
        if metadata.permissions().mode() & 0o111 == 0 {
            return Ok(false);
        }
    }
    Ok(true)
}

fn sibling_tempdir(parent: &Path, prefix: &str) -> Result<PathBuf, String> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let pid = std::process::id();
    let name = format!("{prefix}.staging-{pid}-{ts}");
    let path = parent.join(&name);
    fs::create_dir(&path).map_err(|err| {
        format!(
            "failed to create staging directory {}: {err}",
            path.display()
        )
    })?;
    Ok(path)
}

fn install_archive_dir(
    temp_dir: &Path,
    install_dir_from: &str,
    install_dir_to: &Path,
    binary_path: &str,
    install_to: &Path,
) -> Result<(), String> {
    let source_dir = temp_dir.join(install_dir_from);
    if !source_dir.is_dir() {
        return Err(format!(
            "install_dir_from not found after unpack: {install_dir_from}"
        ));
    }

    let binary = temp_dir.join(binary_path);
    if !binary.is_file() {
        return Err(format!("binary_path not found after unpack: {binary_path}"));
    }

    let parent_dir = install_dir_to.parent().ok_or_else(|| {
        "install_dir_to has no parent directory".to_string()
    })?;
    fs::create_dir_all(parent_dir).map_err(|err| {
        format!(
            "failed to create install directory {}: {err}",
            parent_dir.display()
        )
    })?;

    let staging = sibling_tempdir(
        parent_dir,
        install_dir_to
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("staging"),
    )?;

    // Copy into staging. If this fails, the old install is still intact.
    if let Err(err) = copy_dir_recursive(&source_dir, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("AGENT_ARCHIVE_DIR_STAGE_FAILED: failed to stage directory install: {err}"));
    }

    let old_path = if install_dir_to.exists() {
        let old = install_dir_to.with_file_name(format!(
            "{}.old",
            install_dir_to
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("backup")
        ));
        // If an old backup from a previous failed run exists, remove it first.
        if old.exists() {
            fs::remove_dir_all(&old).map_err(|err| {
                format!("failed to remove leftover backup {}: {err}", old.display())
            })?;
        }
        fs::rename(install_dir_to, &old).map_err(|err| {
            let _ = fs::remove_dir_all(&staging);
            format!(
                "AGENT_ARCHIVE_DIR_RENAME_FAILED: failed to move old install directory {}: {err}",
                install_dir_to.display()
            )
        })?;
        Some(old)
    } else {
        None
    };

    // Promote staging to final location.
    if let Err(err) = fs::rename(&staging, install_dir_to) {
        // Attempt to restore the old directory.
        if let Some(ref old) = old_path
            && old.exists()
        {
            let _ = fs::rename(old, install_dir_to);
        }
        let _ = fs::remove_dir_all(&staging);
        return Err(format!(
            "AGENT_ARCHIVE_DIR_PROMOTE_FAILED: failed to promote staged directory to {}: {err}",
            install_dir_to.display()
        ));
    }

    // Symlink setup.
    let relative_binary = Path::new(binary_path)
        .strip_prefix(install_dir_from)
        .map_err(|_| "binary_path must be inside install_dir_from".to_string())?;
    let target_binary = install_dir_to.join(relative_binary);

    if let Some(parent) = install_to.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create install directory {}: {err}",
                parent.display()
            )
        })?;
    }
    if install_to.exists() {
        fs::remove_file(install_to).map_err(|err| {
            format!(
                "failed to remove existing binary {}: {err}",
                install_to.display()
            )
        })?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target_binary, install_to).map_err(|err| {
            format!(
                "AGENT_ARCHIVE_DIR_SYMLINK_FAILED: failed to link binary {} -> {}: {err}",
                install_to.display(),
                target_binary.display()
            )
        })?;
    }
    #[cfg(not(unix))]
    {
        install_binary(&target_binary, install_to)?;
    }

    // Clean up the old backup.
    if let Some(old) = old_path
        && old.exists()
        && let Err(err) = fs::remove_dir_all(&old)
    {
        eprintln!(
            "AGENT_ARCHIVE_DIR_CLEANUP_FAILED: warn: failed to remove old install backup {}: {err}",
            old.display()
        );
    }

    Ok(())
}


fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest)
        .map_err(|err| format!("failed to create directory {}: {err}", dest.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|err| format!("failed to read directory {}: {err}", source.display()))?
    {
        let entry = entry.map_err(|err| format!("failed to read directory entry: {err}"))?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|err| format!("failed to read file type {}: {err}", source_path.display()))?;
        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &dest_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &dest_path).map_err(|err| {
                format!(
                    "failed to copy file {} -> {}: {err}",
                    source_path.display(),
                    dest_path.display()
                )
            })?;
            let perms = fs::metadata(&source_path)
                .map_err(|err| {
                    format!(
                        "failed to read permissions on {}: {err}",
                        source_path.display()
                    )
                })?
                .permissions();
            fs::set_permissions(&dest_path, perms)
                .map_err(|err| format!("failed to preserve file permissions: {err}"))?;
        } else if file_type.is_symlink() {
            // TODO: preserve symlinks if an upstream archive starts using them.
        }
    }
    Ok(())
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

fn install_ppa(entry: &InstallEntry, host: &Host) -> Result<(), String> {
    if host.platform != Platform::Linux || host.distro.as_deref() != Some("ubuntu") {
        return Err("ppa supports Ubuntu Linux only".to_string());
    }

    let package = package(entry)?;
    let ppa = required_string(entry, "ppa")?;
    let bootstrap = bootstrap_package(entry);

    if !package_command("dpkg", &["-s", bootstrap])? {
        run_capture_checked("sudo", &["apt-get", "install", "-y", bootstrap])?;
    }

    run_capture_checked("sudo", &["add-apt-repository", "-y", ppa])?;
    run_capture_checked("sudo", &["apt-get", "update"])?;
    run_capture_checked("sudo", &["apt-get", "install", "-y", package])
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
            distros: None,
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

    #[cfg(unix)]
    #[test]
    fn existing_install_state_keeps_symlink_invalid_for_regular_installers() {
        let temp = tempfile::tempdir().expect("tempdir");
        let target = temp.path().join("target");
        let link = temp.path().join("tool");
        fs::write(&target, b"#!/bin/sh\necho hi\n").expect("write target");
        let mut perms = fs::metadata(&target).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target, perms).expect("chmod");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let state = existing_install_state(&link).expect("state");
        assert!(matches!(state, ExistingInstall::Invalid(_)));
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
    fn install_archive_dir_preserves_runtime_and_links_binary() {
        let temp = tempfile::tempdir().expect("tempdir");
        let unpacked = temp.path().join("nvim-linux-x86_64");
        fs::create_dir_all(unpacked.join("bin")).expect("bin dir");
        fs::create_dir_all(unpacked.join("share/nvim/runtime/lua/vim")).expect("runtime dir");
        fs::write(unpacked.join("bin/nvim"), b"#!/bin/sh\necho nvim\n").expect("binary");
        fs::write(
            unpacked.join("share/nvim/runtime/lua/vim/uri.lua"),
            b"return {}\n",
        )
        .expect("runtime file");
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(unpacked.join("bin/nvim"))
                .expect("meta")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(unpacked.join("bin/nvim"), perms).expect("chmod");
        }

        let install_dir_to = temp.path().join("opt/nvim");
        let install_to = temp.path().join("bin/nvim");
        install_archive_dir(
            temp.path(),
            "nvim-linux-x86_64",
            &install_dir_to,
            "nvim-linux-x86_64/bin/nvim",
            &install_to,
        )
        .expect("install archive dir");

        assert!(
            install_dir_to
                .join("share/nvim/runtime/lua/vim/uri.lua")
                .exists()
        );
        let installed_binary = fs::canonicalize(&install_to).expect("canonicalize installed bin");
        let expected_binary =
            fs::canonicalize(install_dir_to.join("bin/nvim")).expect("canonicalize target bin");
        assert_eq!(installed_binary, expected_binary);
    }

    #[test]
    fn download_binary_with_install_dir_requires_directory_to_be_installed() {
        let temp = tempfile::tempdir().expect("tempdir");
        let install_to = temp.path().join("bin/nvim");
        fs::create_dir_all(install_to.parent().expect("parent")).expect("bin dir");
        fs::write(&install_to, b"#!/bin/sh\necho old nvim\n").expect("binary");
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&install_to).expect("meta").permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&install_to, perms).expect("chmod");
        }
        let missing_install_dir = temp.path().join("opt/nvim");
        let entry = fake_entry_with(&[
            (
                "url",
                toml::Value::String("https://example.invalid/nvim.tar.gz".to_string()),
            ),
            ("sha256", toml::Value::String("deadbeef".to_string())),
            ("archive_kind", toml::Value::String("tar.gz".to_string())),
            (
                "binary_path",
                toml::Value::String("nvim/bin/nvim".to_string()),
            ),
            (
                "install_to",
                toml::Value::String(install_to.to_string_lossy().to_string()),
            ),
            ("install_dir_from", toml::Value::String("nvim".to_string())),
            (
                "install_dir_to",
                toml::Value::String(missing_install_dir.to_string_lossy().to_string()),
            ),
        ]);

        assert!(!is_installed("nvim", &entry).expect("installed check"));
    }

    #[cfg(unix)]
    #[test]
    fn download_binary_with_install_dir_ignores_existing_symlink_when_target_needs_reinstall() {
        let temp = tempfile::tempdir().expect("tempdir");
        let install_dir_to = temp.path().join("opt/nvim");
        let install_to = temp.path().join("bin/nvim");
        let target = install_dir_to.join("bin/nvim");
        fs::create_dir_all(target.parent().expect("target parent")).expect("target dir");
        fs::write(&target, b"#!/bin/sh\necho broken\n").expect("target");
        let mut perms = fs::metadata(&target).expect("meta").permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&target, perms).expect("chmod");
        fs::create_dir_all(install_to.parent().expect("parent")).expect("bin dir");
        std::os::unix::fs::symlink(&target, &install_to).expect("symlink");
        let entry = fake_entry_with(&[
            (
                "url",
                toml::Value::String("https://example.invalid/nvim.tar.gz".to_string()),
            ),
            ("sha256", toml::Value::String("deadbeef".to_string())),
            ("archive_kind", toml::Value::String("tar.gz".to_string())),
            (
                "binary_path",
                toml::Value::String("nvim/bin/nvim".to_string()),
            ),
            (
                "install_to",
                toml::Value::String(install_to.to_string_lossy().to_string()),
            ),
            ("install_dir_from", toml::Value::String("nvim".to_string())),
            (
                "install_dir_to",
                toml::Value::String(install_dir_to.to_string_lossy().to_string()),
            ),
        ]);

        let err = install_download_binary(&entry).expect_err("must reach download");
        assert!(!err.contains("download_binary invalid install_to"));
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
            distros: None,
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
            distros: None,
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
            distros: None,
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
            distros: None,
            params: std::collections::BTreeMap::new(),
        };
        let err = install_missing("tool", &entry, &host_linux_ubuntu()).expect_err("must fail");
        assert!(err.contains("missing package param"));
    }

    #[test]
    fn ppa_bootstrap_package_defaults_to_software_properties_common() {
        let entry = fake_entry_with(&[]);
        assert_eq!(bootstrap_package(&entry), "software-properties-common");
    }

    #[test]
    fn ppa_bootstrap_package_can_be_overridden() {
        let entry = fake_entry_with(&[(
            "bootstrap_package",
            toml::Value::String("custom-package".to_string()),
        )]);
        assert_eq!(bootstrap_package(&entry), "custom-package");
    }

    #[test]
    fn ppa_rejects_non_ubuntu_before_command_execution() {
        let entry = InstallEntry {
            installer: Installer::Ppa,
            version: "latest".to_string(),
            source: Some("https://example.invalid".to_string()),
            distros: None,
            params: std::collections::BTreeMap::new(),
        };
        let err_mac = install_ppa(&entry, &host_mac()).expect_err("must fail on mac");
        assert!(err_mac.contains("Ubuntu Linux only"));

        let host_debian = Host {
            platform: Platform::Linux,
            arch: Arch::X86_64,
            distro: Some("debian".to_string()),
        };
        let err_debian = install_ppa(&entry, &host_debian).expect_err("must fail on debian");
        assert!(err_debian.contains("Ubuntu Linux only"));
    }

    #[test]
    fn expand_script_arg_expands_leading_home() {
        let home = std::env::var("HOME").expect("HOME");
        assert_eq!(
            expand_script_arg("~/.local/bin").expect("expand"),
            format!("{home}/.local/bin")
        );
    }

    #[test]
    fn expand_script_arg_keeps_env_var_literal() {
        assert_eq!(
            expand_script_arg("$HOME/.local/bin").expect("expand"),
            "$HOME/.local/bin"
        );
    }

    #[test]
    fn expand_script_arg_keeps_embedded_tilde_literal() {
        assert_eq!(
            expand_script_arg("prefix~/path").expect("expand"),
            "prefix~/path"
        );
    }


    // --- Atomic directory install tests ---

    fn setup_fake_archive_dir(temp: &Path, dir_name: &str, binary_name: &str) -> Result<(), String> {
        let source = temp.join(dir_name);
        fs::create_dir_all(&source).map_err(|e| format!("create source dir: {e}"))?;
        let bin = source.join(binary_name);
        fs::write(&bin, b"fake binary").map_err(|e| format!("write binary: {e}"))?;
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&bin).map_err(|e| format!("metadata: {e}"))?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&bin, perms).map_err(|e| format!("chmod: {e}"))?;
        }
        Ok(())
    }

    #[test]
    fn atomic_install_first_time_no_existing_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let install_root = tempfile::tempdir().expect("install root");
        let install_dir_to = install_root.path().join("mytool");
        let install_to = install_root.path().join("bin").join("mytool");

        setup_fake_archive_dir(temp.path(), "mytool-v1.0", "mytool").expect("setup");
        install_archive_dir(
            temp.path(),
            "mytool-v1.0",
            &install_dir_to,
            "mytool-v1.0/mytool",
            &install_to,
        )
        .expect("install_archive_dir should succeed");

        assert!(install_dir_to.exists());
        assert!(install_dir_to.join("mytool").exists());
        assert!(!install_dir_to.with_file_name(format!(
            "{}.old",
            install_dir_to.file_name().unwrap().to_str().unwrap()
        )).exists());
        #[cfg(unix)]
        {
            let link_target = fs::read_link(&install_to).expect("read_link");
            assert_eq!(link_target, install_dir_to.join("mytool"));
        }
    }

    #[test]
    fn atomic_install_upgrade_existing_dir() {
        let temp = tempfile::tempdir().expect("tempdir");
        let install_root = tempfile::tempdir().expect("install root");
        let install_dir_to = install_root.path().join("mytool");
        let install_to = install_root.path().join("bin").join("mytool");

        // First install — create a pre-existing "old" install.
        fs::create_dir_all(&install_dir_to).expect("create old dir");
        fs::write(install_dir_to.join("mytool"), b"old binary").expect("write old");
        fs::create_dir_all(install_to.parent().unwrap()).expect("create bin dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(install_dir_to.join("mytool"), &install_to).expect("symlink");

        // Second install with new content.
        setup_fake_archive_dir(temp.path(), "mytool-v2.0", "mytool").expect("setup");
        install_archive_dir(
            temp.path(),
            "mytool-v2.0",
            &install_dir_to,
            "mytool-v2.0/mytool",
            &install_to,
        )
        .expect("install_archive_dir should succeed");

        assert!(install_dir_to.exists());
        // Old backup should be cleaned up.
        assert!(!install_dir_to.with_file_name(format!(
            "{}.old",
            install_dir_to.file_name().unwrap().to_str().unwrap()
        )).exists());
        // Symlink should point to new path.
        #[cfg(unix)]
        {
            let link_target = fs::read_link(&install_to).expect("read_link");
            assert_eq!(link_target, install_dir_to.join("mytool"));
        }
    }

    #[test]
    fn atomic_install_staging_failure_preserves_old_install() {
        let install_root = tempfile::tempdir().expect("install root");
        let install_dir_to = install_root.path().join("mytool");

        // Create pre-existing old install.
        fs::create_dir_all(&install_dir_to).expect("create old dir");
        let old_binary_path = install_dir_to.join("mytool");
        fs::write(&old_binary_path, b"old binary").expect("write old");

        // Source dir is intentionally left empty/missing so staging copy fails.
        let temp = tempfile::tempdir().expect("tempdir");
        let result = install_archive_dir(
            temp.path(),
            "nonexistent-dir",
            &install_dir_to,
            "nonexistent-dir/mytool",
            &install_root.path().join("bin").join("mytool"),
        );

        assert!(result.is_err());
        // Old install should still be intact.
        assert!(install_dir_to.exists());
        assert!(old_binary_path.exists());
        assert_eq!(fs::read_to_string(&old_binary_path).expect("read"), "old binary");
    }

    #[test]
    fn atomic_install_cleanup_leftover_old_backup_before_rename() {
        let temp = tempfile::tempdir().expect("tempdir");
        let install_root = tempfile::tempdir().expect("install root");
        let install_dir_to = install_root.path().join("mytool");
        let old_backup = install_dir_to.with_file_name(format!(
            "{}.old",
            install_dir_to.file_name().unwrap().to_str().unwrap()
        ));

        // Simulate leftover .old from a previous crashed run.
        fs::create_dir_all(&install_dir_to).expect("create dir");
        fs::create_dir_all(&old_backup).expect("create leftover backup");
        fs::write(old_backup.join("stale"), b"stale").expect("write stale");

        let install_to = install_root.path().join("bin").join("mytool");
        setup_fake_archive_dir(temp.path(), "mytool-v1.0", "mytool").expect("setup");
        install_archive_dir(
            temp.path(),
            "mytool-v1.0",
            &install_dir_to,
            "mytool-v1.0/mytool",
            &install_to,
        )
        .expect("install_archive_dir should succeed");

        // After success, .old should be cleaned up.
        assert!(!old_backup.exists());
        assert!(install_dir_to.exists());
    }

    #[test]
    fn atomic_install_creates_intermediate_dirs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let install_root = tempfile::tempdir().expect("install root");

        // install_to has a deep parent path that doesn't exist.
        let install_dir_to = install_root.path().join("deep").join("nested").join("mytool");
        let install_to = install_root.path().join("bin").join("deep").join("mytool");

        setup_fake_archive_dir(temp.path(), "mytool-v1.0", "mytool").expect("setup");
        install_archive_dir(
            temp.path(),
            "mytool-v1.0",
            &install_dir_to,
            "mytool-v1.0/mytool",
            &install_to,
        )
        .expect("install_archive_dir should succeed");

        assert!(install_dir_to.exists());
        #[cfg(unix)]
        {
            let link_target = fs::read_link(&install_to).expect("read_link");
            assert_eq!(link_target, install_dir_to.join("mytool"));
        }
    }

}
