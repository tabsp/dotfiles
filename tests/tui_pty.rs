use assert_cmd::cargo::cargo_bin;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn history_tui_starts_and_exits_under_pty() {
    assert!(
        !Command::new("sh")
            .args(["-c", "command -v script >/dev/null 2>&1"])
            .status()
            .map_or(true, |status| !status.success()),
        "PTY test requires the `script` command"
    );

    let temp = tempfile::tempdir().unwrap();
    let data_home = temp.path().join("data");
    let runs_dir = data_home.join("dotman").join("runs");
    std::fs::create_dir_all(&runs_dir).unwrap();
    std::fs::write(runs_dir.join("01TESTPTYRUN000000000000.json"), sample_run()).unwrap();

    let bin = cargo_bin("dotman");
    let mut command = script_command(&bin.to_string_lossy());
    command
        .env("DOTMAN_ICONS", "plain")
        .env("XDG_DATA_HOME", &data_home)
        .env("HOME", temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().unwrap();
    child.stdin.as_mut().unwrap().write_all(b"qq").unwrap();

    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let output = child.wait_with_output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                status.success(),
                "status={status} stdout={stdout} stderr={stderr}"
            );
            break;
        }
        if started.elapsed() > Duration::from_secs(8) {
            let _ = child.kill();
            panic!("history TUI did not exit within timeout");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn script_command(bin: &str) -> Command {
    if cfg!(target_os = "linux") {
        let mut cmd = Command::new("script");
        cmd.args([
            "-q",
            "-c",
            &format!("{} history", shell_quote(bin)),
            "/dev/null",
        ]);
        cmd
    } else {
        let mut cmd = Command::new("script");
        cmd.args(["-q", "/dev/null", bin, "history"]);
        cmd
    }
}

/// Run `dotman` with the given args under a PTY so `is_tty()` is true.
fn script_command_with_args(bin: &str, args: &[&str]) -> Command {
    let shell_line = [bin.to_string(), args.join(" ")]
        .join(" ")
        .trim_end()
        .to_string();
    if cfg!(target_os = "linux") {
        let mut cmd = Command::new("script");
        cmd.args(["-q", "-c", &shell_line, "/dev/null"]);
        cmd
    } else {
        let mut cmd = Command::new("script");
        cmd.arg("-q").arg("/dev/null");
        cmd.arg(bin);
        cmd.args(args);
        cmd
    }
}

#[test]
fn headless_without_subcommand_exits_without_side_effects_under_pty() {
    assert!(
        !Command::new("sh")
            .args(["-c", "command -v script >/dev/null 2>&1"])
            .status()
            .map_or(true, |status| !status.success()),
        "PTY test requires the `script` command"
    );

    let temp = tempfile::tempdir().unwrap();
    let bin = cargo_bin("dotman");
    let mut command = script_command_with_args(&bin.to_string_lossy(), &["--headless"]);
    command
        .env("DOTMAN_ICONS", "plain")
        .env("HOME", temp.path())
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .env("XDG_DATA_HOME", temp.path().join("data"))
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "status={} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // `script` merges the child's stderr into the PTY, so the hint may land
    // on stdout here.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("headless mode requires an explicit subcommand")
            || stderr.contains("headless mode requires an explicit subcommand")
    );
    // No auto-init: no profile config was written and nothing was cloned.
    assert!(!temp.path().join(".config/dotman/config.toml").exists());
    assert!(!temp.path().join(".local/share/dotman/repos").exists());
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn sample_run() -> &'static str {
    r#"{
  "id": "01TESTPTYRUN000000000000",
  "mode": "Deploy",
  "started_at": "2026-01-01T00:00:00Z",
  "finished_at": "2026-01-01T00:00:01Z",
  "status": "Success",
  "config_hash": "test",
  "items": []
}"#
}
