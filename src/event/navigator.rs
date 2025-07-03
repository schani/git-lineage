use crate::app::{App, PanelFocus};
use crate::async_task::Task;
use crate::event::{file_loader, EventResult};
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

pub fn handle_navigator_event(
    key: KeyEvent,
    app: &mut App,
    task_sender: &mpsc::Sender<Task>,
) -> EventResult {
    if app.ui.active_panel != PanelFocus::Navigator {
        return Ok(false);
    }

    let mut event_handled = false;

    // Handle search mode first
    if app.navigator.is_searching() {
        match key.code {
            KeyCode::Esc => {
                app.navigator
                    .handle_event(crate::navigator::NavigatorEvent::EndSearch)?;
                event_handled = true;
            }
            KeyCode::Enter => {
                app.navigator
                    .handle_event(crate::navigator::NavigatorEvent::EndSearchKeepQuery)?;
                event_handled = true;
            }
            KeyCode::Char(c) => {
                let mut query = app.navigator.get_search_query();
                query.push(c);
                app.navigator
                    .handle_event(crate::navigator::NavigatorEvent::UpdateSearchQuery(query))?;
                event_handled = true;
            }
            KeyCode::Backspace => {
                let mut query = app.navigator.get_search_query();
                query.pop();
                app.navigator
                    .handle_event(crate::navigator::NavigatorEvent::UpdateSearchQuery(query))?;
                event_handled = true;
            }
            _ => {}
        }
    }

    if event_handled {
        return Ok(true);
    }

    // Handle normal mode navigation
    match key.code {
        KeyCode::Up => {
            app.navigator
                .handle_event(crate::navigator::NavigatorEvent::NavigateUp)?;
        }
        KeyCode::Down => {
            app.navigator
                .handle_event(crate::navigator::NavigatorEvent::NavigateDown)?;
        }
        KeyCode::Left => {
            app.navigator
                .handle_event(crate::navigator::NavigatorEvent::CollapseSelected)?;
        }
        KeyCode::Right => {
            app.navigator
                .handle_event(crate::navigator::NavigatorEvent::ExpandSelected)?;
        }
        KeyCode::Enter => {
            if let Some(selection) = app.navigator.get_selection() {
                if app.navigator.is_path_directory(&selection) {
                    // Toggle directory expansion
                    app.navigator.handle_event(
                        crate::navigator::NavigatorEvent::ToggleExpanded(selection),
                    )?;
                } else {
                    // Select file and load history
                    app.active_file_context = Some(selection);
                    file_loader::load_commit_history_for_selected_file(app, task_sender)?;
                    app.ui.active_panel = PanelFocus::History;
                }
            }
        }
        KeyCode::Char('/') => {
            app.navigator
                .handle_event(crate::navigator::NavigatorEvent::StartSearch)?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}
