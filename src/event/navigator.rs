use crossterm::event::KeyCode;
use log::{debug, warn};
use tokio::sync::mpsc;

use crate::app::App;
use crate::async_task::Task;
use crate::event::file_loader::handle_file_selection_change;

pub fn handle_navigator_event(
    app: &mut App,
    key: KeyCode,
    task_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Use new navigator if available, otherwise fall back to old navigator
    if app.new_navigator.is_some() {
        debug!("🆕 Using new navigator for key: {:?}", key);
        return handle_new_navigator_event(app, key, task_sender);
    } else {
        debug!("🕴️ Using old navigator for key: {:?}", key);
    }
    
    // Legacy navigator handling (fallback)
    if app.navigator.file_tree_state.in_search_mode {
        match key {
            KeyCode::Char(c) => {
                app.navigator.file_tree_state.search_query.push(c);
                app.navigator.file_tree_state.set_search_query(app.navigator.file_tree_state.search_query.clone());
                debug!("⌨️  Added '{}' to search query, now: '{}'", c, app.navigator.file_tree_state.search_query);
                // Filtering happens automatically in UI rendering
                return Ok(());
            }
            KeyCode::Backspace => {
                app.navigator.file_tree_state.search_query.pop();
                app.navigator.file_tree_state.set_search_query(app.navigator.file_tree_state.search_query.clone());
                return Ok(());
            }
            KeyCode::Enter | KeyCode::Esc => {
                if key == KeyCode::Esc {
                    app.navigator.file_tree_state.clear_search();
                } else {
                    app.navigator.file_tree_state.exit_search_mode();
                }
                // Reset cursor to top when exiting search mode
                app.navigator.cursor_position = 0;
                app.navigator.scroll_offset = 0;
                // Update commit history and inspector panels if a file is selected
                handle_file_selection_change(app, task_sender);
                return Ok(());
            }
            _ => {
                // Let navigation keys fall through to normal handling
            }
        }
    }

    match key {
        KeyCode::Up => {
            if app.navigate_tree_up() {
                app.ui.status_message = "Navigated up".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Down => {
            if app.navigate_tree_down() {
                app.ui.status_message = "Navigated down".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Right => {
            if app.expand_selected_node() {
                app.ui.status_message = "Expanded directory".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Left => {
            if app.collapse_selected_node() {
                app.ui.status_message = "Collapsed directory".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Enter => {
            if let Some(selected_path) = app.get_selected_file_path() {
                let is_dir = app
                    .navigator
                    .file_tree_state
                    .find_node_in_tree(app.navigator.file_tree_state.display_tree(), &selected_path)
                    .map(|node| node.is_dir)
                    .unwrap_or(false);

                if is_dir {
                    let was_expanded = app
                        .navigator
                        .file_tree_state
                        .find_node_in_tree(app.navigator.file_tree_state.display_tree(), &selected_path)
                        .map(|node| node.is_expanded)
                        .unwrap_or(false);

                    app.toggle_selected_node();
                    app.ui.status_message = if was_expanded {
                        "Collapsed directory".to_string()
                    } else {
                        "Expanded directory".to_string()
                    };
                    handle_file_selection_change(app, task_sender);
                } else {
                    // For files, Enter switches to the Inspector panel to view content
                    app.ui.active_panel = crate::app::PanelFocus::Inspector;
                    app.ui.status_message =
                        format!("Viewing content for {}", selected_path.display());
                }
            }
        }
        KeyCode::Char('/') => {
            app.navigator.file_tree_state.enter_search_mode();
            // Reset cursor to top when entering search mode
            app.navigator.cursor_position = 0;
            app.navigator.scroll_offset = 0;
        }
        _ => {}
    }

    Ok(())
}

pub fn handle_new_navigator_event(
    app: &mut App,
    key: KeyCode,
    task_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::navigator::NavigatorEvent;
    
    // Check current state without rebuilding view model
    let navigator = app.new_navigator.as_mut().unwrap();
    let is_searching = navigator.is_searching();
    let search_query = navigator.get_search_query();
    
    if is_searching {
        match key {
            KeyCode::Char(c) => {
                let mut new_query = search_query.clone();
                new_query.push(c);
                debug!("🔍 Updating search query to: '{}'", new_query);
                if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::UpdateSearchQuery(new_query)) {
                    warn!("Failed to update search query: {}", e);
                } else {
                    debug!("🔍 Search query updated successfully");
                    // Don't trigger file selection change - we're just filtering, not selecting
                }
                return Ok(());
            }
            KeyCode::Backspace => {
                let mut new_query = search_query.clone();
                new_query.pop();
                if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::UpdateSearchQuery(new_query)) {
                    warn!("Failed to update search query: {}", e);
                } else {
                    // Don't trigger file selection change - we're just filtering, not selecting
                }
                return Ok(());
            }
            KeyCode::Enter => {
                // Exit search mode - keep query if it's not empty, clear if empty
                let event = if search_query.is_empty() {
                    NavigatorEvent::EndSearch
                } else {
                    NavigatorEvent::EndSearchKeepQuery
                };
                
                if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(event) {
                    warn!("Failed to end search: {}", e);
                } else {
                    // Update other panels when exiting search mode
                    handle_new_navigator_file_selection_change(app, task_sender);
                }
                
                app.ui.status_message = if search_query.is_empty() {
                    "Exited search mode".to_string()
                } else {
                    "Exited search mode - query preserved".to_string()
                };
                return Ok(());
            }
            KeyCode::Esc => {
                if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::EndSearch) {
                    warn!("Failed to end search: {}", e);
                }
                // Update file selection after exiting search
                handle_new_navigator_file_selection_change(app, task_sender);
                return Ok(());
            }
            KeyCode::Up => {
                if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::NavigateUp) {
                    warn!("Failed to navigate up in search: {}", e);
                } else {
                    // Update other panels when search navigation changes selection
                    handle_new_navigator_file_selection_change(app, task_sender);
                }
                return Ok(());
            }
            KeyCode::Down => {
                if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::NavigateDown) {
                    warn!("Failed to navigate down in search: {}", e);
                } else {
                    // Update other panels when search navigation changes selection
                    handle_new_navigator_file_selection_change(app, task_sender);
                }
                return Ok(());
            }
            _ => {
                // Other keys ignored in search mode
                return Ok(());
            }
        }
    }

    // Non-search mode navigation
    match key {
        KeyCode::Up => {
            if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::NavigateUp) {
                warn!("Failed to navigate up: {}", e);
            } else {
                app.ui.status_message = "Navigated up".to_string();
                handle_new_navigator_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Down => {
            if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::NavigateDown) {
                warn!("Failed to navigate down: {}", e);
            } else {
                app.ui.status_message = "Navigated down".to_string();
                handle_new_navigator_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Right => {
            if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::ExpandSelected) {
                warn!("Failed to expand: {}", e);
            } else {
                app.ui.status_message = "Expanded directory".to_string();
                handle_new_navigator_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Left => {
            if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::CollapseSelected) {
                warn!("Failed to collapse: {}", e);
            } else {
                app.ui.status_message = "Collapsed directory".to_string();
                handle_new_navigator_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Enter => {
            if let Some(selected_path) = app.new_navigator.as_mut().unwrap().get_selection() {
                // Check if it's a directory without rebuilding the view model
                let is_dir = app.new_navigator.as_mut().unwrap().is_path_directory(&selected_path);
                
                if is_dir {
                    // Toggle directory expansion
                    if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::ToggleExpanded(selected_path.clone())) {
                        warn!("Failed to toggle directory: {}", e);
                    } else {
                        app.ui.status_message = "Toggled directory".to_string();
                        handle_new_navigator_file_selection_change(app, task_sender);
                    }
                } else {
                    // Switch to Inspector panel for files
                    app.ui.active_panel = crate::app::PanelFocus::Inspector;
                    app.ui.status_message = format!("Viewing content for {}", selected_path.display());
                }
            }
        }
        KeyCode::Char('/') => {
            if let Err(e) = app.new_navigator.as_mut().unwrap().handle_event(NavigatorEvent::StartSearch) {
                warn!("Failed to start search: {}", e);
            } else {
                app.ui.status_message = "Search mode active".to_string();
            }
        }
        _ => {}
    }

    Ok(())
}

pub fn handle_new_navigator_file_selection_change(app: &mut App, task_sender: &mpsc::Sender<Task>) {
    let selected_path = app.new_navigator.as_mut().unwrap().get_selection();
    debug!("🔄 handle_new_navigator_file_selection_change: selected_file_path = {:?}", selected_path);
    
    // Update the old navigator's selection to keep things in sync
    if let Some(ref selected_path) = selected_path {
        app.navigator.file_tree_state.current_selection = Some(selected_path.clone());
        debug!("🔗 Synced old navigator selection to: {:?}", selected_path);
    } else {
        app.navigator.file_tree_state.current_selection = None;
        debug!("🔗 Cleared old navigator selection");
    }
    
    // Use the shared file selection change handler
    handle_file_selection_change(app, task_sender);
}