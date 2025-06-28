use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{io, time::Duration};
use tokio::sync::mpsc;
use clap::Parser;

mod app;
mod ui;
mod event;
mod async_task;
mod git_utils;
mod error;
mod config;
mod cli;
mod test_config;
mod screenshot;
mod command;
mod executor;

use app::App;
use async_task::{Task, TaskResult};
use error::Result;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => run_interactive().await,
        Commands::Screenshot { config, output, width, height } => {
            screenshot::generate_screenshot(&config, output.as_deref(), width, height)?;
            Ok(())
        }
        Commands::Execute { config, command, output, screenshot, width, height } => {
            execute_command(&config, &command, output.as_deref(), screenshot, width, height)?;
            Ok(())
        }
    }
}

async fn run_interactive() -> Result<()> {
    // Initialize Git repository
    let repo = git_utils::open_repository(".")?;
    
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
        app.status_message = format!("Failed to load file tree: {}", e);
    }
    
    // Main application loop
    let tick_rate = Duration::from_millis(250);
    loop {
        // Draw UI
        terminal.draw(|f| ui::draw(f, &app))?;
        
        // Handle events with timeout
        let timeout = tick_rate;
        if crossterm::event::poll(timeout)? {
            let event = crossterm::event::read()?;
            if let Err(e) = event::handle_event(event, &mut app, &task_sender) {
                app.status_message = format!("Error handling event: {}", e);
            }
        }
        
        // Handle async task results
        while let Ok(result) = result_receiver.try_recv() {
            handle_task_result(&mut app, result);
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

fn handle_task_result(app: &mut App, result: TaskResult) {
    app.is_loading = false;
    
    match result {
        TaskResult::FileTreeLoaded { files } => {
            app.file_tree = files;
            app.status_message = "File tree loaded".to_string();
        }
        TaskResult::CommitHistoryLoaded { commits } => {
            app.commit_list = commits;
            app.status_message = "Commit history loaded".to_string();
        }
        TaskResult::FileContentLoaded { content, blame_info: _ } => {
            app.current_content = content;
            app.status_message = "File content loaded".to_string();
        }
        TaskResult::NextChangeFound { commit_hash } => {
            // Find the commit in the list and select it
            if let Some(index) = app.commit_list.iter().position(|c| c.hash == commit_hash) {
                app.commit_list_state.select(Some(index));
                app.active_panel = app::PanelFocus::History;
                app.status_message = "Found next change".to_string();
            } else {
                app.status_message = "Next change found but commit not in history".to_string();
            }
        }
        TaskResult::NextChangeNotFound => {
            app.status_message = "No subsequent changes found for this line".to_string();
        }
        TaskResult::Error { message } => {
            app.status_message = format!("Error: {}", message);
        }
    }
}

fn execute_command(
    config_path: &str,
    command_str: &str,
    output_path: Option<&str>,
    generate_screenshot: bool,
    width: u16,
    height: u16,
) -> Result<()> {
    use std::fs;
    
    // Load the configuration
    let config = test_config::TestConfig::load_from_file(config_path)?;
    
    // Parse the command
    let command = command::Command::from_string(command_str)
        .map_err(|e| error::GitLineageError::Generic(e))?;
    
    // Execute the command
    let result = executor::Executor::execute(&config, command);
    
    // Convert result to JSON
    let result_json = serde_json::to_string_pretty(&result.config)?;
    
    // Output the result
    match output_path {
        Some(path) => {
            fs::write(path, &result_json)?;
            println!("Result saved to: {}", path);
        }
        None => {
            println!("{}", result_json);
        }
    }
    
    // Show execution summary
    if let Some(status) = result.status_message {
        eprintln!("Status: {}", status);
    }
    if result.should_quit {
        eprintln!("Command resulted in quit");
    }
    
    // Generate screenshot if requested
    if generate_screenshot {
        let screenshot_path = output_path
            .map(|p| format!("{}.screenshot.txt", p.trim_end_matches(".json")))
            .unwrap_or_else(|| "command_result_screenshot.txt".to_string());
            
        // Save the result config temporarily for screenshot generation
        let temp_config_path = "temp_config.json";
        fs::write(temp_config_path, &result_json)?;
        
        screenshot::generate_screenshot(&temp_config_path, Some(&screenshot_path), width, height)?;
        
        // Clean up temp file
        let _ = fs::remove_file(temp_config_path);
        
        eprintln!("Screenshot saved to: {}", screenshot_path);
    }
    
    Ok(())
}
