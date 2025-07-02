use log::{debug, info};

use crate::app::App;

// Supporting data structures
pub struct CommitData {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

pub struct CursorState {
    pub old_cursor_line: usize,
    pub old_commit_hash: Option<String>,
    pub old_scroll_vertical: u16,
    pub old_cursor_viewport_offset: usize,
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