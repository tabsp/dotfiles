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
    let mut cmd = command_with_dotman_env();
    let output = cmd
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
    let mut cmd = command_with_dotman_env();
    let status = cmd
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
    let mut cmd = command_with_dotman_env();
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

fn command_with_dotman_env() -> Command {
    let mut cmd = Command::new("sh");
    let path = path_with_homebrew();
    cmd.env("PATH", path);
    cmd
}

fn path_with_homebrew() -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    let mut paths: Vec<String> = Vec::new();
    for candidate in [
        "/home/linuxbrew/.linuxbrew/bin",
        "/opt/homebrew/bin",
        "/usr/local/bin",
    ] {
        if std::path::Path::new(candidate).is_dir() {
            paths.push(candidate.to_string());
        }
    }
    if !current.is_empty() {
        paths.push(current);
    }
    paths.join(":")
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

/// Returns true if any whitespace-separated word in the command equals "sudo".
///
/// Used to detect commands that may need a TTY for password input.
pub fn command_contains_sudo(cmd: &str) -> bool {
    cmd.split_whitespace().any(|w| w == "sudo")
}

/// Run `sudo -v` to cache credentials for subsequent sudo commands.
///
/// Call this before executing a plan that needs sudo. Once cached, sudo
/// credentials are valid for ~5 minutes, so subsequent sudo commands pass
/// without prompts even when stdin is `/dev/null`.
///
/// Inherits stdin/stdout/stderr so the user can enter their password.
/// Returns `true` if sudo authentication succeeded.
pub fn pre_cache_sudo() -> Result<bool> {
    match Command::new("sudo")
        .arg("-v")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => Ok(status.success()),
        Err(_) => Ok(false),
    }
}

/// Refresh the sudo timestamp only if previously cached and still valid.
///
/// Unlike `pre_cache_sudo`, this is non-interactive: it first probes with
/// `sudo -n true` to check whether there is an active cached session.  If the
/// session is still valid a `sudo -v` silently extends it; if it has expired we
/// do NOT prompt the user again — the command stays on `/dev/null` and instead
/// fails with a clear message.
///
/// Call before each `Action::Shell` that contains sudo, to prevent cache expiry
/// during long runs.
pub fn refresh_sudo() -> Result<bool> {
    // Non-interactive probe: is the timestamp still fresh?
    let ok = Command::new("sudo")
        .arg("-n")
        .arg("true")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if !ok {
        return Ok(false);
    }

    // Extend the timestamp without prompting.
    Command::new("sudo")
        .arg("-n")
        .arg("-v")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .map_err(Into::into)
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

    #[test]
    fn command_contains_sudo_detects_sudo() {
        assert!(command_contains_sudo("sudo apt install -y fish"));
        assert!(command_contains_sudo("sudo dnf install -y git"));
        assert!(command_contains_sudo("sudo pacman -S neovim"));
    }

    #[test]
    fn command_contains_sudo_rejects_non_sudo() {
        assert!(!command_contains_sudo("brew install fish"));
        assert!(!command_contains_sudo("echo hello"));
        assert!(!command_contains_sudo("make install"));
    }

    #[test]
    fn command_contains_sudo_exact_word_match() {
        // "sudo" must be a whole word, not a substring.
        assert!(!command_contains_sudo("pseudocode_here"));
        assert!(command_contains_sudo("echo sudo echo")); // word match
    }

    #[test]
    fn path_with_homebrew_preserves_existing_path() {
        let current = std::env::var("PATH").unwrap_or_default();
        let path = path_with_homebrew();

        if !current.is_empty() {
            assert!(path.ends_with(&current));
        }
    }
}
