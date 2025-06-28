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
}