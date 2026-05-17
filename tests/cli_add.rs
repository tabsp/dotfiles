mod common;

use common::{current_host_table, write_minimal_dotfiles};
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn read_or_empty(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn write_deps(repo: &Path, content: &str) {
    fs::write(repo.join("deps.toml"), content).expect("write deps.toml");
}

/// Return (installer_number, installer_name, extra_params) for a safe
/// cross-platform installer that passes `dotman check`.
fn safe_installer() -> (&'static str, &'static str, &'static str) {
    if cfg!(target_os = "macos") {
        ("2", "brew", "testdep\n")
    } else {
        // Linux: use system (1), no params needed
        ("1", "system", "")
    }
}

#[test]
fn add_dep_dry_run_prints_toml_no_file_change() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    write_deps(temp.path(), "");

    let original_deps = read_or_empty(&temp.path().join("deps.toml"));
    let (installer_num, _installer_name, extra) = safe_installer();

    // name, command(default), installer, version(default), source(skip),
    // extra params, confirm(default=yes)
    let stdin_input = format!("testdep\n\n{installer_num}\n\n\n{extra}\n");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("add")
        .arg("dep")
        .arg("--dry-run")
        .write_stdin(stdin_input)
        .assert()
        .success()
        .stdout(predicate::str::contains("[deps.testdep]"))
        .stdout(predicate::str::contains("command = \"testdep\""))
        .stdout(predicate::str::contains("dry-run"));

    assert_eq!(
        read_or_empty(&temp.path().join("deps.toml")),
        original_deps,
        "deps.toml unchanged after dry-run"
    );
}

#[test]
fn add_config_dry_run_prints_toml_no_file_change() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    write_deps(
        temp.path(),
        &format!(
            "[deps.tmux]\ncommand = \"tmux\"\n{}\ninstaller = \"system\"\nversion = \"latest\"\n",
            current_host_table("tmux")
        ),
    );

    let original_dotfiles = read_or_empty(&temp.path().join("dotfiles.toml"));

    // source, target, kind(default=file), platforms(default=all),
    // enabled(default=yes), notes(skip)
    // dry_run: create_source skipped
    let stdin_input = "config/ripgreprc\n~/.ripgreprc\n\n\n\n\n";

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("add")
        .arg("config")
        .arg("--dry-run")
        .write_stdin(stdin_input)
        .assert()
        .success()
        .stdout(predicate::str::contains("[[files]]"))
        .stdout(predicate::str::contains("source = \"config/ripgreprc\""))
        .stdout(predicate::str::contains("target = \"~/.ripgreprc\""))
        .stdout(predicate::str::contains("dry-run"));

    assert_eq!(
        read_or_empty(&temp.path().join("dotfiles.toml")),
        original_dotfiles,
        "dotfiles.toml unchanged after dry-run"
    );
}

#[test]
fn add_dep_rejects_duplicate_command() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    write_deps(
        temp.path(),
        &format!(
            "[deps.existing]\ncommand = \"existing\"\n{}\ninstaller = \"system\"\nversion = \"latest\"\n",
            current_host_table("existing")
        ),
    );

    let original_deps = read_or_empty(&temp.path().join("deps.toml"));

    // name=dupdep, command=existing (duplicate!) → fails immediately
    let stdin_input = "dupdep\nexisting\n";

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("add")
        .arg("dep")
        .write_stdin(stdin_input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already"));

    assert_eq!(
        read_or_empty(&temp.path().join("deps.toml")),
        original_deps,
        "deps.toml unchanged after rejection"
    );
}

#[test]
fn add_config_rejects_duplicate_target() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    write_deps(
        temp.path(),
        &format!(
            "[deps.tmux]\ncommand = \"tmux\"\n{}\ninstaller = \"system\"\nversion = \"latest\"\n",
            current_host_table("tmux")
        ),
    );

    let original_dotfiles = read_or_empty(&temp.path().join("dotfiles.toml"));

    // source=config/other, target=~/.tmux.conf (duplicate!)
    // Then retry attempts consume: "file", "y", "all" → fail after 3
    let stdin_input = "config/other\n~/.tmux.conf\nfile\ny\nall\n";

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("add")
        .arg("config")
        .write_stdin(stdin_input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("already"));

    assert_eq!(
        read_or_empty(&temp.path().join("dotfiles.toml")),
        original_dotfiles,
        "dotfiles.toml unchanged after rejection"
    );
}

#[test]
fn add_dep_writes_valid_entry_that_passes_check() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    write_deps(temp.path(), "");

    let (installer_num, _installer_name, extra) = safe_installer();

    // name, command(default), installer, version(default),
    // source(skip), extra params, confirm(yes)
    let stdin_input = format!("testcli\n\n{installer_num}\n\n\n{extra}\n");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("add")
        .arg("dep")
        .write_stdin(stdin_input)
        .assert()
        .success();

    let mut check_cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check_cmd
        .current_dir(temp.path())
        .arg("check")
        .assert()
        .success();
}

#[test]
fn add_config_writes_valid_entry_and_creates_source_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    write_deps(
        temp.path(),
        &format!(
            "[deps.tmux]\ncommand = \"tmux\"\n{}\ninstaller = \"system\"\nversion = \"latest\"\n",
            current_host_table("tmux")
        ),
    );

    // source, target, kind(default=file), create(yes), platforms(default=all),
    // enabled(yes), notes(skip), confirm(yes)
    let stdin_input = "config/newfile\n~/.newfile\n\n\n\n\n\n\n";

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("add")
        .arg("config")
        .write_stdin(stdin_input)
        .assert()
        .success();

    assert!(
        temp.path().join("config/newfile").exists(),
        "config/newfile must be created"
    );

    let mut check_cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check_cmd
        .current_dir(temp.path())
        .arg("check")
        .assert()
        .success();
}

#[test]
fn add_dep_atomic_failure_leaves_original_intact() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());

    let original_deps = format!(
        "[deps.tmux]\ncommand = \"tmux\"\n{}\ninstaller = \"system\"\nversion = \"latest\"\n",
        current_host_table("tmux")
    );
    write_deps(temp.path(), &original_deps);

    // Add a download_binary dep but provide "n" for all required params
    // The prompts will re-prompt (max 3), then fail. The deps.toml stays unchanged.
    // name=baddep, command(default), installer(8=download_binary), version=0.1.0,
    // source=https://...  → then params: url="n" invalid (3 attempts)
    let stdin_input = "baddep\n\n8\n0.1.0\nhttps://github.com/test/dep\nn\nn\nn\n";

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("add")
        .arg("dep")
        .write_stdin(stdin_input)
        .assert()
        .failure();

    assert_eq!(
        read_or_empty(&temp.path().join("deps.toml")),
        original_deps,
        "deps.toml unchanged after failure"
    );
}

#[test]
fn add_config_creates_source_directory_when_kind_is_dir() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    write_deps(
        temp.path(),
        &format!(
            "[deps.tmux]\ncommand = \"tmux\"\n{}\ninstaller = \"system\"\nversion = \"latest\"\n",
            current_host_table("tmux")
        ),
    );

    // source, target, kind=dir, create(yes), platforms(default=all),
    // enabled(yes), notes(skip), confirm(yes)
    let stdin_input = "config/newdir\n~/.newdir\ndir\n\n\n\n\n\n";

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("add")
        .arg("config")
        .write_stdin(stdin_input)
        .assert()
        .success();

    assert!(
        temp.path().join("config/newdir").is_dir(),
        "config/newdir must be a directory"
    );

    let mut check_cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    check_cmd
        .current_dir(temp.path())
        .arg("check")
        .assert()
        .success();
}
