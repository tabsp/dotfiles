mod common;

use common::current_host_table;
use predicates::prelude::*;

fn write_link_manifests(repo: &std::path::Path, target: &str) {
    std::fs::create_dir_all(repo.join("config")).expect("config dir");
    std::fs::write(repo.join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(
        repo.join("deps.toml"),
        format!(
            r#"
[deps.git]
command = "git"
{}
installer = "system"
version = "latest"
"#,
            current_host_table("git")
        ),
    )
    .expect("deps");
    std::fs::write(
        repo.join("dotfiles.toml"),
        format!(
            r#"
[[files]]
source = "config/tmux.conf"
target = "{target}"
kind = "file"
"#
        ),
    )
    .expect("dotfiles");
}

#[test]
fn link_dry_run_reports_conflict_reason_and_fails() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_link_manifests(repo.path(), "~/.tmux.conf");
    std::fs::write(home.path().join(".tmux.conf"), "user file\n").expect("target");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .args(["link", "--dry-run", "--conflict", "fail"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("WouldFail"))
        .stdout(predicate::str::contains(
            "reason: target is an existing file",
        ));
}

#[test]
fn link_conflict_backup_creates_backup_and_symlink() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_link_manifests(repo.path(), "~/.tmux.conf");
    std::fs::write(home.path().join(".tmux.conf"), "user file\n").expect("target");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .args(["link", "--conflict", "backup"])
        .assert()
        .success();

    assert!(std::fs::read_link(home.path().join(".tmux.conf")).is_ok());
    let backups = std::fs::read_dir(home.path())
        .expect("home entries")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".backup."))
        .count();
    assert_eq!(backups, 1);
}

#[test]
fn link_conflict_overwrite_replaces_target() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_link_manifests(repo.path(), "~/.tmux.conf");
    std::fs::write(home.path().join(".tmux.conf"), "user file\n").expect("target");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .args(["link", "--conflict", "overwrite"])
        .assert()
        .success();

    assert!(std::fs::read_link(home.path().join(".tmux.conf")).is_ok());
}
