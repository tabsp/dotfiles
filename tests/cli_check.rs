mod common;

use common::{
    current_host_params_table, current_host_table, non_current_host_table, write_minimal_dotfiles,
};
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
        .stderr(predicate::str::contains(
            "target must be absolute or ~-based",
        ));
}

#[test]
fn check_validates_download_binary_required_params() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("config")).expect("config");
    std::fs::write(temp.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.nv]
command = "nvim"
{}
installer = "download_binary"
version = "0.10.4"
source = "https://example.invalid/nvim"

[deps.nv.version_check]
regex = 'v?([0-9]+\.[0-9]+\.[0-9]+)'
"#,
            current_host_table("nv")
        ),
    )
    .expect("deps");
    std::fs::write(
        temp.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#,
    )
    .expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required param url"))
        .stderr(predicate::str::contains("missing required param sha256"))
        .stderr(predicate::str::contains(
            "missing required param archive_kind",
        ))
        .stderr(predicate::str::contains(
            "missing required param binary_path",
        ))
        .stderr(predicate::str::contains(
            "missing required param install_to",
        ));
}

#[test]
fn check_validates_official_script_required_params() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("config")).expect("config");
    std::fs::write(temp.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.starship]
command = "starship"
{}
installer = "official_script"
version = "latest"
source = "https://example.invalid/starship"
"#,
            current_host_table("starship")
        ),
    )
    .expect("deps");
    std::fs::write(
        temp.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#,
    )
    .expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "missing required param script_url",
        ));
}

#[test]
fn check_validates_official_script_url_and_args_type() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("config")).expect("config");
    std::fs::write(temp.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.starship]
command = "starship"
{}
installer = "official_script"
version = "latest"
source = "https://example.invalid/starship"
{}
script_url = "http://example.invalid/install.sh"
args = ["--yes", 1]
"#,
            current_host_table("starship"),
            current_host_params_table("starship")
        ),
    )
    .expect("deps");
    std::fs::write(
        temp.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#,
    )
    .expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "param script_url must use https://",
        ))
        .stderr(predicate::str::contains("param args must be string array"));
}

#[test]
fn check_validates_repo_package_required_params() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("config")).expect("config");
    std::fs::write(temp.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.ghostty]
command = "ghostty"
{}
installer = "repo_package"
version = "1.0.0"
source = "https://example.invalid/ghostty"

[deps.ghostty.version_check]
regex = '([0-9]+\.[0-9]+\.[0-9]+)'
"#,
            current_host_table("ghostty")
        ),
    )
    .expect("deps");
    std::fs::write(
        temp.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#,
    )
    .expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required param package"))
        .stderr(predicate::str::contains("missing required param repo_url"))
        .stderr(predicate::str::contains(
            "missing required param repo_key_url",
        ))
        .stderr(predicate::str::contains(
            "missing required param repo_channel",
        ))
        .stderr(predicate::str::contains(
            "missing required param repo_components",
        ));
}

#[test]
fn check_validates_repo_package_param_types_and_https() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("config")).expect("config");
    std::fs::write(temp.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.ghostty]
command = "ghostty"
{}
installer = "repo_package"
version = "1.0.0"
source = "https://example.invalid/ghostty"

[deps.ghostty.version_check]
regex = '([0-9]+\.[0-9]+\.[0-9]+)'
{}
package = "ghostty"
repo_url = "http://example.invalid/repo"
repo_key_url = "http://example.invalid/key.asc"
repo_channel = "stable"
repo_components = []
"#,
            current_host_table("ghostty"),
            current_host_params_table("ghostty")
        ),
    )
    .expect("deps");
    std::fs::write(
        temp.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#,
    )
    .expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("param repo_url must use https://"))
        .stderr(predicate::str::contains(
            "param repo_key_url must use https://",
        ))
        .stderr(predicate::str::contains(
            "param repo_components must be non-empty string array",
        ));
}

#[test]
fn check_validates_install_to_path_constraints() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("config")).expect("config");
    std::fs::write(temp.path().join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.nv]
command = "nvim"
{}
installer = "download_binary"
version = "0.10.4"
source = "https://example.invalid/nvim"
{}
url = "https://example.invalid/nvim.tar.gz"
sha256 = "deadbeef"
archive_kind = "tar.gz"
binary_path = "nvim/bin/nvim"
install_to = "relative/path"
"#,
            current_host_table("nv"),
            current_host_params_table("nv")
        ),
    )
    .expect("deps");
    std::fs::write(
        temp.path().join("dotfiles.toml"),
        r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#,
    )
    .expect("dotfiles");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "install_to must be absolute or ~-based",
        ));
}

#[test]
fn check_rejects_distros_on_mac_entry() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    let mac_table_with_distros = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "[deps.fish.mac.x86_64]"
    } else {
        "[deps.fish.mac.arm64]"
    };
    let mac_params_with_distros = if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "[deps.fish.mac.x86_64.params]"
    } else {
        "[deps.fish.mac.arm64.params]"
    };
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.fish]
command = "fish"

{}
installer = "brew"
version = "latest"
distros = ["ubuntu"]

{}
package = "fish"

{}
installer = "system"
version = "latest"
"#,
            mac_table_with_distros,
            mac_params_with_distros,
            current_host_table("fish")
        ),
    )
    .expect("deps");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "distros is only valid on linux entries",
        ));
}

#[test]
fn check_validates_ppa_required_params() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.fish]
command = "fish"

{}
installer = "ppa"
version = "latest"
"#,
            current_host_table("fish")
        ),
    )
    .expect("deps");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains("missing required param ppa"))
        .stderr(predicate::str::contains("missing required param package"));
}

#[test]
fn check_validates_ppa_bootstrap_package_type() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_minimal_dotfiles(temp.path());
    std::fs::write(
        temp.path().join("deps.toml"),
        format!(
            r#"
[deps.fish]
command = "fish"

{}
installer = "ppa"
version = "latest"

{}
ppa = "ppa:fish-shell/release-4"
package = "fish"
bootstrap_package = 1
"#,
            current_host_table("fish"),
            current_host_params_table("fish")
        ),
    )
    .expect("deps");

    let mut cmd = assert_cmd::Command::cargo_bin("dotman").expect("dotman binary");
    cmd.current_dir(temp.path())
        .arg("check")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "param bootstrap_package must be string",
        ));
}
