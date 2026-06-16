mod deploy;
mod path;

use clap::{Parser, Subcommand};
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(about = "Small dotfiles deployer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Deploy {
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_delimiter = ',')]
        only: Vec<deploy::Directive>,
        #[arg(long, value_delimiter = ',')]
        except: Vec<deploy::Directive>,
        #[arg(long, default_value = "dotman.yaml")]
        config: String,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Command::Deploy {
            dry_run,
            only,
            except,
            config,
        } => deploy::run_deploy(Path::new(&config), dry_run, &only, &except),
    }
}
