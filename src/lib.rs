//! dotman — Dotfiles deployment manager.
//!
//! This library crate exposes the public API for integration testing.
//! The binary crate (main.rs) is a thin wrapper around it.

pub mod bootstrap;
pub mod cli;
pub mod config;
pub mod execute;
pub mod headless;
pub mod icons;
pub mod init;
pub mod model;
pub mod ops;
pub mod package_managers;
pub mod path;
pub mod plan;
pub mod profile;
pub mod self_update;
pub mod store;
pub mod theme;
pub mod tui;

pub use cli::Mode;
