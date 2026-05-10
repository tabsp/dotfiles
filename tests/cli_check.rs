mod common;

use common::{current_host_params_table, current_host_table, non_current_host_table};
use predicates::prelude::*;

#[test]
fn check_rejects_missing_manifests() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:"));
}

#[test]
fn check_reports_missing_current_host_entry() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.git]
command = "git"

{}
installer = "system"
version = "latest"
"#,
            non_current_host_table("git")
        ),
    )
    .expect("deps");
    std::fs::write(temp.path().join("dotfiles.toml"), "files = []\n").expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("has no current-host entry"));
}

#[test]
fn check_aggregates_manifest_errors() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("config/a")).expect("config");
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.one]
command = "same"
{}
installer = "brew"
version = "1.0.0"
{}
package = "one"

[deps.two]
command = "same"
{}
installer = "brew"
version = "latest"
source = "http://example.invalid"
{}
package = "two"
"#,
            current_host_table("one"),
            current_host_params_table("one"),
            current_host_table("two"),
            current_host_params_table("two")
        ),
    )
    .expect("deps");
    std::fs::write(
        temp.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/a"
target = "relative-target"
kind = "dir"
"#,
    )
    .expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("duplicate command"))
        .stderr(predicate::str::contains("pins version"))
        .stderr(predicate::str::contains("source must use https"))
        .stderr(predicate::str::contains("target must be absolute or ~-based"));
}
