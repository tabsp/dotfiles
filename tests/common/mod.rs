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
