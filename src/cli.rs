use clap::{Parser, Subcommand, ValueHint};
use std::path::PathBuf;

/// Operational mode for TUI or headless execution.
#[derive(Debug, Clone)]
pub enum Mode {
    Menu,
    Deploy,
    Plan,
    History,
    Run(String),
}

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(version)]
#[command(about = "Dotfiles deployment manager")]
pub struct Cli {
    /// Run in headless mode: no prompts, safe defaults, fail on ambiguity.
    /// Suitable for scripts, CI, and automated bootstrap.
    #[arg(long, global = true)]
    pub headless: bool,

    /// Allow auto-installing prerequisites (git, etc.) even in headless mode
    #[arg(long, global = true)]
    pub bootstrap_git: bool,

    /// Path to dotman.yaml to use directly (bypasses profile resolution)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Skip auto-initialization; fail if no config found
    #[arg(long, global = true)]
    pub no_init: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialize dotman with a dotfiles repository
    ///
    /// Clones the repo, writes profile config to ~/.config/dotman/config.toml,
    /// and prepares for deployment. Default repo: https://github.com/tabsp/dotfiles.git
    Init {
        /// Repository URL (default: https://github.com/tabsp/dotfiles.git)
        repo: Option<String>,

        /// Git branch (default: main)
        #[arg(long)]
        branch: Option<String>,

        /// Profile name (default: main)
        #[arg(long)]
        profile: Option<String>,

        /// Repository checkout path
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Sync the active profile's repository (git pull)
    Sync,

    /// Show the deployment plan (syncs repo first)
    Plan,

    /// Deploy dotfiles (sync + plan + execute)
    Deploy,

    /// Show current profile status and repo state
    Status,

    /// Check system readiness (git, config, etc.)
    Doctor,

    /// Manage the dotman executable
    Self_ {
        #[command(subcommand)]
        action: SelfAction,
    },

    /// Manage named profiles
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },

    /// Open TUI history view
    History,

    /// Replay a past run by id
    Run {
        /// ULID of the run to replay
        id: String,
    },

    /// Add a link entry to dotman.yaml (skill/agent use)
    NewLink {
        /// Target path (e.g. ~/.config/fish)
        target: String,

        /// Source path relative to repo (e.g. config/fish)
        #[arg(value_hint = ValueHint::FilePath)]
        source: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum SelfAction {
    /// Update dotman to the latest GitHub Release
    Update,
}

#[derive(Debug, Subcommand)]
pub enum ProfileAction {
    /// List all configured profiles
    List,
    /// Add a new profile
    Add {
        /// Profile name
        name: String,
        /// Repository URL
        repo: String,
    },
    /// Remove a profile
    Remove {
        /// Profile name to remove
        name: String,
    },
}
