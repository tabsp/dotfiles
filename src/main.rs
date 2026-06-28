mod deploy;
mod path;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(version)]
#[command(about = "Small dotfiles deployer")]
struct Cli {
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto, global = true)]
    color: ColorChoice,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
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
        #[arg(long)]
        config: Option<String>,
        #[arg(long)]
        summary: bool,
    },
    Bootstrap {
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_delimiter = ',')]
        only: Vec<deploy::Directive>,
        #[arg(long, value_delimiter = ',')]
        except: Vec<deploy::Directive>,
        #[arg(long)]
        config: Option<String>,
        #[arg(long)]
        summary: bool,
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
            summary,
        } => {
            let config = resolve_config_path(config, "dotman.yaml")?;
            deploy::run_deploy(
                "deploy", &config, dry_run, &only, &except, cli.color, summary,
            )
        }
        Command::Bootstrap {
            dry_run,
            only,
            except,
            config,
            summary,
        } => {
            let config = resolve_config_path(config, "dotman.bootstrap.yaml")?;
            deploy::run_deploy(
                "bootstrap",
                &config,
                dry_run,
                &only,
                &except,
                cli.color,
                summary,
            )
        }
    }
}

fn resolve_config_path(config: Option<String>, default_file: &str) -> Result<PathBuf, String> {
    if let Some(config) = config {
        return Ok(PathBuf::from(config));
    }

    let local_config = Path::new(default_file);
    if local_config.exists() {
        return Ok(local_config.to_path_buf());
    }

    if let Some(dotfiles_dir) = std::env::var_os("DOTFILES_DIR") {
        return Ok(PathBuf::from(dotfiles_dir).join(default_file));
    }

    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home)
        .join(".local/share/tabsp-dotfiles")
        .join(default_file))
}
