use super::super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::tui) enum ReviewSeverity {
    Success,
    Skip,
    Run,
    Warning,
    Danger,
}

#[derive(Debug, Clone)]
pub(in crate::tui) struct ReviewEntry {
    pub(in crate::tui) order: usize,
    pub(in crate::tui) item: String,
    pub(in crate::tui) kind: &'static str,
    pub(in crate::tui) kind_icon: &'static str,
    pub(in crate::tui) severity: ReviewSeverity,
    pub(in crate::tui) status: String,
    pub(in crate::tui) detail: String,
}

pub(in crate::tui) fn review_entries(
    plan: &Plan,
    _config: Option<&config::Config>,
) -> Vec<ReviewEntry> {
    let icon_set = icons::current();
    let config_dir = plan
        .config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut entries = Vec::new();
    for item in plan.items.iter().filter(|item| item.selected) {
        for action in &item.actions {
            let mut entry = match action {
                Action::Install { spec } => review_install_entry(item, spec),
                Action::Link {
                    target,
                    source,
                    backup,
                    relink,
                } => review_link_entry(
                    item,
                    config_dir,
                    target,
                    source,
                    *backup,
                    *relink,
                    icon_set.action_link,
                ),
                Action::Create { target } => {
                    review_create_entry(item, target, icon_set.action_create)
                }
                Action::Shell {
                    command,
                    description,
                    optional,
                    if_condition,
                } => review_shell_entry(
                    item,
                    command,
                    description.as_deref(),
                    *optional,
                    if_condition.as_deref(),
                    config_dir,
                    icon_set.action_shell,
                ),
                Action::Clean { target, force } => {
                    review_clean_entry(item, target, *force, icon_set.action_clean)
                }
            };
            entry.order = entries.len();
            entries.push(entry);
        }
    }
    entries
}

pub(in crate::tui) fn review_install_entry(
    item: &PlanItem,
    spec: &install::InstallSpec,
) -> ReviewEntry {
    let icon_set = icons::current();
    let detail = spec
        .command
        .clone()
        .or_else(|| (!spec.entry.source_url.is_empty()).then(|| spec.entry.source_url.clone()))
        .or_else(|| spec.error.clone())
        .unwrap_or_else(|| format!("install {}", spec.entry.name));

    let presence = install::detect_presence(&spec.entry, spec.command.as_deref());
    if presence == install::InstallPresence::Present {
        return ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Success,
            status: "present".into(),
            detail,
        };
    }
    if let Some(error) = &spec.error {
        return ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Danger,
            status: "unavailable".into(),
            detail: error.clone(),
        };
    }

    match presence {
        install::InstallPresence::Present => unreachable!("handled above"),
        install::InstallPresence::Missing => ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Run,
            status: "missing".into(),
            detail,
        },
        install::InstallPresence::Unknown => ReviewEntry {
            order: 0,
            item: item.name.clone(),
            kind: "install",
            kind_icon: icon_set.action_install,
            severity: ReviewSeverity::Warning,
            status: "unknown".into(),
            detail,
        },
    }
}

pub(in crate::tui) fn review_link_entry(
    item: &PlanItem,
    config_dir: &Path,
    target: &Path,
    source: &Path,
    backup: bool,
    relink: bool,
    kind_icon: &'static str,
) -> ReviewEntry {
    let (severity, status) = match link::plan_link(
        config_dir,
        target,
        source,
        LinkSettings {
            create: true,
            relative: true,
            backup,
            relink,
        },
    ) {
        Ok(link_plan) => describe_link_review(&link_plan.action),
        Err(e) => (ReviewSeverity::Danger, format!("inspect failed: {e}")),
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "link",
        kind_icon,
        severity,
        status,
        detail: format!("{} -> {}", target.display(), source.display()),
    }
}

pub(in crate::tui) fn review_create_entry(
    item: &PlanItem,
    target: &Path,
    kind_icon: &'static str,
) -> ReviewEntry {
    let expanded = crate::path::expand_home(&target.to_string_lossy())
        .unwrap_or_else(|_| target.to_path_buf());
    let (severity, status) = if expanded.exists() {
        (ReviewSeverity::Success, "exists".into())
    } else {
        (ReviewSeverity::Run, "create".into())
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "create",
        kind_icon,
        severity,
        status,
        detail: target.display().to_string(),
    }
}

pub(in crate::tui) fn review_shell_entry(
    item: &PlanItem,
    command: &str,
    description: Option<&str>,
    optional: bool,
    if_condition: Option<&str>,
    config_dir: &Path,
    kind_icon: &'static str,
) -> ReviewEntry {
    let mut status = if optional {
        "optional".to_string()
    } else {
        "run".to_string()
    };
    let mut severity = ReviewSeverity::Run;
    if let Some(cond) = if_condition {
        match shell::condition_matches(cond, config_dir) {
            Ok(shell::ConditionResult::Matched) => status = format!("if ok · {status}"),
            Ok(shell::ConditionResult::NotMatched) => {
                status = "if skip".into();
                severity = ReviewSeverity::Skip;
            }
            Ok(shell::ConditionResult::Error(error)) => {
                status = format!("if error: {error}");
                severity = ReviewSeverity::Danger;
            }
            Err(error) => {
                status = format!("if error: {error}");
                severity = ReviewSeverity::Danger;
            }
        }
    }
    if shell::command_contains_sudo(command) && !matches!(severity, ReviewSeverity::Skip) {
        status = format!("{status} · sudo");
        if !matches!(severity, ReviewSeverity::Danger) {
            severity = ReviewSeverity::Warning;
        }
    }
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "shell",
        kind_icon,
        severity,
        status,
        detail: description.unwrap_or(command).to_string(),
    }
}

pub(in crate::tui) fn review_clean_entry(
    item: &PlanItem,
    target: &Path,
    force: bool,
    kind_icon: &'static str,
) -> ReviewEntry {
    let (severity, status) = match clean::plan_clean(target, force) {
        Ok(clean::CleanAction::Skip) => (ReviewSeverity::Skip, "skip".into()),
        Ok(clean::CleanAction::RemoveSymlink) => (ReviewSeverity::Warning, "remove symlink".into()),
        Ok(clean::CleanAction::BackupAndRemove(_)) => {
            (ReviewSeverity::Warning, "backup remove".into())
        }
        Err(e) => (ReviewSeverity::Danger, format!("inspect failed: {e}")),
    };
    ReviewEntry {
        order: 0,
        item: item.name.clone(),
        kind: "clean",
        kind_icon,
        severity,
        status,
        detail: target.display().to_string(),
    }
}

pub(in crate::tui) fn describe_link_review(action: &LinkAction) -> (ReviewSeverity, String) {
    match action {
        LinkAction::Skip => (ReviewSeverity::Success, "linked".into()),
        LinkAction::Link => (ReviewSeverity::Run, "link".into()),
        LinkAction::Backup(_) => (ReviewSeverity::Warning, "backup link".into()),
        LinkAction::Relink => (ReviewSeverity::Warning, "relink".into()),
        LinkAction::Fail(reason) => (ReviewSeverity::Danger, format!("fail: {reason}")),
    }
}
