mod check;
mod config;
mod link;
mod output;
mod path;
mod platform;

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
    Bootstrap,
    Link {
        #[arg(long, default_value = "backup")]
        conflict: Conflict,
        #[arg(long)]
        dry_run: bool,
    },
    Doctor,
    Check,
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
        Command::Bootstrap => {
            output::progress("bootstrap");
            Ok(())
        }
        Command::Link { conflict, dry_run } => run_link(conflict, dry_run),
        Command::Doctor => {
            output::progress("doctor");
            Ok(())
        }
        Command::Check => run_check(),
    }
}

fn run_check() -> Result<(), String> {
    let repo = std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
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
    let repo = std::env::current_dir().map_err(|err| format!("failed to read current dir: {err}"))?;
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
