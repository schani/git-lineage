use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a single node in the file tree
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub git_status: Option<char>,
    pub is_expanded: bool,
    pub children: Vec<TreeNode>,
    pub parent_path: Option<PathBuf>,
}

impl TreeNode {
    /// Create a new tree node
    pub fn new(name: String, path: PathBuf, is_dir: bool) -> Self {
        Self {
            name,
            path: path.clone(),
            is_dir,
            git_status: None,
            is_expanded: false,
            children: Vec::new(),
            parent_path: path.parent().map(|p| p.to_path_buf()),
        }
    }

    /// Create a new directory node
    pub fn new_dir(name: String, path: PathBuf) -> Self {
        Self::new(name, path, true)
    }

    /// Create a new file node
    pub fn new_file(name: String, path: PathBuf) -> Self {
        Self::new(name, path, false)
    }

    /// Set the git status for this node
    pub fn with_git_status(mut self, status: char) -> Self {
        self.git_status = Some(status);
        self
    }

    /// Add a child node
    pub fn add_child(&mut self, child: TreeNode) {
        if self.is_dir {
            self.children.push(child);
            // Keep children sorted: directories first, then files, both alphabetically
            self.children.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });
        }
    }

    /// Remove a child node by path
    pub fn remove_child(&mut self, path: &Path) -> Option<TreeNode> {
        if let Some(index) = self.children.iter().position(|child| child.path == path) {
            Some(self.children.remove(index))
        } else {
            None
        }
    }

    /// Find a child node by path
    pub fn find_child(&self, path: &Path) -> Option<&TreeNode> {
        self.children.iter().find(|child| child.path == path)
    }

    /// Find a child node by path (mutable)
    pub fn find_child_mut(&mut self, path: &Path) -> Option<&mut TreeNode> {
        self.children.iter_mut().find(|child| child.path == path)
    }

    /// Expand this directory node
    pub fn expand(&mut self) {
        if self.is_dir {
            self.is_expanded = true;
        }
    }

    /// Collapse this directory node
    pub fn collapse(&mut self) {
        if self.is_dir {
            self.is_expanded = false;
        }
    }

    /// Toggle expansion state
    pub fn toggle_expansion(&mut self) {
        if self.is_dir {
            self.is_expanded = !self.is_expanded;
        }
    }

    /// Check if this node has children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Get the depth of this node in the tree relative to the project root
    pub fn depth(&self) -> usize {
        // Handle paths that start with "./" - these should be treated as root level
        let path_str = self.path.to_string_lossy();
        if path_str.starts_with("./") {
            // Remove the "./" prefix and count remaining components
            let without_dot_slash = &path_str[2..];
            if without_dot_slash.is_empty() || !without_dot_slash.contains('/') {
                // "./src" or "./Cargo.toml" = root level = depth 0
                0
            } else {
                // "./src/main.rs" = count slashes for depth
                without_dot_slash.matches('/').count()
            }
        } else {
            // Fallback for other path formats
            let component_count = self.path.components().count();
            if component_count <= 1 {
                0
            } else {
                component_count - 1
            }
        }
    }
}

/// Manages the file tree structure and operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTree {
    pub root: Vec<TreeNode>,
    pub current_selection: Option<PathBuf>,
    pub git_status_map: HashMap<PathBuf, char>,
    #[serde(skip)]
    pub repo_root: PathBuf,
}

impl Default for FileTree {
    fn default() -> Self {
        Self::new()
    }
}

impl FileTree {
    /// Create a new empty file tree
    pub fn new() -> Self {
        Self {
            root: Vec::new(),
            current_selection: None,
            git_status_map: HashMap::new(),
            repo_root: PathBuf::new(),
        }
    }

    /// Build tree from a directory path
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let mut tree = Self::new();
        tree.repo_root = path.as_ref().to_path_buf();
        tree.scan_directory_with_gitignore(path.as_ref())?;
        Ok(tree)
    }

    /// Scan a directory and build the tree structure
    fn scan_directory(&mut self, dir_path: &Path) -> Result<(), std::io::Error> {
        let entries = fs::read_dir(dir_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files and directories (starting with .)
            if name.starts_with('.') {
                continue;
            }

            let is_dir = path.is_dir();
            let mut node = TreeNode::new(name, path.clone(), is_dir);

            // Apply git status if available
            if let Some(&status) = self.git_status_map.get(&path) {
                node.git_status = Some(status);
            }

            // Recursively scan subdirectories
            if is_dir {
                self.scan_directory_into_node(&mut node, &path)?;
            }

            self.root.push(node);
        }

        // Sort root level
        self.root.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(())
    }

    /// Scan directory contents into a specific node
    fn scan_directory_into_node(
        &mut self,
        parent: &mut TreeNode,
        dir_path: &Path,
    ) -> Result<(), std::io::Error> {
        let entries = fs::read_dir(dir_path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files and directories
            if name.starts_with('.') {
                continue;
            }

            let is_dir = path.is_dir();
            let mut node = TreeNode::new(name, path.clone(), is_dir);

            // Apply git status if available
            if let Some(&status) = self.git_status_map.get(&path) {
                node.git_status = Some(status);
            }

            // Recursively scan subdirectories
            if is_dir {
                self.scan_directory_into_node(&mut node, &path)?;
            }

            parent.add_child(node);
        }

        Ok(())
    }

    /// Scan a directory with gitignore filtering using the ignore crate
    fn scan_directory_with_gitignore(&mut self, dir_path: &Path) -> Result<(), std::io::Error> {
        // Use ignore crate's WalkBuilder for efficient gitignore-aware traversal
        let walk = WalkBuilder::new(dir_path)
            .max_depth(Some(1)) // Only get immediate children for this directory
            .hidden(false) // We'll handle hidden files manually
            .git_ignore(true) // Respect .gitignore files
            .git_global(true) // Respect global git ignore
            .git_exclude(true) // Respect .git/info/exclude
            .parents(true) // Look at parent directories for gitignore files
            .build();

        let mut entries = Vec::new();

        for result in walk {
            match result {
                Ok(entry) => {
                    let path = entry.path();

                    // Skip the root directory itself
                    if path == dir_path {
                        continue;
                    }

                    // Skip hidden files and directories (starting with .)
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with('.') {
                            continue;
                        }
                    }

                    entries.push(path.to_path_buf());
                }
                Err(err) => {
                    eprintln!("Warning: Error walking directory: {}", err);
                    continue;
                }
            }
        }

        // Process the collected entries
        for path in entries {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            // Convert absolute path to relative path from repo root
            let relative_path = match path.strip_prefix(&self.repo_root) {
                Ok(rel_path) => rel_path.to_path_buf(),
                Err(_) => path.clone(), // Fallback to absolute path if strip fails
            };

            let is_dir = path.is_dir();
            let mut node = TreeNode::new(name, relative_path.clone(), is_dir);

            // Apply git status if available (using original absolute path for git status lookup)
            if let Some(&status) = self.git_status_map.get(&path) {
                node.git_status = Some(status);
            }

            // Recursively scan subdirectories with gitignore filtering
            if is_dir {
                self.scan_directory_into_node_with_gitignore(&mut node, &path)?;
            }

            self.root.push(node);
        }

        // Sort root level
        self.root.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        Ok(())
    }

    /// Scan directory contents into a specific node with gitignore filtering
    fn scan_directory_into_node_with_gitignore(
        &mut self,
        parent: &mut TreeNode,
        dir_path: &Path,
    ) -> Result<(), std::io::Error> {
        // Use ignore crate's WalkBuilder for this subdirectory
        let walk = WalkBuilder::new(dir_path)
            .max_depth(Some(1)) // Only get immediate children
            .hidden(false) // We'll handle hidden files manually
            .git_ignore(true) // Respect .gitignore files
            .git_global(true) // Respect global git ignore
            .git_exclude(true) // Respect .git/info/exclude
            .parents(true) // Look at parent directories for gitignore files
            .build();

        let mut entries = Vec::new();

        for result in walk {
            match result {
                Ok(entry) => {
                    let path = entry.path();

                    // Skip the directory itself
                    if path == dir_path {
                        continue;
                    }

                    // Skip hidden files and directories (starting with .)
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with('.') {
                            continue;
                        }
                    }

                    entries.push(path.to_path_buf());
                }
                Err(err) => {
                    eprintln!("Warning: Error walking directory: {}", err);
                    continue;
                }
            }
        }

        // Process the collected entries
        for path in entries {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            // Convert absolute path to relative path from repo root
            let relative_path = match path.strip_prefix(&self.repo_root) {
                Ok(rel_path) => rel_path.to_path_buf(),
                Err(_) => path.clone(), // Fallback to absolute path if strip fails
            };

            let is_dir = path.is_dir();
            let mut node = TreeNode::new(name, relative_path, is_dir);

            // Apply git status if available (using original absolute path for git status lookup)
            if let Some(&status) = self.git_status_map.get(&path) {
                node.git_status = Some(status);
            }

            // Recursively scan subdirectories with gitignore filtering
            if is_dir {
                self.scan_directory_into_node_with_gitignore(&mut node, &path)?;
            }

            parent.add_child(node);
        }

        Ok(())
    }

    /// Set git status information for the tree
    pub fn set_git_status(&mut self, status_map: HashMap<PathBuf, char>) {
        self.git_status_map = status_map;
        self.apply_git_status_to_tree();
    }

    /// Apply git status to all nodes in the tree
    fn apply_git_status_to_tree(&mut self) {
        let git_status_map = self.git_status_map.clone();
        for node in &mut self.root {
            Self::apply_git_status_to_node_static(node, &git_status_map);
        }
    }

    /// Recursively apply git status to a node and its children (static version)
    fn apply_git_status_to_node_static(
        node: &mut TreeNode,
        git_status_map: &HashMap<PathBuf, char>,
    ) {
        if let Some(&status) = git_status_map.get(&node.path) {
            node.git_status = Some(status);
        }

        for child in &mut node.children {
            Self::apply_git_status_to_node_static(child, git_status_map);
        }
    }

    /// Find a node by path
    pub fn find_node(&self, path: &Path) -> Option<&TreeNode> {
        for node in &self.root {
            if let Some(found) = self.find_node_recursive(node, path) {
                return Some(found);
            }
        }
        None
    }

    /// Find a node by path (mutable)
    pub fn find_node_mut(&mut self, path: &Path) -> Option<&mut TreeNode> {
        for node in &mut self.root {
            if let Some(found) = Self::find_node_recursive_mut_static(node, path) {
                return Some(found);
            }
        }
        None
    }

    /// Recursively search for a node
    fn find_node_recursive<'a>(&self, node: &'a TreeNode, path: &Path) -> Option<&'a TreeNode> {
        if node.path == path {
            return Some(node);
        }

        for child in &node.children {
            if let Some(found) = self.find_node_recursive(child, path) {
                return Some(found);
            }
        }

        None
    }

    /// Recursively search for a node (mutable, static version)
    fn find_node_recursive_mut_static<'a>(
        node: &'a mut TreeNode,
        path: &Path,
    ) -> Option<&'a mut TreeNode> {
        if node.path == path {
            return Some(node);
        }

        for child in &mut node.children {
            if let Some(found) = Self::find_node_recursive_mut_static(child, path) {
                return Some(found);
            }
        }

        None
    }

    /// Expand a directory node
    pub fn expand_node(&mut self, path: &Path) -> bool {
        if let Some(node) = self.find_node_mut(path) {
            if node.is_dir {
                node.expand();
                return true;
            }
        }
        false
    }

    /// Collapse a directory node
    pub fn collapse_node(&mut self, path: &Path) -> bool {
        if let Some(node) = self.find_node_mut(path) {
            if node.is_dir {
                node.collapse();
                return true;
            }
        }
        false
    }

    /// Toggle expansion of a directory node
    pub fn toggle_node(&mut self, path: &Path) -> bool {
        if let Some(node) = self.find_node_mut(path) {
            if node.is_dir {
                node.toggle_expansion();
                return true;
            }
        }
        false
    }

    /// Select a node
    pub fn select_node(&mut self, path: &Path) -> bool {
        if self.find_node(path).is_some() {
            self.current_selection = Some(path.to_path_buf());
            true
        } else {
            false
        }
    }

    /// Get the currently selected node
    pub fn get_selected_node(&self) -> Option<&TreeNode> {
        self.current_selection
            .as_ref()
            .and_then(|path| self.find_node(path))
    }

    /// Get all visible nodes (flattened view respecting expansion state)
    pub fn get_visible_nodes(&self) -> Vec<&TreeNode> {
        let mut visible = Vec::new();
        for node in &self.root {
            self.collect_visible_nodes(node, &mut visible);
        }
        visible
    }

    /// Get visible nodes with their display depth (how deep they appear in the UI)
    pub fn get_visible_nodes_with_depth(&self) -> Vec<(&TreeNode, usize)> {
        let mut visible = Vec::new();
        for node in &self.root {
            self.collect_visible_nodes_with_depth(node, &mut visible, 0);
        }
        visible
    }

    /// Recursively collect visible nodes
    fn collect_visible_nodes<'a>(&self, node: &'a TreeNode, visible: &mut Vec<&'a TreeNode>) {
        visible.push(node);

        if node.is_dir && node.is_expanded {
            for child in &node.children {
                self.collect_visible_nodes(child, visible);
            }
        }
    }

    /// Recursively collect visible nodes with their display depth
    fn collect_visible_nodes_with_depth<'a>(
        &self,
        node: &'a TreeNode,
        visible: &mut Vec<(&'a TreeNode, usize)>,
        depth: usize,
    ) {
        visible.push((node, depth));

        if node.is_dir && node.is_expanded {
            for child in &node.children {
                self.collect_visible_nodes_with_depth(child, visible, depth + 1);
            }
        }
    }

    /// Get the next visible node after the current selection
    pub fn get_next_node(&self) -> Option<&TreeNode> {
        let visible = self.get_visible_nodes();
        if let Some(current_path) = &self.current_selection {
            if let Some(current_index) = visible.iter().position(|node| &node.path == current_path)
            {
                if current_index + 1 < visible.len() {
                    return Some(visible[current_index + 1]);
                }
            }
        }
        None
    }

    /// Get the previous visible node before the current selection
    pub fn get_previous_node(&self) -> Option<&TreeNode> {
        let visible = self.get_visible_nodes();
        if let Some(current_path) = &self.current_selection {
            if let Some(current_index) = visible.iter().position(|node| &node.path == current_path)
            {
                if current_index > 0 {
                    return Some(visible[current_index - 1]);
                }
            }
        }
        None
    }

    /// Navigate to the next node
    pub fn navigate_down(&mut self) -> bool {
        if let Some(next_node) = self.get_next_node() {
            self.current_selection = Some(next_node.path.clone());
            true
        } else {
            false
        }
    }

    /// Navigate to the previous node
    pub fn navigate_up(&mut self) -> bool {
        if let Some(prev_node) = self.get_previous_node() {
            self.current_selection = Some(prev_node.path.clone());
            true
        } else {
            false
        }
    }

    /// Get the first visible node
    pub fn get_first_node(&self) -> Option<&TreeNode> {
        self.get_visible_nodes().first().copied()
    }

    /// Get the last visible node
    pub fn get_last_node(&self) -> Option<&TreeNode> {
        self.get_visible_nodes().last().copied()
    }

    /// Navigate to the first node
    pub fn navigate_to_first(&mut self) -> bool {
        if let Some(first_node) = self.get_first_node() {
            self.current_selection = Some(first_node.path.clone());
            true
        } else {
            false
        }
    }

    /// Navigate to the last node
    pub fn navigate_to_last(&mut self) -> bool {
        if let Some(last_node) = self.get_last_node() {
            self.current_selection = Some(last_node.path.clone());
            true
        } else {
            false
        }
    }

    /// Filter nodes by search query
    pub fn filter_nodes(&self, query: &str) -> Vec<&TreeNode> {
        let mut results = Vec::new();
        let lower_query = query.to_lowercase();

        for node in &self.root {
            self.filter_nodes_recursive(node, &lower_query, &mut results);
        }

        results
    }

    /// Recursively filter nodes
    fn filter_nodes_recursive<'a>(
        &self,
        node: &'a TreeNode,
        query: &str,
        results: &mut Vec<&'a TreeNode>,
    ) {
        if node.name.to_lowercase().contains(query) {
            results.push(node);
        }

        for child in &node.children {
            self.filter_nodes_recursive(child, query, results);
        }
    }

    /// Get tree statistics
    pub fn get_stats(&self) -> TreeStats {
        let mut stats = TreeStats::default();
        for node in &self.root {
            self.collect_stats(node, &mut stats);
        }
        stats
    }

    /// Recursively collect tree statistics
    fn collect_stats(&self, node: &TreeNode, stats: &mut TreeStats) {
        if node.is_dir {
            stats.directories += 1;
            if node.is_expanded {
                stats.expanded_directories += 1;
            }
        } else {
            stats.files += 1;
        }

        if node.git_status.is_some() {
            stats.files_with_git_status += 1;
        }

        stats.total_nodes += 1;
        stats.max_depth = stats.max_depth.max(node.depth());

        for child in &node.children {
            self.collect_stats(child, stats);
        }
    }
}

/// Statistics about the file tree
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct TreeStats {
    pub total_nodes: usize,
    pub files: usize,
    pub directories: usize,
    pub expanded_directories: usize,
    pub files_with_git_status: usize,
    pub max_depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_tree_node_creation() {
        let file_node = TreeNode::new_file("test.rs".to_string(), PathBuf::from("src/test.rs"));
        assert_eq!(file_node.name, "test.rs");
        assert_eq!(file_node.path, PathBuf::from("src/test.rs"));
        assert!(!file_node.is_dir);
        assert!(!file_node.is_expanded);
        assert!(file_node.children.is_empty());

        let dir_node = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        assert_eq!(dir_node.name, "src");
        assert_eq!(dir_node.path, PathBuf::from("src"));
        assert!(dir_node.is_dir);
        assert!(!dir_node.is_expanded);
        assert!(dir_node.children.is_empty());
    }

    #[test]
    fn test_tree_node_with_git_status() {
        let node = TreeNode::new_file("test.rs".to_string(), PathBuf::from("test.rs"))
            .with_git_status('M');
        assert_eq!(node.git_status, Some('M'));
    }

    #[test]
    fn test_tree_node_add_child() {
        let mut parent = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        let child1 = TreeNode::new_file("main.rs".to_string(), PathBuf::from("src/main.rs"));
        let child2 = TreeNode::new_file("lib.rs".to_string(), PathBuf::from("src/lib.rs"));
        let child3 = TreeNode::new_dir("utils".to_string(), PathBuf::from("src/utils"));

        parent.add_child(child1);
        parent.add_child(child2);
        parent.add_child(child3);

        assert_eq!(parent.children.len(), 3);
        // Should be sorted: directories first, then files alphabetically
        assert_eq!(parent.children[0].name, "utils"); // directory first
        assert_eq!(parent.children[1].name, "lib.rs"); // files alphabetically
        assert_eq!(parent.children[2].name, "main.rs");
    }

    #[test]
    fn test_tree_node_expansion() {
        let mut node = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        assert!(!node.is_expanded);

        node.expand();
        assert!(node.is_expanded);

        node.collapse();
        assert!(!node.is_expanded);

        node.toggle_expansion();
        assert!(node.is_expanded);

        node.toggle_expansion();
        assert!(!node.is_expanded);
    }

    #[test]
    fn test_tree_node_find_child() {
        let mut parent = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        let child = TreeNode::new_file("main.rs".to_string(), PathBuf::from("src/main.rs"));
        let child_path = child.path.clone();

        parent.add_child(child);

        let found = parent.find_child(&child_path);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "main.rs");

        let not_found = parent.find_child(&PathBuf::from("src/nonexistent.rs"));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_tree_node_remove_child() {
        let mut parent = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        let child = TreeNode::new_file("main.rs".to_string(), PathBuf::from("src/main.rs"));
        let child_path = child.path.clone();

        parent.add_child(child);
        assert_eq!(parent.children.len(), 1);

        let removed = parent.remove_child(&child_path);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "main.rs");
        assert_eq!(parent.children.len(), 0);
    }

    #[test]
    fn test_file_tree_creation() {
        let tree = FileTree::new();
        assert!(tree.root.is_empty());
        assert!(tree.current_selection.is_none());
        assert!(tree.git_status_map.is_empty());
    }

    #[test]
    fn test_file_tree_navigation() {
        let mut tree = FileTree::new();

        // Create a simple tree structure
        let mut root_dir = TreeNode::new_dir("project".to_string(), PathBuf::from("project"));
        root_dir.expand(); // Expand to make children visible
        root_dir.add_child(TreeNode::new_file(
            "README.md".to_string(),
            PathBuf::from("project/README.md"),
        ));
        root_dir.add_child(TreeNode::new_file(
            "Cargo.toml".to_string(),
            PathBuf::from("project/Cargo.toml"),
        ));

        let mut src_dir = TreeNode::new_dir("src".to_string(), PathBuf::from("project/src"));
        src_dir.add_child(TreeNode::new_file(
            "main.rs".to_string(),
            PathBuf::from("project/src/main.rs"),
        ));
        root_dir.add_child(src_dir);

        tree.root.push(root_dir);

        // Test initial selection
        let first_node_path = tree.get_first_node().unwrap().path.clone();
        tree.select_node(&first_node_path);
        assert_eq!(tree.current_selection, Some(PathBuf::from("project")));

        // Test navigation (children are now visible because directory is expanded)
        // The order should be: project -> src (directory first) -> Cargo.toml -> README.md
        assert!(tree.navigate_down());
        assert_eq!(tree.current_selection, Some(PathBuf::from("project/src")));

        assert!(tree.navigate_down());
        assert_eq!(
            tree.current_selection,
            Some(PathBuf::from("project/Cargo.toml"))
        );

        assert!(tree.navigate_down());
        assert_eq!(
            tree.current_selection,
            Some(PathBuf::from("project/README.md"))
        );

        assert!(tree.navigate_up());
        assert_eq!(
            tree.current_selection,
            Some(PathBuf::from("project/Cargo.toml"))
        );
    }

    #[test]
    fn test_file_tree_expansion() {
        let mut tree = FileTree::new();

        let mut root_dir = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        root_dir.add_child(TreeNode::new_file(
            "main.rs".to_string(),
            PathBuf::from("src/main.rs"),
        ));
        tree.root.push(root_dir);

        let src_path = PathBuf::from("src");

        // Initially collapsed
        let visible_before = tree.get_visible_nodes();
        assert_eq!(visible_before.len(), 1);
        assert_eq!(visible_before[0].name, "src");

        // Expand the directory
        assert!(tree.expand_node(&src_path));

        let visible_after = tree.get_visible_nodes();
        assert_eq!(visible_after.len(), 2);
        assert_eq!(visible_after[0].name, "src");
        assert_eq!(visible_after[1].name, "main.rs");

        // Collapse the directory
        assert!(tree.collapse_node(&src_path));

        let visible_collapsed = tree.get_visible_nodes();
        assert_eq!(visible_collapsed.len(), 1);
        assert_eq!(visible_collapsed[0].name, "src");
    }

    #[test]
    fn test_file_tree_git_status() {
        let mut tree = FileTree::new();

        let root = TreeNode::new_file("main.rs".to_string(), PathBuf::from("main.rs"));
        tree.root.push(root);

        let mut git_status = HashMap::new();
        git_status.insert(PathBuf::from("main.rs"), 'M');

        tree.set_git_status(git_status);

        let node = tree.find_node(&PathBuf::from("main.rs")).unwrap();
        assert_eq!(node.git_status, Some('M'));
    }

    #[test]
    fn test_file_tree_search() {
        let mut tree = FileTree::new();

        tree.root.push(TreeNode::new_file(
            "main.rs".to_string(),
            PathBuf::from("main.rs"),
        ));
        tree.root.push(TreeNode::new_file(
            "lib.rs".to_string(),
            PathBuf::from("lib.rs"),
        ));
        tree.root.push(TreeNode::new_file(
            "config.toml".to_string(),
            PathBuf::from("config.toml"),
        ));

        let results = tree.filter_nodes("rs");
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|n| n.name == "main.rs"));
        assert!(results.iter().any(|n| n.name == "lib.rs"));

        let results = tree.filter_nodes("config");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "config.toml");

        let results = tree.filter_nodes("nonexistent");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_file_tree_stats() {
        let mut tree = FileTree::new();

        let mut root_dir = TreeNode::new_dir("project".to_string(), PathBuf::from("project"));
        root_dir.expand();

        let mut src_dir = TreeNode::new_dir("src".to_string(), PathBuf::from("project/src"));
        src_dir.add_child(
            TreeNode::new_file("main.rs".to_string(), PathBuf::from("project/src/main.rs"))
                .with_git_status('M'),
        );
        src_dir.add_child(TreeNode::new_file(
            "lib.rs".to_string(),
            PathBuf::from("project/src/lib.rs"),
        ));

        root_dir.add_child(src_dir);
        root_dir.add_child(TreeNode::new_file(
            "README.md".to_string(),
            PathBuf::from("project/README.md"),
        ));

        tree.root.push(root_dir);

        let stats = tree.get_stats();
        assert_eq!(stats.total_nodes, 5);
        assert_eq!(stats.files, 3);
        assert_eq!(stats.directories, 2);
        assert_eq!(stats.expanded_directories, 1);
        assert_eq!(stats.files_with_git_status, 1);
        assert_eq!(stats.max_depth, 2);
    }

    #[test]
    fn test_tree_node_depth() {
        let root = TreeNode::new_dir("project".to_string(), PathBuf::from("project"));
        assert_eq!(root.depth(), 0);

        let src = TreeNode::new_dir("src".to_string(), PathBuf::from("project/src"));
        assert_eq!(src.depth(), 1);

        let main = TreeNode::new_file("main.rs".to_string(), PathBuf::from("project/src/main.rs"));
        assert_eq!(main.depth(), 2);
    }

    #[test]
    fn test_find_node() {
        let mut tree = FileTree::new();

        let mut root = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        root.add_child(TreeNode::new_file(
            "main.rs".to_string(),
            PathBuf::from("src/main.rs"),
        ));
        tree.root.push(root);

        let found = tree.find_node(&PathBuf::from("src"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "src");

        let found_child = tree.find_node(&PathBuf::from("src/main.rs"));
        assert!(found_child.is_some());
        assert_eq!(found_child.unwrap().name, "main.rs");

        let not_found = tree.find_node(&PathBuf::from("nonexistent"));
        assert!(not_found.is_none());
    }
}
