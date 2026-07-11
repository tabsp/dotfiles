//! Explicit, configuration-independent binary self-update support.

use flate2::read::GzDecoder;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

const REPOSITORY: &str = "tabsp/dotfiles";
const GITHUB_API: &str = "https://api.github.com";
const GITHUB_DOWNLOAD: &str = "https://github.com";
const USER_AGENT: &str = "dotman-self-update";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
}

pub fn run() -> Result<(), String> {
    let executable =
        std::env::current_exe().map_err(|e| format!("cannot locate current executable: {e}"))?;
    update(
        env!("CARGO_PKG_VERSION"),
        &executable,
        GITHUB_API,
        GITHUB_DOWNLOAD,
    )
}

fn update(
    current: &str,
    executable: &Path,
    api_base: &str,
    download_base: &str,
) -> Result<(), String> {
    let release_url = format!("{api_base}/repos/{REPOSITORY}/releases/latest");
    let release: Release = get(&release_url, "release metadata")?
        .into_json()
        .map_err(|e| format!("invalid GitHub release response: {e}"))?;
    let latest = parse_version(&release.tag_name)?;
    let current_version = parse_version(current)?;

    if latest <= current_version {
        println!("dotman {current} is already the latest version.");
        return Ok(());
    }

    let target = release_target()?;
    let artifact = format!("dotman-{target}.tar.gz");
    let base = format!(
        "{download_base}/{REPOSITORY}/releases/download/{}/{}",
        release.tag_name, artifact
    );
    println!("Updating dotman {current} -> {latest} ({target})...");

    let archive = read_response(get(&base, "release artifact")?, "release artifact")?;
    let checksum_url = format!("{base}.sha256");
    let checksum = read_response(get(&checksum_url, "checksum")?, "checksum")?;
    verify_checksum(&archive, &checksum)?;
    let binary = extract_binary(&archive)?;
    install(&binary, executable)?;

    println!("dotman updated successfully to {latest}.");
    Ok(())
}

fn get(url: &str, what: &str) -> Result<ureq::Response, String> {
    ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("failed to download {what}: {e}"))
}

fn read_response(response: ureq::Response, what: &str) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("failed to read {what}: {e}"))?;
    Ok(bytes)
}

fn parse_version(value: &str) -> Result<Version, String> {
    Version::parse(value.trim().strip_prefix('v').unwrap_or(value.trim()))
        .map_err(|e| format!("invalid release version '{value}': {e}"))
}

fn release_target() -> Result<&'static str, String> {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("aarch64", "macos") => Ok("aarch64-apple-darwin"),
        ("x86_64", "macos") => Ok("x86_64-apple-darwin"),
        ("aarch64", "linux") => Ok("aarch64-unknown-linux-gnu"),
        ("x86_64", "linux") => Ok("x86_64-unknown-linux-gnu"),
        (arch, os) => Err(format!("self-update is not supported on {os}/{arch}")),
    }
}

fn verify_checksum(archive: &[u8], checksum_file: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(checksum_file)
        .map_err(|_| "checksum file is not valid UTF-8".to_string())?;
    let expected = text
        .split_whitespace()
        .next()
        .filter(|value| value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| "checksum file does not contain a valid SHA-256 digest".to_string())?;
    let actual = format!("{:x}", Sha256::digest(archive));
    if !actual.eq_ignore_ascii_case(expected) {
        return Err("release artifact checksum verification failed".into());
    }
    Ok(())
}

fn extract_binary(archive: &[u8]) -> Result<Vec<u8>, String> {
    let decoder = GzDecoder::new(Cursor::new(archive));
    let mut tar = tar::Archive::new(decoder);
    let entries = tar
        .entries()
        .map_err(|e| format!("invalid release archive: {e}"))?;
    for entry in entries {
        let mut entry = entry.map_err(|e| format!("invalid release archive entry: {e}"))?;
        let path = entry
            .path()
            .map_err(|e| format!("invalid release archive path: {e}"))?;
        if path == Path::new("dotman") && entry.header().entry_type().is_file() {
            let mut binary = Vec::new();
            entry
                .read_to_end(&mut binary)
                .map_err(|e| format!("failed to extract dotman: {e}"))?;
            if binary.is_empty() {
                return Err("release archive contains an empty dotman binary".into());
            }
            return Ok(binary);
        }
    }
    Err("release archive does not contain a dotman binary".into())
}

fn install(binary: &[u8], executable: &Path) -> Result<(), String> {
    let dir = executable
        .parent()
        .ok_or_else(|| "current executable has no parent directory".to_string())?;
    let name = executable
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "current executable has an invalid filename".to_string())?;
    let temp = unique_temp_path(dir, name);

    let result = (|| -> Result<(), String> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o755);
        }
        let mut file = options
            .open(&temp)
            .map_err(|e| format!("failed to create update beside executable: {e}"))?;
        file.write_all(binary)
            .map_err(|e| format!("failed to write updated executable: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("failed to sync updated executable: {e}"))?;
        drop(file);
        fs::rename(&temp, executable)
            .map_err(|e| format!("failed to install updated executable: {e}"))?;
        sync_directory(dir);
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn unique_temp_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!(".{name}.update-{}", std::process::id()))
}

#[cfg(unix)]
fn sync_directory(dir: &Path) {
    let _ = fs::File::open(dir).and_then(|file| file.sync_all());
}

#[cfg(not(unix))]
fn sync_directory(_dir: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{Compression, write::GzEncoder};

    fn archive(binary: &[u8]) -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(binary.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder.append_data(&mut header, "dotman", binary).unwrap();
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[test]
    fn verifies_and_extracts_release() {
        let bytes = archive(b"new binary");
        let checksum = format!("{:x}  artifact.tar.gz\n", Sha256::digest(&bytes));
        verify_checksum(&bytes, checksum.as_bytes()).unwrap();
        assert_eq!(extract_binary(&bytes).unwrap(), b"new binary");
    }

    #[test]
    fn rejects_bad_checksum_without_installing() {
        let bytes = archive(b"new binary");
        assert!(
            verify_checksum(
                &bytes,
                b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  x"
            )
            .is_err()
        );
    }

    #[test]
    fn installs_by_replacing_executable() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("dotman");
        fs::write(&executable, b"old").unwrap();
        install(b"new", &executable).unwrap();
        assert_eq!(fs::read(executable).unwrap(), b"new");
    }

    #[test]
    fn install_failure_leaves_destination_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("dotman");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("sentinel"), b"old").unwrap();
        assert!(install(b"new", &destination).is_err());
        assert_eq!(fs::read(destination.join("sentinel")).unwrap(), b"old");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn compares_versions_with_optional_v_prefix() {
        assert!(parse_version("v0.3.0").unwrap() > parse_version("0.2.0").unwrap());
    }
}
