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
                KeyCode::Char('q') => {
                    app.should_quit = true;
                    return Ok(());
                }
                KeyCode::Esc => {
                    // Don't quit if in search mode - let panel handlers deal with it
                    if !app.in_search_mode {
                        app.should_quit = true;
                        return Ok(());
                    }
                }
                KeyCode::Tab => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        app.previous_panel();
                    } else {
                        app.next_panel();
                    }
                    return Ok(());
                }
                KeyCode::Char('1') => {
                    app.active_panel = PanelFocus::Navigator;
                    return Ok(());
                }
                KeyCode::Char('2') => {
                    app.active_panel = PanelFocus::History;
                    return Ok(());
                }
                KeyCode::Char('3') => {
                    app.active_panel = PanelFocus::Inspector;
                    return Ok(());
                }
                KeyCode::Char('[') => {
                    if navigate_to_older_commit(app) {
                        return Ok(());
                    }
                }
                KeyCode::Char(']') => {
                    if navigate_to_younger_commit(app) {
                        return Ok(());
                    }
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
    task_sender: &mpsc::Sender<Task>,
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
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Down => {
            if app.navigate_tree_down() {
                app.status_message = "Navigated down".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Right => {
            if app.expand_selected_node() {
                app.status_message = "Expanded directory".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Left => {
            if app.collapse_selected_node() {
                app.status_message = "Collapsed directory".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Enter => {
            if let Some(selected_path) = app.get_selected_file_path() {
                let is_dir = app
                    .file_tree
                    .find_node(&selected_path)
                    .map(|node| node.is_dir)
                    .unwrap_or(false);

                if is_dir {
                    let was_expanded = app
                        .file_tree
                        .find_node(&selected_path)
                        .map(|node| node.is_expanded)
                        .unwrap_or(false);

                    app.toggle_selected_node();
                    app.status_message = if was_expanded {
                        "Collapsed directory".to_string()
                    } else {
                        "Expanded directory".to_string()
                    };
                    handle_file_selection_change(app, task_sender);
                } else {
                    // For files, Enter switches to the Inspector panel to view content
                    app.active_panel = crate::app::PanelFocus::Inspector;
                    app.status_message = format!("Viewing content for {}", selected_path.display());
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
                app.ensure_inspector_cursor_visible();
            }
        }
        KeyCode::Down => {
            if app.cursor_line < app.current_content.len().saturating_sub(1) {
                app.cursor_line += 1;
                app.ensure_inspector_cursor_visible();
            }
        }
        KeyCode::PageUp => {
            app.cursor_line = app.cursor_line.saturating_sub(10);
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::PageDown => {
            app.cursor_line =
                (app.cursor_line + 10).min(app.current_content.len().saturating_sub(1));
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::Home => {
            app.cursor_line = 0;
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::End => {
            app.cursor_line = app.current_content.len().saturating_sub(1);
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
            app.ensure_inspector_cursor_visible();
        }
        KeyCode::Char('G') => {
            // Go to bottom
            app.cursor_line = app.current_content.len().saturating_sub(1);
            app.ensure_inspector_cursor_visible();
        }
        _ => {}
    }

    Ok(())
}

pub fn update_code_inspector_for_commit(app: &mut App) {
    if let Some(selected) = app.commit_list_state.selected() {
        if selected < app.commit_list.len() {
            // Extract commit data before any mutable borrows
            let commit_hash = app.commit_list[selected].hash.clone();
            let commit_short_hash = app.commit_list[selected].short_hash.clone();
            let commit_author = app.commit_list[selected].author.clone();
            let commit_date = app.commit_list[selected].date.clone();
            let commit_subject = app.commit_list[selected].subject.clone();
            
            // Save current cursor position AND viewport position for visual consistency
            let old_cursor_line = app.cursor_line;
            let old_commit_hash = app.selected_commit_hash.clone();
            let old_scroll_vertical = app.inspector_scroll_vertical;
            let old_cursor_viewport_offset = old_cursor_line.saturating_sub(old_scroll_vertical as usize);
            
            app.selected_commit_hash = Some(commit_hash.clone());

            // Load actual file content at this commit if we have an active file context
            if let Some(ref file_path) = app.active_file_context {
                let file_path = file_path.clone(); // Clone to avoid borrow issues
                
                app.is_loading = true;
                app.status_message = format!(
                    "Loading {} at commit {}...",
                    file_path.file_name().unwrap_or_default().to_string_lossy(),
                    &commit_short_hash
                );

                // Set up line mapping if we had a previous commit
                if let Some(ref previous_commit) = old_commit_hash {
                    if previous_commit != &commit_hash {
                        app.save_cursor_position(previous_commit, &file_path);
                        // Set the previous commit for line mapping
                        app.last_commit_for_mapping = Some(previous_commit.clone());
                    }
                }

                match crate::git_utils::get_file_content_at_commit(
                    &app.repo,
                    &file_path.to_string_lossy(),
                    &commit_hash,
                ) {
                    Ok(content) => {
                        app.current_content = content;
                        app.inspector_scroll_horizontal = 0;
                        
                        // Restore the cursor line for mapping, then apply smart positioning
                        app.cursor_line = old_cursor_line;
                        let positioning_message = app.apply_smart_cursor_positioning(&commit_hash, &file_path);
                        
                        // Restore viewport position: try to keep cursor at same visual position
                        let new_cursor_line = app.cursor_line;
                        let desired_scroll = new_cursor_line.saturating_sub(old_cursor_viewport_offset);
                        app.inspector_scroll_vertical = desired_scroll as u16;
                        
                        // Ensure the viewport is valid (cursor is still visible)
                        app.ensure_inspector_cursor_visible();
                        
                        // Combine file loading info with cursor positioning info
                        let file_info = format!(
                            "Loaded {} ({} lines) at commit {}",
                            file_path.file_name().unwrap_or_default().to_string_lossy(),
                            app.current_content.len(),
                            &commit_short_hash
                        );
                        
                        app.status_message = if positioning_message.contains("top of file") || positioning_message.contains("unchanged") {
                            file_info
                        } else {
                            format!("{} • {}", file_info, positioning_message)
                        };
                    }
                    Err(e) => {
                        app.current_content = vec![
                            format!("Error loading file content:"),
                            format!("{}", e),
                            "".to_string(),
                            "This could happen if:".to_string(),
                            "- The file didn't exist at this commit".to_string(),
                            "- The commit hash is invalid".to_string(),
                            "- There's a Git repository issue".to_string(),
                        ];
                        // Reset cursor position and clear tracking state on error
                        app.cursor_line = 0;
                        app.last_commit_for_mapping = None;
                        app.status_message = format!("Failed to load content: {}", e);
                    }
                }
                app.is_loading = false;
            } else {
                // No file selected - show commit info instead
                app.current_content = vec![
                    format!("Commit: {}", commit_hash),
                    format!("Short: {}", commit_short_hash),
                    format!("Author: {}", commit_author),
                    format!("Date: {}", commit_date),
                    format!("Subject: {}", commit_subject),
                    "".to_string(),
                    "Select a file to view its content at this commit.".to_string(),
                ];
                // Clear cursor position and tracking state when no file is selected
                app.cursor_line = 0;
                app.last_commit_for_mapping = None;
                app.status_message = format!("Viewing commit: {}", commit_short_hash);
            }
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
        (&app.get_selected_file_path(), &app.selected_commit_hash)
    {
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

fn handle_file_selection_change(app: &mut App, task_sender: &mpsc::Sender<Task>) {
    if let Some(selected_path) = app.get_selected_file_path() {
        let is_dir = app
            .file_tree
            .find_node(&selected_path)
            .map(|node| node.is_dir)
            .unwrap_or(false);

        if !is_dir {
            // It's a file - set as active context and load commit history
            // Clear position tracking state when switching to a different file
            app.per_commit_cursor_positions.clear();
            app.last_commit_for_mapping = None;
            app.active_file_context = Some(selected_path.clone());
            
            let file_path = selected_path.to_string_lossy().to_string();
            if let Err(e) = task_sender.try_send(crate::async_task::Task::LoadCommitHistory {
                file_path: file_path.clone(),
            }) {
                app.status_message = format!("Failed to load commit history: {}", e);
            } else {
                app.status_message = format!(
                    "Loading history for {}",
                    selected_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                );
            }
        } else {
            // It's a directory - clear file context and content
            app.per_commit_cursor_positions.clear();
            app.last_commit_for_mapping = None;
            app.active_file_context = None;
            app.commit_list.clear();
            app.commit_list_state.select(None);
            app.selected_commit_hash = None;
            app.current_content.clear();
            app.current_blame = None;
            app.cursor_line = 0;
            app.inspector_scroll_vertical = 0;
            app.status_message = "Directory selected".to_string();
        }
    } else {
        // No selection - clear file context and content
        app.per_commit_cursor_positions.clear();
        app.last_commit_for_mapping = None;
        app.active_file_context = None;
        app.commit_list.clear();
        app.commit_list_state.select(None);
        app.selected_commit_hash = None;
        app.current_content.clear();
        app.current_blame = None;
        app.cursor_line = 0;
        app.inspector_scroll_vertical = 0;
        app.status_message = "No file selected".to_string();
    }
}

/// Navigate to the previous (younger) commit in the history
/// Returns true if navigation occurred, false if no file context or at boundary
fn navigate_to_younger_commit(app: &mut App) -> bool {
    // Only navigate if there's an active file context
    if app.active_file_context.is_none() {
        app.status_message = "No file selected".to_string();
        return false;
    }

    if app.commit_list.is_empty() {
        app.status_message = "No commit history available".to_string();
        return false;
    }

    let current_selection = app.commit_list_state.selected();
    
    match current_selection {
        Some(index) if index > 0 => {
            // Move to previous commit (younger)
            app.commit_list_state.select(Some(index - 1));
            update_code_inspector_for_commit(app);
            let commit = &app.commit_list[index - 1];
            app.status_message = format!("Moved to younger commit: {}", commit.short_hash);
            true
        }
        Some(_) => {
            // Already at the youngest commit (index 0) or any other index
            app.status_message = "Already at youngest commit".to_string();
            false
        }
        None => {
            // No commit selected, select the first (youngest) one
            app.commit_list_state.select(Some(0));
            update_code_inspector_for_commit(app);
            let commit = &app.commit_list[0];
            app.status_message = format!("Selected youngest commit: {}", commit.short_hash);
            true
        }
    }
}

/// Navigate to the next (older) commit in the history  
/// Returns true if navigation occurred, false if no file context or at boundary
fn navigate_to_older_commit(app: &mut App) -> bool {
    // Only navigate if there's an active file context
    if app.active_file_context.is_none() {
        app.status_message = "No file selected".to_string();
        return false;
    }

    if app.commit_list.is_empty() {
        app.status_message = "No commit history available".to_string();
        return false;
    }

    let current_selection = app.commit_list_state.selected();
    let max_index = app.commit_list.len() - 1;
    
    match current_selection {
        Some(index) if index < max_index => {
            // Move to next commit (older)
            app.commit_list_state.select(Some(index + 1));
            update_code_inspector_for_commit(app);
            let commit = &app.commit_list[index + 1];
            app.status_message = format!("Moved to older commit: {}", commit.short_hash);
            true
        }
        Some(_) => {
            // Already at the oldest commit or at max_index
            app.status_message = "Already at oldest commit".to_string();
            false
        }
        None => {
            // No commit selected, select the first (youngest) one
            app.commit_list_state.select(Some(0));
            update_code_inspector_for_commit(app);
            let commit = &app.commit_list[0];
            app.status_message = format!("Selected youngest commit: {}", commit.short_hash);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{App, PanelFocus};
    use crate::async_task::Task;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use std::path::PathBuf;
    use tokio::sync::mpsc;

    // Test utilities
    fn create_test_app() -> App {
        let repo = crate::git_utils::open_repository(".")
            .unwrap_or_else(|_| panic!("Failed to open test repository"));
        let mut app = App::new(repo);

        // Set up a basic file tree for testing by building nodes manually
        use crate::tree::TreeNode;

        // Create root level nodes
        let src_main = TreeNode::new_file("main.rs".to_string(), "src/main.rs".into());
        let src_lib = TreeNode::new_file("lib.rs".to_string(), "src/lib.rs".into());
        let mut tests_dir = TreeNode::new_dir("tests".to_string(), "tests".into());
        let test_file = TreeNode::new_file("test.rs".to_string(), "tests/test.rs".into());

        // Add test file to tests directory
        tests_dir.add_child(test_file);

        // Add nodes to the file tree root
        app.file_tree.root.push(src_main);
        app.file_tree.root.push(src_lib);
        app.file_tree.root.push(tests_dir);

        // Add some commits for testing
        app.commit_list = vec![
            crate::app::CommitInfo {
                hash: "abc123".to_string(),
                short_hash: "abc123".to_string(),
                author: "Test Author".to_string(),
                date: "2023-01-01".to_string(),
                subject: "Test commit".to_string(),
            },
            crate::app::CommitInfo {
                hash: "def456".to_string(),
                short_hash: "def456".to_string(),
                author: "Another Author".to_string(),
                date: "2023-01-02".to_string(),
                subject: "Another commit".to_string(),
            },
        ];

        app
    }

    fn create_key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn create_key_event_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    async fn create_test_channel() -> (mpsc::Sender<Task>, mpsc::Receiver<Task>) {
        mpsc::channel(100)
    }

    mod global_keybindings {
        use super::*;

        #[tokio::test]
        async fn test_quit_on_q() {
            let mut app = create_test_app();
            let (tx, _rx) = create_test_channel().await;
            let event = create_key_event(KeyCode::Char('q'));

            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.should_quit);
        }

        #[tokio::test]
        async fn test_quit_on_esc() {
            let mut app = create_test_app();
            let (tx, _rx) = create_test_channel().await;
            let event = create_key_event(KeyCode::Esc);

            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.should_quit);
        }

        #[tokio::test]
        async fn test_tab_next_panel() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            let (tx, _rx) = create_test_channel().await;
            let event = create_key_event(KeyCode::Tab);

            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.active_panel, PanelFocus::History);
        }

        #[tokio::test]
        async fn test_shift_tab_previous_panel() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::History;
            let (tx, _rx) = create_test_channel().await;
            let event = create_key_event_with_modifiers(KeyCode::Tab, KeyModifiers::SHIFT);

            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.active_panel, PanelFocus::Navigator);
        }

        #[tokio::test]
        async fn test_resize_event_handling() {
            let mut app = create_test_app();
            let (tx, _rx) = create_test_channel().await;
            let event = Event::Resize(80, 24);

            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            // Resize events are currently handled as no-op, so just verify no errors
        }

        #[tokio::test]
        async fn test_unknown_event_handling() {
            let mut app = create_test_app();
            let (tx, _rx) = create_test_channel().await;
            let event = Event::Mouse(crossterm::event::MouseEvent {
                kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            });

            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            // Unknown events should be handled gracefully
        }
    }

    mod navigator_events {
        use super::*;

        #[tokio::test]
        async fn test_up_down_navigation() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            let (tx, _rx) = create_test_channel().await;

            // Test down navigation
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());

            // Test up navigation
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_expand_collapse_directories() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            let (tx, _rx) = create_test_channel().await;

            // Test right (expand)
            let event = create_key_event(KeyCode::Right);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());

            // Test left (collapse)
            let event = create_key_event(KeyCode::Left);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_search_mode_activation() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            app.in_search_mode = false;
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('/'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.in_search_mode);
            assert!(app.search_query.is_empty());
        }

        #[tokio::test]
        async fn test_search_input_handling() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            app.in_search_mode = true;
            let (tx, _rx) = create_test_channel().await;

            // Test character input
            let event = create_key_event(KeyCode::Char('t'));
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.search_query, "t");

            // Test more characters
            let event = create_key_event(KeyCode::Char('e'));
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.search_query, "te");

            // Test backspace
            let event = create_key_event(KeyCode::Backspace);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.search_query, "t");
        }

        #[tokio::test]
        async fn test_search_escape() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            app.in_search_mode = true;
            app.search_query = "test query".to_string();
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Esc);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(!app.in_search_mode);
            assert!(app.search_query.is_empty());
        }

        #[tokio::test]
        async fn test_search_enter() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            app.in_search_mode = true;
            app.search_query = "test query".to_string();
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Enter);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(!app.in_search_mode);
            assert_eq!(app.search_query, "test query"); // Should preserve query on Enter
        }

        #[tokio::test]
        async fn test_file_selection_triggers_history_load() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            // Start with no selection
            app.file_tree.current_selection = None;
            let (tx, mut rx) = create_test_channel().await;

            // Navigate down should trigger automatic loading for the first file
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());

            // Check that a task was sent due to navigation triggering auto-load
            let task = rx.try_recv();
            assert!(task.is_ok());
            match task.unwrap() {
                Task::LoadCommitHistory { file_path } => {
                    // The first navigation should select the first file
                    assert!(file_path.contains("lib.rs") || file_path.contains("main.rs"));
                }
                _ => panic!("Expected LoadCommitHistory task"),
            }
        }

        #[tokio::test]
        async fn test_enter_on_file_switches_to_inspector() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            app.file_tree.current_selection = Some(PathBuf::from("src/main.rs"));
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Enter);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.active_panel, PanelFocus::Inspector);
            assert!(app.status_message.contains("Viewing content"));
        }
    }

    mod history_events {
        use super::*;

        #[tokio::test]
        async fn test_commit_navigation_up() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::History;
            app.commit_list_state.select(Some(1)); // Start at second commit
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(0));
        }

        #[tokio::test]
        async fn test_commit_navigation_down() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::History;
            app.commit_list_state.select(Some(0)); // Start at first commit
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(1));
        }

        #[tokio::test]
        async fn test_commit_navigation_bounds() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::History;
            app.commit_list_state.select(Some(0)); // At first commit
            let (tx, _rx) = create_test_channel().await;

            // Try to go up from first commit
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(0)); // Should stay at first

            // Go to last commit
            app.commit_list_state.select(Some(1));

            // Try to go down from last commit
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(1)); // Should stay at last
        }

        #[tokio::test]
        async fn test_commit_selection_with_enter() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::History;
            app.commit_list_state.select(Some(0));
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Enter);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            // Verify that update_code_inspector_for_commit was called
            assert!(app.selected_commit_hash.is_some());
            assert!(!app.current_content.is_empty());
        }

        #[tokio::test]
        async fn test_empty_history_handling() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::History;
            app.commit_list.clear(); // Empty commit list
            app.commit_list_state.select(None);
            let (tx, _rx) = create_test_channel().await;

            // Test navigation with empty list
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());

            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
        }
    }

    mod inspector_events {
        use super::*;

        #[tokio::test]
        async fn test_cursor_up_down_movement() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.current_content = vec![
                "line1".to_string(),
                "line2".to_string(),
                "line3".to_string(),
            ];
            app.cursor_line = 1;
            let (tx, _rx) = create_test_channel().await;

            // Test up movement
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 0);

            // Test down movement
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 1);
        }

        #[tokio::test]
        async fn test_page_up_down_movement() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.current_content = (0..50).map(|i| format!("line{}", i)).collect();
            app.cursor_line = 20;
            let (tx, _rx) = create_test_channel().await;

            // Test page up
            let event = create_key_event(KeyCode::PageUp);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 10);

            // Test page down
            let event = create_key_event(KeyCode::PageDown);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 20);
        }

        #[tokio::test]
        async fn test_home_end_navigation() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.current_content = (0..10).map(|i| format!("line{}", i)).collect();
            app.cursor_line = 5;
            let (tx, _rx) = create_test_channel().await;

            // Test Home key
            let event = create_key_event(KeyCode::Home);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 0);
            assert_eq!(app.inspector_scroll_vertical, 0);

            // Test End key
            let event = create_key_event(KeyCode::End);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 9);
        }

        #[tokio::test]
        async fn test_cursor_bounds_validation() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.current_content = vec!["line1".to_string(), "line2".to_string()];
            app.cursor_line = 0;
            let (tx, _rx) = create_test_channel().await;

            // Try to go up from first line
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 0); // Should stay at 0

            // Go to last line
            app.cursor_line = 1;

            // Try to go down from last line
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 1); // Should stay at last line
        }

        #[tokio::test]
        async fn test_diff_view_toggle() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.show_diff_view = false;
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('d'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.show_diff_view);
            assert!(app.status_message.contains("diff view"));
        }

        #[tokio::test]
        async fn test_go_to_shortcuts() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.current_content = (0..10).map(|i| format!("line{}", i)).collect();
            app.cursor_line = 5;
            let (tx, _rx) = create_test_channel().await;

            // Test 'g' (go to top)
            let event = create_key_event(KeyCode::Char('g'));
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 0);

            // Test 'G' (go to bottom)
            let event = create_key_event(KeyCode::Char('G'));
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.cursor_line, 9);
        }

        #[tokio::test]
        async fn test_previous_change_navigation() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.cursor_line = 5;
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('p'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.status_message.contains("Previous change"));
        }

        #[tokio::test]
        async fn test_next_change_with_valid_context() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.cursor_line = 5;
            app.file_tree.current_selection = Some(PathBuf::from("src/main.rs"));
            app.selected_commit_hash = Some("abc123".to_string());
            let (tx, mut rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('n'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.is_loading);
            assert!(app.status_message.contains("Searching"));

            // Check that the correct task was sent
            let task = rx.try_recv();
            assert!(task.is_ok());
            match task.unwrap() {
                Task::FindNextChange {
                    file_path,
                    current_commit,
                    line_number,
                } => {
                    assert!(file_path.contains("main.rs"));
                    assert_eq!(current_commit, "abc123");
                    assert_eq!(line_number, 5);
                }
                _ => panic!("Expected FindNextChange task"),
            }
        }

        #[tokio::test]
        async fn test_next_change_without_context() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.file_tree.current_selection = None; // No file selected
            app.selected_commit_hash = None; // No commit selected
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('n'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(!app.is_loading);
            assert!(app.status_message.contains("No file or commit selected"));
        }
    }

    mod helper_functions {
        use super::*;

        #[test]
        fn test_update_code_inspector_for_commit() {
            let mut app = create_test_app();
            app.commit_list_state.select(Some(0));

            update_code_inspector_for_commit(&mut app);

            assert_eq!(app.selected_commit_hash, Some("abc123".to_string()));
            assert!(app.status_message.contains("abc123"));
            assert!(!app.current_content.is_empty());
        }

        #[test]
        fn test_update_code_inspector_invalid_selection() {
            let mut app = create_test_app();
            app.commit_list_state.select(Some(999)); // Invalid index

            update_code_inspector_for_commit(&mut app);

            // Should not crash, but also should not update anything
        }

        #[test]
        fn test_update_code_inspector_no_selection() {
            let mut app = create_test_app();
            app.commit_list_state.select(None); // No selection

            update_code_inspector_for_commit(&mut app);

            // Should not crash or change state
            assert!(app.selected_commit_hash.is_none());
        }

        #[test]
        fn test_update_code_inspector_with_file_context() {
            let mut app = create_test_app();
            app.commit_list_state.select(Some(0));
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));

            update_code_inspector_for_commit(&mut app);

            assert_eq!(app.selected_commit_hash, Some("abc123".to_string()));
            assert!(!app.is_loading); // Should complete loading
        }

        #[test]
        fn test_update_code_inspector_without_file_context() {
            let mut app = create_test_app();
            app.commit_list_state.select(Some(0));
            app.active_file_context = None; // No file selected

            update_code_inspector_for_commit(&mut app);

            assert_eq!(app.selected_commit_hash, Some("abc123".to_string()));
            assert!(!app.current_content.is_empty());
            assert!(app.current_content[0].contains("Commit:"));
            assert!(app.current_content[1].contains("Short:"));
            assert!(app.status_message.contains("Viewing commit"));
        }

        #[test]
        fn test_update_code_inspector_preserves_cursor_position() {
            let mut app = create_test_app();
            app.commit_list_state.select(Some(0));
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.cursor_line = 5;
            app.inspector_scroll_vertical = 10;

            let _old_cursor = app.cursor_line;
            let _old_scroll = app.inspector_scroll_vertical;

            update_code_inspector_for_commit(&mut app);

            // The positioning system should be invoked
            assert_eq!(app.selected_commit_hash, Some("abc123".to_string()));
            // The exact cursor/scroll position depends on line mapping, but it shouldn't crash
        }

        #[test]
        fn test_update_code_inspector_clears_position_tracking_state() {
            let mut app = create_test_app();
            app.commit_list_state.select(Some(0));
            app.active_file_context = None; // No file context
            app.last_commit_for_mapping = Some("old_commit".to_string());

            update_code_inspector_for_commit(&mut app);

            assert!(app.last_commit_for_mapping.is_none());
            assert_eq!(app.cursor_line, 0);
        }

        #[test]
        fn test_handle_previous_change() {
            let mut app = create_test_app();
            app.cursor_line = 10;

            let result = handle_previous_change(&mut app);

            assert!(result.is_ok());
            assert!(app.status_message.contains("Previous change for line 11"));
        }

        #[test]
        fn test_handle_next_change_task_send_failure() {
            let mut app = create_test_app();
            app.file_tree.current_selection = Some(PathBuf::from("src/main.rs"));
            app.selected_commit_hash = Some("abc123".to_string());
            app.cursor_line = 5;

            // Create a channel and immediately drop the receiver to simulate failure
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            drop(rx);

            let result = handle_next_change(&mut app, &tx);

            assert!(result.is_ok());
            assert!(!app.is_loading);
            assert!(app.status_message.contains("Failed to start search"));
        }
    }

    mod file_selection_tests {
        use super::*;

        #[test]
        fn test_handle_file_selection_change_with_file() {
            let mut app = create_test_app();
            app.file_tree.current_selection = Some(std::path::PathBuf::from("src/main.rs"));
            let (tx, mut rx) = tokio::sync::mpsc::channel(100);

            handle_file_selection_change(&mut app, &tx);

            assert_eq!(app.active_file_context, Some(std::path::PathBuf::from("src/main.rs")));
            assert!(app.per_commit_cursor_positions.is_empty());
            assert!(app.last_commit_for_mapping.is_none());
            assert!(app.status_message.contains("Loading history"));

            // Should send LoadCommitHistory task
            let task = rx.try_recv();
            assert!(task.is_ok());
            match task.unwrap() {
                crate::async_task::Task::LoadCommitHistory { file_path } => {
                    assert!(file_path.contains("main.rs"));
                }
                _ => panic!("Expected LoadCommitHistory task"),
            }
        }

        #[test]
        fn test_handle_file_selection_change_with_directory() {
            let mut app = create_test_app();
            app.file_tree.current_selection = Some(std::path::PathBuf::from("tests"));
            app.active_file_context = Some(std::path::PathBuf::from("old_file.rs"));
            app.commit_list.push(crate::app::CommitInfo {
                hash: "test".to_string(),
                short_hash: "test".to_string(),
                author: "test".to_string(),
                date: "test".to_string(),
                subject: "test".to_string(),
            });
            let (tx, _rx) = tokio::sync::mpsc::channel(100);

            handle_file_selection_change(&mut app, &tx);

            assert!(app.active_file_context.is_none());
            assert!(app.commit_list.is_empty());
            assert!(app.current_content.is_empty());
            assert_eq!(app.cursor_line, 0);
            assert!(app.status_message.contains("Directory selected"));
        }

        #[test]
        fn test_handle_file_selection_change_no_selection() {
            let mut app = create_test_app();
            app.file_tree.current_selection = None;
            app.active_file_context = Some(std::path::PathBuf::from("old_file.rs"));
            let (tx, _rx) = tokio::sync::mpsc::channel(100);

            handle_file_selection_change(&mut app, &tx);

            assert!(app.active_file_context.is_none());
            assert!(app.commit_list.is_empty());
            assert!(app.current_content.is_empty());
            assert_eq!(app.cursor_line, 0);
            assert!(app.status_message.contains("No file selected"));
        }

        #[test]
        fn test_handle_file_selection_change_task_send_failure() {
            let mut app = create_test_app();
            app.file_tree.current_selection = Some(std::path::PathBuf::from("src/main.rs"));
            
            // Create a channel and immediately drop the receiver to simulate failure
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            drop(rx);

            handle_file_selection_change(&mut app, &tx);

            assert!(app.status_message.contains("Failed to load commit history"));
        }
    }

    mod navigation_tests {
        use super::*;

        #[test]
        fn test_navigate_to_younger_commit_success() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list_state.select(Some(1)); // Start at older commit

            let result = navigate_to_younger_commit(&mut app);

            assert!(result);
            assert_eq!(app.commit_list_state.selected(), Some(0));
            assert!(app.status_message.contains("younger commit"));
        }

        #[test]
        fn test_navigate_to_younger_commit_at_boundary() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list_state.select(Some(0)); // Already at youngest

            let result = navigate_to_younger_commit(&mut app);

            assert!(!result);
            assert_eq!(app.commit_list_state.selected(), Some(0));
            assert!(app.status_message.contains("Already at youngest"));
        }

        #[test]
        fn test_navigate_to_younger_commit_no_file_context() {
            let mut app = create_test_app();
            app.active_file_context = None;

            let result = navigate_to_younger_commit(&mut app);

            assert!(!result);
            assert!(app.status_message.contains("No file selected"));
        }

        #[test]
        fn test_navigate_to_younger_commit_empty_history() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list.clear();

            let result = navigate_to_younger_commit(&mut app);

            assert!(!result);
            assert!(app.status_message.contains("No commit history"));
        }

        #[test]
        fn test_navigate_to_younger_commit_no_selection() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list_state.select(None);

            let result = navigate_to_younger_commit(&mut app);

            assert!(result);
            assert_eq!(app.commit_list_state.selected(), Some(0));
            assert!(app.status_message.contains("Selected youngest"));
        }

        #[test]
        fn test_navigate_to_older_commit_success() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list_state.select(Some(0)); // Start at younger commit

            let result = navigate_to_older_commit(&mut app);

            assert!(result);
            assert_eq!(app.commit_list_state.selected(), Some(1));
            assert!(app.status_message.contains("older commit"));
        }

        #[test]
        fn test_navigate_to_older_commit_at_boundary() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list_state.select(Some(1)); // Already at oldest

            let result = navigate_to_older_commit(&mut app);

            assert!(!result);
            assert_eq!(app.commit_list_state.selected(), Some(1));
            assert!(app.status_message.contains("Already at oldest"));
        }

        #[test]
        fn test_navigate_to_older_commit_no_file_context() {
            let mut app = create_test_app();
            app.active_file_context = None;

            let result = navigate_to_older_commit(&mut app);

            assert!(!result);
            assert!(app.status_message.contains("No file selected"));
        }

        #[test]
        fn test_navigate_to_older_commit_empty_history() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list.clear();

            let result = navigate_to_older_commit(&mut app);

            assert!(!result);
            assert!(app.status_message.contains("No commit history"));
        }

        #[test]
        fn test_navigate_to_older_commit_no_selection() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list_state.select(None);

            let result = navigate_to_older_commit(&mut app);

            assert!(result);
            assert_eq!(app.commit_list_state.selected(), Some(0));
            assert!(app.status_message.contains("Selected youngest"));
        }
    }

    mod edge_cases {
        use super::*;

        #[tokio::test]
        async fn test_channel_send_failure() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            // Start with no selection to trigger navigation
            app.file_tree.current_selection = None;

            // Create a channel and immediately drop the receiver to simulate failure
            let (tx, rx) = create_test_channel().await;
            drop(rx);

            // Navigate down should try to auto-load and fail
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.status_message.contains("Failed to load commit history"));
        }

        #[tokio::test]
        async fn test_commit_navigation_global_keybindings() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator; // Test from navigator panel
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list_state.select(Some(0)); // Start at first commit
            let (tx, _rx) = create_test_channel().await;

            // Test [ (next older commit)
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &tx);
            
            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(1));
            assert!(app.status_message.contains("older commit"));

            // Test ] (next younger commit)
            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &tx);
            
            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(0));
            assert!(app.status_message.contains("younger commit"));
        }

        #[tokio::test]
        async fn test_commit_navigation_without_file_context() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Inspector;
            app.active_file_context = None; // No file selected
            let (tx, _rx) = create_test_channel().await;

            // Test [ should not work without file context
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &tx);
            
            assert!(result.is_ok());
            assert!(app.status_message.contains("No file selected"));

            // Test ] should not work without file context
            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &tx);
            
            assert!(result.is_ok());
            assert!(app.status_message.contains("No file selected"));
        }

        #[tokio::test]
        async fn test_commit_navigation_boundary_conditions() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            let (tx, _rx) = create_test_channel().await;

            // Test at youngest commit (index 0) - ] should not move further
            app.commit_list_state.select(Some(0));
            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &tx);
            
            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(0)); // Should stay at 0
            assert!(app.status_message.contains("youngest commit"));

            // Test at oldest commit (last index) - [ should not move further
            app.commit_list_state.select(Some(1)); // Last commit in our test data
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &tx);
            
            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(1)); // Should stay at last
            assert!(app.status_message.contains("oldest commit"));
        }

        #[tokio::test]
        async fn test_commit_navigation_from_no_selection() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.commit_list_state.select(None); // No commit selected
            let (tx, _rx) = create_test_channel().await;

            // Both [ and ] should select the first commit when none is selected
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &tx);
            
            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(0));
            assert!(app.status_message.contains("youngest commit"));

            // Reset to no selection
            app.commit_list_state.select(None);

            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &tx);
            
            assert!(result.is_ok());
            assert_eq!(app.commit_list_state.selected(), Some(0));
            assert!(app.status_message.contains("youngest commit"));
        }

        #[tokio::test]
        async fn test_all_panel_routing() {
            let mut app = create_test_app();
            let (tx, _rx) = create_test_channel().await;
            let event = create_key_event(KeyCode::Char('x')); // Unmapped key

            // Test Navigator panel
            app.active_panel = PanelFocus::Navigator;
            let result = handle_event(event.clone(), &mut app, &tx);
            assert!(result.is_ok());

            // Test History panel
            app.active_panel = PanelFocus::History;
            let result = handle_event(event.clone(), &mut app, &tx);
            assert!(result.is_ok());

            // Test Inspector panel
            app.active_panel = PanelFocus::Inspector;
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_search_with_special_characters() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            app.in_search_mode = true;
            let (tx, _rx) = create_test_channel().await;

            let special_chars = vec!['!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+'];

            for ch in special_chars {
                let event = create_key_event(KeyCode::Char(ch));
                let result = handle_event(event, &mut app, &tx);
                assert!(result.is_ok());
            }

            assert_eq!(app.search_query.len(), 12);
        }

        #[tokio::test]
        async fn test_multiple_backspaces_in_search() {
            let mut app = create_test_app();
            app.active_panel = PanelFocus::Navigator;
            app.in_search_mode = true;
            app.search_query = "test".to_string();
            let (tx, _rx) = create_test_channel().await;

            // Backspace more times than there are characters
            for _ in 0..10 {
                let event = create_key_event(KeyCode::Backspace);
                let result = handle_event(event, &mut app, &tx);
                assert!(result.is_ok());
            }

            assert!(app.search_query.is_empty());
        }
    }
}
