use crate::app::{App, PanelFocus};
use crate::async_task::Task;
use crate::event::EventResult;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

pub fn handle_navigation_event(key: KeyEvent, app: &mut App, task_sender: &mpsc::Sender<Task>) -> EventResult {
    match key.code {
        // Direct panel focus
        KeyCode::Char('1') => {
            app.ui.active_panel = PanelFocus::Navigator;
        }
        KeyCode::Char('2') => {
            app.ui.active_panel = PanelFocus::History;
        }
        KeyCode::Char('3') => {
            app.ui.active_panel = PanelFocus::Inspector;
        }

        // Older/Younger commit navigation (global)
        KeyCode::Char('[') => {
            // Select previous (older) commit
            if let Some(selected) = app.history.selected_commit_index {
                if selected < app.history.commit_list.len() - 1 {
                    app.history.selected_commit_index = Some(selected + 1);
                    crate::event::update_code_inspector_for_commit(app, task_sender);
                }
            }
        }
        KeyCode::Char(']') => {
            // Select next (younger) commit
            if let Some(selected) = app.history.selected_commit_index {
                if selected > 0 {
                    app.history.selected_commit_index = Some(selected - 1);
                    crate::event::update_code_inspector_for_commit(app, task_sender);
                }
            }
        }

        _ => return Ok(false),
    }

    Ok(true)
}
