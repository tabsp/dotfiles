use assert_cmd::cargo::cargo_bin;
use serde_yaml::Value;
use std::process::Command;

fn isolated_command(temp: &tempfile::TempDir) -> Command {
    let mut command = Command::new(cargo_bin("dotman"));
    command
        .env("HOME", temp.path())
        .env("XDG_CONFIG_HOME", temp.path().join("config"))
        .env("XDG_DATA_HOME", temp.path().join("data"))
        .env("XDG_STATE_HOME", temp.path().join("state"))
        .env("XDG_CACHE_HOME", temp.path().join("cache"));
    command
}

#[test]
fn no_subcommand_outside_a_tty_is_a_safe_noop() {
    let temp = tempfile::tempdir().unwrap();
    let output = isolated_command(&temp).output().unwrap();

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("headless mode requires an explicit subcommand"));
    assert!(stderr.contains("dotman --help"));
}

#[test]
fn headless_plan_with_direct_config_emits_json_without_profile_resolution() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    std::fs::write(
        &config,
        "install: []\nlinks: {}\ncreate:\n  - ~/.config/example\nshell: []\nclean: []\n",
    )
    .unwrap();

    let output = isolated_command(&temp)
        .args(["plan", "--headless", "--config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(plan["config_path"], config.to_string_lossy().as_ref());
    assert_eq!(plan["items"].as_array().unwrap().len(), 1);
    assert_eq!(plan["items"][0]["actions"][0]["kind"], "create");
}

#[test]
fn invalid_direct_config_exits_nonzero_with_context() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    std::fs::write(&config, "unknown-field: true\n").unwrap();

    let output = isolated_command(&temp)
        .args(["plan", "--headless", "--config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("error:"));
    assert!(stderr.contains("failed to parse"));
    assert!(stderr.contains("dotman.yaml"));
}

#[test]
fn new_link_updates_the_config_in_the_current_repository() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    std::fs::write(&config, "install: []\nlinks: {}\n").unwrap();

    let output = isolated_command(&temp)
        .current_dir(temp.path())
        .args(["new-link", "~/.config/example", "config/example"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("added link: ~/.config/example -> config/example")
    );
    let document: Value = serde_yaml::from_str(&std::fs::read_to_string(config).unwrap()).unwrap();
    assert_eq!(
        document["links"]["~/.config/example"].as_str(),
        Some("config/example")
    );
}

#[test]
fn new_link_failure_leaves_the_config_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    let original =
        "links:\n  - target: ~/.config/existing\n    source: config/existing\n    backup: false\n";
    std::fs::write(&config, original).unwrap();

    let output = isolated_command(&temp)
        .current_dir(temp.path())
        .args(["new-link", "~/.config/example", "config/example"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr).unwrap().contains("backup"));
    assert_eq!(std::fs::read_to_string(config).unwrap(), original);
}

#[test]
fn headless_history_reports_an_empty_isolated_store() {
    let temp = tempfile::tempdir().unwrap();
    let output = isolated_command(&temp)
        .args(["history", "--headless"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "no runs yet\n");
    assert!(output.stderr.is_empty());
}

#[test]
fn headless_run_with_unknown_id_exits_nonzero_with_log_context() {
    let temp = tempfile::tempdir().unwrap();
    let output = isolated_command(&temp)
        .args(["run", "missing-run", "--headless"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("failed to load run missing-run"));
    assert!(stderr.contains("failed to read run log"));
    assert!(stderr.contains("missing-run.json"));
}

#[test]
fn sync_with_direct_config_does_not_require_a_profile() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    std::fs::write(&config, "install: []\nlinks: {}\n").unwrap();

    let output = isolated_command(&temp)
        .args(["sync", "--headless", "--config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("config loaded directly from"));
    assert!(stdout.contains(config.to_string_lossy().as_ref()));
    assert!(stdout.contains("no profile to sync"));
    assert!(output.stderr.is_empty());
}

#[test]
fn no_init_without_any_config_exits_nonzero_without_bootstrapping() {
    let temp = tempfile::tempdir().unwrap();
    let output = isolated_command(&temp)
        .current_dir(temp.path())
        .args(["plan", "--headless", "--no-init"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("no config found and --no-init is set"));
}

#[test]
fn profile_commands_preserve_state_and_report_duplicate_or_missing_names() {
    let temp = tempfile::tempdir().unwrap();

    let empty = isolated_command(&temp)
        .args(["profile", "list"])
        .output()
        .unwrap();
    assert!(empty.status.success());
    assert!(
        String::from_utf8(empty.stdout)
            .unwrap()
            .contains("No profiles configured")
    );

    let added = isolated_command(&temp)
        .args(["profile", "add", "work", "https://example.com/dotfiles.git"])
        .output()
        .unwrap();
    assert!(
        added.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&added.stderr)
    );
    assert!(
        String::from_utf8(added.stdout)
            .unwrap()
            .contains("profile 'work' added")
    );

    let listed = isolated_command(&temp)
        .args(["profile", "list"])
        .output()
        .unwrap();
    assert!(listed.status.success());
    let stdout = String::from_utf8(listed.stdout).unwrap();
    assert!(stdout.contains("work"));
    assert!(stdout.contains("https://example.com/dotfiles.git"));

    let duplicate = isolated_command(&temp)
        .args(["profile", "add", "work", "https://example.com/other.git"])
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert!(
        String::from_utf8(duplicate.stderr)
            .unwrap()
            .contains("profile 'work' already exists")
    );

    let removed = isolated_command(&temp)
        .args(["profile", "remove", "work"])
        .output()
        .unwrap();
    assert!(removed.status.success());
    assert!(
        String::from_utf8(removed.stdout)
            .unwrap()
            .contains("profile 'work' removed")
    );

    let missing = isolated_command(&temp)
        .args(["profile", "remove", "work"])
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(
        String::from_utf8(missing.stderr)
            .unwrap()
            .contains("profile 'work' not found")
    );
}
