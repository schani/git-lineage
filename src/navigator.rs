//! Simplified navigator implementation
//! 
//! This module implements the NavigatorState with a unified architecture
//! that treats search as a filter rather than a separate mode.

use crate::tree::{FileTree, TreeNode};
use std::collections::HashSet;
use std::path::PathBuf;

/// Events that can be sent to the navigator
#[derive(Debug, Clone, PartialEq)]
pub enum NavigatorEvent {
    SelectFile(PathBuf),
    StartSearch,
    UpdateSearchQuery(String),
    EndSearch,
    EndSearchKeepQuery,
    NavigateUp,
    NavigateDown,
    ToggleExpanded(PathBuf),
    ExpandSelected,
    CollapseSelected,
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

/// The simplified navigator state
#[derive(Debug)]
pub struct NavigatorState {
    tree: FileTree,
    selection: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    scroll_offset: usize,
    query: String,  // Empty string means show all files, non-empty means filter
    editing_search: bool,  // UI state for showing search cursor
    
    // View model caching
    cached_view_model: Option<NavigatorViewModel>,
    view_model_dirty: bool,
    last_state_hash: u64,
}

impl NavigatorState {
    /// Create a new navigator state
    pub fn new(tree: FileTree) -> Self {
        let expanded = Self::extract_expanded_paths(&tree);
        Self {
            tree,
            selection: None,
            expanded,
            scroll_offset: 0,
            query: String::new(),
            editing_search: false,
            cached_view_model: None,
            view_model_dirty: true,
            last_state_hash: 0,
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
        let state_before = (self.selection.clone(), self.query.clone(), self.editing_search, self.expanded.clone());
        
        match event {
            NavigatorEvent::StartSearch => {
                self.editing_search = true;
                // Only ensure selection if we don't have one
                if self.selection.is_none() {
                    self.ensure_valid_selection();
                }
            }
            
            NavigatorEvent::UpdateSearchQuery(new_query) => {
                if self.editing_search {
                    self.query = new_query;
                    // After updating query, ensure selection is still valid
                    self.ensure_valid_selection();
                }
            }
            
            NavigatorEvent::EndSearch => {
                self.editing_search = false;
                self.query.clear();
                // When returning to full tree, keep current selection if it exists
                // No need to call ensure_valid_selection - let user navigate if needed
            }
            
            NavigatorEvent::EndSearchKeepQuery => {
                self.editing_search = false;
                // query stays as-is, selection should remain valid
            }
            
            NavigatorEvent::NavigateUp => {
                let visible_items = self.get_current_visible_items();
                self.selection = self.find_previous_item(&visible_items, &self.selection);
                self.scroll_offset = self.calculate_scroll_offset(&self.selection, &visible_items);
            }
            
            NavigatorEvent::NavigateDown => {
                let visible_items = self.get_current_visible_items();
                self.selection = self.find_next_item(&visible_items, &self.selection);
                self.scroll_offset = self.calculate_scroll_offset(&self.selection, &visible_items);
            }
            
            NavigatorEvent::ToggleExpanded(path) => {
                if self.expanded.contains(&path) {
                    self.expanded.remove(&path);
                } else {
                    self.expanded.insert(path);
                }
            }
            
            NavigatorEvent::SelectFile(path) => {
                self.selection = Some(path);
                let visible_items = self.get_current_visible_items();
                self.scroll_offset = self.calculate_scroll_offset(&self.selection, &visible_items);
            }
            
            NavigatorEvent::ExpandSelected => {
                if let Some(ref sel) = self.selection {
                    if let Some(node) = self.tree.find_node(sel) {
                        if node.is_dir {
                            self.expanded.insert(sel.clone());
                        }
                    }
                }
            }
            
            NavigatorEvent::CollapseSelected => {
                if let Some(ref sel) = self.selection {
                    if let Some(node) = self.tree.find_node(sel) {
                        if node.is_dir {
                            self.expanded.remove(sel);
                        }
                    }
                }
            }
        }
        
        let state_after = (self.selection.clone(), self.query.clone(), self.editing_search, self.expanded.clone());
        let state_changed = state_before != state_after;
        
        if state_changed {
            self.invalidate_view_model();
        }
        
        Ok(state_changed)
    }

    /// Get the current selection
    pub fn get_selection(&self) -> Option<PathBuf> {
        self.selection.clone()
    }

    /// Check if currently editing search
    pub fn is_searching(&self) -> bool {
        self.editing_search
    }

    /// Get current search query
    pub fn get_search_query(&self) -> String {
        self.query.clone()
    }

    /// Build view model for rendering (with caching)
    pub fn build_view_model(&mut self) -> &NavigatorViewModel {
        let current_hash = self.compute_state_hash();
        
        // Only rebuild if state actually changed
        if !self.view_model_dirty && self.last_state_hash == current_hash {
            if let Some(ref cached) = self.cached_view_model {
                log::debug!("View model: using cached (no state change)");
                return cached;
            }
        }
        
        log::debug!("View model: rebuilding due to state change");
        let start = std::time::Instant::now();
        
        // Expensive computation only when needed
        let view_model = self.rebuild_view_model();
        
        self.cached_view_model = Some(view_model);
        self.view_model_dirty = false;
        self.last_state_hash = current_hash;
        
        log::debug!("View model: rebuilt in {:?}", start.elapsed());
        self.cached_view_model.as_ref().unwrap()
    }
    
    /// Actually rebuild the view model (expensive operation)
    fn rebuild_view_model(&self) -> NavigatorViewModel {
        let items = if self.query.is_empty() {
            // Show full tree
            self.get_browsing_visible_items(&self.expanded, &self.selection)
        } else {
            // Show filtered tree (same logic always)  
            let results = self.search_files(&self.query);
            self.get_search_visible_items(&results, &self.selection)
        };
        
        log::debug!("View model: computed {} items", items.len());
        
        let cursor_position = self.selection
            .as_ref()
            .and_then(|sel| items.iter().position(|item| &item.path == sel))
            .unwrap_or(0);
        
        NavigatorViewModel {
            items,
            scroll_offset: self.scroll_offset,
            cursor_position,
            search_query: self.query.clone(),
            is_searching: self.editing_search,
        }
    }
    
    /// Compute a fast hash of state that affects view model
    fn compute_state_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        self.query.hash(&mut hasher);
        self.selection.hash(&mut hasher);
        self.expanded.len().hash(&mut hasher); // Just count, not full set
        self.editing_search.hash(&mut hasher);
        self.scroll_offset.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Mark view model as needing rebuild
    pub fn invalidate_view_model(&mut self) {
        self.view_model_dirty = true;
    }
    
    /// Get the currently visible items based on current state
    fn get_current_visible_items(&self) -> Vec<VisibleItem> {
        if self.query.is_empty() {
            self.get_browsing_visible_items(&self.expanded, &self.selection)
        } else {
            let results = self.search_files(&self.query);
            self.get_search_visible_items(&results, &self.selection)
        }
    }

    /// Search for files matching the query (with global cache to prevent repeated work)
    fn search_files(&self, query: &str) -> Vec<PathBuf> {
        use std::sync::Mutex;
        use std::collections::HashMap;
        
        lazy_static::lazy_static! {
            static ref SEARCH_CACHE: Mutex<HashMap<String, Vec<PathBuf>>> = Mutex::new(HashMap::new());
        }
        
        // Check global cache first
        if let Ok(cache) = SEARCH_CACHE.lock() {
            if let Some(cached_results) = cache.get(query) {
                log::debug!("Search: using global cache for query '{}'", query);
                return cached_results.clone();
            }
        }
        
        let mut results = Vec::new();
        
        // Collect all file paths from the tree
        let start = std::time::Instant::now();
        self.collect_all_paths(&self.tree.root, &mut results);
        let collect_time = start.elapsed();
        
        log::debug!("Search: collected {} paths in {:?}", results.len(), collect_time);
        
        if query.is_empty() {
            // When search query is empty, show all files
            if let Ok(mut cache) = SEARCH_CACHE.lock() {
                cache.insert(query.to_string(), results.clone());
            }
            return results;
        }

        // Filter by substring match in filename (case-insensitive)
        let query_lower = query.to_lowercase();
        let filter_start = std::time::Instant::now();
        let mut filtered_results: Vec<PathBuf> = results
            .into_iter()
            .filter(|path| {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_lowercase();
                    file_name_str.contains(&query_lower)
                } else {
                    false
                }
            })
            .collect();

        // Sort alphabetically for consistent results
        filtered_results.sort();
        let filter_time = filter_start.elapsed();
        
        log::debug!("Search: computed {} results in {:?}", filtered_results.len(), filter_time);
        
        // Cache the results globally
        if let Ok(mut cache) = SEARCH_CACHE.lock() {
            cache.insert(query.to_string(), filtered_results.clone());
        }
        
        filtered_results
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
    fn get_search_visible_items(&self, results: &[PathBuf], selection: &Option<PathBuf>) -> Vec<VisibleItem> {
        let start = std::time::Instant::now();
        log::info!("🔍 Search display: processing {} results for display", results.len());
        
        // Convert results to HashSet for O(1) lookups
        let results_set: HashSet<&PathBuf> = results.iter().collect();
        
        // OPTIMIZATION: Pre-build directory indices for O(1) lookups
        // Previously, we checked if each directory contained results by iterating through
        // all search results for each directory (O(dirs × results) = potentially millions of comparisons).
        // Now we build the index once in O(results) time.
        let mut expanded_dirs = HashSet::new();
        let mut dirs_with_results = HashSet::new();
        
        for path in results {
            // Mark all ancestors as containing results AND needing expansion
            let mut current_parent = path.parent();
            while let Some(parent) = current_parent {
                if parent != std::path::Path::new("") && parent != std::path::Path::new(".") {
                    expanded_dirs.insert(parent.to_path_buf());
                    dirs_with_results.insert(parent.to_path_buf());
                    current_parent = parent.parent();
                } else {
                    break;
                }
            }
        }
        
        log::debug!("Search indices built: {} expanded dirs, {} dirs with results", 
                   expanded_dirs.len(), dirs_with_results.len());
        // for dir in &expanded_dirs {
        //     log::debug!("  📂 Expanded: {}", dir.display());
        // }
        
        // Process the tree and collect visible items
        let mut items = Vec::new();
        
        for node in &self.tree.root {
            if node.is_dir {
                // Check if directory contains matches using O(1) lookup
                if dirs_with_results.contains(&node.path) {
                    self.collect_search_visible_items_optimized(
                        node, &mut items, 0, &expanded_dirs, &results_set, &dirs_with_results, selection
                    );
                }
            } else if results_set.contains(&node.path) {
                // Show file if it's in the search results
                self.collect_search_visible_items_optimized(
                    node, &mut items, 0, &expanded_dirs, &results_set, &dirs_with_results, selection
                );
            }
        }
        
        let elapsed = start.elapsed();
        log::info!("📋 Search display: generated {} visible items in {:?}", items.len(), elapsed);
        
        // Log all computed display items for debugging
        // for (i, item) in items.iter().enumerate() {
        //     log::debug!("  📁 Display item {}: {} {} (depth: {}, dir: {}, selected: {})", 
        //                i + 1, 
        //                item.path.display(),
        //                if item.is_dir { "(dir)" } else { "(file)" },
        //                item.depth,
        //                item.is_dir,
        //                item.is_selected);
        // }
        
        items
    }
    
    
    /// Recursively collect visible items for search mode - OPTIMIZED with O(1) directory checks
    fn collect_search_visible_items_optimized(
        &self,
        node: &crate::tree::TreeNode,
        items: &mut Vec<VisibleItem>,
        depth: usize,
        expanded: &HashSet<PathBuf>,
        search_results: &HashSet<&PathBuf>,
        dirs_with_results: &HashSet<PathBuf>,
        selection: &Option<PathBuf>,
    ) {
        // Include this node if:
        // 1. It's a file that's in the search results, OR
        // 2. It's a directory that contains files in the search results
        let should_include = if node.is_dir {
            // O(1) lookup instead of O(n) string comparisons!
            dirs_with_results.contains(&node.path)
        } else {
            search_results.contains(&node.path)
        };
        
        if !should_include {
            return;
        }
        
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
                self.collect_search_visible_items_optimized(
                    child, items, depth + 1, expanded, search_results, dirs_with_results, selection
                );
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
    
    /// Ensure we have a valid selection if there are visible items
    fn ensure_valid_selection(&mut self) {
        let visible_items = self.get_current_visible_items();
        
        if visible_items.is_empty() {
            // No visible items, clear selection
            self.selection = None;
            return;
        }
        
        // Check if current selection is still visible
        if let Some(ref selection) = self.selection {
            let is_visible = visible_items.iter().any(|item| &item.path == selection);
            if is_visible {
                // Current selection is still valid, keep it
                return;
            }
        }
        
        // Either no selection or current selection is not visible
        // Select first visible item
        self.selection = Some(visible_items[0].path.clone());
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
        
        let original_view = navigator.build_view_model().clone();
        
        // Enter search mode
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        navigator.handle_event(NavigatorEvent::UpdateSearchQuery("cargo".to_string())).unwrap();
        
        // Selection should now be Cargo.toml (first search result)
        assert_eq!(navigator.get_selection(), Some(PathBuf::from("Cargo.toml")));
        
        // Exit search mode
        navigator.handle_event(NavigatorEvent::EndSearch).unwrap();
        
        // Selection should remain Cargo.toml (still valid in full tree)
        assert_eq!(navigator.get_selection(), Some(PathBuf::from("Cargo.toml")));
        assert!(!navigator.is_searching());
        
        let restored_view = navigator.build_view_model().clone();
        
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
        
        let view_model = navigator.build_view_model().clone();
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