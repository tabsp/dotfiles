use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::*;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn current_target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        other => panic!("unsupported test host: {other:?}"),
    }
}

fn sha256(path: &std::path::Path) -> String {
    let output = if cfg!(target_os = "macos") {
        Command::new("shasum")
            .arg("-a")
            .arg("256")
            .arg(path)
            .output()
            .expect("shasum")
    } else {
        Command::new("sha256sum")
            .arg(path)
            .output()
            .expect("sha256sum")
    };
    assert!(output.status.success(), "checksum command failed");
    String::from_utf8(output.stdout)
        .expect("utf8")
        .split_whitespace()
        .next()
        .expect("digest")
        .to_string()
}

fn checksum_file(path: &std::path::Path, filename: &str) -> (String, String) {
    let digest = sha256(path);
    let content = format!("{digest}  {filename}\n");
    (digest, content)
}

#[test]
fn install_script_installs_dotfiles_source_and_prints_bootstrap_directory() {
    let temp = tempfile::tempdir().expect("tempdir");
    let releases = temp.path().join("releases/download/v0.1.0");
    std::fs::create_dir_all(&releases).expect("release dir");

    let payload = temp.path().join("payload");
    std::fs::create_dir_all(&payload).expect("payload dir");
    let dotman = payload.join("dotman");
    std::fs::write(&dotman, "#!/bin/sh\necho fake dotman\n").expect("fake dotman");
    let mut perms = std::fs::metadata(&dotman).expect("metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&dotman, perms).expect("chmod");

    let target = current_target();
    let archive_name = format!("dotman-{target}-0.1.0.tar.gz");
    let archive_path = releases.join(&archive_name);
    Command::new("tar")
        .arg("-czf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&payload)
        .arg("dotman")
        .assert()
        .success();
    let (_digest, checksum) = checksum_file(&archive_path, &archive_name);
    std::fs::write(releases.join(format!("{archive_name}.sha256")), checksum).expect("checksum");

    let source = temp.path().join("source/dotfiles-0.1.0");
    std::fs::create_dir_all(source.join("config")).expect("source config");
    std::fs::write(source.join("deps.toml"), "[deps]\n").expect("deps");
    std::fs::write(source.join("dotfiles.toml"), "files = []\n").expect("dotfiles");
    std::fs::write(source.join("config/example"), "example\n").expect("config file");
    let source_archive = temp.path().join("dotfiles-0.1.0.tar.gz");
    Command::new("tar")
        .arg("-czf")
        .arg(&source_archive)
        .arg("-C")
        .arg(temp.path().join("source"))
        .arg("dotfiles-0.1.0")
        .assert()
        .success();
    let (_sdigest, schecksum) = checksum_file(&source_archive, "dotfiles-0.1.0.tar.gz");
    std::fs::write(releases.join("dotfiles-0.1.0.tar.gz.sha256"), schecksum)
        .expect("source checksum");

    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home");
    let expected_repo = home.join(".local/share/dotman/dotfiles");

    assert_cmd::Command::new("sh")
        .arg("scripts/install.sh")
        .env("HOME", &home)
        .env(
            "BASE_URL",
            format!("file://{}", temp.path().join("releases/download").display()),
        )
        .env(
            "DOTFILES_ARCHIVE_URL",
            format!("file://{}", source_archive.display()),
        )
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "cd {} && dotman bootstrap",
            expected_repo.display()
        )));

    assert!(home.join(".local/bin/dotman").exists());
    assert!(expected_repo.join("deps.toml").exists());
    assert!(expected_repo.join("dotfiles.toml").exists());
    assert!(expected_repo.join("config/example").exists());
}

#[test]
fn install_script_fails_on_missing_binary_checksum() {
    let temp = tempfile::tempdir().expect("tempdir");
    let releases = temp.path().join("releases/download/v0.1.0");
    std::fs::create_dir_all(&releases).expect("release dir");

    let payload = temp.path().join("payload");
    std::fs::create_dir_all(&payload).expect("payload dir");
    let dotman = payload.join("dotman");
    std::fs::write(&dotman, "#!/bin/sh\necho fake\n").expect("fake dotman");
    std::fs::set_permissions(&dotman, PermissionsExt::from_mode(0o755)).expect("chmod");

    let target = current_target();
    let archive_name = format!("dotman-{target}-0.1.0.tar.gz");
    let archive_path = releases.join(&archive_name);
    Command::new("tar")
        .arg("-czf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&payload)
        .arg("dotman")
        .assert()
        .success();
    // No checksum file written — installer should fail

    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home");

    assert_cmd::Command::new("sh")
        .arg("scripts/install.sh")
        .env("HOME", &home)
        .env(
            "BASE_URL",
            format!("file://{}", temp.path().join("releases/download").display()),
        )
        .assert()
        .failure();
}

#[test]
fn install_script_fails_on_binary_checksum_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let releases = temp.path().join("releases/download/v0.1.0");
    std::fs::create_dir_all(&releases).expect("release dir");

    let payload = temp.path().join("payload");
    std::fs::create_dir_all(&payload).expect("payload dir");
    let dotman = payload.join("dotman");
    std::fs::write(&dotman, "#!/bin/sh\necho fake\n").expect("fake dotman");
    std::fs::set_permissions(&dotman, PermissionsExt::from_mode(0o755)).expect("chmod");

    let target = current_target();
    let archive_name = format!("dotman-{target}-0.1.0.tar.gz");
    let archive_path = releases.join(&archive_name);
    Command::new("tar")
        .arg("-czf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&payload)
        .arg("dotman")
        .assert()
        .success();
    // Write a deliberately wrong checksum
    let wrong_checksum = format!(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  {archive_name}\n"
    );
    std::fs::write(
        releases.join(format!("{archive_name}.sha256")),
        wrong_checksum,
    )
    .expect("wrong checksum");

    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home");

    assert_cmd::Command::new("sh")
        .arg("scripts/install.sh")
        .env("HOME", &home)
        .env(
            "BASE_URL",
            format!("file://{}", temp.path().join("releases/download").display()),
        )
        .assert()
        .failure();
}

#[test]
fn install_script_fails_on_missing_dotfiles_source_checksum() {
    let temp = tempfile::tempdir().expect("tempdir");
    let releases = temp.path().join("releases/download/v0.1.0");
    std::fs::create_dir_all(&releases).expect("release dir");

    // Binary setup
    let payload = temp.path().join("payload");
    std::fs::create_dir_all(&payload).expect("payload dir");
    let dotman = payload.join("dotman");
    std::fs::write(&dotman, "#!/bin/sh\necho fake\n").expect("fake dotman");
    std::fs::set_permissions(&dotman, PermissionsExt::from_mode(0o755)).expect("chmod");

    let target = current_target();
    let archive_name = format!("dotman-{target}-0.1.0.tar.gz");
    let archive_path = releases.join(&archive_name);
    Command::new("tar")
        .arg("-czf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&payload)
        .arg("dotman")
        .assert()
        .success();
    let (_digest, checksum) = checksum_file(&archive_path, &archive_name);
    std::fs::write(releases.join(format!("{archive_name}.sha256")), checksum).expect("checksum");

    // Dotfiles source setup (archive exists but NO checksum file)
    let source = temp.path().join("source/dotfiles-0.1.0");
    std::fs::create_dir_all(source.join("config")).expect("source config");
    std::fs::write(source.join("deps.toml"), "[deps]\n").expect("deps");
    let source_archive = temp.path().join("dotfiles-0.1.0.tar.gz");
    Command::new("tar")
        .arg("-czf")
        .arg(&source_archive)
        .arg("-C")
        .arg(temp.path().join("source"))
        .arg("dotfiles-0.1.0")
        .assert()
        .success();
    // No source checksum written — should fail

    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home");

    assert_cmd::Command::new("sh")
        .arg("scripts/install.sh")
        .env("HOME", &home)
        .env(
            "BASE_URL",
            format!("file://{}", temp.path().join("releases/download").display()),
        )
        .env(
            "DOTFILES_ARCHIVE_URL",
            format!("file://{}", source_archive.display()),
        )
        .assert()
        .failure();
}

#[test]
fn install_script_fails_on_dotfiles_source_checksum_mismatch() {
    let temp = tempfile::tempdir().expect("tempdir");
    let releases = temp.path().join("releases/download/v0.1.0");
    std::fs::create_dir_all(&releases).expect("release dir");

    // Binary setup
    let payload = temp.path().join("payload");
    std::fs::create_dir_all(&payload).expect("payload dir");
    let dotman = payload.join("dotman");
    std::fs::write(&dotman, "#!/bin/sh\necho fake\n").expect("fake dotman");
    std::fs::set_permissions(&dotman, PermissionsExt::from_mode(0o755)).expect("chmod");

    let target = current_target();
    let archive_name = format!("dotman-{target}-0.1.0.tar.gz");
    let archive_path = releases.join(&archive_name);
    Command::new("tar")
        .arg("-czf")
        .arg(&archive_path)
        .arg("-C")
        .arg(&payload)
        .arg("dotman")
        .assert()
        .success();
    let (_digest, checksum) = checksum_file(&archive_path, &archive_name);
    std::fs::write(releases.join(format!("{archive_name}.sha256")), checksum).expect("checksum");

    // Dotfiles source setup with WRONG checksum
    let source = temp.path().join("source/dotfiles-0.1.0");
    std::fs::create_dir_all(source.join("config")).expect("source config");
    std::fs::write(source.join("deps.toml"), "[deps]\n").expect("deps");
    let source_archive = temp.path().join("dotfiles-0.1.0.tar.gz");
    Command::new("tar")
        .arg("-czf")
        .arg(&source_archive)
        .arg("-C")
        .arg(temp.path().join("source"))
        .arg("dotfiles-0.1.0")
        .assert()
        .success();
    std::fs::write(
        releases.join("dotfiles-0.1.0.tar.gz.sha256"),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  dotfiles-0.1.0.tar.gz\n",
    )
    .expect("wrong source checksum");

    let home = temp.path().join("home");
    std::fs::create_dir_all(&home).expect("home");

    assert_cmd::Command::new("sh")
        .arg("scripts/install.sh")
        .env("HOME", &home)
        .env(
            "BASE_URL",
            format!("file://{}", temp.path().join("releases/download").display()),
        )
        .env(
            "DOTFILES_ARCHIVE_URL",
            format!("file://{}", source_archive.display()),
        )
        .assert()
        .failure();
}
