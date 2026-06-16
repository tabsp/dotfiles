use crate::path::expand_home;
use clap::ValueEnum;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Directive {
    Defaults,
    Link,
    Create,
    Shell,
    Clean,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum Step {
    Defaults { defaults: DefaultsStep },
    Link { link: BTreeMap<String, LinkValue> },
    Create { create: CreateValue },
    Shell { shell: Vec<ShellValue> },
    Clean { clean: Vec<String> },
}

#[derive(Clone, Debug, Default, Deserialize)]
struct DefaultsStep {
    #[serde(default)]
    link: LinkOptions,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct LinkOptions {
    #[serde(default)]
    create: Option<bool>,
    #[serde(default)]
    relink: Option<bool>,
    #[serde(default)]
    backup: Option<bool>,
    #[serde(default)]
    relative: Option<bool>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum LinkValue {
    Path(String),
    Options(LinkItem),
}

#[derive(Clone, Debug, Deserialize)]
struct LinkItem {
    path: String,
    #[serde(flatten)]
    options: LinkOptions,
    #[serde(rename = "if")]
    condition: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum CreateValue {
    Paths(Vec<String>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum ShellValue {
    Command(String),
    Options(ShellItem),
}

#[derive(Clone, Debug, Deserialize)]
struct ShellItem {
    command: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    stdout: bool,
    #[serde(default)]
    stderr: bool,
}

#[derive(Clone, Debug, Default)]
struct LinkDefaults {
    create: bool,
    relink: bool,
    backup: bool,
    relative: bool,
}

impl LinkDefaults {
    fn apply(&mut self, options: &LinkOptions) {
        if let Some(value) = options.create {
            self.create = value;
        }
        if let Some(value) = options.relink {
            self.relink = value;
        }
        if let Some(value) = options.backup {
            self.backup = value;
        }
        if let Some(value) = options.relative {
            self.relative = value;
        }
    }

    fn merged(&self, options: &LinkOptions) -> LinkSettings {
        LinkSettings {
            create: options.create.unwrap_or(self.create),
            relink: options.relink.unwrap_or(self.relink),
            backup: options.backup.unwrap_or(self.backup),
            relative: options.relative.unwrap_or(self.relative),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct LinkSettings {
    create: bool,
    relink: bool,
    backup: bool,
    relative: bool,
}

#[derive(Clone, Debug)]
struct LinkPlan {
    target: PathBuf,
    link_source: PathBuf,
    settings: LinkSettings,
    action: LinkAction,
}

#[derive(Clone, Debug)]
enum LinkAction {
    Link,
    Relink,
    Backup(PathBuf),
    Skip,
    Fail(String),
}

pub fn run_deploy(
    config_path: &Path,
    dry_run: bool,
    only: &[Directive],
    except: &[Directive],
) -> Result<(), String> {
    if !only.is_empty() && !except.is_empty() {
        return Err("--only and --except cannot be used together".to_string());
    }

    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let steps = load_steps(config_path)?;
    let mut link_defaults = LinkDefaults::default();

    for step in steps {
        let directive = step.directive();
        if !should_run(directive, only, except) {
            continue;
        }

        match step {
            Step::Defaults { defaults } => {
                link_defaults.apply(&defaults.link);
                if dry_run {
                    println!("defaults: link");
                }
            }
            Step::Link { link } => run_link_step(&repo, &link, link_defaults.clone(), dry_run)?,
            Step::Create { create } => run_create_step(create, dry_run)?,
            Step::Shell { shell } => run_shell_step(&shell, dry_run)?,
            Step::Clean { clean } => run_clean_step(&clean, dry_run)?,
        }
    }

    Ok(())
}

impl Step {
    fn directive(&self) -> Directive {
        match self {
            Step::Defaults { .. } => Directive::Defaults,
            Step::Link { .. } => Directive::Link,
            Step::Create { .. } => Directive::Create,
            Step::Shell { .. } => Directive::Shell,
            Step::Clean { .. } => Directive::Clean,
        }
    }
}

fn should_run(directive: Directive, only: &[Directive], except: &[Directive]) -> bool {
    if directive == Directive::Defaults {
        return !except.contains(&Directive::Defaults);
    }
    if !only.is_empty() {
        only.contains(&directive)
    } else {
        !except.contains(&directive)
    }
}

fn load_steps(path: &Path) -> Result<Vec<Step>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_yaml::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn run_link_step(
    repo: &Path,
    items: &BTreeMap<String, LinkValue>,
    defaults: LinkDefaults,
    dry_run: bool,
) -> Result<(), String> {
    for (target, value) in items {
        let (source, settings, condition) = match value {
            LinkValue::Path(path) => (
                path.as_str(),
                defaults.merged(&LinkOptions::default()),
                None,
            ),
            LinkValue::Options(item) => (
                item.path.as_str(),
                defaults.merged(&item.options),
                item.condition.as_deref(),
            ),
        };

        if let Some(condition) = condition
            && !condition_matches(condition)?
        {
            if dry_run {
                println!("link skip: {target} (condition false: {condition})");
            }
            continue;
        }

        let plan = plan_link(repo, source, target, settings)?;
        print_link_plan(&plan, dry_run);
        if dry_run {
            if let LinkAction::Fail(reason) = &plan.action {
                return Err(format!("dry-run: would fail linking {}: {reason}", target));
            }
            continue;
        }
        apply_link_plan(plan)?;
    }
    Ok(())
}

fn plan_link(
    repo: &Path,
    source: &str,
    target: &str,
    settings: LinkSettings,
) -> Result<LinkPlan, String> {
    let source = repo.join(source);
    let target = expand_home(target)?;

    if !source.exists() {
        return Ok(LinkPlan {
            target: target.clone(),
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

    let action = if target.exists() || target.is_symlink() {
        if is_expected_symlink(&target, &source) {
            LinkAction::Skip
        } else if target.is_symlink() && settings.relink {
            LinkAction::Relink
        } else if settings.backup {
            LinkAction::Backup(unique_backup_path(&target))
        } else {
            LinkAction::Fail(describe_conflict(&target, &source))
        }
    } else {
        LinkAction::Link
    };

    Ok(LinkPlan {
        target,
        link_source,
        settings,
        action,
    })
}

fn print_link_plan(plan: &LinkPlan, dry_run: bool) {
    let prefix = if dry_run { "link dry-run" } else { "link" };
    match &plan.action {
        LinkAction::Link => println!(
            "{prefix}: {} -> {}",
            plan.target.display(),
            plan.link_source.display()
        ),
        LinkAction::Relink => println!(
            "{prefix}: relink {} -> {}",
            plan.target.display(),
            plan.link_source.display()
        ),
        LinkAction::Backup(backup) => println!(
            "{prefix}: backup {} to {}; link -> {}",
            plan.target.display(),
            backup.display(),
            plan.link_source.display()
        ),
        LinkAction::Skip => println!("{prefix}: ok {}", plan.target.display()),
        LinkAction::Fail(reason) => println!("{prefix}: fail {} ({reason})", plan.target.display()),
    }
}

fn apply_link_plan(plan: LinkPlan) -> Result<(), String> {
    match &plan.action {
        LinkAction::Fail(reason) => Err(format!(
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
                LinkAction::Relink => fs::remove_file(&plan.target).map_err(|err| {
                    format!("failed to remove link {}: {err}", plan.target.display())
                })?,
                LinkAction::Backup(backup) => fs::rename(&plan.target, backup)
                    .map_err(|err| format!("failed to backup {}: {err}", plan.target.display()))?,
                _ => {}
            }

            unix_fs::symlink(&plan.link_source, &plan.target)
                .map_err(|err| format!("failed to link {}: {err}", plan.target.display()))
        }
    }
}

fn run_create_step(create: CreateValue, dry_run: bool) -> Result<(), String> {
    let CreateValue::Paths(paths) = create;
    for path in paths {
        let path = expand_home(&path)?;
        if dry_run {
            println!("create dry-run: {}", path.display());
        } else {
            println!("create: {}", path.display());
            create_dir_all_following_symlinks(&path)?;
        }
    }
    Ok(())
}

fn run_shell_step(items: &[ShellValue], dry_run: bool) -> Result<(), String> {
    for item in items {
        let shell = match item {
            ShellValue::Command(command) => ShellItem {
                command: command.clone(),
                description: None,
                stdout: false,
                stderr: false,
            },
            ShellValue::Options(item) => item.clone(),
        };

        let label = shell.description.as_deref().unwrap_or(&shell.command);
        if dry_run {
            println!("shell dry-run: {label}");
            println!("  command: {}", shell.command);
            continue;
        }

        println!("shell: {label}");
        let mut command = Command::new("sh");
        command.arg("-c").arg(&shell.command);
        if !shell.stdout {
            command.stdout(Stdio::null());
        }
        if !shell.stderr {
            command.stderr(Stdio::null());
        }
        let status = command
            .status()
            .map_err(|err| format!("failed to run shell command '{}': {err}", shell.command))?;
        if !status.success() {
            return Err(format!("shell command failed: {}", shell.command));
        }
    }
    Ok(())
}

fn run_clean_step(paths: &[String], dry_run: bool) -> Result<(), String> {
    for path in paths {
        let path = expand_home(path)?;
        if dry_run {
            println!("clean dry-run: {}", path.display());
        } else {
            println!("clean: {} (not implemented)", path.display());
        }
    }
    Ok(())
}

fn condition_matches(condition: &str) -> Result<bool, String> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(condition)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("failed to evaluate condition '{condition}': {err}"))?;
    Ok(status.success())
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("target has no parent: {}", path.display()))?;
    if parent.exists() && !parent.is_dir() {
        return Err(format!(
            "target parent is not a directory: {}",
            parent.display()
        ));
    }
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create {}: {err}", parent.display()))
}

fn create_dir_all_following_symlinks(path: &Path) -> Result<(), String> {
    if path.exists() {
        return if path.is_dir() {
            Ok(())
        } else {
            Err(format!("path is not a directory: {}", path.display()))
        };
    }

    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if current.as_os_str().is_empty() || current.exists() {
            if current.exists() && !current.is_dir() {
                return Err(format!("path is not a directory: {}", current.display()));
            }
            continue;
        }
        match fs::create_dir(&current) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists && current.is_dir() => {}
            Err(err) => return Err(format!("failed to create {}: {err}", current.display())),
        }
    }
    Ok(())
}

fn ensure_existing_parent_dir(path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("target has no parent: {}", path.display()))?;
    if parent.is_dir() {
        Ok(())
    } else {
        Err(format!(
            "target parent does not exist or is not a directory: {}",
            parent.display()
        ))
    }
}

fn is_expected_symlink(target: &Path, source: &Path) -> bool {
    fs::read_link(target)
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
    match (fs::canonicalize(&actual_abs), fs::canonicalize(expected)) {
        (Ok(actual), Ok(expected)) => actual == expected,
        _ => actual_abs == expected,
    }
}

fn describe_conflict(target: &Path, source: &Path) -> String {
    if let Ok(actual) = fs::read_link(target) {
        return format!(
            "symlink points to {}, expected {}",
            actual.display(),
            source.display()
        );
    }
    if target.is_dir() {
        "target is an existing directory".to_string()
    } else if target.is_file() {
        "target is an existing file".to_string()
    } else {
        "target exists with unsupported file type".to_string()
    }
}

fn unique_backup_path(target: &Path) -> PathBuf {
    let ts = timestamp();
    let mut candidate = PathBuf::from(format!("{}.backup.{ts}", target.display()));
    let mut counter = 1;
    while candidate.exists() {
        candidate = PathBuf::from(format!("{}.backup.{ts}.{counter}", target.display()));
        counter += 1;
    }
    candidate
}

fn timestamp() -> String {
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    let format = time::macros::format_description!("[year][month][day][hour][minute][second]");
    now.format(&format)
        .unwrap_or_else(|_| "19700101000000".to_string())
}

fn relative_link_source(source: &Path, target: &Path) -> Result<PathBuf, String> {
    let target_parent = target
        .parent()
        .ok_or_else(|| format!("target has no parent: {}", target.display()))?;
    diff_paths(&absolute_path(source)?, &absolute_path(target_parent)?)
        .ok_or_else(|| format!("failed to compute relative link for {}", target.display()))
}

fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Ok(canonical);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .map_err(|err| format!("failed to read current dir: {err}"))?
    };

    let mut missing = Vec::new();
    let mut existing = absolute.as_path();
    while !existing.exists() {
        let file_name = existing
            .file_name()
            .ok_or_else(|| format!("failed to resolve path: {}", path.display()))?;
        missing.push(file_name.to_os_string());
        existing = existing
            .parent()
            .ok_or_else(|| format!("failed to resolve path: {}", path.display()))?;
    }

    let mut resolved = fs::canonicalize(existing)
        .map_err(|err| format!("failed to resolve {}: {err}", existing.display()))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotbot_style_steps() {
        let raw = r#"
- defaults:
    link:
      create: true
      relink: true
      relative: true
- link:
    ~/.config/fish: config/fish
    ~/.tmux.conf:
      path: config/tmux.conf
      backup: true
- create:
    - ~/.config/fish/local.d
- shell:
    - command: fish -lc 'fisher update'
      description: Sync fish plugins
      stdout: true
"#;
        let steps: Vec<Step> = serde_yaml::from_str(raw).expect("parse");
        assert_eq!(steps.len(), 4);
        assert!(matches!(steps[0], Step::Defaults { .. }));
        assert!(matches!(steps[1], Step::Link { .. }));
        assert!(matches!(steps[2], Step::Create { .. }));
        assert!(matches!(steps[3], Step::Shell { .. }));
    }

    #[test]
    fn relative_link_source_points_from_target_parent_to_source() {
        let repo = tempfile::tempdir().expect("repo");
        let home = tempfile::tempdir().expect("home");
        let source = repo.path().join("config/fish");
        let target_parent = home.path().join(".config");
        let target = target_parent.join("fish");
        fs::create_dir_all(&source).expect("source");
        fs::create_dir_all(&target_parent).expect("target parent");

        let relative = relative_link_source(&source, &target).expect("relative");
        assert!(!relative.is_absolute(), "expected relative path");
    }
}
