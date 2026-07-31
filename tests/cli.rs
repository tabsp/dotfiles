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
fn new_link_appends_to_sequence_form_preserving_extra_fields() {
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

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let updated = std::fs::read_to_string(&config).unwrap();
    // The existing item — including its backup field — is preserved, and a
    // new item is appended without rewriting the section.
    assert!(updated.contains("    backup: false\n"));
    assert!(updated.contains("  - target: ~/.config/example\n    source: config/example\n"));
    let document: Value = serde_yaml::from_str(&updated).unwrap();
    let items = document["links"].as_sequence().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0]["target"].as_str(), Some("~/.config/existing"));
    assert_eq!(items[0]["backup"].as_bool(), Some(false));
    assert_eq!(items[1]["target"].as_str(), Some("~/.config/example"));
}

#[test]
fn new_link_failure_leaves_the_config_byte_identical() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    // A non-empty inline flow map cannot be extended safely — the command
    // must fail without touching a single byte of the file.
    let original = "# header\r\nlinks: {a: b}\r\ncreate:\r\n  - ~/.config/x\r\n";
    std::fs::write(&config, original).unwrap();

    let output = isolated_command(&temp)
        .current_dir(temp.path())
        .args(["new-link", "~/.config/example", "config/example"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("edit dotman.yaml manually")
    );
    assert_eq!(std::fs::read(&config).unwrap(), original.as_bytes());
}

#[test]
fn new_link_keeps_a_symlink_config_intact() {
    let temp = tempfile::tempdir().unwrap();
    let real = temp.path().join("real-dotman.yaml");
    let link = temp.path().join("dotman.yaml");
    std::fs::write(&real, "install: []\nlinks: {}\n").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();

    let output = isolated_command(&temp)
        .current_dir(&cwd)
        .args([
            "new-link",
            "~/.config/example",
            "config/example",
            "--config",
        ])
        .arg(&link)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The symlink still points at the real file, which holds the edit.
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read_link(&link).unwrap(), real);
    let document: Value = serde_yaml::from_str(&std::fs::read_to_string(&real).unwrap()).unwrap();
    assert_eq!(
        document["links"]["~/.config/example"].as_str(),
        Some("config/example")
    );
}

#[test]
fn new_link_duplicate_target_fails_byte_identical() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    let original = "links:\n  - target: ~/.config/dup\n    source: config/a\n  - target: ~/.config/dup\n    source: config/b\n";
    std::fs::write(&config, original).unwrap();

    let output = isolated_command(&temp)
        .current_dir(temp.path())
        .args(["new-link", "~/.config/dup", "config/c"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("appears multiple times")
    );
    assert_eq!(std::fs::read(&config).unwrap(), original.as_bytes());
}

#[test]
fn new_link_preserves_comments_and_format_outside_the_links_section() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    let original = "# top comment\nlinks:\n  # layer comment\n  ~/.config/fish: config/fish\ncreate:\n  - ~/.config/x # keep me\n";
    std::fs::write(&config, original).unwrap();

    let output = isolated_command(&temp)
        .current_dir(temp.path())
        .args(["new-link", "~/.tmux.conf", "config/tmux.conf"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let updated = std::fs::read_to_string(&config).unwrap();
    assert_eq!(
        updated,
        "# top comment\nlinks:\n  # layer comment\n  ~/.config/fish: config/fish\n  ~/.tmux.conf: config/tmux.conf\ncreate:\n  - ~/.config/x # keep me\n"
    );
}

#[test]
fn new_link_uses_config_flag_outside_the_current_directory() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("elsewhere").join("dotman.yaml");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    std::fs::write(&config, "install: []\nlinks: {}\n").unwrap();
    let cwd = temp.path().join("repo");
    std::fs::create_dir_all(&cwd).unwrap();

    let output = isolated_command(&temp)
        .current_dir(&cwd)
        .args([
            "new-link",
            "~/.config/example",
            "config/example",
            "--config",
        ])
        .arg(&config)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("added link: ~/.config/example -> config/example"));
    let document: Value = serde_yaml::from_str(&std::fs::read_to_string(&config).unwrap()).unwrap();
    assert_eq!(
        document["links"]["~/.config/example"].as_str(),
        Some("config/example")
    );
    // The cwd is untouched: no dotman.yaml appears there.
    assert!(!cwd.join("dotman.yaml").exists());
}

#[test]
fn new_link_uses_the_active_profile_checkout_outside_the_current_directory() {
    let temp = tempfile::tempdir().unwrap();
    let checkout = temp.path().join("checkout");
    std::fs::create_dir_all(&checkout).unwrap();
    std::fs::write(
        checkout.join("dotman.yaml"),
        "install: []\nlinks:\n  ~/.config/fish: config/fish\n",
    )
    .unwrap();
    let profile_dir = temp.path().join(".config/dotman");
    std::fs::create_dir_all(&profile_dir).unwrap();
    std::fs::write(
        profile_dir.join("config.toml"),
        format!(
            "default_profile = \"main\"\n\n[profiles.main]\nrepo = \"https://example.com/dotfiles.git\"\nbranch = \"main\"\npath = \"{}\"\nconfig = \"dotman.yaml\"\nauto_sync = true\n",
            checkout.display()
        ),
    )
    .unwrap();
    let cwd = temp.path().join("elsewhere");
    std::fs::create_dir_all(&cwd).unwrap();

    let output = isolated_command(&temp)
        .current_dir(&cwd)
        .args(["new-link", "~/.tmux.conf", "config/tmux.conf"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let document: Value =
        serde_yaml::from_str(&std::fs::read_to_string(checkout.join("dotman.yaml")).unwrap())
            .unwrap();
    assert_eq!(
        document["links"]["~/.tmux.conf"].as_str(),
        Some("config/tmux.conf")
    );
    assert!(!cwd.join("dotman.yaml").exists());
}

#[test]
fn new_link_without_config_fails_without_auto_init_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path().join("empty");
    std::fs::create_dir_all(&cwd).unwrap();

    let output = isolated_command(&temp)
        .current_dir(&cwd)
        .args(["new-link", "~/.config/example", "config/example"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("run `dotman init` first"));
    // No auto-init: no profile config was written and nothing was cloned.
    assert!(!temp.path().join(".config/dotman/config.toml").exists());
    assert!(!temp.path().join(".local/share/dotman").exists());
}

#[test]
fn new_link_updates_existing_target_source_in_place() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("dotman.yaml");
    std::fs::write(
        &config,
        "links:\n  ~/.config/fish: config/fish # main shell\n",
    )
    .unwrap();

    let output = isolated_command(&temp)
        .current_dir(temp.path())
        .args(["new-link", "~/.config/fish", "config/fish-new"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let updated = std::fs::read_to_string(&config).unwrap();
    // The trailing comment survives the in-place update.
    assert_eq!(
        updated,
        "links:\n  ~/.config/fish: config/fish-new # main shell\n"
    );
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
fn doctor_reports_brew_missing_when_brew_is_not_on_path() {
    let temp = tempfile::tempdir().unwrap();
    // Explicit --config mapping the current OS's package manager to brew, so
    // the test never depends on the repo's dotman.yaml or the cwd.
    let config = temp.path().join("doctor.yaml");
    let yaml = if cfg!(target_os = "macos") {
        "package_managers:\n  macos: brew\ninstall: []\nlinks: {}\n"
    } else {
        "package_managers:\n  ubuntu: brew\n  debian: brew\n  arch: brew\n  fedora: brew\ninstall: []\nlinks: {}\n"
    };
    std::fs::write(&config, yaml).unwrap();
    let empty_bin = temp.path().join("empty-bin");
    std::fs::create_dir_all(&empty_bin).unwrap();
    let cwd = temp.path().join("cwd");
    std::fs::create_dir_all(&cwd).unwrap();

    let output = isolated_command(&temp)
        .current_dir(&cwd)
        .env("PATH", &empty_bin)
        .args(["doctor", "--config"])
        .arg(&config)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("✗ Homebrew not available"));
    assert!(!stdout.contains("All checks passed"));
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
