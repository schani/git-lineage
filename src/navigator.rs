//! New state machine-based navigator implementation
//! 
//! This module implements the new NavigatorState with a state machine architecture
//! that eliminates the dual-tree anti-pattern and provides proper context preservation
//! during search mode transitions.

use crate::tree::{FileTree, TreeNode};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashSet;
use std::path::PathBuf;

/// Events that can be sent to the navigator
#[derive(Debug, Clone, PartialEq)]
pub enum NavigatorEvent {
    SelectFile(PathBuf),
    StartSearch,
    UpdateSearchQuery(String),
    EndSearch,
    NavigateUp,
    NavigateDown,
    ToggleExpanded(PathBuf),
    ExpandSelected,
    CollapseSelected,
}

/// Different modes the navigator can be in
#[derive(Debug, PartialEq)]
pub enum NavigatorMode {
    Browsing {
        selection: Option<PathBuf>,
        expanded: HashSet<PathBuf>,
        scroll_offset: usize,
    },
    Searching {
        query: String,
        results: Vec<PathBuf>,
        selected_index: Option<usize>,
        saved_browsing: Box<NavigatorMode>,
    },
}

/// A visible item in the navigator view
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleItem {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_selected: bool,
    pub is_expanded: bool,
    pub is_dir: bool,
    pub git_status: Option<char>,
}

/// View model for rendering the navigator
#[derive(Debug, Clone)]
pub struct NavigatorViewModel {
    pub items: Vec<VisibleItem>,
    pub scroll_offset: usize,
    pub cursor_position: usize,
    pub search_query: String,
    pub is_searching: bool,
}

/// The new navigator state with state machine architecture
#[derive(Debug)]
pub struct NavigatorState {
    tree: FileTree,
    mode: NavigatorMode,
}

impl NavigatorState {
    /// Create a new navigator state
    pub fn new(tree: FileTree) -> Self {
        let expanded = Self::extract_expanded_paths(&tree);
        Self {
            tree,
            mode: NavigatorMode::Browsing {
                selection: None,
                expanded,
                scroll_offset: 0,
            },
        }
    }

    /// Extract expanded paths from tree nodes
    fn extract_expanded_paths(tree: &FileTree) -> HashSet<PathBuf> {
        let mut expanded = HashSet::new();
        Self::collect_expanded_paths(&tree.root, &mut expanded);
        expanded
    }

    /// Recursively collect expanded paths from tree nodes
    fn collect_expanded_paths(nodes: &[crate::tree::TreeNode], expanded: &mut HashSet<PathBuf>) {
        for node in nodes {
            if node.is_expanded {
                expanded.insert(node.path.clone());
            }
            Self::collect_expanded_paths(&node.children, expanded);
        }
    }

    /// Handle an event and return whether the state changed
    pub fn handle_event(&mut self, event: NavigatorEvent) -> Result<bool, String> {
        let old_mode = self.mode.clone();
        
        self.mode = match (&self.mode, event) {
            // Start search from browsing mode
            (NavigatorMode::Browsing { selection, expanded, scroll_offset }, NavigatorEvent::StartSearch) => {
                // For empty query, show same items as browsing mode
                let browsing_items = self.get_browsing_visible_items(expanded, selection);
                let results: Vec<PathBuf> = browsing_items.iter().map(|item| item.path.clone()).collect();
                NavigatorMode::Searching {
                    query: String::new(),
                    results: results.clone(),
                    selected_index: if results.is_empty() { None } else { Some(0) },
                    saved_browsing: Box::new(NavigatorMode::Browsing {
                        selection: selection.clone(),
                        expanded: expanded.clone(),
                        scroll_offset: *scroll_offset,
                    }),
                }
            }

            // End search and restore browsing context
            (NavigatorMode::Searching { saved_browsing, .. }, NavigatorEvent::EndSearch) => {
                *saved_browsing.clone()
            }

            // Update search query
            (NavigatorMode::Searching { saved_browsing, .. }, NavigatorEvent::UpdateSearchQuery(query)) => {
                let results = if query.is_empty() {
                    // When query becomes empty, show same items as the saved browsing mode
                    if let NavigatorMode::Browsing { selection, expanded, .. } = saved_browsing.as_ref() {
                        let browsing_items = self.get_browsing_visible_items(expanded, selection);
                        browsing_items.iter().map(|item| item.path.clone()).collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    // For non-empty queries, build directory structure containing matching files
                    self.build_search_tree_structure(&query)
                };
                NavigatorMode::Searching {
                    query,
                    selected_index: if results.is_empty() { None } else { Some(0) },
                    results,
                    saved_browsing: saved_browsing.clone(),
                }
            }

            // Navigation in browsing mode
            (NavigatorMode::Browsing { selection, expanded, .. }, NavigatorEvent::NavigateUp) => {
                let visible_items = self.get_browsing_visible_items(expanded, selection);
                let new_selection = self.find_previous_item(&visible_items, selection);
                let new_scroll = self.calculate_scroll_offset(&new_selection, &visible_items);
                
                NavigatorMode::Browsing {
                    selection: new_selection,
                    expanded: expanded.clone(),
                    scroll_offset: new_scroll,
                }
            }

            (NavigatorMode::Browsing { selection, expanded, .. }, NavigatorEvent::NavigateDown) => {
                let visible_items = self.get_browsing_visible_items(expanded, selection);
                let new_selection = self.find_next_item(&visible_items, selection);
                let new_scroll = self.calculate_scroll_offset(&new_selection, &visible_items);
                
                NavigatorMode::Browsing {
                    selection: new_selection,
                    expanded: expanded.clone(),
                    scroll_offset: new_scroll,
                }
            }

            // Navigation in search mode
            (NavigatorMode::Searching { selected_index, query, results, saved_browsing }, NavigatorEvent::NavigateUp) => {
                let new_index = selected_index
                    .and_then(|i| if i > 0 { Some(i - 1) } else { None })
                    .or_else(|| if !results.is_empty() { Some(results.len() - 1) } else { None });
                
                NavigatorMode::Searching {
                    selected_index: new_index,
                    query: query.clone(),
                    results: results.clone(),
                    saved_browsing: saved_browsing.clone(),
                }
            }

            (NavigatorMode::Searching { selected_index, query, results, saved_browsing }, NavigatorEvent::NavigateDown) => {
                let new_index = selected_index
                    .map(|i| (i + 1) % results.len().max(1))
                    .or_else(|| if !results.is_empty() { Some(0) } else { None });
                
                NavigatorMode::Searching {
                    selected_index: new_index,
                    query: query.clone(),
                    results: results.clone(),
                    saved_browsing: saved_browsing.clone(),
                }
            }

            // Toggle expansion in browsing mode
            (NavigatorMode::Browsing { selection, expanded, scroll_offset }, NavigatorEvent::ToggleExpanded(path)) => {
                let mut new_expanded = expanded.clone();
                if new_expanded.contains(&path) {
                    new_expanded.remove(&path);
                } else {
                    new_expanded.insert(path);
                }
                
                NavigatorMode::Browsing {
                    selection: selection.clone(),
                    expanded: new_expanded,
                    scroll_offset: *scroll_offset,
                }
            }

            // Direct selection in browsing mode
            (NavigatorMode::Browsing { expanded, .. }, NavigatorEvent::SelectFile(path)) => {
                let visible_items = self.get_browsing_visible_items(expanded, &Some(path.clone()));
                let new_scroll = self.calculate_scroll_offset(&Some(path.clone()), &visible_items);
                
                NavigatorMode::Browsing {
                    selection: Some(path),
                    expanded: expanded.clone(),
                    scroll_offset: new_scroll,
                }
            }

            // Expand/collapse selected in browsing mode
            (NavigatorMode::Browsing { selection, expanded, scroll_offset }, NavigatorEvent::ExpandSelected) => {
                if let Some(ref sel) = selection {
                    if let Some(node) = self.tree.find_node(sel) {
                        if node.is_dir {
                            let mut new_expanded = expanded.clone();
                            new_expanded.insert(sel.clone());
                            
                            NavigatorMode::Browsing {
                                selection: selection.clone(),
                                expanded: new_expanded,
                                scroll_offset: *scroll_offset,
                            }
                        } else {
                            NavigatorMode::Browsing {
                                selection: selection.clone(),
                                expanded: expanded.clone(),
                                scroll_offset: *scroll_offset,
                            }
                        }
                    } else {
                        NavigatorMode::Browsing {
                            selection: selection.clone(),
                            expanded: expanded.clone(),
                            scroll_offset: *scroll_offset,
                        }
                    }
                } else {
                    NavigatorMode::Browsing {
                        selection: selection.clone(),
                        expanded: expanded.clone(),
                        scroll_offset: *scroll_offset,
                    }
                }
            }

            (NavigatorMode::Browsing { selection, expanded, scroll_offset }, NavigatorEvent::CollapseSelected) => {
                if let Some(ref sel) = selection {
                    if let Some(node) = self.tree.find_node(sel) {
                        if node.is_dir {
                            let mut new_expanded = expanded.clone();
                            new_expanded.remove(sel);
                            
                            NavigatorMode::Browsing {
                                selection: selection.clone(),
                                expanded: new_expanded,
                                scroll_offset: *scroll_offset,
                            }
                        } else {
                            NavigatorMode::Browsing {
                                selection: selection.clone(),
                                expanded: expanded.clone(),
                                scroll_offset: *scroll_offset,
                            }
                        }
                    } else {
                        NavigatorMode::Browsing {
                            selection: selection.clone(),
                            expanded: expanded.clone(),
                            scroll_offset: *scroll_offset,
                        }
                    }
                } else {
                    NavigatorMode::Browsing {
                        selection: selection.clone(),
                        expanded: expanded.clone(),
                        scroll_offset: *scroll_offset,
                    }
                }
            }

            // Events not applicable to current mode - no state change
            _ => self.mode.clone(),
        };

        Ok(old_mode != self.mode)
    }

    /// Get the current selection
    pub fn get_selection(&self) -> Option<PathBuf> {
        match &self.mode {
            NavigatorMode::Browsing { selection, .. } => selection.clone(),
            NavigatorMode::Searching { results, selected_index, .. } => {
                selected_index.and_then(|i| results.get(i).cloned())
            }
        }
    }

    /// Check if currently in search mode
    pub fn is_searching(&self) -> bool {
        matches!(self.mode, NavigatorMode::Searching { .. })
    }

    /// Get current search query
    pub fn get_search_query(&self) -> String {
        match &self.mode {
            NavigatorMode::Searching { query, .. } => query.clone(),
            _ => String::new(),
        }
    }

    /// Build view model for rendering
    pub fn build_view_model(&self) -> NavigatorViewModel {
        match &self.mode {
            NavigatorMode::Browsing { selection, expanded, scroll_offset } => {
                let items = self.get_browsing_visible_items(expanded, selection);
                let cursor_position = selection
                    .as_ref()
                    .and_then(|sel| items.iter().position(|item| &item.path == sel))
                    .unwrap_or(0);

                NavigatorViewModel {
                    items,
                    scroll_offset: *scroll_offset,
                    cursor_position,
                    search_query: String::new(),
                    is_searching: false,
                }
            }
            NavigatorMode::Searching { query, results, selected_index, .. } => {
                let items = self.get_search_visible_items(results, selected_index);
                let cursor_position = selected_index.unwrap_or(0);

                NavigatorViewModel {
                    items,
                    scroll_offset: 0, // Search results start at top
                    cursor_position,
                    search_query: query.clone(),
                    is_searching: true,
                }
            }
        }
    }

    /// Search for files matching the query
    fn search_files(&self, query: &str) -> Vec<PathBuf> {
        let mut results = Vec::new();
        
        // Collect all file paths from the tree
        self.collect_all_paths(&self.tree.root, &mut results);
        
        if query.is_empty() {
            // When search query is empty, show all files
            return results;
        }

        let matcher = SkimMatcherV2::default();

        // Filter and sort by fuzzy match score
        let mut scored_results: Vec<(PathBuf, i64)> = results
            .into_iter()
            .filter_map(|path| {
                let file_name = path.file_name()?.to_string_lossy();
                matcher.fuzzy_match(&file_name, query)
                    .map(|score| (path, score))
            })
            .collect();

        // Sort by score (higher is better)
        scored_results.sort_by(|a, b| b.1.cmp(&a.1));

        scored_results.into_iter().map(|(path, _)| path).collect()
    }

    /// Recursively collect all file paths from tree nodes
    fn collect_all_paths(&self, nodes: &[TreeNode], paths: &mut Vec<PathBuf>) {
        for node in nodes {
            // Only collect files, not directories
            if !node.is_dir {
                paths.push(node.path.clone());
            }
            self.collect_all_paths(&node.children, paths);
        }
    }

    /// Get visible items for browsing mode
    fn get_browsing_visible_items(&self, expanded: &HashSet<PathBuf>, selection: &Option<PathBuf>) -> Vec<VisibleItem> {
        let mut items = Vec::new();
        
        for node in &self.tree.root {
            self.collect_browsing_visible_items(node, &mut items, 0, expanded, selection);
        }
        
        items
    }

    /// Recursively collect visible items in browsing mode
    fn collect_browsing_visible_items(
        &self,
        node: &TreeNode,
        items: &mut Vec<VisibleItem>,
        depth: usize,
        expanded: &HashSet<PathBuf>,
        selection: &Option<PathBuf>,
    ) {
        let is_selected = selection.as_ref() == Some(&node.path);
        let is_expanded = expanded.contains(&node.path);

        items.push(VisibleItem {
            path: node.path.clone(),
            name: node.name.clone(),
            depth,
            is_selected,
            is_expanded,
            is_dir: node.is_dir,
            git_status: node.git_status,
        });

        // If directory is expanded, show children
        if node.is_dir && is_expanded {
            for child in &node.children {
                self.collect_browsing_visible_items(child, items, depth + 1, expanded, selection);
            }
        }
    }

    /// Get visible items for search mode
    fn get_search_visible_items(&self, results: &[PathBuf], selected_index: &Option<usize>) -> Vec<VisibleItem> {
        // For search mode, we need to display results as a tree structure
        // with directories containing matches automatically expanded
        
        // First, determine which directories should be expanded
        let mut expanded_dirs = HashSet::new();
        for path in results {
            if let Some(parent) = path.parent() {
                if parent != std::path::Path::new("") && parent != std::path::Path::new(".") {
                    expanded_dirs.insert(parent.to_path_buf());
                }
            }
        }
        
        // Use the browsing visible items logic but with search expansion state
        // Process directories first, then files
        let mut items = Vec::new();
        
        // First pass: directories
        for node in &self.tree.root {
            if node.is_dir && results.contains(&node.path) {
                self.collect_search_visible_items(node, &mut items, 0, &expanded_dirs, results, selected_index);
            }
        }
        
        // Second pass: files
        for node in &self.tree.root {
            if !node.is_dir && results.contains(&node.path) {
                self.collect_search_visible_items(node, &mut items, 0, &expanded_dirs, results, selected_index);
            }
        }
        
        items
    }
    
    /// Recursively collect visible items for search mode with tree structure
    fn collect_search_visible_items(
        &self,
        node: &crate::tree::TreeNode,
        items: &mut Vec<VisibleItem>,
        depth: usize,
        expanded: &HashSet<PathBuf>,
        search_results: &[PathBuf],
        selected_index: &Option<usize>,
    ) {
        // Only include this node if it's in the search results
        if !search_results.contains(&node.path) {
            return;
        }
        
        let current_index = items.len();
        let is_selected = selected_index == &Some(current_index);
        let is_expanded = expanded.contains(&node.path);

        items.push(VisibleItem {
            path: node.path.clone(),
            name: node.name.clone(),
            depth,
            is_selected,
            is_expanded,
            is_dir: node.is_dir,
            git_status: node.git_status,
        });

        // If directory is expanded, show children that are in search results
        if node.is_dir && is_expanded {
            for child in &node.children {
                self.collect_search_visible_items(child, items, depth + 1, expanded, search_results, selected_index);
            }
        }
    }

    /// Find the next item in the visible list
    fn find_next_item(&self, visible_items: &[VisibleItem], current_selection: &Option<PathBuf>) -> Option<PathBuf> {
        if visible_items.is_empty() {
            return None;
        }

        match current_selection {
            Some(selection) => {
                let current_index = visible_items
                    .iter()
                    .position(|item| &item.path == selection)?;
                
                if current_index < visible_items.len() - 1 {
                    Some(visible_items[current_index + 1].path.clone())
                } else {
                    // Already at the end
                    Some(selection.clone())
                }
            }
            None => {
                // No current selection, select first item
                Some(visible_items[0].path.clone())
            }
        }
    }

    /// Find the previous item in the visible list
    fn find_previous_item(&self, visible_items: &[VisibleItem], current_selection: &Option<PathBuf>) -> Option<PathBuf> {
        if visible_items.is_empty() {
            return None;
        }

        match current_selection {
            Some(selection) => {
                let current_index = visible_items
                    .iter()
                    .position(|item| &item.path == selection)?;
                
                if current_index > 0 {
                    Some(visible_items[current_index - 1].path.clone())
                } else {
                    // Already at the beginning
                    Some(selection.clone())
                }
            }
            None => {
                // No current selection, select first item
                Some(visible_items[0].path.clone())
            }
        }
    }

    /// Calculate scroll offset to keep selection visible
    fn calculate_scroll_offset(&self, _selection: &Option<PathBuf>, _visible_items: &[VisibleItem]) -> usize {
        // For now, just return 0. This will be implemented when we integrate with the UI
        // The UI layer will handle viewport management
        0
    }

    /// Build directory structure containing search results
    fn build_search_tree_structure(&self, query: &str) -> Vec<PathBuf> {
        let (results, _) = self.build_search_tree_structure_with_expansion(query);
        results
    }
    
    /// Build directory structure containing search results with expansion info
    fn build_search_tree_structure_with_expansion(&self, query: &str) -> (Vec<PathBuf>, HashSet<PathBuf>) {
        let matching_files = self.search_files(query);
        
        if matching_files.is_empty() {
            return (Vec::new(), HashSet::new());
        }
        
        let mut tree_paths = HashSet::new();
        let mut expanded_dirs = HashSet::new();
        
        // For each matching file, add all its parent directories and mark them as expanded
        for file_path in &matching_files {
            tree_paths.insert(file_path.clone());
            
            // Add all parent directories and mark them as expanded
            let mut current_path = file_path.clone();
            while let Some(parent) = current_path.parent() {
                if parent == std::path::Path::new("") || parent == std::path::Path::new(".") {
                    break;
                }
                let parent_buf = parent.to_path_buf();
                tree_paths.insert(parent_buf.clone());
                expanded_dirs.insert(parent_buf);
                current_path = parent.to_path_buf();
            }
        }
        
        // Convert to sorted vector with directories first
        let mut result: Vec<PathBuf> = tree_paths.into_iter().collect();
        result.sort_by(|a, b| {
            // First compare by whether they're directories
            let a_is_dir = self.tree.find_node(a).map(|n| n.is_dir).unwrap_or(false);
            let b_is_dir = self.tree.find_node(b).map(|n| n.is_dir).unwrap_or(false);
            
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });
        
        (result, expanded_dirs)
    }
}

// Implement Clone for NavigatorMode manually since Box doesn't auto-derive Clone
impl Clone for NavigatorMode {
    fn clone(&self) -> Self {
        match self {
            NavigatorMode::Browsing { selection, expanded, scroll_offset } => {
                NavigatorMode::Browsing {
                    selection: selection.clone(),
                    expanded: expanded.clone(),
                    scroll_offset: *scroll_offset,
                }
            }
            NavigatorMode::Searching { query, results, selected_index, saved_browsing } => {
                NavigatorMode::Searching {
                    query: query.clone(),
                    results: results.clone(),
                    selected_index: *selected_index,
                    saved_browsing: saved_browsing.clone(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{FileTree, TreeNode};
    use std::path::PathBuf;

    fn create_test_tree() -> FileTree {
        let mut tree = FileTree::new();
        
        // Create a simple tree structure:
        // src/
        //   main.rs
        //   lib.rs
        //   utils/
        //     helpers.rs
        // README.md
        // Cargo.toml
        
        let mut src_dir = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        src_dir.add_child(TreeNode::new_file("main.rs".to_string(), PathBuf::from("src/main.rs")));
        src_dir.add_child(TreeNode::new_file("lib.rs".to_string(), PathBuf::from("src/lib.rs")));
        
        let mut utils_dir = TreeNode::new_dir("utils".to_string(), PathBuf::from("src/utils"));
        utils_dir.add_child(TreeNode::new_file("helpers.rs".to_string(), PathBuf::from("src/utils/helpers.rs")));
        src_dir.add_child(utils_dir);
        
        tree.root.push(src_dir);
        tree.root.push(TreeNode::new_file("README.md".to_string(), PathBuf::from("README.md")));
        tree.root.push(TreeNode::new_file("Cargo.toml".to_string(), PathBuf::from("Cargo.toml")));
        
        // Sort root level like FileTree does
        tree.root.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
        
        tree
    }

    #[test]
    fn test_navigator_state_creation() {
        let tree = create_test_tree();
        let navigator = NavigatorState::new(tree);
        
        assert!(!navigator.is_searching());
        assert_eq!(navigator.get_search_query(), "");
        assert_eq!(navigator.get_selection(), None);
    }

    #[test]
    fn test_browsing_navigation() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Navigate down - should select first item
        let changed = navigator.handle_event(NavigatorEvent::NavigateDown).unwrap();
        assert!(changed);
        assert_eq!(navigator.get_selection(), Some(PathBuf::from("src")));
        
        // Navigate down again - should select next item
        let changed = navigator.handle_event(NavigatorEvent::NavigateDown).unwrap();
        assert!(changed);
        assert_eq!(navigator.get_selection(), Some(PathBuf::from("Cargo.toml")));
        
        // Navigate up - should go back to src
        let changed = navigator.handle_event(NavigatorEvent::NavigateUp).unwrap();
        assert!(changed);
        assert_eq!(navigator.get_selection(), Some(PathBuf::from("src")));
    }

    #[test]
    fn test_directory_expansion() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Select src directory
        navigator.handle_event(NavigatorEvent::SelectFile(PathBuf::from("src"))).unwrap();
        
        // Expand it
        let changed = navigator.handle_event(NavigatorEvent::ExpandSelected).unwrap();
        assert!(changed);
        
        // Build view model to check expanded state
        let view_model = navigator.build_view_model();
        let src_item = view_model.items.iter().find(|item| item.path == PathBuf::from("src")).unwrap();
        assert!(src_item.is_expanded);
        
        // Should now see src children in visible items
        assert!(view_model.items.iter().any(|item| item.path == PathBuf::from("src/main.rs")));
        assert!(view_model.items.iter().any(|item| item.path == PathBuf::from("src/lib.rs")));
        assert!(view_model.items.iter().any(|item| item.path == PathBuf::from("src/utils")));
    }

    #[test]
    fn test_search_mode_basic() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Start search
        let changed = navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        assert!(changed);
        assert!(navigator.is_searching());
        assert_eq!(navigator.get_search_query(), "");
        
        // Update search query
        let changed = navigator.handle_event(NavigatorEvent::UpdateSearchQuery("main".to_string())).unwrap();
        assert!(changed);
        assert_eq!(navigator.get_search_query(), "main");
        
        // Should have found main.rs
        let view_model = navigator.build_view_model();
        assert!(view_model.items.iter().any(|item| item.path == PathBuf::from("src/main.rs")));
        
        // End search
        let changed = navigator.handle_event(NavigatorEvent::EndSearch).unwrap();
        assert!(changed);
        assert!(!navigator.is_searching());
    }

    #[test]
    fn test_search_preserves_browsing_context() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Set up browsing state
        navigator.handle_event(NavigatorEvent::SelectFile(PathBuf::from("README.md"))).unwrap();
        navigator.handle_event(NavigatorEvent::ToggleExpanded(PathBuf::from("src"))).unwrap();
        
        let original_selection = navigator.get_selection();
        let original_view = navigator.build_view_model();
        
        // Enter search mode
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        navigator.handle_event(NavigatorEvent::UpdateSearchQuery("cargo".to_string())).unwrap();
        
        // Exit search mode
        navigator.handle_event(NavigatorEvent::EndSearch).unwrap();
        
        // Verify context is restored
        assert_eq!(navigator.get_selection(), original_selection);
        assert!(!navigator.is_searching());
        
        let restored_view = navigator.build_view_model();
        
        // Check that expanded state is preserved
        let src_expanded_original = original_view.items.iter()
            .find(|item| item.path == PathBuf::from("src"))
            .map(|item| item.is_expanded)
            .unwrap_or(false);
        
        let src_expanded_restored = restored_view.items.iter()
            .find(|item| item.path == PathBuf::from("src"))
            .map(|item| item.is_expanded)
            .unwrap_or(false);
        
        assert_eq!(src_expanded_original, src_expanded_restored);
    }

    #[test]
    fn test_search_navigation() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Enter search mode and search for "rs" files
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        navigator.handle_event(NavigatorEvent::UpdateSearchQuery("rs".to_string())).unwrap();
        
        let view_model = navigator.build_view_model();
        assert!(!view_model.items.is_empty());
        
        // Should start with first item selected
        assert_eq!(view_model.cursor_position, 0);
        let first_selection = navigator.get_selection();
        
        // Navigate down in search results
        navigator.handle_event(NavigatorEvent::NavigateDown).unwrap();
        let second_selection = navigator.get_selection();
        
        assert_ne!(first_selection, second_selection);
        
        // Navigate up - should go back to first
        navigator.handle_event(NavigatorEvent::NavigateUp).unwrap();
        assert_eq!(navigator.get_selection(), first_selection);
    }

    #[test]
    fn test_search_empty_query() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        navigator.handle_event(NavigatorEvent::UpdateSearchQuery("".to_string())).unwrap();
        
        let view_model = navigator.build_view_model();
        // Empty search should show all files
        assert!(!view_model.items.is_empty());
        assert!(view_model.items.len() >= 3); // At least file1.rs, file2.rs, subdir/file3.rs
        assert!(view_model.is_searching);
        assert_eq!(view_model.search_query, "");
        // First item should be selected
        assert_eq!(navigator.get_selection(), Some(view_model.items[0].path.clone()));
    }

    #[test]
    fn test_search_no_results() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        navigator.handle_event(NavigatorEvent::UpdateSearchQuery("nonexistent".to_string())).unwrap();
        
        let view_model = navigator.build_view_model();
        assert!(view_model.items.is_empty());
        assert_eq!(navigator.get_selection(), None);
    }

    #[test]
    fn test_direct_file_selection() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Directly select a file
        let target_path = PathBuf::from("Cargo.toml");
        navigator.handle_event(NavigatorEvent::SelectFile(target_path.clone())).unwrap();
        
        assert_eq!(navigator.get_selection(), Some(target_path));
        
        let view_model = navigator.build_view_model();
        let selected_item = view_model.items.iter().find(|item| item.is_selected);
        assert!(selected_item.is_some());
        assert_eq!(selected_item.unwrap().path, PathBuf::from("Cargo.toml"));
    }

    #[test]
    fn test_toggle_expansion() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        let src_path = PathBuf::from("src");
        
        // Initially not expanded
        let view_model = navigator.build_view_model();
        let src_item = view_model.items.iter().find(|item| item.path == src_path);
        assert!(!src_item.unwrap().is_expanded);
        
        // Expand
        navigator.handle_event(NavigatorEvent::ToggleExpanded(src_path.clone())).unwrap();
        let view_model = navigator.build_view_model();
        let src_item = view_model.items.iter().find(|item| item.path == src_path);
        assert!(src_item.unwrap().is_expanded);
        
        // Collapse
        navigator.handle_event(NavigatorEvent::ToggleExpanded(src_path.clone())).unwrap();
        let view_model = navigator.build_view_model();
        let src_item = view_model.items.iter().find(|item| item.path == src_path);
        assert!(!src_item.unwrap().is_expanded);
    }

    #[test]
    fn test_view_model_structure() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Test browsing view model
        let view_model = navigator.build_view_model();
        assert!(!view_model.is_searching);
        assert_eq!(view_model.search_query, "");
        assert!(!view_model.items.is_empty());
        
        // All items should have depth 0 initially (no expansion)
        for item in &view_model.items {
            assert_eq!(item.depth, 0);
        }
        
        // Test search view model
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        navigator.handle_event(NavigatorEvent::UpdateSearchQuery("main".to_string())).unwrap();
        
        let search_view_model = navigator.build_view_model();
        assert!(search_view_model.is_searching);
        assert_eq!(search_view_model.search_query, "main");
        
        // Search results should now show directory structure with proper depths
        // The src directory should have depth 0, main.rs should have depth 1
        let src_item = search_view_model.items.iter().find(|item| item.path == PathBuf::from("src"));
        let main_item = search_view_model.items.iter().find(|item| item.path == PathBuf::from("src/main.rs"));
        
        if let Some(src_item) = src_item {
            assert_eq!(src_item.depth, 0);
        }
        if let Some(main_item) = main_item {
            assert_eq!(main_item.depth, 1);
        }
    }

    #[test]
    fn test_nested_directory_expansion() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Expand src directory
        navigator.handle_event(NavigatorEvent::ToggleExpanded(PathBuf::from("src"))).unwrap();
        
        // Now expand utils subdirectory
        navigator.handle_event(NavigatorEvent::ToggleExpanded(PathBuf::from("src/utils"))).unwrap();
        
        let view_model = navigator.build_view_model();
        
        // Should see helpers.rs with depth 2
        let helpers_item = view_model.items.iter()
            .find(|item| item.path == PathBuf::from("src/utils/helpers.rs"));
        assert!(helpers_item.is_some());
        assert_eq!(helpers_item.unwrap().depth, 2);
        
        // src should have depth 0
        let src_item = view_model.items.iter()
            .find(|item| item.path == PathBuf::from("src"));
        assert_eq!(src_item.unwrap().depth, 0);
        
        // utils should have depth 1
        let utils_item = view_model.items.iter()
            .find(|item| item.path == PathBuf::from("src/utils"));
        assert_eq!(utils_item.unwrap().depth, 1);
    }
    
    #[test]
    fn test_start_search_shows_browsing_items() {
        let tree = create_test_tree();
        let mut navigator = NavigatorState::new(tree);
        
        // Start search
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        
        let view_model = navigator.build_view_model();
        
        // When starting search, should show same items as browsing mode
        assert!(view_model.is_searching);
        assert_eq!(view_model.search_query, "");
        assert!(!view_model.items.is_empty());
        
        // Should have top-level items (directories first, then files)
        assert!(view_model.items.iter().any(|item| item.path == PathBuf::from("src")));
        assert!(view_model.items.iter().any(|item| item.path == PathBuf::from("README.md")));
        assert!(view_model.items.iter().any(|item| item.path == PathBuf::from("Cargo.toml")));
        
        // Should NOT show nested files unless directories are expanded
        assert!(!view_model.items.iter().any(|item| item.path == PathBuf::from("src/main.rs")));
    }
}