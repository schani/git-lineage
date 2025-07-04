use crate::app::{App, PanelFocus};
use crate::async_task::Task;
use crossterm::event::{Event, KeyCode};
use tokio::sync::mpsc;

pub mod code_inspector;
pub mod file_loader;
pub mod history;
pub mod inspector;
pub mod navigation;
pub mod navigator;

pub type EventResult = Result<bool, Box<dyn std::error::Error>>;

/// Handle all incoming events and dispatch to appropriate handlers
pub fn handle_event(
    event: Event,
    app: &mut App,
    task_sender: &mpsc::Sender<Task>,
) -> EventResult {
    if let Event::Key(key) = event {
        // Global keybindings
        if key.code == KeyCode::Char('q') {
            app.should_quit = true;
            return Ok(true);
        }

        if key.code == KeyCode::Tab {
            app.next_panel();
            return Ok(true);
        }

        if key.code == KeyCode::BackTab {
            app.previous_panel();
            return Ok(true);
        }

        // Panel-specific keybindings
        match app.ui.active_panel {
            PanelFocus::Navigator => {
                if navigator::handle_navigator_event(key, app, task_sender)? {
                    return Ok(true);
                }
            }
            PanelFocus::History => {
                if history::handle_history_event(key, app, task_sender)? {
                    return Ok(true);
                }
            }
            PanelFocus::Inspector => {
                if code_inspector::handle_code_inspector_event(key, app, task_sender)? {
                    return Ok(true);
                }
                if inspector::handle_inspector_event(key, app, task_sender)? {
                    return Ok(true);
                }
            }
        }

        // Other global keybindings
        if navigation::handle_navigation_event(key, app)? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Update the code inspector with content from the selected commit
pub fn update_code_inspector_for_commit(app: &mut App) {
    if let Some(selected) = app.history.selected_commit_index {
        if let Some(commit) = app.history.commit_list.get(selected) {
            let commit_hash = commit.hash.clone();
            app.history.selected_commit_hash = Some(commit_hash.clone());
            
            // Clear diff data when switching commits
            app.inspector.diff_lines = None;
            app.inspector.parent_commit_hash = None;
            if app.inspector.show_diff_view {
                app.inspector.show_diff_view = false;
                app.ui.status_message = "Diff view cleared - press 'd' to regenerate".to_string();
            }

            if let Some(file_path) = app.get_active_file() {
                // Save current cursor position before switching
                if let Some(last_commit) = app.last_commit_for_mapping.clone() {
                    app.save_cursor_position(&last_commit, &file_path);
                }

                // Load file content at the new commit
                match crate::git_utils::get_file_content_at_commit(
                    &app.repo,
                    &file_path.to_string_lossy(),
                    &commit_hash,
                ) {
                    Ok(content) => {
                        app.inspector.current_content = content;
                        app.inspector.scroll_horizontal = 0;

                        // Apply smart cursor positioning
                        let status_message =
                            app.apply_smart_cursor_positioning(&commit_hash, &file_path);
                        app.ui.status_message = status_message;

                        app.ensure_inspector_cursor_visible();
                    }
                    Err(e) => {
                        app.inspector.current_content.clear();
                        app.ui.status_message = format!("Error loading file: {}", e);
                    }
                }
                
                // Update the last commit for future line mapping
                app.last_commit_for_mapping = Some(commit_hash);
            }
        }
    }
}
