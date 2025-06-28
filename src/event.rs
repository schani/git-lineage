use crossterm::event::{Event, KeyCode, KeyModifiers};
use tokio::sync::mpsc;

use crate::app::{App, PanelFocus};
use crate::async_task::Task;

pub fn handle_event(
    event: Event,
    app: &mut App,
    async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    match event {
        Event::Key(key) => {
            // Global keybindings
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    app.should_quit = true;
                    return Ok(());
                }
                KeyCode::Tab => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        app.previous_panel();
                    } else {
                        app.next_panel();
                    }
                    return Ok(());
                }
                _ => {}
            }

            // Panel-specific keybindings
            match app.active_panel {
                PanelFocus::Navigator => handle_navigator_event(app, key.code, async_sender)?,
                PanelFocus::History => handle_history_event(app, key.code, async_sender)?,
                PanelFocus::Inspector => handle_inspector_event(app, key.code, async_sender)?,
            }
        }
        Event::Resize(_, _) => {
            // Handle terminal resize if needed
        }
        _ => {}
    }

    Ok(())
}

fn handle_navigator_event(
    app: &mut App,
    key: KeyCode,
    _async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    if app.in_search_mode {
        match key {
            KeyCode::Char(c) => {
                app.search_query.push(c);
                // TODO: Filter file tree based on search query
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            KeyCode::Enter | KeyCode::Esc => {
                app.in_search_mode = false;
                if key == KeyCode::Esc {
                    app.search_query.clear();
                }
            }
            _ => {}
        }
        return Ok(());
    }

    match key {
        KeyCode::Up => {
            if app.navigate_tree_up() {
                app.status_message = "Navigated up".to_string();
            }
        }
        KeyCode::Down => {
            if app.navigate_tree_down() {
                app.status_message = "Navigated down".to_string();
            }
        }
        KeyCode::Right => {
            if app.expand_selected_node() {
                app.status_message = "Expanded directory".to_string();
            }
        }
        KeyCode::Left => {
            if app.collapse_selected_node() {
                app.status_message = "Collapsed directory".to_string();
            }
        }
        KeyCode::Enter => {
            if let Some(selected_path) = app.get_selected_file_path() {
                let is_dir = app.file_tree.find_node(&selected_path)
                    .map(|node| node.is_dir)
                    .unwrap_or(false);
                
                if is_dir {
                    let was_expanded = app.file_tree.find_node(&selected_path)
                        .map(|node| node.is_expanded)
                        .unwrap_or(false);
                    
                    app.toggle_selected_node();
                    app.status_message = if was_expanded {
                        "Collapsed directory".to_string()
                    } else {
                        "Expanded directory".to_string()
                    };
                } else {
                    app.status_message = format!("Selected: {}", selected_path.display());
                    // TODO: Load commit history for this file
                }
            }
        }
        KeyCode::Char('/') => {
            app.in_search_mode = true;
            app.search_query.clear();
        }
        _ => {}
    }

    Ok(())
}

fn handle_history_event(
    app: &mut App,
    key: KeyCode,
    _async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        KeyCode::Up => {
            if let Some(selected) = app.commit_list_state.selected() {
                if selected > 0 {
                    app.commit_list_state.select(Some(selected - 1));
                    update_code_inspector_for_commit(app);
                }
            } else if !app.commit_list.is_empty() {
                app.commit_list_state.select(Some(0));
                update_code_inspector_for_commit(app);
            }
        }
        KeyCode::Down => {
            if let Some(selected) = app.commit_list_state.selected() {
                if selected < app.commit_list.len() - 1 {
                    app.commit_list_state.select(Some(selected + 1));
                    update_code_inspector_for_commit(app);
                }
            } else if !app.commit_list.is_empty() {
                app.commit_list_state.select(Some(0));
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

fn handle_inspector_event(
    app: &mut App,
    key: KeyCode,
    async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        KeyCode::Up => {
            if app.cursor_line > 0 {
                app.cursor_line -= 1;
                if app.cursor_line < app.inspector_scroll_vertical as usize {
                    app.inspector_scroll_vertical = app.cursor_line as u16;
                }
            }
        }
        KeyCode::Down => {
            if app.cursor_line < app.current_content.len().saturating_sub(1) {
                app.cursor_line += 1;
                // TODO: Implement scroll logic based on visible area
            }
        }
        KeyCode::PageUp => {
            app.cursor_line = app.cursor_line.saturating_sub(10);
            app.inspector_scroll_vertical = app.cursor_line as u16;
        }
        KeyCode::PageDown => {
            app.cursor_line = (app.cursor_line + 10).min(app.current_content.len().saturating_sub(1));
        }
        KeyCode::Home => {
            app.cursor_line = 0;
            app.inspector_scroll_vertical = 0;
        }
        KeyCode::End => {
            app.cursor_line = app.current_content.len().saturating_sub(1);
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
            app.show_diff_view = !app.show_diff_view;
            app.status_message = if app.show_diff_view {
                "Switched to diff view".to_string()
            } else {
                "Switched to full file view".to_string()
            };
        }
        KeyCode::Char('g') => {
            // Go to top
            app.cursor_line = 0;
            app.inspector_scroll_vertical = 0;
        }
        KeyCode::Char('G') => {
            // Go to bottom
            app.cursor_line = app.current_content.len().saturating_sub(1);
        }
        _ => {}
    }

    Ok(())
}

fn update_code_inspector_for_commit(app: &mut App) {
    if let Some(selected) = app.commit_list_state.selected() {
        if selected < app.commit_list.len() {
            let commit = &app.commit_list[selected];
            app.selected_commit_hash = Some(commit.hash.clone());
            app.status_message = format!("Viewing commit: {}", commit.short_hash);
            
            // TODO: Load file content and blame info for this commit
            // For now, just set placeholder content
            app.current_content = vec![
                format!("// Content for commit {}", commit.short_hash),
                "// TODO: Load actual file content from Git".to_string(),
                "".to_string(),
                "fn main() {".to_string(),
                "    println!(\"Hello, world!\");".to_string(),
                "}".to_string(),
            ];
        }
    }
}

fn handle_previous_change(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Look up the current line in blame info and jump to that commit
    app.status_message = format!("Previous change for line {}", app.cursor_line + 1);
    Ok(())
}

fn handle_next_change(
    app: &mut App,
    async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let (Some(ref file_path), Some(ref commit_hash)) = 
        (&app.get_selected_file_path(), &app.selected_commit_hash) {
        
        let task = Task::FindNextChange {
            file_path: file_path.to_string_lossy().to_string(),
            current_commit: commit_hash.clone(),
            line_number: app.cursor_line,
        };

        app.is_loading = true;
        app.status_message = "Searching for next change...".to_string();

        if let Err(e) = async_sender.try_send(task) {
            app.is_loading = false;
            app.status_message = format!("Failed to start search: {}", e);
        }
    } else {
        app.status_message = "No file or commit selected".to_string();
    }

    Ok(())
}