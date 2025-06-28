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

mod app;
mod ui;
mod event;
mod async_task;
mod git_utils;
mod error;
mod config;

use app::App;
use async_task::{Task, TaskResult};
use error::Result;

#[tokio::main]
async fn main() -> Result<()> {
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
