use crate::app::App;
use crate::async_task::Task;
use crate::event::EventResult;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Load commit history for the currently selected file
pub fn load_commit_history_for_selected_file(
    app: &mut App,
    task_sender: &mpsc::Sender<Task>,
) -> EventResult {
    if let Some(path) = app.navigator.get_selection() {
        let file_path = path.to_string_lossy().to_string();

        // Reset history state for the new file
        app.history.reset_for_new_file();

        // Create a cancellation token for this streaming task
        let cancellation_token = CancellationToken::new();
        app.history.streaming_cancellation_token = Some(cancellation_token.clone());

        // Start the streaming task
        let task = Task::LoadCommitHistoryStreaming {
            file_path: file_path.clone(),
            cancellation_token,
        };

        let sender = task_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = sender.send(task).await {
                log::error!("Failed to send LoadCommitHistoryStreaming task: {}", e);
            }
        });

        app.start_background_task();
        app.ui.is_loading = true;
        app.ui.status_message = format!("Loading history for {}...", file_path);
    } else {
        app.history.commit_list.clear();
        app.history.selected_commit_index = None;
        app.history.selected_commit_hash = None;
        app.inspector.current_content.clear();
        app.ui.status_message = "No file selected for history".to_string();
    }

    Ok(true)
}

/// Load more commit history for the currently selected file
pub fn load_more_commit_history(
    app: &mut App,
    task_sender: &mpsc::Sender<Task>,
) -> EventResult {
    if app.history.is_loading_more || app.history.history_complete {
        return Ok(false);
    }

    if let Some(path) = app.active_file_context.clone() {
        let file_path = path.to_string_lossy().to_string();
        let chunk_size = 50; // Load 50 commits at a time
        let start_offset = app.history.next_chunk_offset;

        let task = Task::LoadCommitHistoryProgressive {
            file_path,
            chunk_size,
            start_offset,
        };

        let sender = task_sender.clone();
        tokio::spawn(async move {
            if let Err(e) = sender.send(task).await {
                log::error!("Failed to send LoadCommitHistoryProgressive task: {}", e);
            }
        });

        app.start_background_task();
        app.history.is_loading_more = true;
        app.ui.status_message = "Loading more commits...".to_string();
    }

    Ok(true)
}
