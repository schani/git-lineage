use crate::error::GitLineageError;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use tokio::sync::mpsc;

mod app;
mod async_task;
mod cli;
mod command;
mod config;
mod error;
mod event;
mod executor;
mod git_utils;
mod line_mapping;
mod main_lib;
mod screenshot;
mod test_config;
mod theme;
mod tree;
mod ui;

use app::App;
use async_task::{Task, TaskResult};
use cli::{Cli, Commands};
use error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger only if GIT_LINEAGE_LOG environment variable is set
    if let Ok(log_file) = std::env::var("GIT_LINEAGE_LOG") {
        env_logger::Builder::new()
            .target(env_logger::Target::Pipe(Box::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file)
                    .expect("Failed to open log file"),
            )))
            .filter_level(log::LevelFilter::Debug)
            .init();

        log::info!("Git Lineage starting up");
    }

    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => run_interactive().await,
        Commands::Screenshot {
            config,
            output,
            width,
            height,
        } => {
            screenshot::generate_screenshot(&config, output.as_deref(), width, height)?;
            Ok(())
        }
        Commands::Execute {
            config,
            command,
            output,
            screenshot,
            width,
            height,
        } => {
            main_lib::execute_command(
                &config,
                &command,
                output.as_deref(),
                screenshot,
                width,
                height,
            )?;
            Ok(())
        }
        Commands::SaveState { output } => {
            main_lib::save_current_state(output.as_deref()).await?;
            Ok(())
        }
    }
}

async fn run_interactive() -> Result<()> {
    // Initialize Git repository
    let repo = git_utils::open_repository(".").map_err(|e| GitLineageError::from(e.to_string()))?;

    // Initialize application state
    let mut app = App::new(repo);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup async task channels
    let (task_sender, task_receiver) = mpsc::channel::<Task>(32);
    let (result_sender, mut result_receiver) = mpsc::channel::<TaskResult>(32);

    // Start background worker
    let repo_path = std::env::current_dir()?.to_string_lossy().to_string();
    let worker_handle = tokio::spawn(async_task::run_worker(
        task_receiver,
        result_sender,
        repo_path,
    ));

    // Load initial data
    if let Err(e) = task_sender.send(Task::LoadFileTree).await {
        app.ui.status_message = format!("Failed to load file tree: {}", e);
    }

    // Main application loop
    let tick_rate = Duration::from_millis(250);
    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Handle events with timeout
        let timeout = tick_rate;
        if crossterm::event::poll(timeout)? {
            let event = crossterm::event::read()?;
            if let Err(e) = event::handle_event(event, &mut app, &task_sender) {
                app.ui.status_message = format!("Error handling event: {}", e);
            }
        }

        // Handle async task results
        while let Ok(result) = result_receiver.try_recv() {
            main_lib::handle_task_result(&mut app, result);
        }

        // Check if we should quit
        if app.should_quit {
            break;
        }
    }

    // Cleanup
    worker_handle.abort();

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
