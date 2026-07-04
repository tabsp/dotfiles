#![allow(dead_code)] // Phase 0: many placeholder functions not yet wired in

use clap::Parser;

mod cli;
mod config;
mod execute;
mod headless;
mod icons;
mod model;
mod ops;
mod package_managers;
mod path;
mod plan;
mod store;
mod theme;
mod tui;

use cli::{Cli, Command};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    init_tracing();

    let cli = Cli::parse();

    match cli.command {
        None => run_tui_or_headless(&cli, Mode::Menu),
        Some(Command::Deploy) => run_tui_or_headless(&cli, Mode::Deploy),
        Some(Command::Bootstrap) => run_tui_or_headless(&cli, Mode::Bootstrap),
        Some(Command::Plan) => run_tui_or_headless(&cli, Mode::Plan),
        Some(Command::History) => run_tui_or_headless(&cli, Mode::History),
        Some(Command::Run { ref id }) => run_tui_or_headless(&cli, Mode::Run(id.clone())),
        Some(Command::NewLink { target, source }) => add_link(&target, &source),
    }
}

#[derive(Debug, Clone)]
enum Mode {
    Menu,
    Deploy,
    Bootstrap,
    Plan,
    History,
    Run(String),
}

fn run_tui_or_headless(cli: &Cli, mode: Mode) -> Result<(), String> {
    if cli.auto {
        headless::run(mode)
    } else if is_tty() {
        tui::run(mode)
    } else {
        eprintln!("warning: not a TTY, falling back to headless mode");
        headless::run(mode)
    }
}

fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("dotman=info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

fn add_link(target: &str, source: &str) -> Result<(), String> {
    let path = std::path::Path::new("dotman.yaml");
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let updated = insert_link_entry(&raw, target, source)?;
    std::fs::write(path, updated)
        .map_err(|e| format!("failed to write {}: {e}", path.display()))?;
    println!("added link: {target} -> {source}");
    Ok(())
}

fn insert_link_entry(raw: &str, target: &str, source: &str) -> Result<String, String> {
    let mut lines: Vec<String> = raw.lines().map(str::to_string).collect();
    let Some(links_start) = lines.iter().position(|line| line.trim() == "links:") else {
        return Err("dotman.yaml has no links: section".into());
    };

    let entry = format!("  {target}: {source}");
    let target_prefix = format!("{target}:");
    let existing = lines[links_start + 1..]
        .iter()
        .position(|line| !line.starts_with(' ') || line.trim_start().starts_with(&target_prefix));

    if let Some(offset) = existing {
        let idx = links_start + 1 + offset;
        if !lines[idx].starts_with(' ') {
            lines.insert(idx, entry);
        } else {
            lines[idx] = entry;
        }
    } else {
        lines.push(entry);
    }

    let mut updated = lines.join("\n");
    updated.push('\n');
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_link_entry_adds_to_links_section() {
        let raw = "install: []\nlinks:\n  ~/.config/fish: config/fish\ncreate: []\n";
        let updated = insert_link_entry(raw, "~/.tmux.conf", "config/tmux.conf").unwrap();
        assert!(updated.contains("  ~/.tmux.conf: config/tmux.conf\ncreate: []"));
    }

    #[test]
    fn insert_link_entry_replaces_existing_target() {
        let raw = "links:\n  ~/.tmux.conf: old\n";
        let updated = insert_link_entry(raw, "~/.tmux.conf", "config/tmux.conf").unwrap();
        assert!(updated.contains("  ~/.tmux.conf: config/tmux.conf\n"));
        assert!(!updated.contains("old"));
    }
}
