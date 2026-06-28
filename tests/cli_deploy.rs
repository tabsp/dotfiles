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
        .env_remove("DOTFILES_DIR")
        .args(args)
        .output()
        .expect("run dotman")
}

fn run_dotman_from(cwd: &Path, home: &Path, args: &[&str]) -> Output {
    let exe = env!("CARGO_BIN_EXE_dotman");
    Command::new(exe)
        .current_dir(cwd)
        .env("HOME", home)
        .env_remove("DOTFILES_DIR")
        .args(args)
        .output()
        .expect("run dotman")
}

fn run_dotman_from_with_env(
    cwd: &Path,
    home: &Path,
    envs: &[(&str, &Path)],
    args: &[&str],
) -> Output {
    let exe = env!("CARGO_BIN_EXE_dotman");
    let mut command = Command::new(exe);
    command
        .current_dir(cwd)
        .env("HOME", home)
        .env_remove("DOTFILES_DIR")
        .args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("run dotman")
}

#[test]
fn installer_uses_source_checkout_from_dotfiles_dir_without_downloading_bundle() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let bin = tempfile::tempdir().expect("bin");
    std::fs::create_dir_all(repo.path().join(".git")).expect("git dir");
    std::fs::create_dir_all(repo.path().join("scripts")).expect("scripts dir");
    std::fs::write(repo.path().join("dotman.yaml"), "[]\n").expect("dotman yaml");
    std::fs::write(
        repo.path().join("scripts/install.sh"),
        "#!/usr/bin/env sh\n",
    )
    .expect("install");
    std::fs::write(repo.path().join("sentinel"), "keep me\n").expect("sentinel");
    let dotman = bin.path().join("dotman");
    std::fs::write(
        &dotman,
        r#"#!/usr/bin/env sh
while [ $# -gt 0 ]; do
  case "$1" in
    --version) echo "dotman 0.0.0-test"; exit 0 ;;
    --summary) shift ;;
    --color) shift; shift ;;
    bootstrap|deploy) exit 0 ;;
    *) exit 1 ;;
  esac
done
"#,
    )
    .expect("dotman");
    std::fs::write(bin.path().join("brew"), "#!/usr/bin/env sh\nexit 0\n").expect("brew");
    std::fs::write(bin.path().join("fish"), "#!/usr/bin/env sh\nexit 0\n").expect("fish");
    std::fs::write(
        bin.path().join("sudo"),
        "#!/usr/bin/env sh\nprintf '%s\\n' \"$*\" >>\"$HOME/sudo-args\"\nexit 1\n",
    )
    .expect("sudo");
    std::fs::write(bin.path().join("chsh"), "#!/usr/bin/env sh\nexit 1\n").expect("chsh");
    let mut permissions = std::fs::metadata(&dotman)
        .expect("dotman metadata")
        .permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        permissions.set_mode(0o755);
        std::fs::set_permissions(&dotman, permissions.clone()).expect("dotman executable");
        std::fs::set_permissions(bin.path().join("brew"), permissions.clone())
            .expect("brew executable");
        std::fs::set_permissions(bin.path().join("fish"), permissions.clone())
            .expect("fish executable");
        std::fs::set_permissions(bin.path().join("sudo"), permissions.clone())
            .expect("sudo executable");
        std::fs::set_permissions(bin.path().join("chsh"), permissions).expect("chsh executable");
    }

    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/install.sh");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = Command::new("sh")
        .arg(script)
        .arg("--yes")
        .env("HOME", home.path())
        .env("DOTFILES_DIR", repo.path())
        .env("DOTMAN_BIN", &dotman)
        .env("DOTFILES_SITE_URL", "http://127.0.0.1:9")
        .env("PATH", path)
        .env("SHELL", "/bin/sh")
        .output()
        .expect("run installer");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("skipping published bundle download"),
        "stdout: {stdout}"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("sentinel")).expect("sentinel"),
        "keep me\n"
    );
    let sudo_args = std::fs::read_to_string(home.path().join("sudo-args")).expect("sudo args");
    assert!(
        sudo_args.lines().any(|line| line.starts_with("-n ")),
        "sudo args: {sudo_args}"
    );
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
    assert!(stdout.contains("Dry run complete"), "stdout: {stdout}");
    assert!(stdout.contains("~/.config/fish"), "stdout: {stdout}");
    assert!(stdout.contains("create"), "stdout: {stdout}");

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
fn deploy_rejects_only_and_except_together() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    write_deploy_config(repo.path(), "true");

    let output = run_dotman(
        repo.path(),
        home.path(),
        &["deploy", "--only", "link", "--except", "shell"],
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--only and --except cannot be used together"),
        "stderr: {stderr}"
    );
}

#[test]
fn link_sources_are_resolved_from_config_file_directory() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let profile = repo.path().join("profiles/work");
    std::fs::create_dir_all(profile.join("config/fish")).expect("profile config");
    std::fs::write(
        profile.join("config/fish/config.fish"),
        "set fish_greeting profile\n",
    )
    .expect("profile fish");
    std::fs::write(
        profile.join("dotman.yaml"),
        r#"
- defaults:
    link:
      create: true
      relative: true

- link:
    ~/.config/fish: config/fish
"#,
    )
    .expect("profile dotman yaml");

    let output = run_dotman(
        repo.path(),
        home.path(),
        &["deploy", "--config", "profiles/work/dotman.yaml"],
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let target = home.path().join(".config/fish");
    let link = std::fs::read_link(&target).expect("fish link");
    let actual = std::fs::canonicalize(target.parent().unwrap().join(link)).expect("actual source");
    let expected = std::fs::canonicalize(profile.join("config/fish")).expect("expected source");
    assert_eq!(actual, expected);
}

#[test]
fn deploy_falls_back_to_installed_bundle_when_default_config_is_missing() {
    let cwd = tempfile::tempdir().expect("cwd");
    let home = tempfile::tempdir().expect("home");
    let bundle = home.path().join(".local/share/tabsp-dotfiles");
    std::fs::create_dir_all(bundle.join("config/fish")).expect("bundle config");
    std::fs::write(
        bundle.join("config/fish/config.fish"),
        "set fish_greeting bundle\n",
    )
    .expect("bundle fish");
    std::fs::write(
        bundle.join("dotman.yaml"),
        r#"
- defaults:
    link:
      create: true
      relative: true

- link:
    ~/.config/fish: config/fish
"#,
    )
    .expect("bundle dotman yaml");

    let output = run_dotman_from(cwd.path(), home.path(), &["deploy"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let target = home.path().join(".config/fish");
    let link = std::fs::read_link(&target).expect("fish link");
    assert!(!link.is_absolute(), "expected relative link, got {link:?}");
    let resolved = std::fs::canonicalize(target).expect("resolved link");
    assert_eq!(
        resolved,
        std::fs::canonicalize(bundle.join("config/fish")).expect("bundle source")
    );
}

#[test]
fn deploy_uses_dotfiles_dir_env_before_default_installed_bundle() {
    let cwd = tempfile::tempdir().expect("cwd");
    let home = tempfile::tempdir().expect("home");
    let custom_bundle = tempfile::tempdir().expect("custom bundle");
    std::fs::create_dir_all(custom_bundle.path().join("config/fish")).expect("custom config");
    std::fs::write(
        custom_bundle.path().join("config/fish/config.fish"),
        "set fish_greeting custom\n",
    )
    .expect("custom fish");
    std::fs::write(
        custom_bundle.path().join("dotman.yaml"),
        r#"
- defaults:
    link:
      create: true
      relative: true

- link:
    ~/.config/fish: config/fish
"#,
    )
    .expect("custom dotman yaml");

    let default_bundle = home.path().join(".local/share/tabsp-dotfiles");
    std::fs::create_dir_all(&default_bundle).expect("default bundle");
    std::fs::write(default_bundle.join("dotman.yaml"), "[]\n").expect("default yaml");

    let output = run_dotman_from_with_env(
        cwd.path(),
        home.path(),
        &[("DOTFILES_DIR", custom_bundle.path())],
        &["deploy"],
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let target = home.path().join(".config/fish");
    let resolved = std::fs::canonicalize(target).expect("resolved link");
    assert_eq!(
        resolved,
        std::fs::canonicalize(custom_bundle.path().join("config/fish")).expect("custom source")
    );
}

#[test]
fn link_backup_true_preserves_existing_file_before_linking() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(repo.path().join("config/fish")).expect("source dir");
    std::fs::write(repo.path().join("config/fish/config.fish"), "new\n").expect("source file");
    std::fs::create_dir_all(home.path().join(".config")).expect("target parent");
    std::fs::write(home.path().join(".config/fish"), "old\n").expect("existing file");
    std::fs::write(
        repo.path().join("dotman.yaml"),
        r#"
- defaults:
    link:
      backup: true

- link:
    ~/.config/fish: config/fish
"#,
    )
    .expect("dotman yaml");

    let output = run_dotman(repo.path(), home.path(), &["deploy"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(home.path().join(".config/fish").is_symlink());
    let backups = std::fs::read_dir(home.path().join(".config"))
        .expect("backup parent")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("fish.backup.")
        })
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1, "expected one backup, got {backups:?}");
    assert_eq!(
        std::fs::read_to_string(backups[0].path()).expect("backup content"),
        "old\n"
    );
}

#[test]
fn link_backup_true_preserves_existing_directory_before_linking() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(repo.path().join("config/fish")).expect("source dir");
    std::fs::write(repo.path().join("config/fish/config.fish"), "new\n").expect("source file");
    std::fs::create_dir_all(home.path().join(".config/fish")).expect("existing dir");
    std::fs::write(home.path().join(".config/fish/local.fish"), "old\n").expect("existing file");
    std::fs::write(
        repo.path().join("dotman.yaml"),
        r#"
- defaults:
    link:
      backup: true

- link:
    ~/.config/fish: config/fish
"#,
    )
    .expect("dotman yaml");

    let output = run_dotman(repo.path(), home.path(), &["deploy"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(home.path().join(".config/fish").is_symlink());
    let backups = std::fs::read_dir(home.path().join(".config"))
        .expect("backup parent")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("fish.backup.")
        })
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1, "expected one backup, got {backups:?}");
    assert_eq!(
        std::fs::read_to_string(backups[0].path().join("local.fish")).expect("backup content"),
        "old\n"
    );
}

#[test]
fn link_relink_true_replaces_wrong_symlink() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(repo.path().join("config/fish")).expect("source dir");
    std::fs::create_dir_all(repo.path().join("old/fish")).expect("old source dir");
    std::fs::create_dir_all(home.path().join(".config")).expect("target parent");
    std::os::unix::fs::symlink(
        repo.path().join("old/fish"),
        home.path().join(".config/fish"),
    )
    .expect("old symlink");
    std::fs::write(
        repo.path().join("dotman.yaml"),
        r#"
- defaults:
    link:
      relink: true

- link:
    ~/.config/fish: config/fish
"#,
    )
    .expect("dotman yaml");

    let output = run_dotman(repo.path(), home.path(), &["deploy"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let link = std::fs::read_link(home.path().join(".config/fish")).expect("fish link");
    assert_eq!(
        std::fs::canonicalize(link).expect("actual source"),
        std::fs::canonicalize(repo.path().join("config/fish")).expect("expected source")
    );
}

#[test]
fn missing_link_source_fails_without_creating_target() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    std::fs::write(
        repo.path().join("dotman.yaml"),
        r#"
- defaults:
    link:
      create: true

- link:
    ~/.config/fish: config/missing-fish
"#,
    )
    .expect("dotman yaml");

    let output = run_dotman(repo.path(), home.path(), &["deploy"]);
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("source does not exist"), "stdout: {stdout}");
    assert!(stderr.contains("source does not exist"), "stderr: {stderr}");
    assert!(!home.path().join(".config/fish").exists());
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

#[test]
fn shell_failure_stops_after_completed_links() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(repo.path().join("config/fish")).expect("source dir");
    std::fs::write(
        repo.path().join("dotman.yaml"),
        r#"
- defaults:
    link:
      create: true

- link:
    ~/.config/fish: config/fish

- shell:
    - command: "false"
      description: Required failure

- create:
    - ~/.config/after-failure
"#,
    )
    .expect("dotman yaml");

    let output = run_dotman(repo.path(), home.path(), &["deploy"]);
    assert!(!output.status.success());
    assert!(home.path().join(".config/fish").is_symlink());
    assert!(!home.path().join(".config/after-failure").exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Required failure"), "stdout: {stdout}");
    assert!(stdout.contains("Failed"), "stdout: {stdout}");
    assert!(stdout.contains("1 linked"), "stdout: {stdout}");
    assert!(stderr.contains("shell command failed"), "stderr: {stderr}");
}

#[test]
fn shell_defaults_control_command_output_and_items_can_override() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    std::fs::write(
        repo.path().join("dotman.yaml"),
        r#"
- defaults:
    shell:
      stdout: true
      stderr: true

- shell:
    - command: "printf visible"
      description: Visible stdout
    - command: "printf hidden"
      description: Hidden stdout
      stdout: false
"#,
    )
    .expect("dotman yaml");

    let output = run_dotman(repo.path(), home.path(), &["deploy"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("visible"), "stdout: {stdout}");
    assert!(!stdout.contains("hidden"), "stdout: {stdout}");
}

#[test]
fn shell_condition_skips_command_when_false() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let marker = home.path().join("shell-marker");
    std::fs::write(
        repo.path().join("dotman.yaml"),
        format!(
            r#"
- shell:
    - command: "touch {}"
      description: Skipped shell marker
      if: "false"
"#,
            marker.to_string_lossy()
        ),
    )
    .expect("dotman yaml");

    let output = run_dotman(repo.path(), home.path(), &["deploy"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!marker.exists());
}

#[test]
fn optional_shell_failure_does_not_stop_following_commands() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let marker = home.path().join("shell-marker");
    std::fs::write(
        repo.path().join("dotman.yaml"),
        format!(
            r#"
- shell:
    - command: "false"
      description: Optional failure
      optional: true
    - command: "touch {}"
      description: Touch shell marker
"#,
            marker.to_string_lossy()
        ),
    )
    .expect("dotman yaml");

    let output = run_dotman(repo.path(), home.path(), &["deploy"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(marker.exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("optional command failed"),
        "stdout: {stdout}"
    );
}

#[test]
fn bootstrap_uses_bootstrap_config_by_default() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let marker = home.path().join("bootstrap-marker");
    std::fs::write(
        repo.path().join("dotman.bootstrap.yaml"),
        format!(
            r#"
- shell:
    - command: "touch {}"
      description: Touch bootstrap marker
"#,
            marker.to_string_lossy()
        ),
    )
    .expect("dotman bootstrap yaml");

    let output = run_dotman(repo.path(), home.path(), &["bootstrap"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(marker.exists());
}

#[test]
fn bootstrap_falls_back_to_installed_bundle_when_default_config_is_missing() {
    let cwd = tempfile::tempdir().expect("cwd");
    let home = tempfile::tempdir().expect("home");
    let bundle = home.path().join(".local/share/tabsp-dotfiles");
    std::fs::create_dir_all(&bundle).expect("bundle");
    let marker = home.path().join("bootstrap-marker");
    std::fs::write(
        bundle.join("dotman.bootstrap.yaml"),
        format!(
            r#"
- shell:
    - command: "touch {}"
      description: Touch bootstrap marker
"#,
            marker.to_string_lossy()
        ),
    )
    .expect("bundle bootstrap yaml");

    let output = run_dotman_from(cwd.path(), home.path(), &["bootstrap"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(marker.exists());
}

#[test]
fn bootstrap_can_be_written_idempotently_with_conditions() {
    let repo = tempfile::tempdir().expect("repo");
    let home = tempfile::tempdir().expect("home");
    let marker = home.path().join("bootstrap-marker");
    std::fs::write(
        repo.path().join("dotman.bootstrap.yaml"),
        format!(
            r#"
- shell:
    - command: "printf x >> {}"
      description: Write marker once
      if: "test ! -e {}"
"#,
            marker.to_string_lossy(),
            marker.to_string_lossy()
        ),
    )
    .expect("dotman bootstrap yaml");

    let first = run_dotman(repo.path(), home.path(), &["bootstrap"]);
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let second = run_dotman(repo.path(), home.path(), &["bootstrap"]);
    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    assert_eq!(std::fs::read_to_string(marker).expect("marker"), "x");
}
