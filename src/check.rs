use crate::config::{DepsManifest, DotfilesManifest, FileKind, Installer};
use crate::platform::{Host, Platform, distro_supported};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub fn run_check(
    deps: &DepsManifest,
    files: &DotfilesManifest,
    host: &Host,
    repo: &Path,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let mut commands = BTreeSet::new();
    let mut active_targets = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    let mut active_files = 0usize;

    if !distro_supported(host) {
        errors.push("unsupported Linux distribution; v1 supports Ubuntu and Debian".to_string());
    }

    for (name, dep) in &deps.deps {
        if !commands.insert(dep.command.clone()) {
            errors.push(format!("duplicate command in deps.toml: {}", dep.command));
        }

        validate_distros_scope(name, dep, &mut errors);

        let raw_entries = dep.entries_for(host.platform.key(), host.arch.key());
        let entries: Vec<_> = raw_entries
            .iter()
            .copied()
            .filter(|entry| entry.matches_distro(host))
            .collect();

        match entries.as_slice() {
            [] if host.platform == Platform::Linux && !raw_entries.is_empty() => {
                let distro = host.distro.as_deref().unwrap_or("unknown");
                errors.push(format!(
                    "dependency {name} has no current-host entry for distro {distro}"
                ));
            }
            [] => errors.push(format!("dependency {name} has no current-host entry")),
            [entry] => {
                validate_installer_platform(name, entry.installer, host, &mut errors);
                validate_installer_params(name, entry, repo, &mut errors);
                if entry.version != "latest" && dep.version_check.is_none() {
                    errors.push(format!(
                        "dependency {name} pins version {} but has no version_check",
                        entry.version
                    ));
                }
                validate_https(name, entry.source.as_deref(), &mut errors);
            }
            _ => errors.push(format!(
                "dependency {name} has multiple current-host entries"
            )),
        }
    }

    for file in &files.files {
        let pair = (file.source.clone(), file.target.clone());
        if !pairs.insert(pair) {
            errors.push(format!(
                "duplicate dotfile mapping: {} -> {}",
                file.source, file.target
            ));
        }

        if !file.enabled {
            continue;
        }
        if !file.platforms.is_empty() && !file.platforms.iter().any(|p| p == host.platform.key()) {
            continue;
        }
        active_files += 1;

        if !active_targets.insert(file.target.clone()) {
            errors.push(format!("duplicate active target: {}", file.target));
        }
        validate_source(repo, &file.source, file.kind, &mut errors);
        validate_target(repo, &file.target, &mut errors);
    }

    if active_files == 0 {
        errors.push("dotfiles.toml has no active file mappings for this host".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_installer_platform(
    name: &str,
    installer: Installer,
    host: &Host,
    errors: &mut Vec<String>,
) {
    match installer {
        Installer::Brew | Installer::Cask if host.platform != Platform::Mac => {
            errors.push(format!(
                "dependency {name} uses mac-only installer on non-mac host"
            ));
        }
        Installer::Apt | Installer::RepoPackage | Installer::Ppa
            if host.platform != Platform::Linux =>
        {
            errors.push(format!(
                "dependency {name} uses linux-only installer on non-linux host"
            ));
        }
        Installer::Ppa if host.distro.as_deref() != Some("ubuntu") => {
            errors.push(format!(
                "dependency {name} ppa installer supports Ubuntu only"
            ));
        }
        _ => {}
    }
}

fn validate_distros_scope(name: &str, dep: &crate::config::Dependency, errors: &mut Vec<String>) {
    for entry in dep.mac.values() {
        if entry.distros.is_some() {
            errors.push(format!(
                "dependency {name} distros is only valid on linux entries"
            ));
        }
    }
}

fn validate_https(name: &str, source: Option<&str>, errors: &mut Vec<String>) {
    if let Some(source) = source
        && !source.starts_with("https://")
    {
        errors.push(format!("dependency {name} source must use https://"));
    }
}

fn validate_installer_params(
    name: &str,
    entry: &crate::config::InstallEntry,
    repo: &Path,
    errors: &mut Vec<String>,
) {
    match entry.installer {
        Installer::DownloadBinary => {
            require_string_param(name, entry, "url", errors);
            require_string_param(name, entry, "sha256", errors);
            require_string_param(name, entry, "archive_kind", errors);
            require_string_param(name, entry, "binary_path", errors);
            require_string_param(name, entry, "install_to", errors);
            if let Some(url) = entry.params.get("url").and_then(toml::Value::as_str)
                && !url.starts_with("https://")
            {
                errors.push(format!("dependency {name} param url must use https://"));
            }
            if let Some(kind) = entry
                .params
                .get("archive_kind")
                .and_then(toml::Value::as_str)
            {
                match kind {
                    "raw" | "tar.gz" | "tar.xz" | "zip" => {}
                    _ => errors.push(format!(
                        "dependency {name} has unsupported archive_kind: {kind}"
                    )),
                }
            }
            if let Some(install_to) = entry.params.get("install_to").and_then(toml::Value::as_str) {
                validate_managed_path(repo, "install_to", install_to, errors);
            }
        }
        Installer::OfficialScript => {
            require_string_param(name, entry, "script_url", errors);
            validate_optional_string_array_param(name, entry, "args", errors);
            validate_optional_string_param(name, entry, "install_to", errors);
            if let Some(script_url) = entry.params.get("script_url").and_then(toml::Value::as_str)
                && !script_url.starts_with("https://")
            {
                errors.push(format!(
                    "dependency {name} param script_url must use https://"
                ));
            }
            if let Some(install_to) = entry.params.get("install_to").and_then(toml::Value::as_str) {
                validate_managed_path(repo, "install_to", install_to, errors);
            }
        }
        Installer::RepoPackage => {
            require_string_param(name, entry, "package", errors);
            require_string_param(name, entry, "repo_url", errors);
            require_string_param(name, entry, "repo_key_url", errors);
            require_string_param(name, entry, "repo_channel", errors);
            require_non_empty_string_array_param(name, entry, "repo_components", errors);
            if let Some(repo_url) = entry.params.get("repo_url").and_then(toml::Value::as_str)
                && !repo_url.starts_with("https://")
            {
                errors.push(format!(
                    "dependency {name} param repo_url must use https://"
                ));
            }
            if let Some(key_url) = entry
                .params
                .get("repo_key_url")
                .and_then(toml::Value::as_str)
                && !key_url.starts_with("https://")
            {
                errors.push(format!(
                    "dependency {name} param repo_key_url must use https://"
                ));
            }
        }
        Installer::Ppa => {
            require_string_param(name, entry, "ppa", errors);
            require_string_param(name, entry, "package", errors);
            validate_optional_string_param(name, entry, "bootstrap_package", errors);
        }
        _ => {}
    }
}

fn require_string_param(
    name: &str,
    entry: &crate::config::InstallEntry,
    key: &str,
    errors: &mut Vec<String>,
) {
    match entry.params.get(key) {
        Some(value) if value.is_str() => {}
        Some(_) => errors.push(format!("dependency {name} param {key} must be string")),
        None => errors.push(format!("dependency {name} missing required param {key}")),
    }
}

fn validate_optional_string_param(
    name: &str,
    entry: &crate::config::InstallEntry,
    key: &str,
    errors: &mut Vec<String>,
) {
    if let Some(value) = entry.params.get(key)
        && !value.is_str()
    {
        errors.push(format!("dependency {name} param {key} must be string"));
    }
}

fn validate_optional_string_array_param(
    name: &str,
    entry: &crate::config::InstallEntry,
    key: &str,
    errors: &mut Vec<String>,
) {
    let Some(value) = entry.params.get(key) else {
        return;
    };
    let Some(array) = value.as_array() else {
        errors.push(format!(
            "dependency {name} param {key} must be string array"
        ));
        return;
    };
    if array.iter().any(|v| !v.is_str()) {
        errors.push(format!(
            "dependency {name} param {key} must be string array"
        ));
    }
}

fn require_non_empty_string_array_param(
    name: &str,
    entry: &crate::config::InstallEntry,
    key: &str,
    errors: &mut Vec<String>,
) {
    let Some(value) = entry.params.get(key) else {
        errors.push(format!("dependency {name} missing required param {key}"));
        return;
    };
    let Some(array) = value.as_array() else {
        errors.push(format!(
            "dependency {name} param {key} must be non-empty string array"
        ));
        return;
    };
    if array.is_empty() || array.iter().any(|v| !v.is_str()) {
        errors.push(format!(
            "dependency {name} param {key} must be non-empty string array"
        ));
    }
}

fn validate_source(
    repo: &Path,
    source: &str,
    expected: Option<FileKind>,
    errors: &mut Vec<String>,
) {
    if source.starts_with('/')
        || source.starts_with('~')
        || source.contains('$')
        || source.split('/').any(|part| part == "..")
    {
        errors.push(format!("invalid source path: {source}"));
        return;
    }

    let path = repo.join(source);
    if !path.exists() {
        errors.push(format!("source does not exist: {source}"));
        return;
    }

    if let Some(kind) = expected {
        let ok = match kind {
            FileKind::File => path.is_file(),
            FileKind::Dir => path.is_dir(),
        };
        if !ok {
            errors.push(format!("source kind mismatch: {source}"));
        }
    }
}

fn validate_target(repo: &Path, target: &str, errors: &mut Vec<String>) {
    validate_managed_path(repo, "target", target, errors);
}

fn validate_managed_path(repo: &Path, label: &str, target: &str, errors: &mut Vec<String>) {
    if target.contains('$') {
        errors.push(format!(
            "{label} must not contain environment variables: {target}"
        ));
    }
    if !(target.starts_with("~/") || target.starts_with('/')) {
        errors.push(format!("{label} must be absolute or ~-based: {target}"));
    }
    if let Some(path) = expand_home(target)
        && path.starts_with(repo)
    {
        errors.push(format!(
            "{label} must not point inside repository: {target}"
        ));
    }
}

fn expand_home(path: &str) -> Option<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(rest))
    } else {
        Some(PathBuf::from(path))
    }
}
