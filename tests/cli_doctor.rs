mod common;

use common::current_host_table;
use predicates::prelude::*;

#[test]
fn doctor_returns_nonzero_for_missing_link() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(repo.path().join("config")).expect("config dir");
    std::fs::write(repo.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(
        repo.path().join("deps.toml"),
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
        repo.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#,
    )
    .expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing target symlink"));
}

#[test]
fn doctor_warns_for_version_drift_but_keeps_exit_zero() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let bin = tempfile::tempdir().expect("bin");
    let fake = bin.path().join("fakecmd");
    std::fs::write(&fake, "#!/bin/sh\necho fakecmd 2.0.0\n").expect("fake command");
    make_executable(&fake);
    std::fs::create_dir_all(repo.path().join("config")).expect("config dir");
    std::fs::write(repo.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::os::unix::fs::symlink(
        repo.path().join("config/tmux.conf"),
        home.path().join(".tmux.conf"),
    )
    .expect("link");
    std::fs::write(
        repo.path().join("deps.toml"),
        format!(
            r#"
[deps.fake]
command = "fakecmd"
[deps.fake.version_check]
args = ["--version"]
regex = 'fakecmd ([0-9]+\.[0-9]+\.[0-9]+)'
{}
installer = "system"
version = "1.0.0"
"#,
            current_host_table("fake")
        ),
    )
    .expect("deps");
    std::fs::write(
        repo.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#,
    )
    .expect("dotfiles");

    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(repo.path())
        .env("HOME", home.path())
        .env("PATH", path)
        .arg("doctor")
        .assert()
        .success()
        .stderr(predicate::str::contains("version drift"));
}

fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = std::fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}
