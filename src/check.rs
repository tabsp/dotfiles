use crate::config::{DepsManifest, DotfilesManifest, FileKind, Installer};
use crate::platform::{distro_supported, Host, Platform};
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

        match dep.entries_for(host.platform.key(), host.arch.key()).as_slice() {
            [] => errors.push(format!("dependency {name} has no current-host entry")),
            [entry] => {
                validate_installer_platform(name, entry.installer, host, &mut errors);
                if entry.version != "latest" && dep.version_check.is_none() {
                    errors.push(format!(
                        "dependency {name} pins version {} but has no version_check",
                        entry.version
                    ));
                }
                validate_https(name, entry.source.as_deref(), &mut errors);
            }
            _ => errors.push(format!("dependency {name} has multiple current-host entries")),
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
        Installer::Apt | Installer::RepoPackage if host.platform != Platform::Linux => {
            errors.push(format!(
                "dependency {name} uses linux-only installer on non-linux host"
            ));
        }
        _ => {}
    }
}

fn validate_https(name: &str, source: Option<&str>, errors: &mut Vec<String>) {
    if let Some(source) = source {
        if !source.starts_with("https://") {
            errors.push(format!("dependency {name} source must use https://"));
        }
    }
}

fn validate_source(repo: &Path, source: &str, expected: Option<FileKind>, errors: &mut Vec<String>) {
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
    if target.contains('$') {
        errors.push(format!(
            "target must not contain environment variables: {target}"
        ));
    }
    if !(target.starts_with("~/") || target.starts_with('/')) {
        errors.push(format!("target must be absolute or ~-based: {target}"));
    }
    if let Some(path) = expand_home(target) {
        if path.starts_with(repo) {
            errors.push(format!("target must not point inside repository: {target}"));
        }
    }
}

fn expand_home(path: &str) -> Option<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(rest))
    } else {
        Some(PathBuf::from(path))
    }
}
