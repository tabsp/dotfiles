//! Data types for plan, run, steps, items.
//!
//! Phase 1: define the shape. Phase 2+ wire it up.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub type StepId = String;
pub type RunId = String;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Deploy,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub id: RunId,
    pub mode: Mode,
    pub created_at: String, // ISO 8601
    pub config_path: PathBuf,
    pub config_hash: String,
    pub host: HostInfo,
    pub items: Vec<PlanItem>,
    /// Whether the plan includes an auto-install-package-manager step.
    /// Stored here so `needs_sudo()` can check it without accessing Config.
    #[serde(default)]
    pub auto_install_pkg_manager: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostInfo {
    pub hostname: String,
    pub os: String,
    pub arch: String,
    pub user: String,
    pub home: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlanItem {
    pub id: StepId,
    pub name: String,
    pub layer: String, // "terminal" / "shell" / "software" / "enhancement" / "misc"
    pub actions: Vec<Action>,
    pub selected: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Action {
    Install {
        pkg_mgr: String,
        binary: String,
        source: String, // install command (e.g., "brew install neovim")
    },
    Link {
        target: PathBuf,
        source: PathBuf,
        #[serde(default = "default_link_backup")]
        backup: bool,
        #[serde(default)]
        relink: bool,
    },
    Create {
        target: PathBuf,
    },
    Shell {
        command: String,
        description: Option<String>,
        optional: bool,
        if_condition: Option<String>,
    },
    Clean {
        target: PathBuf,
        force: bool,
    },
}

impl Action {
    pub fn describe(&self) -> String {
        match self {
            Action::Install { binary, .. } => format!("install {binary}"),
            Action::Link { target, source, .. } => {
                format!("link {} -> {}", target.display(), source.display())
            }
            Action::Create { target } => format!("create {}", target.display()),
            Action::Shell {
                command,
                description,
                ..
            } => description.clone().unwrap_or_else(|| command.clone()),
            Action::Clean { target, force } => {
                if *force {
                    format!("clean (force) {}", target.display())
                } else {
                    format!("clean {}", target.display())
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionStatus {
    WillRun,
    Executed,
    WillSkip,
    NotRun,
    Aborted,
    WillFail,
    NoChange,
    WillInstall,
    WillCreate,
    WillLink,
    WillBackupLink,
    WillClean,
    WillBackupRemove,
}

impl ActionStatus {
    pub fn result_label(self) -> &'static str {
        match self {
            Self::WillFail => "failed",
            Self::Aborted => "aborted",
            Self::WillSkip => "skipped",
            Self::NotRun => "not run",
            Self::NoChange => "no change",
            Self::WillRun | Self::Executed => "ran",
            Self::WillInstall
            | Self::WillCreate
            | Self::WillLink
            | Self::WillBackupLink
            | Self::WillClean
            | Self::WillBackupRemove => "changed",
        }
    }
}

impl PartialOrd for ActionStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ActionStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn rank(s: &ActionStatus) -> u8 {
            match s {
                ActionStatus::NoChange => 0,
                ActionStatus::WillRun | ActionStatus::Executed => 1,
                ActionStatus::WillSkip => 1,
                ActionStatus::NotRun => 1,
                ActionStatus::Aborted => 4,
                ActionStatus::WillLink => 2,
                ActionStatus::WillCreate => 2,
                ActionStatus::WillInstall => 2,
                ActionStatus::WillClean => 2,
                ActionStatus::WillBackupLink => 3,
                ActionStatus::WillBackupRemove => 3,
                ActionStatus::WillFail => 4,
            }
        }
        rank(self).cmp(&rank(other))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<RunId>,
    pub mode: Mode,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: RunStatus,
    pub config_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<HostInfo>,
    pub items: Vec<RunItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Aborted,
}

impl RunStatus {
    pub fn result_label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Aborted => "aborted",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub ran: usize,
    pub changed: usize,
    pub no_change: usize,
    pub failed: usize,
    pub aborted: usize,
    pub skipped: usize,
    pub not_run: usize,
}

impl RunSummary {
    pub fn from_run(run: &Run) -> Self {
        let mut summary = Self::default();
        for item in &run.items {
            if item.actions.is_empty() {
                summary.add(item.status);
            } else {
                for action in &item.actions {
                    summary.add(action.status);
                }
            }
        }
        summary
    }

    fn add(&mut self, status: ActionStatus) {
        match status {
            ActionStatus::WillFail => self.failed += 1,
            ActionStatus::Aborted => self.aborted += 1,
            ActionStatus::WillSkip => self.skipped += 1,
            ActionStatus::NotRun => self.not_run += 1,
            ActionStatus::NoChange => self.no_change += 1,
            ActionStatus::WillRun | ActionStatus::Executed => self.ran += 1,
            ActionStatus::WillInstall
            | ActionStatus::WillCreate
            | ActionStatus::WillLink
            | ActionStatus::WillBackupLink
            | ActionStatus::WillClean
            | ActionStatus::WillBackupRemove => self.changed += 1,
        }
    }

    pub fn display(self) -> String {
        let mut parts = vec![
            format!("{} ran", self.ran),
            format!("{} changed", self.changed),
            format!("{} no change", self.no_change),
            format!("{} failed", self.failed),
        ];
        if self.aborted > 0 {
            parts.push(format!("{} aborted", self.aborted));
        }
        if self.skipped > 0 {
            parts.push(format!("{} skipped", self.skipped));
        }
        if self.not_run > 0 {
            parts.push(format!("{} not run", self.not_run));
        }
        parts.join(", ")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunItem {
    pub id: StepId,
    pub name: String,
    pub status: ActionStatus,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub attempts: u32,
    pub error: Option<String>,
    /// Per-action output lines, capped at MAX_HISTORY_OUTPUT_LINES (500 by default).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output: Vec<OutputLine>,
    /// Per-action execution results. Older history entries may not have this.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<RunAction>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunAction {
    pub kind: String,
    pub name: String,
    pub status: ActionStatus,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output: Vec<OutputLine>,
}

/// A single line of output from an action (for run history).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutputLine {
    pub stream: OutputStream,
    pub line: String,
}

/// Stream kind for output lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputStream {
    #[serde(rename = "stdout")]
    Stdout,
    #[serde(rename = "stderr")]
    Stderr,
    #[serde(rename = "action")]
    Action,
}

/// Maximum output lines stored per step in run history (default 500).
pub const MAX_HISTORY_OUTPUT_LINES: usize = 500;

/// Per-machine selection state (read/written to ~/.local/share/dotman/state.toml).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Selection {
    pub items: BTreeMap<StepId, bool>,
}

fn default_link_backup() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{ActionStatus, Mode, Run, RunItem, RunStatus, RunSummary};

    #[test]
    fn result_labels_use_terminal_user_facing_language() {
        assert_eq!(ActionStatus::WillFail.result_label(), "failed");
        assert_eq!(ActionStatus::WillSkip.result_label(), "skipped");
        assert_eq!(ActionStatus::Aborted.result_label(), "aborted");
        assert_eq!(ActionStatus::NotRun.result_label(), "not run");
        assert_eq!(ActionStatus::NoChange.result_label(), "no change");
        assert_eq!(ActionStatus::WillLink.result_label(), "changed");
        assert_eq!(RunStatus::Success.result_label(), "success");
        assert_eq!(RunStatus::Failed.result_label(), "failed");
        assert_eq!(RunStatus::Aborted.result_label(), "aborted");
    }

    #[test]
    fn run_summary_falls_back_to_item_status_for_legacy_history() {
        let run = Run {
            id: "legacy".into(),
            plan_id: None,
            mode: Mode::Deploy,
            started_at: "2026-01-01T00:00:00Z".into(),
            finished_at: Some("2026-01-01T00:00:01Z".into()),
            status: RunStatus::Failed,
            config_hash: "hash".into(),
            config_path: None,
            host: None,
            items: vec![RunItem {
                id: "step".into(),
                name: "step".into(),
                status: ActionStatus::WillFail,
                started_at: None,
                finished_at: None,
                duration_ms: None,
                attempts: 1,
                error: Some("exit code 7".into()),
                output: vec![],
                actions: vec![],
            }],
        };

        assert_eq!(RunSummary::from_run(&run).failed, 1);
    }
}
