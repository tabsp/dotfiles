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
            Action::Link { target, source } => {
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
    WillSkip,
    WillFail,
    NoChange,
    WillInstall,
    WillCreate,
    WillLink,
    WillBackupLink,
    WillClean,
    WillBackupRemove,
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
                ActionStatus::WillRun => 1,
                ActionStatus::WillSkip => 1,
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
    pub mode: Mode,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: RunStatus,
    pub config_hash: String,
    pub items: Vec<RunItem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Running,
    Success,
    Failed,
    Aborted,
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
