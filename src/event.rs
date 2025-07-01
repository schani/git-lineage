use crossterm::event::{Event, KeyCode, KeyModifiers};
use log::{debug, info, warn};
use tokio::sync::mpsc;

use crate::app::{App, PanelFocus};
use crate::async_task::Task;
use crate::tree::FileTree;

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
                    // Don't quit if in search mode - let panel handlers deal with it
                    if !app.navigator.file_tree_state.in_search_mode {
                        app.should_quit = true;
                        return Ok(());
                    }
                }
                KeyCode::Esc => {
                    // Don't quit if in search mode - let panel handlers deal with it
                    if !app.navigator.file_tree_state.in_search_mode {
                        app.should_quit = true;
                        return Ok(());
                    }
                }
                KeyCode::Tab => {
                    // Don't switch panels if in search mode
                    if !app.navigator.file_tree_state.in_search_mode {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            app.previous_panel();
                        } else {
                            app.next_panel();
                        }
                        return Ok(());
                    }
                }
                KeyCode::Char('1') => {
                    // Don't switch panels if in search mode
                    if !app.navigator.file_tree_state.in_search_mode {
                        app.ui.active_panel = PanelFocus::Navigator;
                        return Ok(());
                    }
                }
                KeyCode::Char('2') => {
                    // Don't switch panels if in search mode
                    if !app.navigator.file_tree_state.in_search_mode {
                        app.ui.active_panel = PanelFocus::History;
                        return Ok(());
                    }
                }
                KeyCode::Char('3') => {
                    // Don't switch panels if in search mode
                    if !app.navigator.file_tree_state.in_search_mode {
                        app.ui.active_panel = PanelFocus::Inspector;
                        return Ok(());
                    }
                }
                KeyCode::Char('[') => {
                    // Don't navigate commits if in search mode
                    if !app.navigator.file_tree_state.in_search_mode {
                        if navigate_to_older_commit(app) {
                            return Ok(());
                        }
                    }
                }
                KeyCode::Char(']') => {
                    // Don't navigate commits if in search mode
                    if !app.navigator.file_tree_state.in_search_mode {
                        if navigate_to_younger_commit(app) {
                            return Ok(());
                        }
                    }
                }
                KeyCode::Char('l') => {
                    // Ctrl+L to force screen redraw
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.ui.force_redraw = true;
                        app.ui.status_message = "Screen refreshed".to_string();
                        return Ok(());
                    }
                }
                _ => {}
            }

            // Panel-specific keybindings
            match app.ui.active_panel {
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
    if app.navigator.file_tree_state.in_search_mode {
        match key {
            KeyCode::Char(c) => {
                app.navigator.file_tree_state.search_query.push(c);
                app.navigator.file_tree_state.set_search_query(app.navigator.file_tree_state.search_query.clone());
                log::debug!("⌨️  Added '{}' to search query, now: '{}'", c, app.navigator.file_tree_state.search_query);
                // Filtering happens automatically in UI rendering
            }
            KeyCode::Backspace => {
                app.navigator.file_tree_state.search_query.pop();
                app.navigator.file_tree_state.set_search_query(app.navigator.file_tree_state.search_query.clone());
            }
            KeyCode::Enter | KeyCode::Esc => {
                if key == KeyCode::Esc {
                    app.navigator.file_tree_state.clear_search();
                } else {
                    app.navigator.file_tree_state.exit_search_mode();
                }
                // Reset cursor to top when exiting search mode
                app.navigator.cursor_position = 0;
                app.navigator.scroll_offset = 0;
                // Update commit history and inspector panels if a file is selected
                handle_file_selection_change(app, task_sender);
            }
            _ => {}
        }
        return Ok(());
    }

    match key {
        KeyCode::Up => {
            if app.navigate_tree_up() {
                app.ui.status_message = "Navigated up".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Down => {
            if app.navigate_tree_down() {
                app.ui.status_message = "Navigated down".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Right => {
            if app.expand_selected_node() {
                app.ui.status_message = "Expanded directory".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Left => {
            if app.collapse_selected_node() {
                app.ui.status_message = "Collapsed directory".to_string();
                handle_file_selection_change(app, task_sender);
            }
        }
        KeyCode::Enter => {
            if let Some(selected_path) = app.get_selected_file_path() {
                let is_dir = app
                    .navigator
                    .file_tree_state
                    .find_node_in_tree(app.navigator.file_tree_state.display_tree(), &selected_path)
                    .map(|node| node.is_dir)
                    .unwrap_or(false);

                if is_dir {
                    let was_expanded = app
                        .navigator
                        .file_tree_state
                        .find_node_in_tree(app.navigator.file_tree_state.display_tree(), &selected_path)
                        .map(|node| node.is_expanded)
                        .unwrap_or(false);

                    app.toggle_selected_node();
                    app.ui.status_message = if was_expanded {
                        "Collapsed directory".to_string()
                    } else {
                        "Expanded directory".to_string()
                    };
                    handle_file_selection_change(app, task_sender);
                } else {
                    // For files, Enter switches to the Inspector panel to view content
                    app.ui.active_panel = crate::app::PanelFocus::Inspector;
                    app.ui.status_message =
                        format!("Viewing content for {}", selected_path.display());
                }
            }
        }
        KeyCode::Char('/') => {
            app.navigator.file_tree_state.enter_search_mode();
            // Reset cursor to top when entering search mode
            app.navigator.cursor_position = 0;
            app.navigator.scroll_offset = 0;
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

fn handle_inspector_event(
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

// Supporting data structures for function decomposition
struct CommitData {
    hash: String,
    short_hash: String,
    author: String,
    date: String,
    subject: String,
}

struct CursorState {
    old_cursor_line: usize,
    old_commit_hash: Option<String>,
    old_scroll_vertical: u16,
    old_cursor_viewport_offset: usize,
}

pub fn update_code_inspector_for_commit(app: &mut App) {
    let Some(selected_index) = app.history.list_state.selected() else {
        return;
    };

    if selected_index >= app.history.commit_list.len() {
        return;
    }

    let commit_data = extract_commit_data(app, selected_index);
    let cursor_state = save_cursor_state(app);
    let file_path = app.active_file_context.clone(); // Clone to avoid borrow issues

    app.history.selected_commit_hash = Some(commit_data.hash.clone());

    if let Some(file_path) = file_path {
        handle_file_content_loading(app, &commit_data, &cursor_state, &file_path);
    } else {
        handle_no_file_context(app, &commit_data);
    }
}

fn extract_commit_data(app: &App, index: usize) -> CommitData {
    let commit = &app.history.commit_list[index];
    CommitData {
        hash: commit.hash.clone(),
        short_hash: commit.short_hash.clone(),
        author: commit.author.clone(),
        date: commit.date.clone(),
        subject: commit.subject.clone(),
    }
}

fn save_cursor_state(app: &App) -> CursorState {
    CursorState {
        old_cursor_line: app.inspector.cursor_line,
        old_commit_hash: app.history.selected_commit_hash.clone(),
        old_scroll_vertical: app.inspector.scroll_vertical,
        old_cursor_viewport_offset: app
            .inspector
            .cursor_line
            .saturating_sub(app.inspector.scroll_vertical as usize),
    }
}

fn handle_file_content_loading(
    app: &mut App,
    commit_data: &CommitData,
    cursor_state: &CursorState,
    file_path: &std::path::PathBuf,
) {
    setup_loading_state(app, commit_data, file_path);
    setup_line_mapping(app, commit_data, cursor_state, file_path);

    match load_file_content(app, commit_data, file_path) {
        Ok(()) => {
            handle_successful_content_load(app, commit_data, cursor_state, file_path);
        }
        Err(e) => {
            handle_content_load_error(app, e);
        }
    }

    app.ui.is_loading = false;
}

fn setup_loading_state(app: &mut App, commit_data: &CommitData, file_path: &std::path::PathBuf) {
    app.ui.is_loading = true;
    app.ui.status_message = format!(
        "Loading {} at commit {}...",
        file_path.file_name().unwrap_or_default().to_string_lossy(),
        &commit_data.short_hash
    );
}

fn setup_line_mapping(
    app: &mut App,
    commit_data: &CommitData,
    cursor_state: &CursorState,
    file_path: &std::path::PathBuf,
) {
    if let Some(ref previous_commit) = cursor_state.old_commit_hash {
        if previous_commit != &commit_data.hash {
            app.save_cursor_position(previous_commit, file_path);
            app.last_commit_for_mapping = Some(previous_commit.clone());
        }
    }
}

fn load_file_content(
    app: &mut App,
    commit_data: &CommitData,
    file_path: &std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = crate::git_utils::get_file_content_at_commit(
        &app.repo,
        &file_path.to_string_lossy(),
        &commit_data.hash,
    )?;

    app.inspector.current_content = content;
    app.inspector.scroll_horizontal = 0;
    Ok(())
}

fn handle_successful_content_load(
    app: &mut App,
    commit_data: &CommitData,
    cursor_state: &CursorState,
    file_path: &std::path::PathBuf,
) {
    restore_cursor_position(app, cursor_state, commit_data, file_path);
    restore_viewport_position(app, cursor_state);
    app.ensure_inspector_cursor_visible();
    update_success_status_message(app, commit_data, file_path);
}

fn restore_cursor_position(
    app: &mut App,
    cursor_state: &CursorState,
    commit_data: &CommitData,
    file_path: &std::path::PathBuf,
) {
    info!(
        "restore_cursor_position: Setting cursor to line {} before smart positioning",
        cursor_state.old_cursor_line
    );
    app.inspector.cursor_line = cursor_state.old_cursor_line;
    let positioning_message = app.apply_smart_cursor_positioning(&commit_data.hash, file_path);
    debug!(
        "restore_cursor_position: Smart positioning result: {}",
        positioning_message
    );
}

fn restore_viewport_position(app: &mut App, cursor_state: &CursorState) {
    let new_cursor_line = app.inspector.cursor_line;
    let desired_scroll = new_cursor_line.saturating_sub(cursor_state.old_cursor_viewport_offset);
    app.inspector.scroll_vertical = desired_scroll as u16;
}

fn update_success_status_message(
    app: &mut App,
    commit_data: &CommitData,
    file_path: &std::path::PathBuf,
) {
    info!(
        "update_success_status_message: Applying smart cursor positioning for commit {}",
        &commit_data.hash
    );
    let positioning_message = app.apply_smart_cursor_positioning(&commit_data.hash, file_path);
    debug!(
        "update_success_status_message: Positioning result: {}",
        positioning_message
    );
    let file_info = format!(
        "Loaded {} ({} lines) at commit {}",
        file_path.file_name().unwrap_or_default().to_string_lossy(),
        app.inspector.current_content.len(),
        &commit_data.short_hash
    );

    app.ui.status_message = if positioning_message.contains("top of file")
        || positioning_message.contains("unchanged")
    {
        file_info
    } else {
        format!("{} • {}", file_info, positioning_message)
    };
}

fn handle_content_load_error(app: &mut App, error: Box<dyn std::error::Error>) {
    app.inspector.current_content = vec![
        "Error loading file content:".to_string(),
        format!("{}", error),
        "".to_string(),
        "This could happen if:".to_string(),
        "- The file didn't exist at this commit".to_string(),
        "- The commit hash is invalid".to_string(),
        "- There's a Git repository issue".to_string(),
    ];

    app.inspector.cursor_line = 0;
    app.last_commit_for_mapping = None;
    app.ui.status_message = format!("Failed to load content: {}", error);
}

fn handle_no_file_context(app: &mut App, commit_data: &CommitData) {
    app.inspector.current_content = vec![
        format!("Commit: {}", commit_data.hash),
        format!("Short: {}", commit_data.short_hash),
        format!("Author: {}", commit_data.author),
        format!("Date: {}", commit_data.date),
        format!("Subject: {}", commit_data.subject),
        "".to_string(),
        "Select a file to view its content at this commit.".to_string(),
    ];

    app.inspector.cursor_line = 0;
    app.last_commit_for_mapping = None;
    app.ui.status_message = format!("Viewing commit: {}", commit_data.short_hash);
}

fn handle_previous_change(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Look up the current line in blame info and jump to that commit
    app.ui.status_message = format!("Previous change for line {}", app.inspector.cursor_line + 1);
    Ok(())
}

fn handle_next_change(
    app: &mut App,
    async_sender: &mpsc::Sender<Task>,
) -> Result<(), Box<dyn std::error::Error>> {
    if let (Some(ref file_path), Some(ref commit_hash)) = (
        &app.get_selected_file_path(),
        &app.history.selected_commit_hash,
    ) {
        let task = Task::FindNextChange {
            file_path: file_path.to_string_lossy().to_string(),
            current_commit: commit_hash.clone(),
            line_number: app.inspector.cursor_line,
        };

        app.ui.is_loading = true;
        app.ui.status_message = "Searching for next change...".to_string();

        if let Err(e) = async_sender.try_send(task) {
            app.ui.is_loading = false;
            app.ui.status_message = format!("Failed to start search: {}", e);
        }
    } else {
        app.ui.status_message = "No file or commit selected".to_string();
    }

    Ok(())
}

fn handle_file_selection_change(app: &mut App, task_sender: &mpsc::Sender<Task>) {
    if let Some(selected_path) = app.get_selected_file_path() {
        let is_dir = app
            .navigator
            .file_tree_state
            .find_node_in_tree(app.navigator.file_tree_state.display_tree(), &selected_path)
            .map(|node| node.is_dir)
            .unwrap_or(false);

        if !is_dir {
            // It's a file - set as active context and implement progressive loading
            // Clear position tracking state when switching to a different file
            app.per_commit_cursor_positions.clear();
            app.last_commit_for_mapping = None;
            app.active_file_context = Some(selected_path.clone());

            let file_path = selected_path.to_string_lossy().to_string();
            
            // Reset history state for new file
            app.history.reset_for_new_file();
            
            // IMMEDIATE: Load file content at HEAD (synchronous, should be fast)
            load_file_content_at_head(app, &selected_path);
            
            // BACKGROUND: Start streaming history loading with cancellation token
            let cancellation_token = tokio_util::sync::CancellationToken::new();
            app.history.streaming_cancellation_token = Some(cancellation_token.clone());
            
            if let Err(e) = task_sender.try_send(crate::async_task::Task::LoadCommitHistoryStreaming {
                file_path: file_path.clone(),
                cancellation_token,
            }) {
                app.ui.status_message = format!("Failed to start history loading: {}", e);
            } else {
                app.history.is_loading_more = true;
                // Status message set by load_file_content_at_head will indicate content loaded + history loading
            }
        } else {
            // It's a directory - clear file context and content
            app.per_commit_cursor_positions.clear();
            app.last_commit_for_mapping = None;
            app.active_file_context = None;
            app.history.reset_for_new_file();
            app.inspector.current_content.clear();
            app.inspector.current_blame = None;
            app.inspector.cursor_line = 0;
            app.inspector.scroll_vertical = 0;
            app.ui.status_message = "Directory selected".to_string();
        }
    } else {
        // No selection - clear file context and content
        app.per_commit_cursor_positions.clear();
        app.last_commit_for_mapping = None;
        app.active_file_context = None;
        app.history.reset_for_new_file();
        app.inspector.current_content.clear();
        app.inspector.current_blame = None;
        app.inspector.cursor_line = 0;
        app.inspector.scroll_vertical = 0;
        app.ui.status_message = "No file selected".to_string();
    }
}

fn load_file_content_at_head(app: &mut App, file_path: &std::path::PathBuf) {
    let file_path_str = file_path.to_string_lossy();
    
    // Synchronous HEAD content loading - should be fast
    match crate::git_utils::get_file_content_at_head(&app.repo, &file_path_str) {
        Ok(content) => {
            app.inspector.current_content = content;
            app.inspector.cursor_line = 0;
            app.inspector.scroll_vertical = 0;
            app.inspector.scroll_horizontal = 0;
            
            let filename = file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            app.ui.status_message = format!("{} loaded (loading history...)", filename);
            
            debug!("✅ load_file_content_at_head: Successfully loaded {} lines for '{}'", 
                  app.inspector.current_content.len(), filename);
        }
        Err(e) => {
            app.inspector.current_content.clear();
            app.ui.status_message = format!("Failed to load file: {}", e);
            warn!("❌ load_file_content_at_head: Failed to load '{}': {}", file_path_str, e);
        }
    }
}

/// Navigate to the previous (younger) commit in the history
/// Returns true if navigation occurred, false if no file context or at boundary
fn navigate_to_younger_commit(app: &mut App) -> bool {
    // Only navigate if there's an active file context
    if app.active_file_context.is_none() {
        app.ui.status_message = "No file selected".to_string();
        return false;
    }

    if app.history.commit_list.is_empty() {
        app.ui.status_message = "No commit history available".to_string();
        return false;
    }

    let current_selection = app.history.list_state.selected();

    match current_selection {
        Some(index) if index > 0 => {
            // Move to previous commit (younger)
            app.history.list_state.select(Some(index - 1));
            update_code_inspector_for_commit(app);
            let commit = &app.history.commit_list[index - 1];
            app.ui.status_message = format!("Moved to younger commit: {}", commit.short_hash);
            true
        }
        Some(_) => {
            // Already at the youngest commit (index 0) or any other index
            app.ui.status_message = "Already at youngest commit".to_string();
            false
        }
        None => {
            // No commit selected, select the first (youngest) one
            app.history.list_state.select(Some(0));
            update_code_inspector_for_commit(app);
            let commit = &app.history.commit_list[0];
            app.ui.status_message = format!("Selected youngest commit: {}", commit.short_hash);
            true
        }
    }
}

/// Navigate to the next (older) commit in the history  
/// Returns true if navigation occurred, false if no file context or at boundary
fn navigate_to_older_commit(app: &mut App) -> bool {
    // Only navigate if there's an active file context
    if app.active_file_context.is_none() {
        app.ui.status_message = "No file selected".to_string();
        return false;
    }

    if app.history.commit_list.is_empty() {
        app.ui.status_message = "No commit history available".to_string();
        return false;
    }

    let current_selection = app.history.list_state.selected();
    let max_index = app.history.commit_list.len() - 1;

    match current_selection {
        Some(index) if index < max_index => {
            // Move to next commit (older)
            app.history.list_state.select(Some(index + 1));
            update_code_inspector_for_commit(app);
            let commit = &app.history.commit_list[index + 1];
            app.ui.status_message = format!("Moved to older commit: {}", commit.short_hash);
            true
        }
        Some(_) => {
            // Already at the oldest commit or at max_index
            app.ui.status_message = "Already at oldest commit".to_string();
            false
        }
        None => {
            // No commit selected, select the first (youngest) one
            app.history.list_state.select(Some(0));
            update_code_inspector_for_commit(app);
            let commit = &app.history.commit_list[0];
            app.ui.status_message = format!("Selected youngest commit: {}", commit.short_hash);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
    use tokio::sync::mpsc;
    use crate::git_utils;
    
    #[test]
    fn test_ctrl_l_force_redraw() {
        // Setup
        let repo = git_utils::open_repository(".").expect("Should open test repo");
        let mut app = App::new(repo);
        let (task_sender, _task_receiver) = mpsc::channel::<Task>(32);
        
        // Initially force_redraw should be false
        assert!(!app.ui.force_redraw);
        
        // Create Ctrl+L key event
        let key_event = KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        let event = Event::Key(key_event);
        
        // Handle the event
        handle_event(event, &mut app, &task_sender).expect("Should handle Ctrl+L");
        
        // Verify force_redraw is set and status message is updated
        assert!(app.ui.force_redraw, "force_redraw should be set to true");
        assert_eq!(app.ui.status_message, "Screen refreshed");
    }

    // Test utilities
    use std::path::PathBuf;
    
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
        let mut tree = FileTree::new();
        tree.root.push(src_main);
        tree.root.push(src_lib);
        tree.root.push(tests_dir);
        app.navigator.file_tree_state.set_tree_data(tree, String::new(), false);

        // Add some commits for testing
        app.history.commit_list = vec![
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
            app.ui.active_panel = PanelFocus::Navigator;
            let (tx, _rx) = create_test_channel().await;
            let event = create_key_event(KeyCode::Tab);

            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, PanelFocus::History);
        }

        #[tokio::test]
        async fn test_shift_tab_previous_panel() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::History;
            let (tx, _rx) = create_test_channel().await;
            let event = create_key_event_with_modifiers(KeyCode::Tab, KeyModifiers::SHIFT);

            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, PanelFocus::Navigator);
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
            app.ui.active_panel = PanelFocus::Navigator;
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
            app.ui.active_panel = PanelFocus::Navigator;
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
            app.ui.active_panel = PanelFocus::Navigator;
            app.navigator.file_tree_state.in_search_mode = false;
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('/'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.navigator.file_tree_state.in_search_mode);
            assert!(app.navigator.file_tree_state.search_query.is_empty());
        }

        #[tokio::test]
        async fn test_search_input_handling() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            app.navigator.file_tree_state.in_search_mode = true;
            let (tx, _rx) = create_test_channel().await;

            // Test character input
            let event = create_key_event(KeyCode::Char('t'));
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.navigator.file_tree_state.search_query, "t");

            // Test more characters
            let event = create_key_event(KeyCode::Char('e'));
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.navigator.file_tree_state.search_query, "te");

            // Test backspace
            let event = create_key_event(KeyCode::Backspace);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.navigator.file_tree_state.search_query, "t");
        }

        #[tokio::test]
        async fn test_search_escape() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            app.navigator.file_tree_state.in_search_mode = true;
            app.navigator.file_tree_state.search_query = "test query".to_string();
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Esc);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(!app.navigator.file_tree_state.in_search_mode);
            assert!(app.navigator.file_tree_state.search_query.is_empty());
        }

        #[tokio::test]
        async fn test_search_enter() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            app.navigator.file_tree_state.in_search_mode = true;
            app.navigator.file_tree_state.search_query = "test query".to_string();
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Enter);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(!app.navigator.file_tree_state.in_search_mode);
            assert_eq!(app.navigator.file_tree_state.search_query, "test query"); // Should preserve query on Enter
        }

        #[tokio::test]
        async fn test_file_selection_triggers_history_load() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            // Start with no selection
            app.navigator.file_tree_state.current_selection = None;
            let (tx, mut rx) = create_test_channel().await;

            // Navigate down should trigger automatic loading for the first file
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());

            // Check that a task was sent due to navigation triggering auto-load
            let task = rx.try_recv();
            assert!(task.is_ok());
            match task.unwrap() {
                Task::LoadCommitHistoryStreaming { file_path, .. } => {
                    // The first navigation should select the first file
                    assert!(file_path.contains("lib.rs") || file_path.contains("main.rs"));
                }
                _ => panic!("Expected LoadCommitHistoryStreaming task"),
            }
        }

        #[tokio::test]
        async fn test_enter_on_file_switches_to_inspector() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            app.navigator.file_tree_state.current_selection = Some(PathBuf::from("src/main.rs"));
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Enter);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, PanelFocus::Inspector);
            assert!(app.ui.status_message.contains("Viewing content"));
        }

        #[tokio::test]
        async fn test_navigation_updates_visual_cursor() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            let (tx, _rx) = create_test_channel().await;

            // Start with cursor at top
            app.navigator.cursor_position = 0;
            app.navigator.scroll_offset = 0;

            // Navigate down - should move cursor and update selection
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());

            // Verify that both the file tree state and cursor position are updated
            assert!(app.navigator.file_tree_state.current_selection.is_some());
            let first_cursor_pos = app.navigator.cursor_position;

            // Navigate down again
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());

            // Verify cursor position moved to next item
            let second_cursor_pos = app.navigator.cursor_position;
            assert!(second_cursor_pos > first_cursor_pos || 
                   (second_cursor_pos == first_cursor_pos && app.navigator.scroll_offset > 0), 
                "Cursor should move down or scroll should occur: cursor {} -> {} (scroll: {})", 
                first_cursor_pos, second_cursor_pos, app.navigator.scroll_offset);

            // Navigate up
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());

            // Verify cursor position moved back (accounting for scrolling)
            let final_cursor_pos = app.navigator.cursor_position;
            let final_scroll = app.navigator.scroll_offset;
            
            // The actual visual position is cursor_position + scroll_offset
            let first_visual_pos = first_cursor_pos + 0; // Initial scroll was 0
            let final_visual_pos = final_cursor_pos + final_scroll;
            
            assert_eq!(final_visual_pos, first_visual_pos,
                "Visual position should return to original: {} -> {} (cursor: {}, scroll: {})", 
                first_visual_pos, final_visual_pos, final_cursor_pos, final_scroll);
        }
    }

    mod history_events {
        use super::*;

        #[tokio::test]
        async fn test_commit_navigation_up() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::History;
            app.history.list_state.select(Some(1)); // Start at second commit
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(0));
        }

        #[tokio::test]
        async fn test_commit_navigation_down() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::History;
            app.history.list_state.select(Some(0)); // Start at first commit
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(1));
        }

        #[tokio::test]
        async fn test_commit_navigation_bounds() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::History;
            app.history.list_state.select(Some(0)); // At first commit
            let (tx, _rx) = create_test_channel().await;

            // Try to go up from first commit
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(0)); // Should stay at first

            // Go to last commit
            app.history.list_state.select(Some(1));

            // Try to go down from last commit
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(1)); // Should stay at last
        }

        #[tokio::test]
        async fn test_commit_selection_with_enter() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::History;
            app.history.list_state.select(Some(0));
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Enter);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            // Verify that update_code_inspector_for_commit was called
            assert!(app.history.selected_commit_hash.is_some());
            assert!(!app.inspector.current_content.is_empty());
        }

        #[tokio::test]
        async fn test_empty_history_handling() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::History;
            app.history.commit_list.clear(); // Empty commit list
            app.history.list_state.select(None);
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
            app.ui.active_panel = PanelFocus::Inspector;
            app.inspector.current_content = vec![
                "line1".to_string(),
                "line2".to_string(),
                "line3".to_string(),
            ];
            app.inspector.cursor_line = 1;
            let (tx, _rx) = create_test_channel().await;

            // Test up movement
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 0);

            // Test down movement
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 1);
        }

        #[tokio::test]
        async fn test_page_up_down_movement() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Inspector;
            app.inspector.current_content = (0..50).map(|i| format!("line{}", i)).collect();
            app.inspector.cursor_line = 20;
            let (tx, _rx) = create_test_channel().await;

            // Test page up
            let event = create_key_event(KeyCode::PageUp);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 10);

            // Test page down
            let event = create_key_event(KeyCode::PageDown);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 20);
        }

        #[tokio::test]
        async fn test_home_end_navigation() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Inspector;
            app.inspector.current_content = (0..10).map(|i| format!("line{}", i)).collect();
            app.inspector.cursor_line = 5;
            let (tx, _rx) = create_test_channel().await;

            // Test Home key
            let event = create_key_event(KeyCode::Home);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 0);
            assert_eq!(app.inspector.scroll_vertical, 0);

            // Test End key
            let event = create_key_event(KeyCode::End);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 9);
        }

        #[tokio::test]
        async fn test_cursor_bounds_validation() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Inspector;
            app.inspector.current_content = vec!["line1".to_string(), "line2".to_string()];
            app.inspector.cursor_line = 0;
            let (tx, _rx) = create_test_channel().await;

            // Try to go up from first line
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 0); // Should stay at 0

            // Go to last line
            app.inspector.cursor_line = 1;

            // Try to go down from last line
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 1); // Should stay at last line
        }

        #[tokio::test]
        async fn test_diff_view_toggle() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Inspector;
            app.inspector.show_diff_view = false;
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('d'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.inspector.show_diff_view);
            assert!(app.ui.status_message.contains("diff view"));
        }

        #[tokio::test]
        async fn test_go_to_shortcuts() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Inspector;
            app.inspector.current_content = (0..10).map(|i| format!("line{}", i)).collect();
            app.inspector.cursor_line = 5;
            let (tx, _rx) = create_test_channel().await;

            // Test 'g' (go to top)
            let event = create_key_event(KeyCode::Char('g'));
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 0);

            // Test 'G' (go to bottom)
            let event = create_key_event(KeyCode::Char('G'));
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
            assert_eq!(app.inspector.cursor_line, 9);
        }

        #[tokio::test]
        async fn test_previous_change_navigation() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Inspector;
            app.inspector.cursor_line = 5;
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('p'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.ui.status_message.contains("Previous change"));
        }

        #[tokio::test]
        async fn test_next_change_with_valid_context() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Inspector;
            app.inspector.cursor_line = 5;
            app.navigator.file_tree_state.current_selection = Some(PathBuf::from("src/main.rs"));
            app.history.selected_commit_hash = Some("abc123".to_string());
            let (tx, mut rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('n'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.ui.is_loading);
            assert!(app.ui.status_message.contains("Searching"));

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
            app.ui.active_panel = PanelFocus::Inspector;
            app.navigator.file_tree_state.current_selection = None; // No file selected
            app.history.selected_commit_hash = None; // No commit selected
            let (tx, _rx) = create_test_channel().await;

            let event = create_key_event(KeyCode::Char('n'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(!app.ui.is_loading);
            assert!(app.ui.status_message.contains("No file or commit selected"));
        }
    }

    mod helper_functions {
        use super::*;

        #[test]
        fn test_update_code_inspector_for_commit() {
            let mut app = create_test_app();
            app.history.list_state.select(Some(0));

            update_code_inspector_for_commit(&mut app);

            assert_eq!(app.history.selected_commit_hash, Some("abc123".to_string()));
            assert!(app.ui.status_message.contains("abc123"));
            assert!(!app.inspector.current_content.is_empty());
        }

        #[test]
        fn test_update_code_inspector_invalid_selection() {
            let mut app = create_test_app();
            app.history.list_state.select(Some(999)); // Invalid index

            update_code_inspector_for_commit(&mut app);

            // Should not crash, but also should not update anything
        }

        #[test]
        fn test_update_code_inspector_no_selection() {
            let mut app = create_test_app();
            app.history.list_state.select(None); // No selection

            update_code_inspector_for_commit(&mut app);

            // Should not crash or change state
            assert!(app.history.selected_commit_hash.is_none());
        }

        #[test]
        fn test_update_code_inspector_with_file_context() {
            let mut app = create_test_app();
            app.history.list_state.select(Some(0));
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));

            update_code_inspector_for_commit(&mut app);

            assert_eq!(app.history.selected_commit_hash, Some("abc123".to_string()));
            assert!(!app.ui.is_loading); // Should complete loading
        }

        #[test]
        fn test_update_code_inspector_without_file_context() {
            let mut app = create_test_app();
            app.history.list_state.select(Some(0));
            app.active_file_context = None; // No file selected

            update_code_inspector_for_commit(&mut app);

            assert_eq!(app.history.selected_commit_hash, Some("abc123".to_string()));
            assert!(!app.inspector.current_content.is_empty());
            assert!(app.inspector.current_content[0].contains("Commit:"));
            assert!(app.inspector.current_content[1].contains("Short:"));
            assert!(app.ui.status_message.contains("Viewing commit"));
        }

        #[test]
        fn test_update_code_inspector_preserves_cursor_position() {
            let mut app = create_test_app();
            app.history.list_state.select(Some(0));
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.inspector.cursor_line = 5;
            app.inspector.scroll_vertical = 10;

            let _old_cursor = app.inspector.cursor_line;
            let _old_scroll = app.inspector.scroll_vertical;

            update_code_inspector_for_commit(&mut app);

            // The positioning system should be invoked
            assert_eq!(app.history.selected_commit_hash, Some("abc123".to_string()));
            // The exact cursor/scroll position depends on line mapping, but it shouldn't crash
        }

        #[test]
        fn test_update_code_inspector_clears_position_tracking_state() {
            let mut app = create_test_app();
            app.history.list_state.select(Some(0));
            app.active_file_context = None; // No file context
            app.last_commit_for_mapping = Some("old_commit".to_string());

            update_code_inspector_for_commit(&mut app);

            assert!(app.last_commit_for_mapping.is_none());
            assert_eq!(app.inspector.cursor_line, 0);
        }

        #[test]
        fn test_handle_previous_change() {
            let mut app = create_test_app();
            app.inspector.cursor_line = 10;

            let result = handle_previous_change(&mut app);

            assert!(result.is_ok());
            assert!(app
                .ui
                .status_message
                .contains("Previous change for line 11"));
        }

        #[test]
        fn test_handle_next_change_task_send_failure() {
            let mut app = create_test_app();
            app.navigator.file_tree_state.current_selection = Some(PathBuf::from("src/main.rs"));
            app.history.selected_commit_hash = Some("abc123".to_string());
            app.inspector.cursor_line = 5;

            // Create a channel and immediately drop the receiver to simulate failure
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            drop(rx);

            let result = handle_next_change(&mut app, &tx);

            assert!(result.is_ok());
            assert!(!app.ui.is_loading);
            assert!(app.ui.status_message.contains("Failed to start search"));
        }
    }

    mod file_selection_tests {
        use super::*;

        #[test]
        fn test_handle_file_selection_change_with_file() {
            let mut app = create_test_app();
            app.navigator.file_tree_state.current_selection =
                Some(std::path::PathBuf::from("src/main.rs"));
            let (tx, mut rx) = tokio::sync::mpsc::channel(100);

            handle_file_selection_change(&mut app, &tx);

            assert_eq!(
                app.active_file_context,
                Some(std::path::PathBuf::from("src/main.rs"))
            );
            assert!(app.per_commit_cursor_positions.is_empty());
            assert!(app.last_commit_for_mapping.is_none());
            assert!(app.ui.status_message.contains("loaded"));

            // Should send LoadCommitHistoryStreaming task
            let task = rx.try_recv();
            assert!(task.is_ok());
            match task.unwrap() {
                crate::async_task::Task::LoadCommitHistoryStreaming { file_path, .. } => {
                    assert!(file_path.contains("main.rs"));
                }
                _ => panic!("Expected LoadCommitHistoryStreaming task"),
            }
        }

        #[test]
        fn test_handle_file_selection_change_with_directory() {
            let mut app = create_test_app();
            app.navigator.file_tree_state.current_selection = Some(std::path::PathBuf::from("tests"));
            app.active_file_context = Some(std::path::PathBuf::from("old_file.rs"));
            app.history.commit_list.push(crate::app::CommitInfo {
                hash: "test".to_string(),
                short_hash: "test".to_string(),
                author: "test".to_string(),
                date: "test".to_string(),
                subject: "test".to_string(),
            });
            let (tx, _rx) = tokio::sync::mpsc::channel(100);

            handle_file_selection_change(&mut app, &tx);

            assert!(app.active_file_context.is_none());
            assert!(app.history.commit_list.is_empty());
            assert!(app.inspector.current_content.is_empty());
            assert_eq!(app.inspector.cursor_line, 0);
            assert!(app.ui.status_message.contains("Directory selected"));
        }

        #[test]
        fn test_handle_file_selection_change_no_selection() {
            let mut app = create_test_app();
            app.navigator.file_tree_state.current_selection = None;
            app.active_file_context = Some(std::path::PathBuf::from("old_file.rs"));
            let (tx, _rx) = tokio::sync::mpsc::channel(100);

            handle_file_selection_change(&mut app, &tx);

            assert!(app.active_file_context.is_none());
            assert!(app.history.commit_list.is_empty());
            assert!(app.inspector.current_content.is_empty());
            assert_eq!(app.inspector.cursor_line, 0);
            assert!(app.ui.status_message.contains("No file selected"));
        }

        #[test]
        fn test_handle_file_selection_change_task_send_failure() {
            let mut app = create_test_app();
            app.navigator.file_tree_state.current_selection =
                Some(std::path::PathBuf::from("src/main.rs"));

            // Create a channel and immediately drop the receiver to simulate failure
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            drop(rx);

            handle_file_selection_change(&mut app, &tx);

            assert!(app
                .ui
                .status_message
                .contains("Failed to start history loading"));
        }
    }

    mod navigation_tests {
        use super::*;

        #[test]
        fn test_navigate_to_younger_commit_success() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.list_state.select(Some(1)); // Start at older commit

            let result = navigate_to_younger_commit(&mut app);

            assert!(result);
            assert_eq!(app.history.list_state.selected(), Some(0));
            assert!(app.ui.status_message.contains("younger commit"));
        }

        #[test]
        fn test_navigate_to_younger_commit_at_boundary() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.list_state.select(Some(0)); // Already at youngest

            let result = navigate_to_younger_commit(&mut app);

            assert!(!result);
            assert_eq!(app.history.list_state.selected(), Some(0));
            assert!(app.ui.status_message.contains("Already at youngest"));
        }

        #[test]
        fn test_navigate_to_younger_commit_no_file_context() {
            let mut app = create_test_app();
            app.active_file_context = None;

            let result = navigate_to_younger_commit(&mut app);

            assert!(!result);
            assert!(app.ui.status_message.contains("No file selected"));
        }

        #[test]
        fn test_navigate_to_younger_commit_empty_history() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.commit_list.clear();

            let result = navigate_to_younger_commit(&mut app);

            assert!(!result);
            assert!(app.ui.status_message.contains("No commit history"));
        }

        #[test]
        fn test_navigate_to_younger_commit_no_selection() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.list_state.select(None);

            let result = navigate_to_younger_commit(&mut app);

            assert!(result);
            assert_eq!(app.history.list_state.selected(), Some(0));
            assert!(app.ui.status_message.contains("Selected youngest"));
        }

        #[test]
        fn test_navigate_to_older_commit_success() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.list_state.select(Some(0)); // Start at younger commit

            let result = navigate_to_older_commit(&mut app);

            assert!(result);
            assert_eq!(app.history.list_state.selected(), Some(1));
            assert!(app.ui.status_message.contains("older commit"));
        }

        #[test]
        fn test_navigate_to_older_commit_at_boundary() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.list_state.select(Some(1)); // Already at oldest

            let result = navigate_to_older_commit(&mut app);

            assert!(!result);
            assert_eq!(app.history.list_state.selected(), Some(1));
            assert!(app.ui.status_message.contains("Already at oldest"));
        }

        #[test]
        fn test_navigate_to_older_commit_no_file_context() {
            let mut app = create_test_app();
            app.active_file_context = None;

            let result = navigate_to_older_commit(&mut app);

            assert!(!result);
            assert!(app.ui.status_message.contains("No file selected"));
        }

        #[test]
        fn test_navigate_to_older_commit_empty_history() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.commit_list.clear();

            let result = navigate_to_older_commit(&mut app);

            assert!(!result);
            assert!(app.ui.status_message.contains("No commit history"));
        }

        #[test]
        fn test_navigate_to_older_commit_no_selection() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.list_state.select(None);

            let result = navigate_to_older_commit(&mut app);

            assert!(result);
            assert_eq!(app.history.list_state.selected(), Some(0));
            assert!(app.ui.status_message.contains("Selected youngest"));
        }
    }

    mod edge_cases {
        use super::*;

        #[tokio::test]
        async fn test_channel_send_failure() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            // Start with no selection to trigger navigation
            app.navigator.file_tree_state.current_selection = None;

            // Create a channel and immediately drop the receiver to simulate failure
            let (tx, rx) = create_test_channel().await;
            drop(rx);

            // Navigate down should try to auto-load and fail
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app
                .ui
                .status_message
                .contains("Failed to start history loading"));
        }

        #[tokio::test]
        async fn test_commit_navigation_global_keybindings() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator; // Test from navigator panel
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.list_state.select(Some(0)); // Start at first commit
            let (tx, _rx) = create_test_channel().await;

            // Test [ (next older commit)
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(1));
            assert!(app.ui.status_message.contains("older commit"));

            // Test ] (next younger commit)
            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(0));
            assert!(app.ui.status_message.contains("younger commit"));
        }

        #[tokio::test]
        async fn test_commit_navigation_without_file_context() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Inspector;
            app.active_file_context = None; // No file selected
            let (tx, _rx) = create_test_channel().await;

            // Test [ should not work without file context
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.ui.status_message.contains("No file selected"));

            // Test ] should not work without file context
            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert!(app.ui.status_message.contains("No file selected"));
        }

        #[tokio::test]
        async fn test_commit_navigation_boundary_conditions() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            let (tx, _rx) = create_test_channel().await;

            // Test at youngest commit (index 0) - ] should not move further
            app.history.list_state.select(Some(0));
            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(0)); // Should stay at 0
            assert!(app.ui.status_message.contains("youngest commit"));

            // Test at oldest commit (last index) - [ should not move further
            app.history.list_state.select(Some(1)); // Last commit in our test data
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(1)); // Should stay at last
            assert!(app.ui.status_message.contains("oldest commit"));
        }

        #[tokio::test]
        async fn test_commit_navigation_from_no_selection() {
            let mut app = create_test_app();
            app.active_file_context = Some(std::path::PathBuf::from("src/main.rs"));
            app.history.list_state.select(None); // No commit selected
            let (tx, _rx) = create_test_channel().await;

            // Both [ and ] should select the first commit when none is selected
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(0));
            assert!(app.ui.status_message.contains("youngest commit"));

            // Reset to no selection
            app.history.list_state.select(None);

            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &tx);

            assert!(result.is_ok());
            assert_eq!(app.history.list_state.selected(), Some(0));
            assert!(app.ui.status_message.contains("youngest commit"));
        }

        #[tokio::test]
        async fn test_all_panel_routing() {
            let mut app = create_test_app();
            let (tx, _rx) = create_test_channel().await;
            let event = create_key_event(KeyCode::Char('x')); // Unmapped key

            // Test Navigator panel
            app.ui.active_panel = PanelFocus::Navigator;
            let result = handle_event(event.clone(), &mut app, &tx);
            assert!(result.is_ok());

            // Test History panel
            app.ui.active_panel = PanelFocus::History;
            let result = handle_event(event.clone(), &mut app, &tx);
            assert!(result.is_ok());

            // Test Inspector panel
            app.ui.active_panel = PanelFocus::Inspector;
            let result = handle_event(event, &mut app, &tx);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_search_with_special_characters() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            app.navigator.file_tree_state.in_search_mode = true;
            let (tx, _rx) = create_test_channel().await;

            let special_chars = vec!['!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+'];

            for ch in special_chars {
                let event = create_key_event(KeyCode::Char(ch));
                let result = handle_event(event, &mut app, &tx);
                assert!(result.is_ok());
            }

            assert_eq!(app.navigator.file_tree_state.search_query.len(), 12);
        }

        #[tokio::test]
        async fn test_multiple_backspaces_in_search() {
            let mut app = create_test_app();
            app.ui.active_panel = PanelFocus::Navigator;
            app.navigator.file_tree_state.in_search_mode = true;
            app.navigator.file_tree_state.search_query = "test".to_string();
            let (tx, _rx) = create_test_channel().await;

            // Backspace more times than there are characters
            for _ in 0..10 {
                let event = create_key_event(KeyCode::Backspace);
                let result = handle_event(event, &mut app, &tx);
                assert!(result.is_ok());
            }

            assert!(app.navigator.file_tree_state.search_query.is_empty());
        }
    }
}
