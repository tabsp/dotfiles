//! Integration tests for dotman.
//!
//! Tests the public API of key modules end-to-end.
//! Requires no external dependencies or network.

use std::path::PathBuf;

// ---- Profile roundtrip ----

#[test]
fn profile_config_roundtrip() {
    // Test that default config builds, serializes, and deserializes correctly without I/O.
    let mut profiles = std::collections::HashMap::new();
    profiles.insert(
        "test".to_string(),
        dotman::profile::Profile {
            repo: "https://github.com/test/dotfiles.git".to_string(),
            branch: "main".to_string(),
            path: "~/.local/share/dotman/repos/test".to_string(),
            config: "dotman.yaml".to_string(),
            auto_sync: true,
        },
    );
    let cfg = dotman::profile::ProfileConfig {
        default_profile: "test".to_string(),
        profiles,
    };

    // Round-trip through TOML serialization
    let raw = toml::to_string_pretty(&cfg).unwrap();
    let parsed: dotman::profile::ProfileConfig = toml::from_str(&raw).unwrap();
    assert_eq!(parsed.default_profile, "test");
    let p = parsed.profiles.get("test").unwrap();
    assert_eq!(p.repo, "https://github.com/test/dotfiles.git");
    assert_eq!(p.config, "dotman.yaml");
    assert!(p.auto_sync);
}

// ---- Bootstrap git detection ----

#[test]
fn bootstrap_git_installed_does_not_panic() {
    // This test only verifies the function compiles and returns without panicking.
    // It intentionally does not assert on the result — git may or may not be installed.
    let _ = dotman::bootstrap::git_installed();
}

#[test]
fn bootstrap_git_help_does_not_panic() {
    dotman::bootstrap::print_git_help();
}

// ---- Config parsing ----

#[test]
fn config_load_minimal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dotman.yaml");
    std::fs::write(
        &path,
        "install: [fish, tmux]\nlinks:\n  ~/.config/fish: config/fish\n",
    )
    .unwrap();
    let cfg = dotman::config::load(&path).unwrap();
    assert_eq!(cfg.install, vec!["fish", "tmux"]);
    assert_eq!(cfg.links.len(), 1);
    assert!(!cfg.auto_install_pkg_manager);
}

#[test]
fn config_load_with_pkg_mgr() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dotman.yaml");
    std::fs::write(
        &path,
        "package_managers:\n  macos: brew\ninstall: []\nauto_install_pkg_manager: true\n",
    )
    .unwrap();
    let cfg = dotman::config::load(&path).unwrap();
    assert!(cfg.auto_install_pkg_manager);
    assert_eq!(cfg.package_managers.macos.as_deref(), Some("brew"));
}

// ---- Tilde expansion ----

#[test]
fn profile_expand_tilde() {
    let home = std::env::var("HOME").unwrap();
    let expanded = dotman::profile::expand_tilde("~/test/path");
    assert_eq!(expanded, PathBuf::from(&home).join("test/path"));

    let absolute = dotman::profile::expand_tilde("/usr/local");
    assert_eq!(absolute, PathBuf::from("/usr/local"));
}

// ---- Package manager detection (compile-time) ----

#[test]
fn package_managers_detect_os() {
    let os = dotman::package_managers::detect_os();
    assert!(matches!(
        os,
        dotman::package_managers::Os::Mac | dotman::package_managers::Os::Linux
    ));
}

// ---- Doctor diagnostics ----

#[test]
fn doctor_returns_diagnostics() {
    let results = dotman::init::run_doctor();
    assert!(
        !results.is_empty(),
        "doctor should return at least one diagnostic"
    );
    assert!(results.iter().any(|d| d.message.contains("git")));
}

// ---- Model serialization ----

#[test]
fn plan_serialization_roundtrip() {
    use dotman::model::{Action, HostInfo, Mode, Plan, PlanItem};

    let plan = Plan {
        id: "test-id".into(),
        mode: Mode::Deploy,
        created_at: "2026-01-01T00:00:00Z".into(),
        config_path: PathBuf::from("/tmp/dotman.yaml"),
        config_hash: "abc123".into(),
        auto_install_pkg_manager: false,
        host: HostInfo {
            hostname: "testhost".into(),
            os: "Mac".into(),
            arch: "aarch64".into(),
            user: "testuser".into(),
            home: PathBuf::from("/tmp/home"),
        },
        items: vec![PlanItem {
            id: "item-1".into(),
            name: "fish".into(),
            layer: "shell".into(),
            actions: vec![Action::Install {
                spec: dotman::ops::install::resolve_install(
                    &dotman::ops::install::load_db().unwrap(),
                    "fish",
                    "brew",
                )
                .unwrap(),
            }],
            selected: true,
        }],
    };

    let json = serde_json::to_string_pretty(&plan).unwrap();
    let deserialized: dotman::model::Plan = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, plan.id);
    assert_eq!(deserialized.items.len(), 1);
    assert_eq!(deserialized.items[0].name, "fish");
}

// ---- Default profile config ----

#[test]
fn profile_default_config_contains_main() {
    let cfg = dotman::profile::default_config();
    assert_eq!(cfg.default_profile, "main");
    assert!(cfg.profiles.contains_key("main"));
    let p = cfg.profiles.get("main").unwrap();
    assert_eq!(p.repo, dotman::profile::DEFAULT_REPO);
}

// ---- Path utility ----

#[test]
fn path_expand_home() {
    use dotman::path::expand_home;
    let home = std::env::var("HOME").unwrap();
    assert_eq!(
        expand_home("~/test").unwrap(),
        PathBuf::from(&home).join("test")
    );
    assert_eq!(
        expand_home("/abs/path").unwrap(),
        PathBuf::from("/abs/path")
    );
}
