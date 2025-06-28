use gix::Repository;
use ratatui::widgets::ListState;
use tui_tree_widget::TreeState;
use serde::{Deserialize, Serialize};
use crate::tree::FileTree;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelFocus {
    Navigator,
    History,
    Inspector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub git_status: Option<char>,
    pub children: Vec<FileTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

pub struct App {
    pub repo: Repository,
    pub active_panel: PanelFocus,
    pub should_quit: bool,

    // Panel 1 State - File Navigator
    pub file_tree: FileTree,
    pub file_tree_state: TreeState<usize>,
    pub file_navigator_list_state: ListState,
    pub file_navigator_scroll_offset: usize,
    pub file_navigator_cursor_position: usize, // Position within the viewport
    pub file_navigator_viewport_height: usize, // Actual viewport height from UI
    pub search_query: String,
    pub in_search_mode: bool,

    // Panel 2 State - Commit History
    pub commit_list: Vec<CommitInfo>,
    pub commit_list_state: ListState,
    pub selected_commit_hash: Option<String>,

    // Panel 3 State - Code Inspector
    pub current_content: Vec<String>,
    pub current_blame: Option<String>, // Simplified for now
    pub inspector_scroll_vertical: u16,
    pub inspector_scroll_horizontal: u16,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub show_diff_view: bool,

    // UI State
    pub status_message: String,
    pub is_loading: bool,
}

impl App {
    pub fn new(repo: Repository) -> Self {
        Self {
            repo,
            active_panel: PanelFocus::Navigator,
            should_quit: false,

            file_tree: FileTree::new(),
            file_tree_state: TreeState::default(),
            file_navigator_list_state: ListState::default(),
            file_navigator_scroll_offset: 0,
            file_navigator_cursor_position: 0,
            file_navigator_viewport_height: 18, // Default reasonable value
            search_query: String::new(),
            in_search_mode: false,

            commit_list: Vec::new(),
            commit_list_state: ListState::default(),
            selected_commit_hash: None,

            current_content: Vec::new(),
            current_blame: None,
            inspector_scroll_vertical: 0,
            inspector_scroll_horizontal: 0,
            cursor_line: 0,
            cursor_column: 0,
            show_diff_view: false,

            status_message: "Ready".to_string(),
            is_loading: false,
        }
    }

    pub fn next_panel(&mut self) {
        self.active_panel = match self.active_panel {
            PanelFocus::Navigator => PanelFocus::History,
            PanelFocus::History => PanelFocus::Inspector,
            PanelFocus::Inspector => PanelFocus::Navigator,
        };
    }

    pub fn previous_panel(&mut self) {
        self.active_panel = match self.active_panel {
            PanelFocus::Navigator => PanelFocus::Inspector,
            PanelFocus::History => PanelFocus::Navigator,
            PanelFocus::Inspector => PanelFocus::History,
        };
    }

    // File tree navigation methods with viewport-based cursor movement
    pub fn navigate_tree_up(&mut self) -> bool {
        let viewport_height = self.file_navigator_viewport_height;
        self.navigate_file_navigator_up(viewport_height)
    }

    pub fn navigate_tree_down(&mut self) -> bool {
        let viewport_height = self.file_navigator_viewport_height;
        self.navigate_file_navigator_down(viewport_height)
    }

    pub fn expand_selected_node(&mut self) -> bool {
        if let Some(selected_path) = self.file_tree.current_selection.clone() {
            self.file_tree.expand_node(&selected_path)
        } else {
            false
        }
    }

    pub fn collapse_selected_node(&mut self) -> bool {
        if let Some(selected_path) = self.file_tree.current_selection.clone() {
            self.file_tree.collapse_node(&selected_path)
        } else {
            false
        }
    }

    pub fn toggle_selected_node(&mut self) -> bool {
        if let Some(selected_path) = self.file_tree.current_selection.clone() {
            self.file_tree.toggle_node(&selected_path)
        } else {
            false
        }
    }

    pub fn get_selected_file_path(&self) -> Option<PathBuf> {
        self.file_tree.current_selection.clone()
    }

    /// Update the file navigator list state to match the current file tree selection
    pub fn update_file_navigator_list_state(&mut self) {
        if let Some(ref current_selection) = self.file_tree.current_selection {
            // Get visible nodes with depth to find the current selection index
            let visible_nodes_with_depth = self.file_tree.get_visible_nodes_with_depth();
            let selected_index = visible_nodes_with_depth.iter()
                .position(|(node, _)| &node.path == current_selection);
            
            self.file_navigator_list_state.select(selected_index);
        } else {
            self.file_navigator_list_state.select(None);
        }
    }

    /// Navigate up in the file navigator with viewport-based cursor movement
    fn navigate_file_navigator_up(&mut self, viewport_height: usize) -> bool {
        let visible_nodes = self.file_tree.get_visible_nodes_with_depth();
        if visible_nodes.is_empty() {
            return false;
        }

        // Find current absolute position
        let current_absolute_pos = if let Some(ref current_selection) = self.file_tree.current_selection {
            visible_nodes.iter().position(|(node, _)| &node.path == current_selection).unwrap_or(0)
        } else {
            0
        };

        // Can't move up from the first item
        if current_absolute_pos == 0 {
            return false;
        }

        let new_absolute_pos = current_absolute_pos - 1;

        // Calculate what the new cursor position should be within the viewport
        let new_cursor_in_viewport = new_absolute_pos.saturating_sub(self.file_navigator_scroll_offset);
        
        // Calculate the actual available viewport height (nodes that will be rendered)
        let visible_nodes_in_viewport = visible_nodes
            .iter()
            .skip(self.file_navigator_scroll_offset)
            .take(viewport_height)
            .count();
        let actual_viewport_height = visible_nodes_in_viewport.min(viewport_height);

        // Check if the new position would be outside the viewport (above it)
        if new_absolute_pos < self.file_navigator_scroll_offset {
            // Need to scroll up - move the viewport but keep cursor at top
            self.file_navigator_scroll_offset = new_absolute_pos;
            self.file_navigator_cursor_position = 0;
        } else {
            // New position is within viewport - just move cursor
            self.file_navigator_cursor_position = new_cursor_in_viewport;
        }

        // CRITICAL: Ensure cursor position never exceeds actual rendered bounds
        self.file_navigator_cursor_position = self.file_navigator_cursor_position.min(actual_viewport_height.saturating_sub(1));

        // Update the actual file tree selection
        if let Some((node, _)) = visible_nodes.get(new_absolute_pos) {
            self.file_tree.current_selection = Some(node.path.clone());
            self.update_file_navigator_list_state();
            true
        } else {
            false
        }
    }

    /// Navigate down in the file navigator with viewport-based cursor movement
    fn navigate_file_navigator_down(&mut self, viewport_height: usize) -> bool {
        let visible_nodes = self.file_tree.get_visible_nodes_with_depth();
        if visible_nodes.is_empty() {
            return false;
        }

        // Find current absolute position
        let current_absolute_pos = if let Some(ref current_selection) = self.file_tree.current_selection {
            visible_nodes.iter().position(|(node, _)| &node.path == current_selection).unwrap_or(0)
        } else {
            0
        };

        // Can't move down from the last item
        if current_absolute_pos >= visible_nodes.len() - 1 {
            return false;
        }

        let new_absolute_pos = current_absolute_pos + 1;

        // Calculate what the new cursor position should be within the viewport
        let new_cursor_in_viewport = new_absolute_pos.saturating_sub(self.file_navigator_scroll_offset);
        
        // Calculate the actual available viewport height (nodes that will be rendered)
        let visible_nodes_in_viewport = visible_nodes
            .iter()
            .skip(self.file_navigator_scroll_offset)
            .take(viewport_height)
            .count();
        let actual_viewport_height = visible_nodes_in_viewport.min(viewport_height);

        // Check if the new position would be outside the actual viewport
        if new_cursor_in_viewport >= actual_viewport_height {
            // Need to scroll down - move the viewport but keep cursor at bottom
            self.file_navigator_scroll_offset = new_absolute_pos.saturating_sub(actual_viewport_height - 1);
            self.file_navigator_cursor_position = actual_viewport_height - 1;
        } else {
            // New position is within viewport - just move cursor
            self.file_navigator_cursor_position = new_cursor_in_viewport;
        }

        // CRITICAL: Ensure cursor position never exceeds actual rendered bounds
        self.file_navigator_cursor_position = self.file_navigator_cursor_position.min(actual_viewport_height.saturating_sub(1));

        // Update the actual file tree selection
        if let Some((node, _)) = visible_nodes.get(new_absolute_pos) {
            self.file_tree.current_selection = Some(node.path.clone());
            self.update_file_navigator_list_state();
            true
        } else {
            false
        }
    }

    /// Set the viewport height for proper navigation calculations
    pub fn set_file_navigator_viewport_height(&mut self, height: usize) {
        // Store this for navigation calculations
        // For now we'll calculate it dynamically in the navigation methods
    }

    pub fn set_file_tree_from_directory(&mut self, path: &std::path::Path) -> Result<(), std::io::Error> {
        self.file_tree = FileTree::from_directory(path)?;
        Ok(())
    }

    pub fn from_test_config(config: &crate::test_config::TestConfig, repo: Repository) -> Self {
        let mut app = Self {
            repo,
            active_panel: config.active_panel,
            should_quit: false,

            file_tree: config.file_tree.clone(),
            file_tree_state: TreeState::default(),
            file_navigator_list_state: ListState::default(),
            file_navigator_scroll_offset: 0,
            file_navigator_cursor_position: 0,
            file_navigator_viewport_height: 18, // Default reasonable value
            search_query: config.search_query.clone(),
            in_search_mode: config.in_search_mode,

            commit_list: config.commit_list.clone(),
            commit_list_state: ListState::default(),
            selected_commit_hash: None,

            current_content: config.current_content.clone(),
            current_blame: None,
            inspector_scroll_vertical: config.inspector_scroll_vertical,
            inspector_scroll_horizontal: config.inspector_scroll_horizontal,
            cursor_line: config.cursor_line,
            cursor_column: config.cursor_column,
            show_diff_view: config.show_diff_view,

            status_message: config.status_message.clone(),
            is_loading: config.is_loading,
        };

        // Set the selected commit if specified
        if let Some(index) = config.selected_commit_index {
            if index < app.commit_list.len() {
                app.commit_list_state.select(Some(index));
                app.selected_commit_hash = Some(app.commit_list[index].hash.clone());
            }
        }

        // Set the selected file navigator index if specified
        if let Some(index) = config.selected_file_navigator_index {
            app.file_navigator_list_state.select(Some(index));
        }

        app
    }
}