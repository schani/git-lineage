use crate::app::{App, PanelFocus};
use crate::async_task::Task;
use crate::event::EventResult;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

pub fn handle_inspector_event(
    key: KeyEvent,
    app: &mut App,
    task_sender: &mpsc::Sender<Task>,
) -> EventResult {
    if app.ui.active_panel != PanelFocus::Inspector {
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('p') => {
            // Find previous change for the current line
            if let (Some(file_path), Some(commit_hash)) =
                (&app.active_file_context, &app.history.selected_commit_hash)
            {
                let task = Task::FindNextChange {
                    file_path: file_path.to_string_lossy().to_string(),
                    current_commit: commit_hash.clone(),
                    line_number: app.inspector.cursor_line,
                };

                let sender = task_sender.clone();
                tokio::spawn(async move {
                    if let Err(e) = sender.send(task).await {
                        log::error!("Failed to send FindNextChange task: {}", e);
                    }
                });

                app.start_background_task();
                app.ui.is_loading = true;
                app.ui.status_message = "Searching for previous change...".to_string();
            }
        }
        KeyCode::Char('n') => {
            // Find next change for the current line
            if let (Some(file_path), Some(commit_hash)) =
                (&app.active_file_context, &app.history.selected_commit_hash)
            {
                let task = Task::FindNextChange {
                    file_path: file_path.to_string_lossy().to_string(),
                    current_commit: commit_hash.clone(),
                    line_number: app.inspector.cursor_line,
                };

                let sender = task_sender.clone();
                tokio::spawn(async move {
                    if let Err(e) = sender.send(task).await {
                        log::error!("Failed to send FindNextChange task: {}", e);
                    }
                });

                app.start_background_task();
                app.ui.is_loading = true;
                app.ui.status_message = "Searching for next change...".to_string();
            }
        }
        _ => return Ok(false),
    }

    Ok(true)
}
