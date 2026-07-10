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
