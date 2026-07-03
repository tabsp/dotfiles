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
    Bootstrap,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub id: RunId,
    pub plan_id: RunId,
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
}

/// Per-machine selection state (read/written to ~/.local/share/dotman/state.toml).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Selection {
    pub items: BTreeMap<StepId, bool>,
}
