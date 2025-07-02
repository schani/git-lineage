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
mod headless_backend;
mod line_mapping;
mod main_lib;
mod navigator;
mod screenshot;
mod test_config;
mod test_runner;
mod theme;
mod tree;
mod ui;

use app::App;
use async_task::{Task, TaskResult};
use cli::{Cli, Commands};
use error::Result;
use headless_backend::HeadlessBackend;
use test_runner::TestRunner;

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
        Commands::Test {
            script,
            config,
            settle_timeout,
            verbose,
            overwrite,
        } => {
            run_headless_test(&script, config.as_deref(), settle_timeout, verbose, overwrite).await
        }
    }
}

async fn run_headless_test(
    script_path: &str,
    config_path: Option<&str>,
    settle_timeout: u64,
    verbose: bool,
    overwrite: bool,
) -> Result<()> {
    // Set up logging if verbose or if environment variable is set
    if verbose && std::env::var("GIT_LINEAGE_LOG").is_err() {
        env_logger::Builder::new()
            .filter_level(log::LevelFilter::Debug)
            .init();
    }

    log::info!("🧪 Starting headless test run");
    log::info!("🧪 Script: {}", script_path);
    if let Some(config) = config_path {
        log::info!("🧪 Config: {}", config);
    }

    // Initialize Git repository
    let repo = git_utils::open_repository(".").map_err(|e| GitLineageError::from(e.to_string()))?;

    // Initialize application state
    let mut app = if let Some(config_path) = config_path {
        // Load from config file
        let config_content = std::fs::read_to_string(config_path)
            .map_err(|e| GitLineageError::from(format!("Failed to read config file: {}", e)))?;
        let test_config: test_config::TestConfig = serde_json::from_str(&config_content)
            .map_err(|e| GitLineageError::from(format!("Failed to parse config: {}", e)))?;
        App::from_test_config(&test_config, repo)
    } else {
        App::new(repo)
    };

    // Setup headless terminal (using reasonable defaults)
    let backend = HeadlessBackend::new(120, 40);
    let _terminal = Terminal::new(backend)?;

    // Setup async task channels
    let (task_sender, task_receiver) = mpsc::channel::<Task>(32);
    let (result_sender, result_receiver) = mpsc::channel::<TaskResult>(32);

    // Start background worker
    let repo_path = std::env::current_dir()?.to_string_lossy().to_string();
    let worker_handle = tokio::spawn(async_task::run_worker(
        task_receiver,
        result_sender,
        repo_path,
    ));

    // Load initial data (same as interactive mode)
    log::info!("📤 headless: Sending LoadFileTree task");
    if let Err(e) = task_sender.send(Task::LoadFileTree).await {
        log::error!("📤 headless: Failed to send LoadFileTree task: {}", e);
        app.ui.status_message = format!("Failed to load file tree: {}", e);
    } else {
        log::info!("📤 headless: LoadFileTree task sent successfully");
    }

    // Give some time for initial loading
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Load and run test script
    let mut test_runner = TestRunner::from_file(script_path)?;
    test_runner.max_settle_time = Duration::from_secs(settle_timeout);
    test_runner.overwrite_mode = overwrite;

    log::info!("🧪 Running test script with {} commands", test_runner.script.commands.len());

    let test_result = test_runner.run(&mut app, &task_sender, result_receiver).await?;

    // Clean up
    worker_handle.abort();

    // Print results
    test_result.print_summary();

    if test_result.success {
        log::info!("🧪 Test completed successfully");
        Ok(())
    } else {
        log::error!("🧪 Test failed");
        Err(GitLineageError::from("Test failed".to_string()).into())
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
    log::info!("📤 main: Sending LoadFileTree task");
    if let Err(e) = task_sender.send(Task::LoadFileTree).await {
        log::error!("📤 main: Failed to send LoadFileTree task: {}", e);
        app.ui.status_message = format!("Failed to load file tree: {}", e);
    } else {
        log::info!("📤 main: LoadFileTree task sent successfully");
        app.start_background_task();
    }

    // Event-driven main application loop
    #[derive(Debug)]
    enum AppState {
        Idle,                           // No background tasks - true idle state
        Processing(usize),              // N active background tasks
    }
    
    // Start in Processing state since we immediately have the LoadFileTree background task
    let mut app_state = AppState::Processing(1);
    
    // Initial draw to show the UI when app starts
    app.refresh_navigator_view_model();
    terminal.draw(|f| ui::draw(f, &mut app))?;
    
    loop {
        // Handle forced screen redraw
        if app.ui.force_redraw {
            terminal.clear()?;
            app.ui.force_redraw = false;
        }
        
        match app_state {
            AppState::Idle => {
                // TRUE IDLE STATE - Block indefinitely waiting for events
                let event = crossterm::event::read()?;
                match event::handle_event(event, &mut app, &task_sender) {
                    Ok(needs_render) => {
                        if needs_render {
                            app.refresh_navigator_view_model();
                            terminal.draw(|f| ui::draw(f, &mut app))?;
                        }
                    }
                    Err(e) => {
                        app.ui.status_message = format!("Error handling event: {}", e);
                        app.refresh_navigator_view_model();
                        terminal.draw(|f| ui::draw(f, &mut app))?;
                    }
                }
                
                // Check if background tasks were started
                if app.has_active_background_tasks() {
                    app_state = AppState::Processing(app.active_background_tasks);
                }
            }
            
            AppState::Processing(task_count) => {
                // ACTIVE STATE - Poll for events and check background tasks frequently
                if crossterm::event::poll(Duration::from_millis(50))? {
                    let event = crossterm::event::read()?;
                    match event::handle_event(event, &mut app, &task_sender) {
                        Ok(needs_render) => {
                            if needs_render {
                                app.refresh_navigator_view_model();
                                terminal.draw(|f| ui::draw(f, &mut app))?;
                            }
                        }
                        Err(e) => {
                            app.ui.status_message = format!("Error handling event: {}", e);
                            app.refresh_navigator_view_model();
                            terminal.draw(|f| ui::draw(f, &mut app))?;
                        }
                    }
                }
                
                // Check background tasks frequently when active
                let mut tasks_completed = 0;
                while let Ok(result) = result_receiver.try_recv() {
                    log::debug!("📨 main: Received async task result: {:?}", std::mem::discriminant(&result));
                    app.complete_background_task();
                    main_lib::handle_task_result(&mut app, result);
                    tasks_completed += 1;
                    // Render immediately when background task completes
                    app.refresh_navigator_view_model();
                    terminal.draw(|f| ui::draw(f, &mut app))?;
                }
                
                // Update state based on actual task count
                if app.active_background_tasks == 0 {
                    app_state = AppState::Idle;
                } else {
                    app_state = AppState::Processing(app.active_background_tasks);
                }
            }
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
