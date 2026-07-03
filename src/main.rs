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
        Some(Command::Deploy) | None => run_tui_or_headless(&cli, Mode::Deploy),
        Some(Command::Bootstrap) => run_tui_or_headless(&cli, Mode::Bootstrap),
        Some(Command::Plan) => run_tui_or_headless(&cli, Mode::Plan),
        Some(Command::History) => run_tui_or_headless(&cli, Mode::History),
        Some(Command::Run { ref id }) => run_tui_or_headless(&cli, Mode::Run(id.clone())),
        Some(Command::NewLink { target, source }) => add_link(&target, &source),
    }
}

#[derive(Debug, Clone)]
enum Mode {
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
    // Phase 8: real implementation
    eprintln!("would add link: {target} -> {source}");
    Ok(())
}
