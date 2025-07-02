use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::app::App;
use crate::async_task::Task;
use crate::event::update_code_inspector_for_commit;

pub fn handle_history_event(
    app: &mut App,
    key: KeyCode,
    _async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        KeyCode::Up => {
            if let Some(selected) = app.history.list_state.selected() {
                if selected > 0 {
                    app.history.list_state.select(Some(selected - 1));
                    update_code_inspector_for_commit(app);
                }
            } else if !app.history.commit_list.is_empty() {
                app.history.list_state.select(Some(0));
                update_code_inspector_for_commit(app);
            }
        }
        KeyCode::Down => {
            if let Some(selected) = app.history.list_state.selected() {
                if selected < app.history.commit_list.len() - 1 {
                    app.history.list_state.select(Some(selected + 1));
                    update_code_inspector_for_commit(app);
                }
            } else if !app.history.commit_list.is_empty() {
                app.history.list_state.select(Some(0));
                update_code_inspector_for_commit(app);
            }
        }
        KeyCode::Enter => {
            update_code_inspector_for_commit(app);
        }
        _ => {}
    }

    Ok(())
}