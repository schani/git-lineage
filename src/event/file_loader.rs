use log::{debug, warn};
use tokio::sync::mpsc;

use crate::app::App;
use crate::async_task::Task;

pub fn handle_file_selection_change(app: &mut App, task_sender: &mpsc::Sender<Task>) {
    if let Some(selected_path) = app.get_selected_file_path() {
        let is_dir = app
            .navigator
            .file_tree_state
            .find_node_in_tree(app.navigator.file_tree_state.display_tree(), &selected_path)
            .map(|node| node.is_dir)
            .unwrap_or(false);

        if !is_dir {
            // It's a file - set as active context and implement progressive loading
            // Clear position tracking state when switching to a different file
            app.per_commit_cursor_positions.clear();
            app.last_commit_for_mapping = None;
            app.active_file_context = Some(selected_path.clone());

            let file_path = selected_path.to_string_lossy().to_string();
            
            // Reset history state for new file
            app.history.reset_for_new_file();
            
            // IMMEDIATE: Load file content at HEAD (synchronous, should be fast)
            load_file_content_at_head(app, &selected_path);
            
            // BACKGROUND: Start streaming history loading with cancellation token
            let cancellation_token = tokio_util::sync::CancellationToken::new();
            app.history.streaming_cancellation_token = Some(cancellation_token.clone());
            
            if let Err(e) = task_sender.try_send(crate::async_task::Task::LoadCommitHistoryStreaming {
                file_path: file_path.clone(),
                cancellation_token,
            }) {
                app.ui.status_message = format!("Failed to start history loading: {}", e);
            } else {
                app.start_background_task();
                app.history.is_loading_more = true;
                // Status message set by load_file_content_at_head will indicate content loaded + history loading
            }
        } else {
            // It's a directory - clear file context and content
            app.per_commit_cursor_positions.clear();
            app.last_commit_for_mapping = None;
            app.active_file_context = None;
            app.history.reset_for_new_file();
            app.inspector.current_content.clear();
            app.inspector.current_blame = None;
            app.inspector.cursor_line = 0;
            app.inspector.scroll_vertical = 0;
            app.ui.status_message = "Directory selected".to_string();
        }
    } else {
        // No selection - clear file context and content
        app.per_commit_cursor_positions.clear();
        app.last_commit_for_mapping = None;
        app.active_file_context = None;
        app.history.reset_for_new_file();
        app.inspector.current_content.clear();
        app.inspector.current_blame = None;
        app.inspector.cursor_line = 0;
        app.inspector.scroll_vertical = 0;
        app.ui.status_message = "No file selected".to_string();
    }
}

fn load_file_content_at_head(app: &mut App, file_path: &std::path::PathBuf) {
    let file_path_str = file_path.to_string_lossy();
    
    // Synchronous HEAD content loading - should be fast
    match crate::git_utils::get_file_content_at_head(&app.repo, &file_path_str) {
        Ok(content) => {
            app.inspector.current_content = content;
            app.inspector.cursor_line = 0;
            app.inspector.scroll_vertical = 0;
            app.inspector.scroll_horizontal = 0;
            
            let filename = file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            app.ui.status_message = format!("{} loaded (loading history...)", filename);
            
            debug!("✅ load_file_content_at_head: Successfully loaded {} lines for '{}'", 
                  app.inspector.current_content.len(), filename);
        }
        Err(e) => {
            app.inspector.current_content.clear();
            app.ui.status_message = format!("Failed to load file: {}", e);
            warn!("❌ load_file_content_at_head: Failed to load '{}': {}", file_path_str, e);
        }
    }
}