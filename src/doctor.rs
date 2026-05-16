use crate::config::{DepsManifest, DotfilesManifest, VersionCheck, VersionStream};
use crate::path::{expand_home, paths_match, which};
use crate::platform::Host;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_doctor(
    deps: &DepsManifest,
    files: &DotfilesManifest,
    host: &Host,
    repo: &Path,
    json: bool,
) -> Result<(), String> {
    let mut hard_errors = Vec::new();
    let mut warnings = Vec::new();
    let mut oks = Vec::new();

    for (name, dep) in &deps.deps {
        let entries = dep.entries_for_host(host);
        if entries.is_empty() {
            continue;
        }
        let Some(command_path) = which(&dep.command) else {
            hard_errors.push(format!("{name}: missing command {}", dep.command));
            continue;
        };
        oks.push(format!("{name}: command {}", dep.command));

        if let Some(check) = &dep.version_check {
            match read_version(&command_path, check) {
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
            Ok(actual) if paths_match(&actual, &expected) => {
                oks.push(format!(
                    "link {} -> {}",
                    target.display(),
                    expected.display()
                ));
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

    if json {
        let output = DoctorOutput {
            ok: &oks,
            warnings: &warnings,
            errors: &hard_errors,
            summary: DoctorSummary {
                ok: oks.len(),
                warnings: warnings.len(),
                errors: hard_errors.len(),
            },
        };
        println!(
            "{}",
            serde_json::to_string(&output)
                .map_err(|err| format!("failed to serialize doctor output: {err}"))?
        );
    } else {
        for err in &hard_errors {
            eprintln!("error: {err}");
        }
        for warning in &warnings {
            eprintln!("warn: {warning}");
        }
        for ok in &oks {
            println!("ok: {ok}");
        }
        println!(
            "doctor: {} ok, {} warning(s), {} error(s)",
            oks.len(),
            warnings.len(),
            hard_errors.len()
        );
    }

    if hard_errors.is_empty() {
        Ok(())
    } else {
        Err(format!("doctor found {} hard error(s)", hard_errors.len()))
    }
}

fn read_version(command: &Path, check: &VersionCheck) -> Result<String, String> {
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
    extract_version(&text, &check.regex)
}

fn extract_version(text: &str, regex: &str) -> Result<String, String> {
    let regex = Regex::new(regex).map_err(|err| format!("invalid version regex: {err}"))?;
    regex
        .captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "version output did not match regex".to_string())
}

#[derive(Serialize)]
struct DoctorOutput<'a> {
    ok: &'a Vec<String>,
    warnings: &'a Vec<String>,
    errors: &'a Vec<String>,
    summary: DoctorSummary,
}

#[derive(Serialize)]
struct DoctorSummary {
    ok: usize,
    warnings: usize,
    errors: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_handles_multiline_eza_output() {
        let text = "eza - A modern, maintained replacement for ls\nv0.23.4 [+git]\n";
        let version = extract_version(text, r"(?m)^v?([0-9]+\.[0-9]+\.[0-9]+)").expect("version");
        assert_eq!(version, "0.23.4");
    }

    #[test]
    fn extract_version_handles_localized_fish_output() {
        let text = "fish，版本 4.7.1\n";
        let version = extract_version(
            text,
            r"fish[，,]\s*(?:version|版本)\s+([0-9]+\.[0-9]+\.[0-9]+)",
        )
        .expect("version");
        assert_eq!(version, "4.7.1");
    }

    #[test]
    fn extract_version_handles_capitalized_yazi_output() {
        let text = "Yazi 26.5.6 (aa52643 2026-05-05)\n";
        let version =
            extract_version(text, r"(?i)yazi\s+([0-9]+\.[0-9]+\.[0-9]+)").expect("version");
        assert_eq!(version, "26.5.6");
    }
}
