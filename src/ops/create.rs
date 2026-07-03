//! Create operation: mkdir -p following symlinks.
//!
//! Adapted from current dotman deploy.rs.

use anyhow::Result;
use std::path::Path;

pub fn create_dir(path: &Path) -> Result<()> {
    if path.exists() {
        return if path.is_dir() {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "path is not a directory: {}",
                path.display()
            ))
        };
    }

    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if current.as_os_str().is_empty() || current.exists() {
            if current.exists() && !current.is_dir() {
                return Err(anyhow::anyhow!(
                    "path is not a directory: {}",
                    current.display()
                ));
            }
            continue;
        }
        match std::fs::create_dir(&current) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists && current.is_dir() => {}
            Err(err) => {
                return Err(anyhow::anyhow!(
                    "failed to create {}: {err}",
                    current.display()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_directory_tree() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a/b/c");
        create_dir(&target).unwrap();
        assert!(target.is_dir());
    }

    #[test]
    fn idempotent_when_dir_exists() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("x");
        create_dir(&target).unwrap();
        create_dir(&target).unwrap(); // second call no-op
        assert!(target.is_dir());
    }

    #[test]
    fn errors_when_path_is_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("file");
        std::fs::write(&file, "x").unwrap();
        let result = create_dir(&file);
        assert!(result.is_err());
    }
}
