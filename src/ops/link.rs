//! Link operation: symlink with backup/relink semantics.
//!
//! Adapted from current dotman deploy.rs (link logic).

use anyhow::{Context, Result};
use std::os::unix::fs as unix_fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkAction {
    Skip,
    Link,
    Backup(PathBuf),
    Relink,
    Fail(String),
}

#[derive(Debug, Clone)]
pub struct LinkPlan {
    pub target: PathBuf,
    pub source: PathBuf,
    pub link_source: PathBuf,
    pub settings: LinkSettings,
    pub action: LinkAction,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LinkSettings {
    pub create: bool,
    pub relink: bool,
    pub backup: bool,
    pub relative: bool,
}

pub fn plan_link(
    config_dir: &Path,
    target: &Path,
    source: &Path,
    settings: LinkSettings,
) -> Result<LinkPlan> {
    let source = if source.is_absolute() {
        source.to_path_buf()
    } else {
        config_dir.join(source)
    };
    let target = expand_home(target);

    if !source.exists() {
        return Ok(LinkPlan {
            target: target.clone(),
            source: source.clone(),
            link_source: source,
            settings,
            action: LinkAction::Fail("source does not exist".to_string()),
        });
    }

    let link_source = if settings.relative {
        relative_link_source(&source, &target)?
    } else {
        source.clone()
    };

    let action = if target.is_symlink() {
        if is_expected_symlink(&target, &source) {
            LinkAction::Skip
        } else if settings.relink {
            LinkAction::Relink
        } else if settings.backup {
            LinkAction::Backup(unique_backup_path(&target))
        } else {
            LinkAction::Fail(describe_conflict(&target, &source))
        }
    } else if target.exists() {
        if settings.backup {
            LinkAction::Backup(unique_backup_path(&target))
        } else {
            LinkAction::Fail(describe_conflict(&target, &source))
        }
    } else {
        LinkAction::Link
    };

    Ok(LinkPlan {
        target,
        source,
        link_source,
        settings,
        action,
    })
}

pub fn apply_link(plan: LinkPlan) -> Result<()> {
    match &plan.action {
        LinkAction::Fail(reason) => Err(anyhow::anyhow!(
            "target conflict: {} ({reason})",
            plan.target.display()
        )),
        LinkAction::Skip => Ok(()),
        LinkAction::Link | LinkAction::Relink | LinkAction::Backup(_) => {
            if plan.settings.create {
                ensure_parent_dir(&plan.target)?;
            } else {
                ensure_existing_parent_dir(&plan.target)?;
            }

            match &plan.action {
                LinkAction::Relink => std::fs::remove_file(&plan.target)
                    .with_context(|| format!("failed to remove link {}", plan.target.display()))?,
                LinkAction::Backup(backup) => std::fs::rename(&plan.target, backup)
                    .with_context(|| format!("failed to backup {}", plan.target.display()))?,
                _ => {}
            }

            unix_fs::symlink(&plan.link_source, &plan.target)
                .with_context(|| format!("failed to link {}", plan.target.display()))
        }
    }
}

pub fn unique_backup_path(target: &Path) -> PathBuf {
    use time::OffsetDateTime;
    use time::macros::format_description;
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let fmt = format_description!("[year][month][day][hour][minute][second]");
    let ts = now.format(&fmt).unwrap_or_else(|_| "19700101000000".into());
    let mut candidate = target.with_extension(format!("backup.{ts}"));
    let mut counter = 1;
    while candidate.exists() {
        candidate = target.with_extension(format!("backup.{ts}.{counter}"));
        counter += 1;
    }
    candidate
}

fn expand_home(path: &Path) -> PathBuf {
    if let Some(rest) = path.to_str().and_then(|s| s.strip_prefix("~/"))
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    path.to_path_buf()
}

fn relative_link_source(source: &Path, target: &Path) -> Result<PathBuf> {
    let target_parent = target
        .parent()
        .ok_or_else(|| anyhow::anyhow!("target has no parent: {}", target.display()))?;
    diff_paths(&absolute_path(source)?, &absolute_path(target_parent)?)
        .ok_or_else(|| anyhow::anyhow!("failed to compute relative link for {}", target.display()))
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Ok(canonical);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .with_context(|| format!("failed to read current dir for {}", path.display()))?
    };

    let mut missing = Vec::new();
    let mut existing = absolute.as_path();
    while !existing.exists() {
        let file_name = existing
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("failed to resolve path: {}", path.display()))?;
        missing.push(file_name.to_os_string());
        existing = existing
            .parent()
            .ok_or_else(|| anyhow::anyhow!("failed to resolve path: {}", path.display()))?;
    }

    let mut resolved = std::fs::canonicalize(existing)
        .with_context(|| format!("failed to resolve {}", existing.display()))?;
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
    let path_components = normal_components(path)?;
    let base_components = normal_components(base)?;
    let common = path_components
        .iter()
        .zip(base_components.iter())
        .take_while(|(left, right)| left == right)
        .count();

    let mut result = PathBuf::new();
    for _ in common..base_components.len() {
        result.push("..");
    }
    for component in &path_components[common..] {
        result.push(component);
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    Some(result)
}

fn normal_components(path: &Path) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::RootDir => parts.push(String::new()),
            Component::Normal(value) => parts.push(value.to_string_lossy().to_string()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Prefix(_) => return None,
        }
    }
    Some(parts)
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("target has no parent: {}", path.display()))?;
    if parent.exists() && !parent.is_dir() {
        return Err(anyhow::anyhow!(
            "target parent is not a directory: {}",
            parent.display()
        ));
    }
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create {}", parent.display()))
}

fn ensure_existing_parent_dir(path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("target has no parent: {}", path.display()))?;
    if parent.is_dir() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "target parent does not exist or is not a directory: {}",
            parent.display()
        ))
    }
}

fn is_expected_symlink(target: &Path, source: &Path) -> bool {
    std::fs::read_link(target)
        .map(|actual| paths_match_from_link(target, &actual, source))
        .unwrap_or(false)
}

fn paths_match_from_link(link: &Path, actual: &Path, expected: &Path) -> bool {
    let actual_abs = if actual.is_absolute() {
        actual.to_path_buf()
    } else {
        link.parent()
            .map(|parent| parent.join(actual))
            .unwrap_or_else(|| actual.to_path_buf())
    };
    if let (Ok(a), Ok(e)) = (
        std::fs::canonicalize(&actual_abs),
        std::fs::canonicalize(expected),
    ) {
        return a == e;
    }
    actual_abs == expected
}

fn describe_conflict(target: &Path, source: &Path) -> String {
    if let Ok(actual) = std::fs::read_link(target) {
        format!(
            "symlink points to {}, expected {}",
            actual.display(),
            source.display()
        )
    } else if target.is_dir() {
        "target is an existing directory".to_string()
    } else if target.is_file() {
        "target is an existing file".to_string()
    } else {
        "target exists with unsupported file type".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "x").unwrap();
    }

    #[test]
    fn link_creates_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        touch(&src);
        let target = dir.path().join("link");
        let plan = plan_link(
            dir.path(),
            &target,
            Path::new("src"),
            LinkSettings {
                create: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches!(plan.action, LinkAction::Link));
        apply_link(plan).unwrap();
        assert!(target.is_symlink());
    }

    #[test]
    fn link_skips_when_already_correct() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        touch(&src);
        let target = dir.path().join("link");
        std::os::unix::fs::symlink(&src, &target).unwrap();
        let plan = plan_link(
            dir.path(),
            &target,
            Path::new("src"),
            LinkSettings::default(),
        )
        .unwrap();
        assert!(matches!(plan.action, LinkAction::Skip));
    }

    #[test]
    fn link_backs_up_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        touch(&src);
        let target = dir.path().join("target");
        std::fs::write(&target, "old").unwrap();
        let plan = plan_link(
            dir.path(),
            &target,
            Path::new("src"),
            LinkSettings {
                backup: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(matches!(plan.action, LinkAction::Backup(_)));
        apply_link(plan).unwrap();
        // Target should now be a symlink; backup should exist.
        assert!(target.is_symlink());
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("target.backup.")
            })
            .collect();
        assert_eq!(backups.len(), 1);
    }

    #[test]
    fn link_fails_when_source_missing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("link");
        let plan = plan_link(
            dir.path(),
            &target,
            Path::new("nope"),
            LinkSettings::default(),
        )
        .unwrap();
        assert!(matches!(plan.action, LinkAction::Fail(_)));
    }
}
