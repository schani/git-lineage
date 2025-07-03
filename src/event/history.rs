use crate::app::{App, PanelFocus};
use crate::event::{file_loader, update_code_inspector_for_commit, EventResult};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

pub fn handle_history_event(
    key: KeyEvent,
    app: &mut App,
    task_sender: &mpsc::Sender<crate::async_task::Task>,
) -> EventResult {
    if app.ui.active_panel != PanelFocus::History {
        return Ok(false);
    }

    match key.code {
        KeyCode::Up => {
            if let Some(selected) = app.history.selected_commit_index {
                if selected > 0 {
                    app.history.selected_commit_index = Some(selected - 1);
                    update_code_inspector_for_commit(app);
                }
            } else if !app.history.commit_list.is_empty() {
                app.history.selected_commit_index = Some(0);
                update_code_inspector_for_commit(app);
            }
        }
        KeyCode::Down => {
            if let Some(selected) = app.history.selected_commit_index {
                if selected < app.history.commit_list.len() - 1 {
                    app.history.selected_commit_index = Some(selected + 1);
                    update_code_inspector_for_commit(app);
                } else {
                    // At the bottom of the list, try to load more
                    file_loader::load_more_commit_history(app, task_sender)?;
                }
            } else if !app.history.commit_list.is_empty() {
                app.history.selected_commit_index = Some(0);
                update_code_inspector_for_commit(app);
            }
        }
        KeyCode::Enter => {
            // Switch focus to inspector
            app.ui.active_panel = PanelFocus::Inspector;
        }
        _ => return Ok(false),
    }

    Ok(true)
}

