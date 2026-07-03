//! Shell command execution with stdout/stderr capture and if: condition eval.

use anyhow::Result;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn run_shell(command: &str, config_dir: &Path) -> Result<ShellOutput> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to run shell command '{command}'"))?;
    Ok(ShellOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

pub fn condition_matches(cond: &str) -> Result<bool> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(cond)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("failed to evaluate condition '{cond}'"))?;
    Ok(status.success())
}

use anyhow::Context;
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn condition_true_when_command_succeeds() {
        assert!(condition_matches("true").unwrap());
    }

    #[test]
    fn condition_false_when_command_fails() {
        assert!(!condition_matches("false").unwrap());
    }

    #[test]
    fn run_shell_captures_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let out = run_shell("echo hello", dir.path()).unwrap();
        assert_eq!(out.exit_code, 0);
        assert!(out.stdout.contains("hello"));
    }

    #[test]
    fn run_shell_captures_exit_code() {
        let dir = tempfile::tempdir().unwrap();
        let out = run_shell("false", dir.path()).unwrap();
        assert_ne!(out.exit_code, 0);
    }
}
