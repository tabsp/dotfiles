use crate::config::{DepsManifest, DotfilesManifest, VersionCheck, VersionStream};
use crate::path::{expand_home, which};
use crate::platform::Host;
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_doctor(
    deps: &DepsManifest,
    files: &DotfilesManifest,
    host: &Host,
    repo: &Path,
) -> Result<(), String> {
    let mut hard_errors = Vec::new();
    let mut warnings = Vec::new();
    let mut oks = Vec::new();

    for (name, dep) in &deps.deps {
        let entries = dep.entries_for(host.platform.key(), host.arch.key());
        if entries.is_empty() {
            continue;
        }
        if which(&dep.command).is_none() {
            hard_errors.push(format!("{name}: missing command {}", dep.command));
            continue;
        }
        oks.push(format!("{name}: command {}", dep.command));

        if let Some(check) = &dep.version_check {
            match read_version(&dep.command, check) {
                Ok(version) => {
                    let expected = &entries[0].version;
                    if expected == "latest" {
                        oks.push(format!("{name}: version {version}"));
                    } else if expected != &version {
                        warnings.push(format!(
                            "{name}: version drift installed={version} expected={expected}"
                        ));
                    } else {
                        oks.push(format!("{name}: version {version}"));
                    }
                }
                Err(err) => hard_errors.push(format!("{name}: {err}")),
            }
        }
    }

    for file in &files.files {
        if !file.enabled {
            continue;
        }
        if !file.platforms.is_empty() && !file.platforms.iter().any(|p| p == host.platform.key()) {
            continue;
        }

        let target = expand_home(&file.target)?;
        let expected = repo.join(&file.source);
        match fs::read_link(&target) {
            Ok(actual) if actual == expected => {
                oks.push(format!("link {} -> {}", target.display(), expected.display()));
            }
            Ok(actual) => {
                hard_errors.push(format!(
                    "wrong link {} -> {}, expected {}",
                    target.display(),
                    actual.display(),
                    expected.display()
                ));
            }
            Err(_) if target.exists() => {
                hard_errors.push(format!(
                    "target exists but is not a symlink: {}",
                    target.display()
                ));
            }
            Err(_) => hard_errors.push(format!("missing target symlink: {}", target.display())),
        }
    }

    for err in &hard_errors {
        eprintln!("error: {err}");
    }
    for warning in &warnings {
        eprintln!("warn: {warning}");
    }
    for ok in &oks {
        println!("ok: {ok}");
    }

    if hard_errors.is_empty() {
        Ok(())
    } else {
        Err(format!("doctor found {} hard error(s)", hard_errors.len()))
    }
}

fn read_version(command: &str, check: &VersionCheck) -> Result<String, String> {
    let output = Command::new(command)
        .args(&check.args)
        .output()
        .map_err(|err| format!("failed to run version check: {err}"))?;
    if !output.status.success() {
        return Err(format!("version check exited {}", output.status));
    }

    let bytes = match check.stream {
        VersionStream::Stdout => output.stdout,
        VersionStream::Stderr => output.stderr,
    };
    let text = String::from_utf8_lossy(&bytes);
    let regex = Regex::new(&check.regex).map_err(|err| format!("invalid version regex: {err}"))?;
    regex
        .captures(&text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "version output did not match regex".to_string())
}
