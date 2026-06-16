use std::path::Path;
use std::process::{Command, Output};

fn write_deploy_config(repo: &Path, command: &str) {
    std::fs::create_dir_all(repo.join("config/fish")).expect("config");
    std::fs::write(repo.join("config/fish/config.fish"), "set fish_greeting\n").expect("fish");
    std::fs::write(
        repo.join("dotman.yaml"),
        format!(
            r#"
- defaults:
    link:
      create: true
      relink: true
      relative: true

- link:
    ~/.config/fish: config/fish

- create:
    - ~/.config/fish/local.d

- shell:
    - command: "{command}"
      description: Touch shell marker
"#
        ),
    )
    .expect("dotman yaml");
}

fn run_dotman(repo: &Path, home: &Path, args: &[&str]) -> Output {
    let exe = env!("CARGO_BIN_EXE_dotman");
    Command::new(exe)
        .current_dir(repo)
        .env("HOME", home)
        .args(args)
        .output()
        .expect("run dotman")
}

#[test]
fn deploy_dry_run_prints_plan_without_linking() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_deploy_config(repo.path(), "true");

    let output = run_dotman(
        repo.path(),
        home.path(),
        &["deploy", "--dry-run", "--except", "shell"],
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("link dry-run"), "stdout: {stdout}");
    assert!(stdout.contains("create dry-run"), "stdout: {stdout}");

    assert!(!home.path().join(".config/fish").exists());
}

#[test]
fn deploy_links_and_creates_directories() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_deploy_config(repo.path(), "true");

    let output = run_dotman(repo.path(), home.path(), &["deploy", "--except", "shell"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let link = std::fs::read_link(home.path().join(".config/fish")).expect("fish link");
    assert!(!link.is_absolute(), "expected relative link, got {link:?}");
    assert!(home.path().join(".config/fish/local.d").is_dir());
}

#[test]
fn deploy_only_link_still_applies_defaults() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_deploy_config(repo.path(), "false");

    let output = run_dotman(repo.path(), home.path(), &["deploy", "--only", "link"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(std::fs::read_link(home.path().join(".config/fish")).is_ok());
    assert!(!home.path().join(".config/fish/local.d").exists());
}

#[test]
fn deploy_except_shell_skips_shell_commands() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let marker = home.path().join("shell-marker");
    write_deploy_config(repo.path(), &format!("touch {}", marker.to_string_lossy()));

    let output = run_dotman(repo.path(), home.path(), &["deploy", "--except", "shell"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(!marker.exists());
}
