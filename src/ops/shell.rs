//! Shell command execution with streaming stdout/stderr and if: condition eval.

use crate::model::OutputStream;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;

#[derive(Debug, Clone)]
pub struct ShellOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// One line of streaming output from a running command.
#[derive(Debug, Clone)]
pub struct StreamLine {
    pub stream: OutputStream,
    pub line: String,
}

/// Run a shell command and collect all output at once (non-streaming, for conditions).
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

/// Run a shell command with streaming output.
///
/// Spawns two reader threads (stdout + stderr) and sends each line through the
/// provided sender. The main thread waits for the child to exit, then returns the
/// exit code.
///
/// On Unix, the child is spawned in a new process group. When abort is signaled,
/// the entire process group is killed to ensure subprocesses (e.g. brew, curl,
/// scripts) are also terminated.
///
/// Returns the exit code, or `None` if the child was killed.
pub fn run_command_streaming(
    command: &str,
    config_dir: &Path,
    tx: &mpsc::Sender<StreamLine>,
    abort: Arc<AtomicBool>,
) -> Result<Option<i32>> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c")
        .arg(command)
        .current_dir(config_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // On Unix, put the child in its own process group so we can kill the whole
    // subtree on abort.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn command '{command}'"))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let tx_stdout = tx.clone();
    let tx_stderr = tx.clone();
    let abort_stdout = Arc::clone(&abort);
    let abort_stderr = Arc::clone(&abort);

    // Spawn reader threads for stdout and stderr.
    let stdout_thread = thread::spawn(move || {
        if let Some(reader) = stdout {
            let buf = BufReader::new(reader);
            for line in buf.lines() {
                if abort_stdout.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(line) = line {
                    let _ = tx_stdout.send(StreamLine {
                        stream: OutputStream::Stdout,
                        line,
                    });
                }
            }
        }
    });

    let stderr_thread = thread::spawn(move || {
        if let Some(reader) = stderr {
            let buf = BufReader::new(reader);
            for line in buf.lines() {
                if abort_stderr.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(line) = line {
                    let _ = tx_stderr.send(StreamLine {
                        stream: OutputStream::Stderr,
                        line,
                    });
                }
            }
        }
    });

    // Wait for child to exit.
    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                if abort.load(Ordering::SeqCst) {
                    kill_process_tree(&mut child);
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(_) => break None,
        }
    };

    // Drain remaining reader threads.
    let _ = stdout_thread.join();
    let _ = stderr_thread.join();

    Ok(exit_code)
}

/// Kill the child process and all its descendants.
///
/// On Unix: sends SIGKILL to the entire process group.
/// On other platforms: falls back to killing only the immediate child.
fn kill_process_tree(child: &mut std::process::Child) {
    // Kill the process group on Unix.
    #[cfg(unix)]
    {
        // child.id() is the process group leader (we set process_group(0)).
        // -pid means "the process group whose PGID is pid".
        let pgid = -(child.id() as i32);
        unsafe { libc::kill(pgid, libc::SIGKILL) };
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
    // Always try child.kill() / wait() as a fallback on all platforms.
    let _ = child.kill();
    let _ = child.wait();
}

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

    #[test]
    fn streaming_command_captures_stdout_lines() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = mpsc::channel();
        let abort = Arc::new(AtomicBool::new(false));
        let abort_clone = Arc::clone(&abort);

        let handle = std::thread::spawn(move || {
            run_command_streaming("echo hello && echo world", dir.path(), &tx, abort_clone)
        });

        let lines: Vec<String> = rx
            .into_iter()
            .filter(|l| matches!(l.stream, OutputStream::Stdout))
            .map(|l| l.line)
            .collect();

        let exit = handle.join().unwrap().unwrap();
        assert_eq!(exit, Some(0));
        assert_eq!(lines, vec!["hello", "world"]);
    }

    #[test]
    fn streaming_command_captures_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = mpsc::channel();
        let abort = Arc::new(AtomicBool::new(false));
        let abort_clone = Arc::clone(&abort);

        let handle = std::thread::spawn(move || {
            run_command_streaming("echo err >&2", dir.path(), &tx, abort_clone)
        });

        let lines: Vec<String> = rx
            .into_iter()
            .filter(|l| matches!(l.stream, OutputStream::Stderr))
            .map(|l| l.line)
            .collect();

        let exit = handle.join().unwrap().unwrap();
        assert_eq!(exit, Some(0));
        assert!(lines.iter().any(|l| l.contains("err")));
    }

    #[test]
    fn streaming_command_abort_kills_child() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, _rx) = mpsc::channel();
        let abort = Arc::new(AtomicBool::new(true)); // abort immediately
        let exit = run_command_streaming("sleep 10", dir.path(), &tx, Arc::clone(&abort)).unwrap();
        assert_eq!(exit, None); // killed
    }
}
