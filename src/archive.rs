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
            archive
                .unpack(dest)
                .map_err(|err| format!("failed to unpack tar.gz: {err}"))?;
            Ok(None)
        }
        ArchiveKind::TarXz => {
            let cursor = Cursor::new(bytes);
            let decoder = xz2::read::XzDecoder::new(cursor);
            let mut archive = tar::Archive::new(decoder);
            archive
                .unpack(dest)
                .map_err(|err| format!("failed to unpack tar.xz: {err}"))?;
            Ok(None)
        }
        ArchiveKind::Zip => {
            let cursor = Cursor::new(bytes);
            let mut archive =
                zip::ZipArchive::new(cursor).map_err(|err| format!("failed to read zip: {err}"))?;
            for i in 0..archive.len() {
                let mut entry = archive
                    .by_index(i)
                    .map_err(|err| format!("failed to read zip entry: {err}"))?;
                let out_path = dest.join(entry.mangled_name());
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
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_archive_kind_supports_expected_values() {
        assert!(matches!(parse_archive_kind("raw"), Ok(ArchiveKind::Raw)));
        assert!(matches!(parse_archive_kind("tar.gz"), Ok(ArchiveKind::TarGz)));
        assert!(matches!(parse_archive_kind("tar.xz"), Ok(ArchiveKind::TarXz)));
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
}
