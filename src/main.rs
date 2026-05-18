#![allow(dead_code)]
mod add;
mod agent;
mod archive;
mod check;
mod config;
mod deps;
mod doctor;
mod http;
mod installers;
mod link;
mod output;
mod path;
mod platform;
mod process;
mod recovery;
mod shell;
mod status;
mod update;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(about = "Internal dotfiles environment manager")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Agent {
        #[command(subcommand)]
        command: agent::AgentCommand,
    },
    Bootstrap {
        #[arg(long)]
        dry_run: bool,
    },
    Link {
        #[arg(long, default_value = "backup")]
        conflict: Conflict,
        #[arg(long)]
        dry_run: bool,
    },
    Doctor {
        #[arg(long)]
        json: bool,
    },
    Shell,
    Check,
    Cleanup {
        #[arg(long)]
        execute: bool,
    },
    Status {
        #[arg(long)]
        json: bool,
    },
    Update {
        #[arg(long)]
        check: bool,
    },
    Add {
        #[command(subcommand)]
        command: AddCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AddCommand {
    Dep {
        #[arg(long)]
        dry_run: bool,
    },
    Config {
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Conflict {
    Fail,
    Backup,
    Overwrite,
}

fn main() {
    if let Err(err) = run() {
        output::error(err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Command::Bootstrap { dry_run } => run_bootstrap(dry_run),
        Command::Link { conflict, dry_run } => run_link(conflict, dry_run),
        Command::Doctor { json } => run_doctor(json),
        Command::Shell => shell::run_shell(),
        Command::Check => run_check(),
        Command::Cleanup { execute } => recovery::run_cleanup(execute),
        Command::Status { json } => status::run_status(json),
        Command::Update { check } => run_update(check),
        Command::Agent { command } => agent::run_agent(command),
        Command::Add { command } => add::run_add(command),
    }
}

fn run_update(check: bool) -> Result<(), String> {
    if check {
        update::check_deps()
    } else {
        update::list_deps()
    }
}

fn run_check() -> Result<(), String> {
    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;
    match check::run_check(&deps, &files, &host, &repo) {
        Ok(()) => {
            output::progress("check");
            Ok(())
        }
        Err(errors) => Err(errors.join("\nerror: ")),
    }
}

fn run_link(conflict: Conflict, dry_run: bool) -> Result<(), String> {
    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;
    match check::run_check(&deps, &files, &host, &repo) {
        Ok(()) => {}
        Err(errors) => return Err(errors.join("\nerror: ")),
    }

    let conflict = match conflict {
        Conflict::Fail => link::Conflict::Fail,
        Conflict::Backup => link::Conflict::Backup,
        Conflict::Overwrite => link::Conflict::Overwrite,
    };

    link::run_link(&files, &host, &repo, conflict, dry_run)
}

fn run_doctor(json: bool) -> Result<(), String> {
    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;
    match check::run_check(&deps, &files, &host, &repo) {
        Ok(()) => doctor::run_doctor(&deps, &files, &host, &repo, json),
        Err(errors) => Err(errors.join("\nerror: ")),
    }
}

fn run_bootstrap(dry_run: bool) -> Result<(), String> {
    let repo =
        std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
    let deps_manifest = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;

    if dry_run {
        println!("==> bootstrap (dry-run)");
        println!("==> check");
    }
    match check::run_check(&deps_manifest, &files, &host, &repo) {
        Ok(()) => {}
        Err(errors) => return Err(errors.join("\nerror: ")),
    }

    if dry_run {
        println!("==> dependencies");
        for (name, dep) in &deps_manifest.deps {
            let entries: Vec<_> = dep
                .entries_for(host.platform.key(), host.arch.key())
                .into_iter()
                .filter(|entry| entry.matches_distro(&host))
                .collect();
            let Some(entry) = entries.first() else {
                continue;
            };
            match installers::is_installed(&dep.command, entry) {
                Ok(true) => println!("  already installed: {name}"),
                Ok(false) => println!("  would install: {name}"),
                Err(err) => println!("  error checking {name}: {err}"),
            }
        }
        link::run_link(&files, &host, &repo, link::Conflict::Backup, true)?;
        println!("==> dry-run complete (no changes made)");
        return Ok(());
    }

    deps::install_missing(&deps_manifest, &host)?;
    link::run_link(&files, &host, &repo, link::Conflict::Backup, false)?;
    doctor::run_doctor(&deps_manifest, &files, &host, &repo, false)?;
    print_post_bootstrap_hints();
    Ok(())
}

fn print_post_bootstrap_hints() {
    if is_ghostty_terminal(
        std::env::var("TERM_PROGRAM").ok().as_deref(),
        std::env::var("GHOSTTY_RESOURCES_DIR").ok().as_deref(),
    ) {
        return;
    }

    eprintln!("hint: open Ghostty to use the managed terminal config.");
    eprintln!("hint: run `make shell` if you also want fish as your login shell.");
}

fn is_ghostty_terminal(term_program: Option<&str>, ghostty_resources_dir: Option<&str>) -> bool {
    term_program.is_some_and(|value| value.eq_ignore_ascii_case("ghostty"))
        || ghostty_resources_dir.is_some_and(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ghostty_terminal_from_term_program_or_resources_dir() {
        assert!(is_ghostty_terminal(Some("Ghostty"), None));
        assert!(is_ghostty_terminal(Some("ghostty"), None));
        assert!(is_ghostty_terminal(
            None,
            Some("/Applications/Ghostty.app/resources")
        ));
        assert!(!is_ghostty_terminal(Some("Apple_Terminal"), None));
        assert!(!is_ghostty_terminal(None, Some("")));
        assert!(!is_ghostty_terminal(None, None));
    }
}
