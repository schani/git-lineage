use crossterm::event::KeyCode;
use tokio::sync::mpsc;

use crate::app::App;
use crate::async_task::Task;
use crate::event::navigation::{handle_previous_change, handle_next_change};

pub fn handle_inspector_event(
    app: &mut App,
    key: KeyCode,
    async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
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
            app.inspector.cursor_line = app.inspector.cursor_line.saturating_sub(10);
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::PageDown => {
            app.inspector.cursor_line = (app.inspector.cursor_line + 10)
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
        KeyCode::Char('p') => {
            // Previous change - jump to blame commit
            handle_previous_change(app)?;
        }
        KeyCode::Char('n') => {
            // Next change - find next modification (async)
            handle_next_change(app, async_sender)?;
        }
        KeyCode::Char('d') => {
            // Toggle diff view
            app.inspector.show_diff_view = !app.inspector.show_diff_view;
            app.ui.status_message = if app.inspector.show_diff_view {
                "Switched to diff view".to_string()
            } else {
                "Switched to full file view".to_string()
            };
        }
        KeyCode::Char('g') => {
            // Go to top
            app.inspector.cursor_line = 0;
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::Char('G') => {
            // Go to bottom
            app.inspector.cursor_line = app.inspector.current_content.len().saturating_sub(1);
            app.ensure_inspector_cursor_visible();
        }
        _ => {}
    }

    Ok(())
}