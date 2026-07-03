use clap::{Parser, Subcommand, ValueHint};

#[derive(Debug, Parser)]
#[command(name = "dotman")]
#[command(version)]
#[command(about = "Cross-platform dev environment config assistant")]
pub struct Cli {
    /// Run in headless mode (no TUI), useful for scripts
    #[arg(long, global = true)]
    pub auto: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Open TUI and run deploy (default if no subcommand given)
    Deploy,

    /// Open TUI and run bootstrap
    Bootstrap,

    /// Open TUI and show plan only (no execution)
    Plan,

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
