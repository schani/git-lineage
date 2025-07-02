use tokio::sync::mpsc;

use crate::app::App;
use crate::async_task::Task;
use crate::event::update_code_inspector_for_commit;

pub fn handle_previous_change(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Look up the current line in blame info and jump to that commit
    app.ui.status_message = format!("Previous change for line {}", app.inspector.cursor_line + 1);
    Ok(())
}

pub fn handle_next_change(
    app: &mut App,
    async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let (Some(ref file_path), Some(ref commit_hash)) = (
        &app.get_selected_file_path(),
        &app.history.selected_commit_hash,
    ) {
        let task = Task::FindNextChange {
            file_path: file_path.to_string_lossy().to_string(),
            current_commit: commit_hash.clone(),
            line_number: app.inspector.cursor_line,
        };

        app.ui.is_loading = true;
        app.ui.status_message = "Searching for next change...".to_string();

        if let Err(e) = async_sender.try_send(task) {
            app.ui.is_loading = false;
            app.ui.status_message = format!("Failed to start search: {}", e);
        } else {
            app.start_background_task();
        }
    } else {
        app.ui.status_message = "No file or commit selected".to_string();
    }

    Ok(())
}

/// Navigate to the previous (younger) commit in the history
/// Returns true if navigation occurred, false if no file context or at boundary
pub fn navigate_to_younger_commit(app: &mut App) -> bool {
    // Only navigate if there's an active file context
    if app.active_file_context.is_none() {
        app.ui.status_message = "No file selected".to_string();
        return false;
    }

    if app.history.commit_list.is_empty() {
        app.ui.status_message = "No commit history available".to_string();
        return false;
    }

    let current_selection = app.history.list_state.selected();

    match current_selection {
        Some(index) if index > 0 => {
            // Move to previous commit (younger)
            app.history.list_state.select(Some(index - 1));
            update_code_inspector_for_commit(app);
            let commit = &app.history.commit_list[index - 1];
            app.ui.status_message = format!("Moved to younger commit: {}", commit.short_hash);
            true
        }
        Some(_) => {
            // Already at the youngest commit (index 0) or any other index
            app.ui.status_message = "Already at youngest commit".to_string();
            false
        }
        None => {
            // No commit selected, select the first (youngest) one
            app.history.list_state.select(Some(0));
            update_code_inspector_for_commit(app);
            let commit = &app.history.commit_list[0];
            app.ui.status_message = format!("Selected youngest commit: {}", commit.short_hash);
            true
        }
    }
}

/// Navigate to the next (older) commit in the history  
/// Returns true if navigation occurred, false if no file context or at boundary
pub fn navigate_to_older_commit(app: &mut App) -> bool {
    // Only navigate if there's an active file context
    if app.active_file_context.is_none() {
        app.ui.status_message = "No file selected".to_string();
        return false;
    }

    if app.history.commit_list.is_empty() {
        app.ui.status_message = "No commit history available".to_string();
        return false;
    }

    let current_selection = app.history.list_state.selected();
    let max_index = app.history.commit_list.len() - 1;

    match current_selection {
        Some(index) if index < max_index => {
            // Move to next commit (older)
            app.history.list_state.select(Some(index + 1));
            update_code_inspector_for_commit(app);
            let commit = &app.history.commit_list[index + 1];
            app.ui.status_message = format!("Moved to older commit: {}", commit.short_hash);
            true
        }
        Some(_) => {
            // Already at the oldest commit or at max_index
            app.ui.status_message = "Already at oldest commit".to_string();
            false
        }
        None => {
            // No commit selected, select the first (youngest) one
            app.history.list_state.select(Some(0));
            update_code_inspector_for_commit(app);
            let commit = &app.history.commit_list[0];
            app.ui.status_message = format!("Selected youngest commit: {}", commit.short_hash);
            true
        }
    }
}