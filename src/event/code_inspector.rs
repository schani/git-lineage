use crate::app::{App, PanelFocus};
use crate::async_task::Task;
use crate::event::EventResult;
use crossterm::event::{KeyCode, KeyEvent};
use tokio::sync::mpsc;

pub fn handle_code_inspector_event(
    key: KeyEvent,
    app: &mut App,
    _task_sender: &mpsc::Sender<Task>,
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
            app.ui.status_message = if app.inspector.show_diff_view {
                "Switched to diff view".to_string()
            } else {
                "Switched to full file view".to_string()
            };
        }
        _ => return Ok(false),
    }

    Ok(true)
}
