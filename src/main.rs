mod deploy;
mod path;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(version)]
#[command(about = "Small dotfiles deployer")]
struct Cli {
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto, global = true)]
    color: ColorChoice,
    #[arg(long, value_enum, default_value_t = IconChoice::Nerd, global = true)]
    icons: IconChoice,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum IconChoice {
    Nerd,
    Unicode,
    Ascii,
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
    Bootstrap {
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_delimiter = ',')]
        only: Vec<deploy::Directive>,
        #[arg(long, value_delimiter = ',')]
        except: Vec<deploy::Directive>,
        #[arg(long, default_value = "dotman.bootstrap.yaml")]
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
    let style = deploy::OutputStyle::new(cli.color, cli.icons);
    match cli.command {
        Command::Deploy {
            dry_run,
            only,
            except,
            config,
        } => deploy::run_deploy("deploy", Path::new(&config), dry_run, &only, &except, style),
        Command::Bootstrap {
            dry_run,
            only,
            except,
            config,
        } => deploy::run_deploy(
            "bootstrap",
            Path::new(&config),
            dry_run,
            &only,
            &except,
            style,
        ),
    }
}
