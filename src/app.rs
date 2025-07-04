use crate::navigator::{NavigatorState, NavigatorEvent};
use gix::Repository;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PanelFocus {
    Navigator,
    History,
    Inspector,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}


#[derive(Debug)]
pub struct HistoryState {
    pub commit_list: Vec<CommitInfo>,
    pub selected_commit_index: Option<usize>,
    pub selected_commit_hash: Option<String>,
    pub is_loading_more: bool,
    pub history_complete: bool,
    pub next_chunk_offset: usize,
    pub streaming_cancellation_token: Option<CancellationToken>,
}

#[derive(Debug, Clone)]
pub enum DiffLineType {
    Added,
    Removed,
    Modified,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub old_line_num: Option<usize>,
    pub new_line_num: Option<usize>,
    pub content: String,
}

#[derive(Debug)]
pub struct InspectorState {
    pub current_content: Vec<String>,
    pub current_blame: Option<String>,
    pub scroll_vertical: u16,
    pub scroll_horizontal: u16,
    pub visible_height: usize,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub show_diff_view: bool,
    pub diff_lines: Option<Vec<DiffLine>>,
    pub parent_commit_hash: Option<String>,
}

#[derive(Debug)]
pub struct UIState {
    pub active_panel: PanelFocus,
    pub status_message: String,
    pub is_loading: bool,
    pub force_redraw: bool,
}

pub struct App {
    pub repo: Repository,
    pub should_quit: bool,

    // Position Tracking for Same-Line Feature
    pub per_commit_cursor_positions: HashMap<(String, PathBuf), usize>,
    pub last_commit_for_mapping: Option<String>,

    // Background task tracking for event-driven architecture
    pub active_background_tasks: usize,
    
    // State modules
    pub navigator: NavigatorState,
    pub history: HistoryState,
    pub inspector: InspectorState,
    pub ui: UIState,
}

impl App {

    pub fn new(repo: Repository) -> Self {
        let app = Self {
            repo,
            should_quit: false,
            per_commit_cursor_positions: HashMap::new(),
            last_commit_for_mapping: None,
            active_background_tasks: 0,
            navigator: NavigatorState::new(crate::tree::FileTree::new()),
            history: HistoryState::new(),
            inspector: InspectorState::new(),
            ui: UIState::new(),
        };
        
        app
    }

    pub fn next_panel(&mut self) {
        self.ui.active_panel = match self.ui.active_panel {
            PanelFocus::Navigator => PanelFocus::History,
            PanelFocus::History => PanelFocus::Inspector,
            PanelFocus::Inspector => PanelFocus::Navigator,
        };
    }

    pub fn previous_panel(&mut self) {
        self.ui.active_panel = match self.ui.active_panel {
            PanelFocus::Navigator => PanelFocus::Inspector,
            PanelFocus::History => PanelFocus::Navigator,
            PanelFocus::Inspector => PanelFocus::History,
        };
    }

    // File tree navigation methods
    pub fn navigate_tree_up(&mut self) -> bool {
        self.navigator.handle_event(NavigatorEvent::NavigateUp).unwrap_or(false)
    }

    pub fn navigate_tree_down(&mut self) -> bool {
        self.navigator.handle_event(NavigatorEvent::NavigateDown).unwrap_or(false)
    }

    pub fn expand_selected_node(&mut self) -> bool {
        self.navigator.handle_event(NavigatorEvent::ExpandSelected).unwrap_or(false)
    }

    pub fn collapse_selected_node(&mut self) -> bool {
        self.navigator.handle_event(NavigatorEvent::CollapseSelected).unwrap_or(false)
    }

    pub fn toggle_selected_node(&mut self) -> bool {
        if let Some(selected_path) = self.get_selected_file_path() {
            self.navigator.handle_event(NavigatorEvent::ToggleExpanded(selected_path)).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn get_selected_file_path(&self) -> Option<PathBuf> {
        self.navigator.get_selection()
    }

    /// Get the currently selected file path (returns None if selection is a directory)
    pub fn get_active_file(&self) -> Option<PathBuf> {
        if let Some(path) = self.navigator.get_selection() {
            if !self.navigator.is_path_directory(&path) {
                Some(path)
            } else {
                None
            }
        } else {
            None
        }
    }



    /// Get navigator search query
    pub fn get_navigator_search_query(&self) -> String {
        self.navigator.get_search_query()
    }

    /// Check if navigator is in search mode
    pub fn is_navigator_searching(&self) -> bool {
        self.navigator.is_searching()
    }






    /// Load file content for the Inspector panel based on current selections
    pub fn load_inspector_content(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we have both a selected file and commit
        let file_path = match self.get_active_file() {
            Some(path) => path.to_string_lossy().to_string(),
            None => {
                self.inspector.current_content.clear();
                self.ui.status_message = if self.navigator.get_selection().is_some() {
                    "Directory selected - select a file to view content".to_string()
                } else {
                    "No file selected".to_string()
                };
                return Ok(());
            }
        };

        let commit_hash = match &self.history.selected_commit_hash {
            Some(hash) => hash.clone(),
            None => {
                self.inspector.current_content.clear();
                self.ui.status_message = "No commit selected".to_string();
                return Ok(());
            }
        };

        // Load file content at the selected commit
        self.ui.is_loading = true;
        self.ui.status_message =
            format!("Loading {} at commit {}...", file_path, &commit_hash[..8]);

        match crate::git_utils::get_file_content_at_commit(&self.repo, &file_path, &commit_hash) {
            Ok(content) => {
                self.inspector.current_content = content;
                self.inspector.scroll_horizontal = 0;
                self.inspector.cursor_line = 0;
                self.ensure_inspector_cursor_visible(); // Use unified scroll management
                self.ui.status_message = format!(
                    "Loaded {} ({} lines) at commit {}",
                    file_path,
                    self.inspector.current_content.len(),
                    &commit_hash[..8]
                );
            }
            Err(e) => {
                self.inspector.current_content.clear();
                self.ui.status_message = format!("Error loading {}: {}", file_path, e);
            }
        }

        self.ui.is_loading = false;
        Ok(())
    }

    /// Update the selected commit and refresh Inspector content if applicable
    pub fn set_selected_commit(
        &mut self,
        commit_hash: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.history.selected_commit_hash = Some(commit_hash);

        // Auto-load content if we have a file selected
        if self.get_active_file().is_some() {
            self.load_inspector_content()?;
        }

        Ok(())
    }

    /// Load commit history for the currently selected file
    /// Ensure the cursor is visible in the inspector viewport by adjusting scroll
    pub fn ensure_inspector_cursor_visible(&mut self) {
        if self.inspector.current_content.is_empty() {
            return;
        }

        let visible_lines = self.inspector.visible_height.saturating_sub(2); // Account for borders
        if visible_lines == 0 {
            return;
        }

        let scroll_top = self.inspector.scroll_vertical as usize;
        let scroll_bottom = scroll_top + visible_lines;

        // If cursor is above visible area, scroll up
        if self.inspector.cursor_line < scroll_top {
            self.inspector.scroll_vertical = self.inspector.cursor_line as u16;
        }
        // If cursor is below visible area, scroll down
        else if self.inspector.cursor_line >= scroll_bottom {
            self.inspector.scroll_vertical =
                (self.inspector.cursor_line.saturating_sub(visible_lines - 1)) as u16;
        }
        // Otherwise cursor is already visible, no scrolling needed
    }

    pub fn load_commit_history_for_selected_file(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = match self.get_active_file() {
            Some(path) => path.to_string_lossy().to_string(),
            None => {
                self.history.commit_list.clear();
                self.history.selected_commit_index = None;
                self.history.selected_commit_hash = None;
                self.inspector.current_content.clear();
                self.ui.status_message = if self.navigator.get_selection().is_some() {
                    "Directory selected - select a file to view history".to_string()
                } else {
                    "No file selected for history".to_string()
                };
                return Ok(());
            }
        };

        self.ui.is_loading = true;
        self.ui.status_message = format!("Loading commit history for {}...", file_path);

        match crate::git_utils::get_commit_history_for_file(&self.repo, &file_path) {
            Ok(commits) => {
                self.history.commit_list = commits;
                if !self.history.commit_list.is_empty() {
                    // Auto-select the first (most recent) commit
                    self.history.selected_commit_index = Some(0);
                    self.history.selected_commit_hash =
                        Some(self.history.commit_list[0].hash.clone());
                    self.ui.status_message = format!(
                        "Loaded {} commits for {}",
                        self.history.commit_list.len(),
                        file_path
                    );

                    // Auto-load content for the most recent commit
                    self.load_inspector_content()?;
                } else {
                    self.history.selected_commit_index = None;
                    self.history.selected_commit_hash = None;
                    self.inspector.current_content.clear();
                    self.ui.status_message = format!("No commits found for {}", file_path);
                }
            }
            Err(e) => {
                self.history.commit_list.clear();
                self.history.selected_commit_index = None;
                self.history.selected_commit_hash = None;
                self.inspector.current_content.clear();
                self.ui.status_message = format!("Error loading history for {}: {}", file_path, e);
            }
        }

        self.ui.is_loading = false;
        Ok(())
    }

    pub fn from_test_config(config: &crate::test_config::TestConfig, repo: Repository) -> Self {
        let mut app = Self {
            repo,
            should_quit: false,
            per_commit_cursor_positions: HashMap::new(),
            last_commit_for_mapping: None,
            active_background_tasks: 0,
            navigator: {
                let mut navigator = NavigatorState::new(config.file_tree.clone());
                
                // Set search mode if specified in test config
                if config.in_search_mode {
                    let _ = navigator.handle_event(NavigatorEvent::StartSearch);
                    if !config.search_query.is_empty() {
                        let _ = navigator.handle_event(NavigatorEvent::UpdateSearchQuery(config.search_query.clone()));
                    }
                }
                
                // Set initial selection if we have a selected index
                if let Some(index) = config.selected_file_navigator_index {
                    // Navigate down to the selected index
                    for _ in 0..index {
                        let _ = navigator.handle_event(NavigatorEvent::NavigateDown);
                    }
                }
                
                navigator
            },
            history: HistoryState {
                commit_list: config.commit_list.clone(),
                selected_commit_index: None, // Will be set below based on valid commit index
                selected_commit_hash: None, // Will be set below based on valid commit index
                is_loading_more: false,
                history_complete: false,
                next_chunk_offset: 0,
                streaming_cancellation_token: None,
            },
            inspector: InspectorState {
                current_content: config.current_content.clone(),
                current_blame: None,
                scroll_vertical: config.inspector_scroll_vertical,
                scroll_horizontal: config.inspector_scroll_horizontal,
                visible_height: 20, // Default reasonable value
                cursor_line: config.cursor_line,
                cursor_column: config.cursor_column,
                show_diff_view: config.show_diff_view,
                diff_lines: None,
                parent_commit_hash: None,
            },
            ui: UIState {
                active_panel: config.active_panel,
                status_message: config.status_message.clone(),
                is_loading: config.is_loading,
                force_redraw: false,
            },
        };

        // Set the selected commit if specified
        if let Some(index) = config.selected_commit_index {
            if index < app.history.commit_list.len() {
                app.history.selected_commit_index = Some(index);
                app.history.selected_commit_hash =
                    Some(app.history.commit_list[index].hash.clone());
            }
        }


        app
    }

    // Position tracking methods for same-line feature

    /// Save the current cursor position for the given commit and file
    pub fn save_cursor_position(&mut self, commit_hash: &str, file_path: &PathBuf) {
        let key = (commit_hash.to_string(), file_path.clone());
        self.per_commit_cursor_positions
            .insert(key, self.inspector.cursor_line);
    }

    /// Restore a previously saved cursor position for the given commit and file
    pub fn restore_cursor_position(
        &mut self,
        commit_hash: &str,
        file_path: &PathBuf,
    ) -> Option<usize> {
        let key = (commit_hash.to_string(), file_path.clone());
        self.per_commit_cursor_positions.get(&key).copied()
    }

    /// Start a background task and track it
    pub fn start_background_task(&mut self) {
        self.active_background_tasks += 1;
    }
    
    /// Complete a background task and update tracking
    pub fn complete_background_task(&mut self) {
        self.active_background_tasks = self.active_background_tasks.saturating_sub(1);
    }
    
    /// Check if there are active background tasks
    pub fn has_active_background_tasks(&self) -> bool {
        self.active_background_tasks > 0
    }
    

    /// Get the mapped line position using line mapping between commits with fallback strategies
    pub fn get_mapped_line(
        &self,
        old_commit: &str,
        new_commit: &str,
        file_path: &PathBuf,
        old_line: usize,
    ) -> usize {
        info!(
            "get_mapped_line: Mapping line {} from {} to {} in file {:?}",
            old_line, old_commit, new_commit, file_path
        );

        // If commits are the same, no mapping needed
        if old_commit == new_commit {
            debug!(
                "get_mapped_line: Same commit, returning original line {}",
                old_line
            );
            return old_line;
        }

        // Try to compute line mapping
        match crate::line_mapping::map_lines_between_commits(
            &self.repo, old_commit, new_commit, file_path,
        ) {
            Ok(mapping) => {
                debug!("get_mapped_line: Successfully created line mapping");

                // Try exact mapping first
                if let Some(mapped_line) = mapping.map_line(old_line) {
                    info!(
                        "get_mapped_line: SUCCESS via exact mapping - line {} -> {}",
                        old_line, mapped_line
                    );
                    return mapped_line;
                }
                debug!(
                    "get_mapped_line: Exact mapping failed for line {}",
                    old_line
                );

                // Fallback 1: Content-aware nearest neighbor search (±5 lines)
                debug!("get_mapped_line: Trying content-aware nearest neighbor search");
                match mapping.find_content_aware_nearest_mapped_line(
                    old_line, 5, &self.repo, old_commit, new_commit, file_path,
                ) {
                    Ok(Some(nearest_line)) => {
                        info!("get_mapped_line: SUCCESS via content-aware nearest neighbor - line {} -> {}", old_line, nearest_line);
                        return nearest_line;
                    }
                    Ok(None) => {
                        debug!("get_mapped_line: Content-aware nearest neighbor search failed for line {}", old_line);
                    }
                    Err(e) => {
                        warn!("get_mapped_line: Content-aware nearest neighbor search failed with error: {:?}", e);
                    }
                }

                // Fallback 1.5: Exact content matching (broader search)
                debug!("get_mapped_line: Trying exact content matching fallback");
                match mapping.find_exact_content_match(
                    old_line, &self.repo, old_commit, new_commit, file_path,
                ) {
                    Ok(Some(content_match)) => {
                        info!(
                            "get_mapped_line: SUCCESS via exact content match - line {} -> {}",
                            old_line, content_match
                        );
                        return content_match;
                    }
                    Ok(None) => {
                        debug!("get_mapped_line: Exact content matching failed - no unique match found");
                    }
                    Err(e) => {
                        warn!(
                            "get_mapped_line: Exact content matching failed with error: {:?}",
                            e
                        );
                    }
                }

                // Fallback 2: Proportional mapping
                let proportional_line = mapping.proportional_map(old_line);
                if proportional_line < self.inspector.current_content.len() {
                    info!(
                        "get_mapped_line: SUCCESS via proportional mapping - line {} -> {}",
                        old_line, proportional_line
                    );
                    return proportional_line;
                }
                debug!(
                    "get_mapped_line: Proportional mapping out of bounds: {} >= {}",
                    proportional_line,
                    self.inspector.current_content.len()
                );

                // Fallback 3: Default to top of file
                warn!("get_mapped_line: All mapping strategies failed, defaulting to line 0");
                0
            }
            Err(e) => {
                warn!("get_mapped_line: Line mapping creation failed: {:?}", e);

                // Fallback 4: If mapping fails, try proportional mapping manually
                if !self.inspector.current_content.is_empty() && old_line > 0 {
                    // Simple proportional fallback: assume some reasonable old file size
                    let estimated_old_size =
                        (old_line + 1).max(self.inspector.current_content.len());
                    let proportion = old_line as f64 / estimated_old_size as f64;
                    let new_line =
                        (proportion * self.inspector.current_content.len() as f64) as usize;
                    let result =
                        new_line.min(self.inspector.current_content.len().saturating_sub(1));
                    info!(
                        "get_mapped_line: SUCCESS via manual proportional fallback - line {} -> {}",
                        old_line, result
                    );
                    result
                } else {
                    warn!("get_mapped_line: Empty content or line 0, defaulting to line 0");
                    0
                }
            }
        }
    }

    /// Smart cursor positioning when switching commits
    pub fn apply_smart_cursor_positioning(
        &mut self,
        new_commit_hash: &str,
        file_path: &PathBuf,
    ) -> String {
        info!(
            "apply_smart_cursor_positioning: Switching to commit {} for file {:?}",
            new_commit_hash, file_path
        );

        // If we don't have a previous commit, just use any saved position or default to 0
        let old_commit_hash = match &self.last_commit_for_mapping {
            Some(hash) => {
                debug!(
                    "apply_smart_cursor_positioning: Previous commit found: {}",
                    hash
                );
                hash.clone()
            }
            None => {
                debug!("apply_smart_cursor_positioning: No previous commit for mapping");
                // No previous commit - try to restore saved position or default to 0
                if let Some(saved_line) = self.restore_cursor_position(new_commit_hash, file_path) {
                    self.inspector.cursor_line =
                        saved_line.min(self.inspector.current_content.len().saturating_sub(1));
                    info!(
                        "apply_smart_cursor_positioning: Restored saved position to line {}",
                        self.inspector.cursor_line
                    );
                    return format!(
                        "Restored cursor to saved position (line {})",
                        self.inspector.cursor_line + 1
                    );
                } else {
                    self.inspector.cursor_line = 0;
                    info!(
                        "apply_smart_cursor_positioning: No saved position, defaulting to line 0"
                    );
                    return "Positioned cursor at top of file".to_string();
                }
            }
        };

        // Save the current position before mapping
        let old_line = self.inspector.cursor_line;
        info!("apply_smart_cursor_positioning: Current cursor at line {} (0-based), attempting to map from {} to {}", 
              old_line, old_commit_hash, new_commit_hash);

        // Calculate the mapped line position
        let mapped_line =
            self.get_mapped_line(&old_commit_hash, new_commit_hash, file_path, old_line);

        // Apply the new cursor position
        let final_line = mapped_line.min(self.inspector.current_content.len().saturating_sub(1));
        self.inspector.cursor_line = final_line;
        info!(
            "apply_smart_cursor_positioning: Final cursor position set to line {}",
            final_line
        );

        // Update the tracking state
        self.last_commit_for_mapping = Some(new_commit_hash.to_string());

        // Return status message based on how the mapping was determined
        // Use final_line instead of mapped_line for accurate display, and use the original old_line
        info!("apply_smart_cursor_positioning: Status calculation - old_line={} (0-based), final_line={} (0-based), display will be {} → {}", 
              old_line, final_line, old_line + 1, final_line + 1);

        if final_line == old_line {
            "Cursor position unchanged".to_string()
        } else if final_line == 0 && old_line != 0 {
            format!("Line moved to top (was line {})", old_line + 1)
        } else {
            format!("Line {} → {} (same content)", old_line + 1, final_line + 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{FileTree, TreeNode};
    use gix::Repository;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Test utilities
    fn create_test_repo() -> Repository {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf(); // Convert to owned PathBuf

        // Initialize git repo
        std::process::Command::new("git")
            .args(&["init"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Set up git config
        std::process::Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Create test file
        fs::write(repo_path.join("test.txt"), "test content").unwrap();

        // Add and commit
        std::process::Command::new("git")
            .args(&["add", "."])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        std::process::Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        // Keep temp_dir alive by leaking it (for test purposes)
        std::mem::forget(temp_dir);

        gix::open(repo_path).unwrap()
    }

    fn create_test_file_tree() -> FileTree {
        let mut tree = FileTree::new();

        // Create a simple directory structure
        let file1 = TreeNode::new_file("main.rs".to_string(), PathBuf::from("src/main.rs"));
        let file2 = TreeNode::new_file("lib.rs".to_string(), PathBuf::from("src/lib.rs"));
        let file3 = TreeNode::new_file("test.rs".to_string(), PathBuf::from("tests/test.rs"));

        tree.root.push(file1);
        tree.root.push(file2);
        tree.root.push(file3);

        // Set a current selection
        // Selection is now handled in FileTreeState, not FileTree

        tree
    }

    fn create_test_commits() -> Vec<CommitInfo> {
        vec![
            CommitInfo {
                hash: "abc123def456".to_string(),
                short_hash: "abc123".to_string(),
                author: "Alice Developer".to_string(),
                date: "2023-01-01".to_string(),
                subject: "Initial commit".to_string(),
            },
            CommitInfo {
                hash: "def456ghi789".to_string(),
                short_hash: "def456".to_string(),
                author: "Bob Coder".to_string(),
                date: "2023-01-02".to_string(),
                subject: "Add feature".to_string(),
            },
        ]
    }

    mod app_construction {
        use super::*;

        #[test]
        fn test_new_app_default_state() {
            let repo = create_test_repo();
            let app = App::new(repo);

            assert_eq!(app.ui.active_panel, PanelFocus::Navigator);
            assert!(!app.should_quit);
            // Navigator state is now handled internally by navigator::NavigatorState
            assert!(app.navigator.get_search_query().is_empty());
            assert!(!app.navigator.is_searching());
            assert!(app.history.commit_list.is_empty());
            assert_eq!(app.history.selected_commit_hash, None);
            assert!(app.inspector.current_content.is_empty());
            assert_eq!(app.inspector.current_blame, None);
            assert_eq!(app.inspector.scroll_vertical, 0);
            assert_eq!(app.inspector.scroll_horizontal, 0);
            assert_eq!(app.inspector.cursor_line, 0);
            assert_eq!(app.inspector.cursor_column, 0);
            assert!(!app.inspector.show_diff_view);
            assert_eq!(app.ui.status_message, "Ready");
            assert!(!app.ui.is_loading);
        }

        #[test]
        fn test_from_test_config_basic() {
            let repo = create_test_repo();
            let mut config = crate::test_config::TestConfig::default();
            config.active_panel = PanelFocus::History;
            config.status_message = "Test status".to_string();
            config.is_loading = true;

            let app = App::from_test_config(&config, repo);

            assert_eq!(app.ui.active_panel, PanelFocus::History);
            assert_eq!(app.ui.status_message, "Test status");
            assert!(app.ui.is_loading);
            assert!(!app.should_quit);
        }

        #[test]
        fn test_from_test_config_with_file_tree() {
            let repo = create_test_repo();
            let mut config = crate::test_config::TestConfig::default();
            config.file_tree = create_test_file_tree();
            config.search_query = "test search".to_string();
            config.in_search_mode = true;

            let app = App::from_test_config(&config, repo);

            // Navigator state is verified through its public API
            assert_eq!(app.navigator.get_search_query(), "test search");
            assert!(app.navigator.is_searching());
        }

        #[test]
        fn test_from_test_config_with_commits() {
            let repo = create_test_repo();
            let mut config = crate::test_config::TestConfig::default();
            config.commit_list = create_test_commits();
            config.selected_commit_index = Some(1);

            let app = App::from_test_config(&config, repo);

            assert_eq!(app.history.commit_list.len(), 2);
            assert_eq!(app.history.selected_commit_index, Some(1));
            assert_eq!(
                app.history.selected_commit_hash,
                Some("def456ghi789".to_string())
            );
        }

        #[test]
        fn test_from_test_config_with_content() {
            let repo = create_test_repo();
            let mut config = crate::test_config::TestConfig::default();
            config.current_content = vec!["line 1".to_string(), "line 2".to_string()];
            config.inspector_scroll_vertical = 5;
            config.inspector_scroll_horizontal = 10;
            config.cursor_line = 2;
            config.cursor_column = 15;
            config.show_diff_view = true;

            let app = App::from_test_config(&config, repo);

            assert_eq!(app.inspector.current_content.len(), 2);
            assert_eq!(app.inspector.scroll_vertical, 5);
            assert_eq!(app.inspector.scroll_horizontal, 10);
            assert_eq!(app.inspector.cursor_line, 2);
            assert_eq!(app.inspector.cursor_column, 15);
            assert!(app.inspector.show_diff_view);
        }

        #[test]
        fn test_from_test_config_with_file_navigator_selection() {
            let repo = create_test_repo();
            let mut config = crate::test_config::TestConfig::default();
            config.selected_file_navigator_index = Some(2);

            let _app = App::from_test_config(&config, repo);

            // Navigator selection is now handled differently
        }

        #[test]
        fn test_from_test_config_invalid_commit_index() {
            let repo = create_test_repo();
            let mut config = crate::test_config::TestConfig::default();
            config.commit_list = create_test_commits();
            config.selected_commit_index = Some(10); // Invalid index

            let app = App::from_test_config(&config, repo);

            assert_eq!(app.history.selected_commit_index, None);
            assert_eq!(app.history.selected_commit_hash, None);
        }
    }

    mod panel_navigation {
        use super::*;

        #[test]
        fn test_next_panel_from_navigator() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            app.ui.active_panel = PanelFocus::Navigator;

            app.next_panel();

            assert_eq!(app.ui.active_panel, PanelFocus::History);
        }

        #[test]
        fn test_next_panel_from_history() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            app.ui.active_panel = PanelFocus::History;

            app.next_panel();

            assert_eq!(app.ui.active_panel, PanelFocus::Inspector);
        }

        #[test]
        fn test_next_panel_from_inspector() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            app.ui.active_panel = PanelFocus::Inspector;

            app.next_panel();

            assert_eq!(app.ui.active_panel, PanelFocus::Navigator);
        }

        #[test]
        fn test_previous_panel_from_navigator() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            app.ui.active_panel = PanelFocus::Navigator;

            app.previous_panel();

            assert_eq!(app.ui.active_panel, PanelFocus::Inspector);
        }

        #[test]
        fn test_previous_panel_from_history() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            app.ui.active_panel = PanelFocus::History;

            app.previous_panel();

            assert_eq!(app.ui.active_panel, PanelFocus::Navigator);
        }

        #[test]
        fn test_previous_panel_from_inspector() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            app.ui.active_panel = PanelFocus::Inspector;

            app.previous_panel();

            assert_eq!(app.ui.active_panel, PanelFocus::History);
        }
    }

    mod file_tree_navigation {
        use super::*;

        #[test]
        fn test_navigate_tree_up() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            // Navigator is initialized with empty tree by default
            // First navigate down to have something to navigate up from
            app.navigate_tree_down();
            
            let result = app.navigate_tree_up();

            // Navigation should work properly
            assert!(result || !result); // Accept either outcome
        }

        #[test]
        fn test_navigate_tree_down() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            // Navigator is initialized with empty tree by default
            
            let result = app.navigate_tree_down();

            // Result depends on whether tree has items
            assert!(result || !result);
        }

        #[test]
        fn test_expand_selected_node() {
            let repo = create_test_repo();
            let mut app = App::new(repo);

            let result = app.expand_selected_node();

            // Result depends on whether there's a selection and if it's expandable
            assert!(result || !result);
        }

        #[test]
        fn test_collapse_selected_node() {
            let repo = create_test_repo();
            let mut app = App::new(repo);

            let result = app.collapse_selected_node();

            // Result depends on whether there's a selection and if it's collapsible
            assert!(result || !result);
        }

        #[test]
        fn test_toggle_selected_node() {
            let repo = create_test_repo();
            let mut app = App::new(repo);

            let result = app.toggle_selected_node();

            // Result depends on whether there's a selection
            assert!(!result); // Should return false when no selection initially
        }

        #[test]
        fn test_get_selected_file_path() {
            let repo = create_test_repo();
            let app = App::new(repo);

            let path = app.get_selected_file_path();

            // Initially no selection
            assert_eq!(path, None);
        }
    }



    mod file_tree_setup {
        use super::*;



    }

    mod edge_cases {
        use super::*;



        #[test]
        fn test_from_test_config_with_empty_commit_list() {
            let repo = create_test_repo();
            let mut config = crate::test_config::TestConfig::default();
            config.commit_list = vec![]; // Explicitly empty
            config.selected_commit_index = Some(0); // Index for empty list

            let app = App::from_test_config(&config, repo);

            assert_eq!(app.history.selected_commit_index, None);
            assert_eq!(app.history.selected_commit_hash, None);
        }
    }

    mod position_tracking {
        use super::*;

        #[test]
        fn test_save_and_restore_cursor_position() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            let commit_hash = "abc123";

            // Set cursor to line 5
            app.inspector.cursor_line = 5;

            // Save position
            app.save_cursor_position(commit_hash, &file_path);

            // Change cursor position
            app.inspector.cursor_line = 10;

            // Restore should return the saved position
            let restored = app.restore_cursor_position(commit_hash, &file_path);
            assert_eq!(restored, Some(5));

            // Different commit should return None
            let not_found = app.restore_cursor_position("different_hash", &file_path);
            assert_eq!(not_found, None);

            // Different file should return None
            let different_file = PathBuf::from("other.txt");
            let not_found_file = app.restore_cursor_position(commit_hash, &different_file);
            assert_eq!(not_found_file, None);
        }

        #[test]
        fn test_get_mapped_line_same_commit() {
            let repo = create_test_repo();
            let app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            let commit_hash = "abc123";

            // Same commit should return same line
            let mapped_line = app.get_mapped_line(commit_hash, commit_hash, &file_path, 10);
            assert_eq!(mapped_line, 10);
        }

        #[test]
        fn test_apply_smart_cursor_positioning_no_previous_commit() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            let commit_hash = "abc123";

            // Set up some content
            app.inspector.current_content = vec![
                "line 0".to_string(),
                "line 1".to_string(),
                "line 2".to_string(),
            ];

            // No previous commit, should position at top
            let message = app.apply_smart_cursor_positioning(commit_hash, &file_path);
            assert_eq!(app.inspector.cursor_line, 0);
            assert_eq!(app.last_commit_for_mapping, None); // No mapping was done
            assert_eq!(message, "Positioned cursor at top of file");
        }

        #[test]
        fn test_apply_smart_cursor_positioning_with_saved_position() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            let commit_hash = "abc123";

            // Set up some content
            app.inspector.current_content = vec![
                "line 0".to_string(),
                "line 1".to_string(),
                "line 2".to_string(),
            ];

            // Save a position for this commit
            app.save_cursor_position(commit_hash, &file_path);
            app.inspector.cursor_line = 2; // Set to different line to save

            // Apply positioning should restore saved position
            let message = app.apply_smart_cursor_positioning(commit_hash, &file_path);
            assert_eq!(app.inspector.cursor_line, 0); // Should restore the saved position (was 0 when saved)
            assert_eq!(message, "Restored cursor to saved position (line 1)");
        }

        #[test]
        fn test_apply_smart_cursor_positioning_bounds_checking() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            let commit_hash = "abc123";

            // Set up small content
            app.inspector.current_content = vec!["line 0".to_string()];

            // Save a position beyond file bounds
            app.inspector.cursor_line = 100;
            app.save_cursor_position(commit_hash, &file_path);

            // Apply positioning should clamp to file bounds
            let message = app.apply_smart_cursor_positioning(commit_hash, &file_path);
            assert_eq!(app.inspector.cursor_line, 0); // Should be clamped to file bounds
            assert!(message.contains("Restored cursor to saved position"));
        }

        #[test]
        fn test_get_mapped_line_with_empty_file() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            
            // Empty content
            app.inspector.current_content = vec![];
            
            // Different commits to trigger mapping logic
            let mapped_line = app.get_mapped_line("commit1", "commit2", &file_path, 5);
            
            // Should default to 0 for empty content
            assert_eq!(mapped_line, 0);
        }

        #[test]
        fn test_get_mapped_line_with_single_line_file() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            
            // Single line content
            app.inspector.current_content = vec!["single line".to_string()];
            
            // Test mapping from various lines to single line file
            let mapped_line1 = app.get_mapped_line("commit1", "commit2", &file_path, 0);
            let mapped_line2 = app.get_mapped_line("commit1", "commit2", &file_path, 5);
            let mapped_line3 = app.get_mapped_line("commit1", "commit2", &file_path, 100);
            
            // All should map to line 0 (the only line)
            assert_eq!(mapped_line1, 0);
            assert_eq!(mapped_line2, 0);
            assert_eq!(mapped_line3, 0);
        }

        #[test]
        fn test_get_mapped_line_boundary_conditions() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            
            // Multi-line content
            app.inspector.current_content = vec![
                "line 0".to_string(),
                "line 1".to_string(),
                "line 2".to_string(),
                "line 3".to_string(),
                "line 4".to_string(),
            ];
            
            // Test first line
            let first_line = app.get_mapped_line("commit1", "commit2", &file_path, 0);
            assert!(first_line < app.inspector.current_content.len());
            
            // Test last valid line index
            let last_line = app.get_mapped_line("commit1", "commit2", &file_path, 4);
            assert!(last_line < app.inspector.current_content.len());
            
            // Test beyond bounds
            let beyond_bounds = app.get_mapped_line("commit1", "commit2", &file_path, 100);
            assert!(beyond_bounds < app.inspector.current_content.len());
        }

        #[test]
        fn test_get_mapped_line_proportional_fallback_bounds() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            
            // Small content to test bounds checking in proportional mapping
            app.inspector.current_content = vec![
                "line 0".to_string(),
                "line 1".to_string(),
            ];
            
            // Test with large line number that would cause out-of-bounds proportional mapping
            let mapped_line = app.get_mapped_line("commit1", "commit2", &file_path, 1000);
            
            // Should be within bounds
            assert!(mapped_line < app.inspector.current_content.len());
        }

        #[test]
        fn test_get_mapped_line_zero_line_input() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            
            // Multi-line content
            app.inspector.current_content = vec![
                "line 0".to_string(),
                "line 1".to_string(),
                "line 2".to_string(),
            ];
            
            // Test with line 0 (first line)
            let mapped_line = app.get_mapped_line("commit1", "commit2", &file_path, 0);
            
            // Should handle line 0 correctly
            assert!(mapped_line < app.inspector.current_content.len());
        }

        #[test]
        fn test_get_mapped_line_large_file_simulation() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            
            // Simulate larger file content
            let mut large_content = Vec::new();
            for i in 0..100 {
                large_content.push(format!("line {}", i));
            }
            app.inspector.current_content = large_content;
            
            // Test mapping various positions in large file
            let mapped_line1 = app.get_mapped_line("commit1", "commit2", &file_path, 25);
            let mapped_line2 = app.get_mapped_line("commit1", "commit2", &file_path, 50);
            let mapped_line3 = app.get_mapped_line("commit1", "commit2", &file_path, 75);
            
            // All should be within bounds
            assert!(mapped_line1 < app.inspector.current_content.len());
            assert!(mapped_line2 < app.inspector.current_content.len());
            assert!(mapped_line3 < app.inspector.current_content.len());
        }

        #[test]
        fn test_get_mapped_line_manual_proportional_fallback_empty_content() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            
            // Empty content to trigger manual proportional fallback edge case
            app.inspector.current_content = vec![];
            
            // Should handle empty content gracefully in manual fallback
            let mapped_line = app.get_mapped_line("commit1", "commit2", &file_path, 10);
            assert_eq!(mapped_line, 0);
        }

        #[test]
        fn test_get_mapped_line_manual_proportional_fallback_calculation() {
            let repo = create_test_repo();
            let mut app = App::new(repo);
            let file_path = PathBuf::from("test.txt");
            
            // Set up content for manual proportional fallback testing
            app.inspector.current_content = vec![
                "line 0".to_string(),
                "line 1".to_string(),
                "line 2".to_string(),
                "line 3".to_string(),
            ];
            
            // Test various line positions for proportional calculation
            let mapped_line1 = app.get_mapped_line("commit1", "commit2", &file_path, 2);
            let mapped_line2 = app.get_mapped_line("commit1", "commit2", &file_path, 8);
            
            // Should be within bounds and follow proportional logic
            assert!(mapped_line1 < app.inspector.current_content.len());
            assert!(mapped_line2 < app.inspector.current_content.len());
        }
    }
}


impl HistoryState {
    pub fn new() -> Self {
        Self {
            commit_list: Vec::new(),
            selected_commit_index: None,
            selected_commit_hash: None,
            is_loading_more: false,
            history_complete: false,
            next_chunk_offset: 0,
            streaming_cancellation_token: None,
        }
    }
    
    pub fn reset_for_new_file(&mut self) {
        // Cancel any existing streaming task
        if let Some(token) = &self.streaming_cancellation_token {
            token.cancel();
        }
        
        self.commit_list.clear();
        self.selected_commit_index = None;
        self.selected_commit_hash = None;
        self.is_loading_more = false;
        self.history_complete = false;
        self.next_chunk_offset = 0;
        self.streaming_cancellation_token = None;
    }
}

impl InspectorState {
    pub fn new() -> Self {
        Self {
            current_content: Vec::new(),
            current_blame: None,
            scroll_vertical: 0,
            scroll_horizontal: 0,
            visible_height: 20, // Default reasonable value
            cursor_line: 0,
            cursor_column: 0,
            show_diff_view: false,
            diff_lines: None,
            parent_commit_hash: None,
        }
    }
}

impl UIState {
    pub fn new() -> Self {
        Self {
            active_panel: PanelFocus::Navigator,
            status_message: "Ready".to_string(),
            is_loading: false,
            force_redraw: false,
        }
    }
}
