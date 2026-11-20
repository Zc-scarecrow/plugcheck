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

        /// Local template directory (text files may use {{name}} placeholders)
        #[arg(long)]
        template: Option<String>,
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

    /// Pack a plugin into a distributable ZIP archive
    Pack {
        /// Path to the plugin directory (defaults to the current directory)
        path: Option<String>,

        /// Output archive path (defaults to <plugin-dir-name>.zip)
        #[arg(long, short)]
        output: Option<String>,
    },

    /// Re-check a plugin whenever its files change
    Watch {
        /// Path to the plugin directory (defaults to the current directory)
        path: Option<String>,

        /// Polling interval in seconds
        #[arg(long, default_value_t = 2)]
        interval: u64,
    },

    /// Report declared client targets and MCP transports
    Compat {
        /// Path to the plugin directory (defaults to the current directory)
        path: Option<String>,

        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Human,
    Json,
}
