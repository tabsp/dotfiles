//! Archive extraction pipeline for binary downloads.
//!
//! Trust boundaries (in order):
//! 1. Manifest URL validation — only https:// URLs accepted (`src/http.rs`).
//! 2. Final URL validation — redirect target must also be https://.
//! 3. Checksum verification — SHA-256 digest must match the pinned value.
//! 4. Archive path safety — every extracted entry must resolve within the
//!    destination directory (path traversal rejected).
//! 5. Link policy — symlinks and hardlinks in archives are rejected.
//!
//! Supported formats: raw, tar.gz, tar.xz, zip.

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub enum ArchiveKind {
    Raw,
    TarGz,
    TarXz,
    Zip,
}

pub fn parse_archive_kind(value: &str) -> Result<ArchiveKind, String> {
    match value {
        "raw" => Ok(ArchiveKind::Raw),
        "tar.gz" => Ok(ArchiveKind::TarGz),
        "tar.xz" => Ok(ArchiveKind::TarXz),
        "zip" => Ok(ArchiveKind::Zip),
        other => Err(format!("unsupported archive_kind: {other}")),
    }
}

/// Validate that `entry_path` resolved relative to `dest` does not escape `dest`.
fn validate_entry_path(dest: &Path, entry_path: &Path) -> Result<PathBuf, String> {
    // Normalize the entry path: reject if it tries to escape via .. components.
    let mut depth: i32 = 0;
    for component in entry_path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return Err(format!(
                        "DOTMAN_EXTRACT_PATH_TRAVERSAL: archive entry escapes destination: {}",
                        entry_path.display()
                    ));
                }
            }
            Component::Normal(_) => depth += 1,
            Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "DOTMAN_EXTRACT_PATH_TRAVERSAL: archive entry has absolute path: {}",
                    entry_path.display()
                ));
            }
            _ => {}
        }
    }

    let out = dest.join(entry_path);

    // Additional check: if dest can be canonicalized, verify the resolved path
    // (when canonicalized) stays within dest.
    if let (Ok(canonical_dest), Ok(canonical_out)) = (dest.canonicalize(), out.canonicalize())
        && !canonical_out.starts_with(&canonical_dest)
    {
        return Err(format!(
            "DOTMAN_EXTRACT_PATH_TRAVERSAL: archive entry escapes destination: {}",
            entry_path.display()
        ));
    }

    Ok(out)
}

pub fn unpack(bytes: &[u8], kind: &ArchiveKind, dest: &Path) -> Result<Option<PathBuf>, String> {
    match kind {
        ArchiveKind::Raw => {
            let path = dest.join("downloaded-binary");
            fs::write(&path, bytes).map_err(|err| format!("failed to write raw payload: {err}"))?;
            Ok(Some(path))
        }
        ArchiveKind::TarGz => {
            let cursor = Cursor::new(bytes);
            let decoder = flate2::read::GzDecoder::new(cursor);
            let mut archive = tar::Archive::new(decoder);
            unpack_tar_safe(&mut archive, dest)?;
            Ok(None)
        }
        ArchiveKind::TarXz => {
            let cursor = Cursor::new(bytes);
            let decoder = xz2::read::XzDecoder::new(cursor);
            let mut archive = tar::Archive::new(decoder);
            unpack_tar_safe(&mut archive, dest)?;
            Ok(None)
        }
        ArchiveKind::Zip => {
            let cursor = Cursor::new(bytes);
            let mut archive =
                zip::ZipArchive::new(cursor).map_err(|err| format!("failed to read zip: {err}"))?;
            unpack_zip_safe(&mut archive, dest)?;
            Ok(None)
        }
    }
}

fn unpack_tar_safe<R: std::io::Read>(
    archive: &mut tar::Archive<R>,
    dest: &Path,
) -> Result<(), String> {
    for entry in archive
        .entries()
        .map_err(|err| format!("failed to read tar entries: {err}"))?
    {
        let mut entry = entry.map_err(|err| format!("failed to read tar entry: {err}"))?;
        let entry_path = entry
            .path()
            .map_err(|err| format!("failed to read tar entry path: {err}"))?;

        let entry_type = entry.header().entry_type();
        if entry_type == tar::EntryType::Symlink {
            return Err(format!(
                "DOTMAN_EXTRACT_SYMLINK_REJECTED: archive contains symlink: {}",
                entry_path.display()
            ));
        }
        if entry_type == tar::EntryType::Link {
            return Err(format!(
                "DOTMAN_EXTRACT_HARDLINK_REJECTED: archive contains hardlink: {}",
                entry_path.display()
            ));
        }

        let out_path = validate_entry_path(dest, &entry_path)?;

        if entry_type == tar::EntryType::Directory {
            fs::create_dir_all(&out_path).map_err(|err| {
                format!(
                    "failed to create directory from tar: {}: {err}",
                    out_path.display()
                )
            })?;
        } else if entry_type == tar::EntryType::Regular {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    format!(
                        "failed to create parent directory for tar entry: {}: {err}",
                        parent.display()
                    )
                })?;
            }
            entry.unpack(&out_path).map_err(|err| {
                format!(
                    "failed to extract tar entry to {}: {err}",
                    out_path.display()
                )
            })?;
        }
        // Ignore other entry types (e.g., GNULongName, GNULongLink, XGlobalHeader).
    }
    Ok(())
}

fn unpack_zip_safe(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    dest: &Path,
) -> Result<(), String> {
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|err| format!("failed to read zip entry: {err}"))?;

        let mangled = entry.mangled_name();
        if mangled.as_os_str().is_empty() {
            return Err(format!(
                "DOTMAN_EXTRACT_EMPTY_PATH: zip entry at index {i} has empty or invalid path"
            ));
        }

        // Reject symlinks in zip.
        if entry.is_symlink() {
            return Err(format!(
                "DOTMAN_EXTRACT_SYMLINK_REJECTED: zip contains symlink: {}",
                mangled.display()
            ));
        }

        let out_path = validate_entry_path(dest, &mangled)?;

        if entry.name().ends_with('/') {
            fs::create_dir_all(&out_path)
                .map_err(|err| format!("failed to create directory from zip: {err}"))?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create zip parent directory: {err}"))?;
        }
        let mut output = fs::File::create(&out_path)
            .map_err(|err| format!("failed to create zip output file: {err}"))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|err| format!("failed to extract zip entry: {err}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_archive_kind_supports_expected_values() {
        assert!(matches!(parse_archive_kind("raw"), Ok(ArchiveKind::Raw)));
        assert!(matches!(
            parse_archive_kind("tar.gz"),
            Ok(ArchiveKind::TarGz)
        ));
        assert!(matches!(
            parse_archive_kind("tar.xz"),
            Ok(ArchiveKind::TarXz)
        ));
        assert!(matches!(parse_archive_kind("zip"), Ok(ArchiveKind::Zip)));
    }

    #[test]
    fn parse_archive_kind_rejects_unknown_value() {
        match parse_archive_kind("rar") {
            Ok(_) => panic!("expected parse failure"),
            Err(err) => assert!(err.contains("unsupported archive_kind")),
        }
    }

    #[test]
    fn unpack_raw_writes_payload_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bytes = b"binary-payload";
        let out = unpack(bytes, &ArchiveKind::Raw, temp.path()).expect("unpack");
        let path = out.expect("path");
        let written = fs::read(path).expect("read");
        assert_eq!(written, bytes);
    }

    // --- Extraction safety tests ---

    #[test]
    fn tar_extracts_normal_archive() {
        let dest = tempfile::tempdir().expect("dest");
        let mut ar = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_path("mytool/mytool").expect("set path");
        header.set_size(11);
        header.set_mode(0o755);
        header.set_cksum();
        ar.append(&header, b"fake binary".as_slice())
            .expect("append");
        let tar_bytes = ar.into_inner().expect("finish");

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&tar_bytes).expect("write");
        let gz_bytes = gz.finish().expect("finish");

        unpack(&gz_bytes, &ArchiveKind::TarGz, dest.path()).expect("unpack");
        assert!(dest.path().join("mytool/mytool").exists());
    }

    #[test]
    fn tar_rejects_symlink() {
        let dest = tempfile::tempdir().expect("dest");
        let mut ar = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_path("link").expect("set path");
        header.set_link_name("target").expect("set link name");
        header.set_size(0);
        header.set_cksum();
        ar.append(&header, std::io::empty()).expect("append");
        let tar_bytes = ar.into_inner().expect("finish");

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&tar_bytes).expect("write");
        let gz_bytes = gz.finish().expect("finish");

        let err = unpack(&gz_bytes, &ArchiveKind::TarGz, dest.path()).expect_err("should fail");
        assert!(
            err.contains("DOTMAN_EXTRACT_SYMLINK_REJECTED"),
            "expected symlink rejected error, got: {err}"
        );
    }

    #[test]
    fn tar_rejects_hardlink() {
        let dest = tempfile::tempdir().expect("dest");
        let mut ar = tar::Builder::new(Vec::new());
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Link);
        header.set_path("hardlink").expect("set path");
        header.set_link_name("target").expect("set link name");
        header.set_size(0);
        header.set_cksum();
        ar.append(&header, std::io::empty()).expect("append");
        let tar_bytes = ar.into_inner().expect("finish");

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&tar_bytes).expect("write");
        let gz_bytes = gz.finish().expect("finish");

        let err = unpack(&gz_bytes, &ArchiveKind::TarGz, dest.path()).expect_err("should fail");
        assert!(
            err.contains("DOTMAN_EXTRACT_HARDLINK_REJECTED"),
            "expected hardlink rejected error, got: {err}"
        );
    }

    #[test]
    fn zip_extracts_normal_archive() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default();
            writer
                .start_file("mytool/mytool", options)
                .expect("start file");
            writer.write_all(b"fake binary").expect("write");
            writer.add_directory("mytool/", options).expect("add dir");
            writer.finish().expect("finish");
        }
        let zip_bytes = buf.into_inner();

        unpack(&zip_bytes, &ArchiveKind::Zip, temp.path()).expect("unpack");
        assert!(temp.path().join("mytool/mytool").exists());
    }

    #[test]
    fn zip_rejects_path_traversal() {
        let dest = tempfile::tempdir().expect("dest");
        // Build a zip with a traversal path. The zip crate stores the raw name
        // but mangled_name() strips .. components at read time, making it safe.
        // validate_entry_path provides defense-in-depth.
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default();
            // zip crate allows storing traversal paths; mangled_name() defangs them.
            writer
                .start_file("../../../escape", options)
                .expect("start file");
            writer.write_all(b"evil").expect("write");
            writer.finish().expect("finish");
        }
        let zip_bytes = buf.into_inner();

        // After mangled_name(), "../../../escape" becomes "escape" (safe).
        // The extraction should succeed with the mangled, safe path.
        unpack(&zip_bytes, &ArchiveKind::Zip, dest.path()).expect("unpack");
        // The file should be at the mangled path, not the original traversal path.
        assert!(dest.path().join("escape").exists());
        // The traversal path should NOT exist.
        assert!(!dest.path().join("../../../escape").exists());
    }

    #[test]
    fn zip_rejects_empty_path_entry() {
        let dest = tempfile::tempdir().expect("dest");
        // Build a zip where start_file is called with a path that
        // mangled_name() strips to empty (e.g., just "..").
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buf);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("..", options).expect("start file");
            writer.write_all(b"evil").expect("write");
            writer.finish().expect("finish");
        }
        let zip_bytes = buf.into_inner();

        let err = unpack(&zip_bytes, &ArchiveKind::Zip, dest.path()).expect_err("should fail");
        assert!(
            err.contains("DOTMAN_EXTRACT_EMPTY_PATH"),
            "expected empty path error, got: {err}"
        );
    }

    // --- validate_entry_path direct tests ---

    #[test]
    fn validate_entry_path_rejects_traversal() {
        let dest = tempfile::tempdir().expect("dest");
        let err =
            validate_entry_path(dest.path(), Path::new("../escape")).expect_err("should fail");
        assert!(err.contains("DOTMAN_EXTRACT_PATH_TRAVERSAL"), "got: {err}");
    }

    #[test]
    fn validate_entry_path_rejects_deep_traversal() {
        let dest = tempfile::tempdir().expect("dest");
        let err = validate_entry_path(dest.path(), Path::new("foo/../../../escape"))
            .expect_err("should fail");
        assert!(err.contains("DOTMAN_EXTRACT_PATH_TRAVERSAL"), "got: {err}");
    }

    #[test]
    fn validate_entry_path_accepts_normal_path() {
        let dest = tempfile::tempdir().expect("dest");
        let resolved =
            validate_entry_path(dest.path(), Path::new("some/dir/file")).expect("should succeed");
        // The resolved path should be dest/some/dir/file.
        // Canonicalize dest to handle macOS /tmp vs /private/tmp.
        let cd = dest
            .path()
            .canonicalize()
            .unwrap_or_else(|_| dest.path().to_path_buf());
        assert!(resolved.starts_with(&cd) || resolved.starts_with(dest.path()));
    }

    #[test]
    fn validate_entry_path_accepts_sibling_via_normalization() {
        let dest = tempfile::tempdir().expect("dest");
        let resolved =
            validate_entry_path(dest.path(), Path::new("foo/../bar")).expect("should succeed");
        // The entry path normalizes to bar (depth check passes).
        // The returned path is dest/foo/../bar (literal join).
        let cd = dest
            .path()
            .canonicalize()
            .unwrap_or_else(|_| dest.path().to_path_buf());
        assert!(resolved.starts_with(&cd) || resolved.starts_with(dest.path()));
        assert!(resolved.ends_with("bar"));
    }
}
