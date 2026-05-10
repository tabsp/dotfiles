mod output;

use clap::{Parser, Subcommand, ValueEnum};

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
    let cli = Cli::parse();

    let result: Result<(), String> = match cli.command {
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
        Command::Check => {
            output::progress("check");
            Ok(())
        }
    };

    if let Err(err) = result {
        output::error(err);
        std::process::exit(1);
    }
}
