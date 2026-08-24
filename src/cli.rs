use clap::{Parser, Subcommand, ValueEnum};

/// A fast validator and scaffolding CLI for the Agent Plugins 1.0 standard.
#[derive(Parser, Debug)]
#[command(name = "plugcheck", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Scaffold a new Agent Plugins 1.0 plugin
    New {
        /// Plugin name (lowercase letters, digits, hyphens, periods)
        name: String,

        /// Target directory (defaults to the plugin name)
        #[arg(long)]
        dir: Option<String>,
    },

    /// Validate a plugin against the Agent Plugins 1.0 specification
    Check {
        /// Path to the plugin directory (defaults to the current directory)
        path: Option<String>,

        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,

        /// Also report informational findings
        #[arg(long)]
        verbose: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Human,
    Json,
}
