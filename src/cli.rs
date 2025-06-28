use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "git-lineage")]
#[command(about = "A TUI for exploring Git file history with line-level time travel")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Run the interactive TUI (default)
    Run,
    /// Generate a screenshot from a JSON configuration
    Screenshot {
        /// Path to the JSON configuration file
        #[arg(short, long)]
        config: String,
        /// Output file for the screenshot (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
        /// Terminal width for rendering
        #[arg(long, default_value = "120")]
        width: u16,
        /// Terminal height for rendering
        #[arg(long, default_value = "40")]
        height: u16,
    },
    /// Execute a command against a configuration and output the result
    Execute {
        /// Path to the JSON configuration file
        #[arg(short, long)]
        config: String,
        /// Command to execute (e.g., "next_panel", "up", "search:a")
        #[arg(short = 'x', long)]
        command: String,
        /// Output file for the resulting configuration (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
        /// Also generate a screenshot of the result
        #[arg(long)]
        screenshot: bool,
        /// Terminal width for screenshot (if enabled)
        #[arg(long, default_value = "120")]
        width: u16,
        /// Terminal height for screenshot (if enabled)
        #[arg(long, default_value = "40")]
        height: u16,
    },
    /// Save current state to JSON configuration without running TUI
    SaveState {
        /// Output file for the configuration (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
}