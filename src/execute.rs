//! Execute: run a Plan, produce a Run.
//!
//! Phase 3: orchestrate ops/{install, link, create, shell, clean} with retry
//! and real-time streaming output.

use crate::config::Config;
use crate::model::ActionStatus;
use crate::model::{OutputStream, Plan, Run};
use crate::ops::shell;
use anyhow::Result;
use std::sync::mpsc;

mod command;
mod installer;
mod result;
mod runner;

#[cfg(test)]
use installer::shell_quote;
#[cfg(test)]
use installer::{DEFAULT_INSTALL_RETRIES, run_install_streaming};
#[cfg(test)]
use result::{cap_output_len, push_output_line};

/// Maximum output lines per step in TUI (before truncation).
pub const MAX_TUI_OUTPUT_LINES: usize = 1000;

#[derive(Debug, Clone)]
pub enum ExecuteEvent {
    ItemStarted {
        index: usize,
        name: String,
    },
    ActionStarted {
        item_index: usize,
        action_index: usize,
        item: String,
        action: String,
    },
    ActionFinished {
        item_index: usize,
        action_index: usize,
        item: String,
        action: String,
        status: ActionStatus,
    },
    /// Real-time stdout/stderr output line.
    Output {
        item: String,
        stream: OutputStream,
        line: String,
    },
    /// Structured action feedback (link, create, clean).
    ActionMessage {
        item: String,
        message: String,
    },
    ActionError {
        item: String,
        message: String,
    },
    ItemFinished {
        index: usize,
        name: String,
        status: ActionStatus,
    },
    SudoPrompt {
        item: String,
        response: mpsc::Sender<bool>,
    },
    Aborted,
}

pub fn execute(plan: &Plan, config: &Config) -> Result<Run> {
    execute_with_events(plan, config, |_| {}, || false)
}

pub fn execute_with_events<F, C>(
    plan: &Plan,
    config: &Config,
    emit: F,
    should_abort: C,
) -> Result<Run>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
{
    execute_with_events_and_sudo(plan, config, emit, should_abort, |_| {
        shell::pre_cache_sudo().unwrap_or(false)
    })
}

pub fn execute_with_events_and_sudo<F, C, S>(
    plan: &Plan,
    config: &Config,
    emit: F,
    should_abort: C,
    sudo_auth: S,
) -> Result<Run>
where
    F: FnMut(ExecuteEvent),
    C: Fn() -> bool,
    S: FnMut(&str) -> bool,
{
    runner::run(plan, config, emit, should_abort, sudo_auth)
}
#[cfg(test)]
mod tests;
