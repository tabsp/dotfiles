#![allow(dead_code)]

pub fn current_host_table(dep: &str) -> String {
    let platform = if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    };
    format!("[deps.{dep}.{platform}.{arch}]")
}

pub fn current_host_params_table(dep: &str) -> String {
    let platform = if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    };
    format!("[deps.{dep}.{platform}.{arch}.params]")
}

pub fn non_current_host_table(dep: &str) -> String {
    let platform = if cfg!(target_os = "macos") {
        "mac"
    } else {
        "linux"
    };
    let arch = if cfg!(target_arch = "aarch64") {
        "x86_64"
    } else {
        "arm64"
    };
    format!("[deps.{dep}.{platform}.{arch}]")
}

pub fn minimal_dotfiles() -> &'static str {
    r#"
[[files]]
source = "config/tmux.conf"
target = "~/.tmux.conf"
kind = "file"
"#
}

pub fn write_minimal_dotfiles(repo: &std::path::Path) {
    std::fs::create_dir_all(repo.join("config")).expect("config");
    std::fs::write(repo.join("config/tmux.conf"), "set -g mouse on\n").expect("source");
    std::fs::write(repo.join("dotfiles.toml"), minimal_dotfiles()).expect("dotfiles");
}
