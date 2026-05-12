use predicates::prelude::*;

#[test]
fn shell_refuses_non_interactive_shell_change() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fish = temp.path().join("fish");
    std::fs::write(&fish, "#!/bin/sh\nexit 0\n").expect("fish");
    make_executable(&fish);

    let path = format!(
        "{}:{}",
        temp.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.env("PATH", path)
        .env("SHELL", "/bin/sh")
        .arg("shell")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "refusing to change login shell without interactive confirmation",
        ))
        .stderr(predicate::str::contains("chsh -s"));
}

#[test]
fn shell_is_noop_when_login_shell_is_already_fish() {
    let temp = tempfile::tempdir().expect("tempdir");
    let fish = temp.path().join("fish");
    std::fs::write(&fish, "#!/bin/sh\nexit 0\n").expect("fish");
    make_executable(&fish);

    let path = format!(
        "{}:{}",
        temp.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.env("PATH", path)
        .env("SHELL", &fish)
        .arg("shell")
        .assert()
        .success()
        .stdout(predicate::str::contains("login shell already uses fish"));
}

fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = std::fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).expect("chmod");
}
