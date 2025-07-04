use crate::app::{App, PanelFocus};
use crate::async_task::Task;
use crate::event::EventResult;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

pub fn handle_code_inspector_event(
    key: KeyEvent,
    app: &mut App,
    task_sender: &mpsc::Sender<Task>,
) -> EventResult {
    if app.ui.active_panel != PanelFocus::Inspector {
        return Ok(false);
    }

    match key.code {
        KeyCode::Up => {
            if app.inspector.cursor_line > 0 {
                app.inspector.cursor_line -= 1;
                app.ensure_inspector_cursor_visible();
            }
        }
        KeyCode::Down => {
            if app.inspector.cursor_line < app.inspector.current_content.len().saturating_sub(1) {
                app.inspector.cursor_line += 1;
                app.ensure_inspector_cursor_visible();
            }
        }
        KeyCode::PageUp => {
            let page_size = app.inspector.visible_height.saturating_sub(2);
            app.inspector.cursor_line = app.inspector.cursor_line.saturating_sub(page_size);
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::PageDown => {
            let page_size = app.inspector.visible_height.saturating_sub(2);
            app.inspector.cursor_line = (app.inspector.cursor_line + page_size)
                .min(app.inspector.current_content.len().saturating_sub(1));
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::Home => {
            app.inspector.cursor_line = 0;
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::End => {
            app.inspector.cursor_line = app.inspector.current_content.len().saturating_sub(1);
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::Left => {
            app.inspector.scroll_horizontal = app.inspector.scroll_horizontal.saturating_sub(1);
        }
        KeyCode::Right => {
            app.inspector.scroll_horizontal += 1;
        }
        KeyCode::Char('g') => {
            app.inspector.cursor_line = 0;
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::Char('G') => {
            app.inspector.cursor_line = app.inspector.current_content.len().saturating_sub(1);
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::Char('d') => {
            app.inspector.show_diff_view = !app.inspector.show_diff_view;
            
            if app.inspector.show_diff_view {
                // Check if we need to generate diff
                if app.inspector.diff_lines.is_none() {
                    // Get current commit and file
                    if let (Some(current_commit), Some(file_path)) = (
                        &app.history.selected_commit_hash,
                        app.get_active_file()
                    ) {
                        // Get parent commit
                        match crate::git_utils::get_parent_commit(&app.repo, current_commit) {
                            Ok(Some(parent_commit)) => {
                                app.inspector.parent_commit_hash = Some(parent_commit.clone());
                                app.ui.status_message = format!(
                                    "Loading diff view for {} at {}...",
                                    file_path.display(),
                                    &current_commit[..8]
                                );
                                app.ui.is_loading = true;
                                app.active_background_tasks += 1;
                                
                                // Send diff generation task
                                let _ = task_sender.try_send(Task::GenerateDiff {
                                    file_path: file_path.to_string_lossy().to_string(),
                                    current_commit: current_commit.clone(),
                                    parent_commit,
                                });
                            }
                            Ok(None) => {
                                app.ui.status_message = "No parent commit - this is the initial commit".to_string();
                                app.inspector.show_diff_view = false; // Revert toggle
                            }
                            Err(e) => {
                                app.ui.status_message = format!("Failed to get parent commit: {}", e);
                                app.inspector.show_diff_view = false; // Revert toggle
                            }
                        }
                    } else {
                        app.ui.status_message = "No file or commit selected for diff view".to_string();
                        app.inspector.show_diff_view = false; // Revert toggle
                    }
                } else {
                    app.ui.status_message = "Switched to diff view".to_string();
                }
            } else {
                app.ui.status_message = "Switched to full file view".to_string();
            }
        }
        _ => return Ok(false),
    }

    Ok(true)
}
