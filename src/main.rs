mod check;
mod config;
mod output;
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
        Command::Link { conflict, dry_run } => {
            output::progress(format!("link conflict={conflict:?} dry_run={dry_run}"));
            Ok(())
        }
        Command::Doctor => {
            output::progress("doctor");
            Ok(())
        }
        Command::Check => run_check(),
    }
}

fn run_check() -> Result<(), String> {
    let deps = config::load_deps(Path::new("deps.toml"))?;
    let files = config::load_dotfiles(Path::new("dotfiles.toml"))?;
    let host = platform::detect_host()?;
    match check::run_check(&deps, &files, &host, Path::new(".")) {
        Ok(()) => {
            output::progress("check");
            Ok(())
        }
        Err(errors) => Err(errors.join("\nerror: ")),
    }
}
