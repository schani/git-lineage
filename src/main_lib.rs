// Library module containing testable functions from main.rs

use crate::app::App;
use crate::async_task::TaskResult;
use crate::error::Result;
use std::fs;

pub fn handle_task_result(app: &mut App, result: TaskResult) {
    log::debug!(
        "📥 handle_task_result: Processing result type: {:?}",
        std::mem::discriminant(&result)
    );
    app.ui.is_loading = false;
    
    // Decrement active background tasks counter for task types that increment it
    match &result {
        TaskResult::DiffGenerated { .. } => {
            app.active_background_tasks = app.active_background_tasks.saturating_sub(1);
        }
        _ => {}
    }

    match result {
        TaskResult::FileTreeLoaded { files } => {
            // Initialize the new navigator with the file tree
            app.navigator.handle_event(crate::navigator::NavigatorEvent::EndSearch).unwrap(); // Reset to browse mode
            app.navigator = crate::navigator::NavigatorState::new(files);
            app.navigator
                .handle_event(crate::navigator::NavigatorEvent::NavigateDown)
                .unwrap(); // Select first item

            app.ui.status_message = "File tree loaded".to_string();
        }
        TaskResult::CommitHistoryLoaded { file_path, commits } => {
            // Race condition protection: Only apply commits if they're for the currently active file
            let is_still_relevant = app
                .get_active_file()
                .as_ref()
                .map(|active_path| active_path.to_string_lossy() == file_path)
                .unwrap_or(false);

            if is_still_relevant {
                let commit_count = commits.len();
                app.history.commit_list = commits;
                // Reset commit list selection when new commits are loaded
                app.history.selected_commit_index = if commit_count == 0 { None } else { Some(0) };
                app.ui.status_message = if commit_count == 0 {
                    "No commits found for this file".to_string()
                } else {
                    format!("Loaded {} commits", commit_count)
                };

                // Auto-load content for the first (most recent) commit if available
                if !app.history.commit_list.is_empty() {
                    crate::event::update_code_inspector_for_commit_no_diff(app);
                }
            } else {
                // Async result is stale - ignore it
                app.ui.status_message = "Async result ignored (file context changed)".to_string();
            }
        }
        TaskResult::CommitHistoryChunkLoaded {
            file_path,
            commits,
            is_complete,
            chunk_offset,
        } => {
            // Race condition protection: Only apply commits if they're for the currently active file
            let is_still_relevant = app
                .get_active_file()
                .as_ref()
                .map(|active_path| active_path.to_string_lossy() == file_path)
                .unwrap_or(false);

            if is_still_relevant {
                if chunk_offset == 0 {
                    // First chunk - replace entire list and auto-load content
                    app.history.commit_list = commits;
                    app.history.next_chunk_offset = app.history.commit_list.len();

                    // Reset commit list selection when new commits are loaded
                    let commit_count = app.history.commit_list.len();
                    app.history.selected_commit_index = if commit_count == 0 { None } else { Some(0) };

                    // Auto-load content for the first (most recent) commit if available
                    if !app.history.commit_list.is_empty() {
                        crate::event::update_code_inspector_for_commit_no_diff(app);
                    }
                } else {
                    // Subsequent chunks - append to existing list
                    app.history.commit_list.extend(commits);
                    app.history.next_chunk_offset = app.history.commit_list.len();
                }

                app.history.history_complete = is_complete;
                app.history.is_loading_more = false;

                let commit_count = app.history.commit_list.len();
                app.ui.status_message = if commit_count == 0 {
                    "No commits found for this file".to_string()
                } else if is_complete {
                    format!("Loaded {} commits", commit_count)
                } else {
                    format!("Loaded {} commits (loading more...)", commit_count)
                };
            } else {
                // Async result is stale - ignore it
                app.ui.status_message = "Async result ignored (file context changed)".to_string();
            }
        }
        TaskResult::CommitFound {
            file_path,
            commit,
            total_commits_so_far,
        } => {
            // Race condition protection: Only apply commits if they're for the currently active file
            let is_still_relevant = app
                .get_active_file()
                .as_ref()
                .map(|active_path| active_path.to_string_lossy() == file_path)
                .unwrap_or(false);

            if is_still_relevant {
                // Add the new commit to the list
                app.history.commit_list.push(commit);

                // If this is the first commit, auto-select it and load content
                if total_commits_so_far == 1 {
                    app.history.selected_commit_index = Some(0);
                    crate::event::update_code_inspector_for_commit_no_diff(app);
                }

                // Update status message with current progress
                let filename = app
                    .get_active_file()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_default();
                app.ui.status_message =
                    format!("{} loaded ({} commits found...)", filename, total_commits_so_far);
            }
        }
        TaskResult::CommitHistoryComplete {
            file_path,
            total_commits,
        } => {
            // Race condition protection: Only apply if still relevant
            let is_still_relevant = app
                .get_active_file()
                .as_ref()
                .map(|active_path| active_path.to_string_lossy() == file_path)
                .unwrap_or(false);

            if is_still_relevant {
                app.history.history_complete = true;
                app.history.is_loading_more = false;

                let filename = app
                    .get_active_file()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_default();

                app.ui.status_message = if total_commits == 0 {
                    format!("{} loaded (no commit history)", filename)
                } else {
                    format!("{} loaded ({} commits)", filename, total_commits)
                };
            }
        }
        TaskResult::NextChangeFound { commit_hash } => {
            // Find the commit in the list and select it
            if let Some(index) = app
                .history
                .commit_list
                .iter()
                .position(|c| c.hash == commit_hash)
            {
                app.history.selected_commit_index = Some(index);
                app.ui.active_panel = crate::app::PanelFocus::History;
                app.ui.status_message = "Found next change".to_string();
            } else {
                app.ui.status_message = "Next change found but commit not in history".to_string();
            }
        }
        TaskResult::NextChangeNotFound => {
            app.ui.status_message = "No subsequent changes found for this line".to_string();
        }
        TaskResult::DiffGenerated {
            file_path,
            current_commit,
            parent_commit,
            diff_lines,
        } => {
            // Race condition protection: Only apply diff if it's for the currently active file and commit
            let is_still_relevant = app
                .get_active_file()
                .as_ref()
                .map(|active_path| active_path.to_string_lossy() == file_path)
                .unwrap_or(false)
                && app
                    .history
                    .selected_commit_hash
                    .as_ref()
                    .map(|hash| hash == &current_commit)
                    .unwrap_or(false);

            if is_still_relevant {
                app.inspector.diff_lines = Some(diff_lines);
                app.inspector.parent_commit_hash = Some(parent_commit);
                app.ui.status_message = "Diff view loaded".to_string();
            } else {
                // Async result is stale - ignore it
                app.ui.status_message = "Async diff result ignored (context changed)".to_string();
            }
        }
        TaskResult::Error { message } => {
            app.ui.status_message = format!("Error: {}", message);
        }
    }
}

pub fn execute_command(
    config_path: &str,
    command_str: &str,
    output_path: Option<&str>,
    generate_screenshot: bool,
    width: u16,
    height: u16,
) -> Result<()> {
    // Load the configuration
    let config = crate::test_config::TestConfig::load_from_file(config_path)?;

    // Parse the command
    let command = crate::command::Command::from_string(command_str)
        .map_err(|e| crate::error::GitLineageError::Generic(e))?;

    // Execute the command
    let result = crate::executor::Executor::execute(&config, command);

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

        crate::screenshot::generate_screenshot(
            &temp_config_path,
            Some(&screenshot_path),
            width,
            height,
        )?;

        // Clean up temp file
        let _ = fs::remove_file(temp_config_path);

        eprintln!("Screenshot saved to: {}", screenshot_path);
    }

    Ok(())
}

pub async fn save_current_state(output_path: Option<&str>) -> Result<()> {
    // Initialize Git repository - use open instead of discover to get the right error type
    let repo = gix::open(".").map_err(|e| crate::error::GitLineageError::from(e))?;

    // Create initial app state
    let mut app = App::new(repo);

    // Load the file tree directly
    match crate::async_task::load_file_tree(".").await {
        Ok(tree) => {
            app.navigator = crate::navigator::NavigatorState::new(tree);
            // Automatically select the first item in the tree
            app.navigator
                .handle_event(crate::navigator::NavigatorEvent::NavigateDown)
                .unwrap();
            app.ui.is_loading = false;
            app.ui.status_message = "File tree loaded".to_string();
        }
        Err(e) => {
            app.ui.status_message = format!("Error loading file tree: {}", e);
        }
    }

    // Convert app state to TestConfig format
    let config = crate::test_config::TestConfig::from_app(&mut app);

    // Convert to JSON
    let config_json = serde_json::to_string_pretty(&config)?;

    // Output the result
    match output_path {
        Some(path) => {
            fs::write(path, &config_json)?;
            println!("Current state saved to: {}", path);
        }
        None => {
            println!("{}", config_json);
        }
    }

    Ok(())
}