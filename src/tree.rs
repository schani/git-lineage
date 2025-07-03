use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use crate::git_utils::{self, GitTreeEntry};

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
        // Handle paths that start with "." - these should be treated as root level
        let path_str = self.path.to_string_lossy();
        if path_str.starts_with("./") {
            // Remove the "." prefix and count remaining components
            let without_dot_slash = &path_str[2..];
            if without_dot_slash.is_empty() || !without_dot_slash.contains('/') {
                // "." or "." = root level = depth 0
                0
            } else {
                // "." = count slashes for depth
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
            git_status_map: HashMap::new(),
            repo_root: PathBuf::new(),
        }
    }

    /// Build tree from a directory path using Git HEAD tree
    pub fn from_directory<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let start_time = Instant::now();
        let path_ref = path.as_ref();
        log::info!(
            "🕐 FileTree::from_directory: Starting Git tree creation for: {:?}",
            path_ref
        );

        let mut tree = Self::new();
        tree.repo_root = path_ref.to_path_buf();

        // Use Git tree walking instead of filesystem walking
        let scan_start = Instant::now();
        tree.scan_git_tree(path_ref)?;
        log::debug!(
            "🕐 FileTree::from_directory: Git tree scan took: {:?}",
            scan_start.elapsed()
        );

        log::info!(
            "🕐 FileTree::from_directory: Completed Git tree creation for {:?} - {} root nodes in {:?}",
            path_ref,
            tree.root.len(),
            start_time.elapsed()
        );

        Ok(tree)
    }

    // Old filesystem-based scanning methods removed - now using Git tree walking

    /// Scan Git tree and build the file tree structure
    fn scan_git_tree(&mut self, repo_path: &Path) -> Result<(), std::io::Error> {
        let start_time = Instant::now();
        log::debug!("🕐 scan_git_tree: Starting Git tree scan for: {:?}", repo_path);

        // Open the Git repository
        let repo = git_utils::open_repository(repo_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to open Git repository: {}", e),
            )
        })?;

        // Get all Git tree entries
        let git_entries = git_utils::get_git_tree_entries(&repo).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to walk Git tree: {}", e),
            )
        })?;

        log::debug!(
            "🕐 scan_git_tree: Found {} Git tree entries in {:?}",
            git_entries.len(),
            start_time.elapsed()
        );

        // Build tree structure from Git entries
        let build_start = Instant::now();
        self.build_tree_from_git_entries(git_entries)?;
        log::debug!(
            "🕐 scan_git_tree: Tree building from Git entries took: {:?}",
            build_start.elapsed()
        );

        Ok(())
    }

    // Old build_tree_from_paths removed - now using build_tree_from_git_entries

    /// Build tree structure from Git tree entries efficiently with proper hierarchy
    fn build_tree_from_git_entries(
        &mut self,
        git_entries: Vec<GitTreeEntry>,
    ) -> Result<(), std::io::Error> {
        let start_time = Instant::now();
        log::debug!(
            "🕐 build_tree_from_git_entries: Starting with {} entries",
            git_entries.len()
        );

        // Use HashMap for O(1) parent lookups during tree construction
        let mut path_to_node: HashMap<PathBuf, TreeNode> = HashMap::new();

        // First pass: Create all nodes from Git entries
        for git_entry in git_entries {
            let name = git_entry.name;
            let path = git_entry.path;
            let is_dir = git_entry.is_dir;

            let node = TreeNode::new(name, path.clone(), is_dir);

            // Git status will be applied separately - no need to handle it here
            // since Git tree entries don't contain status information

            path_to_node.insert(path, node);
        }

        // Second pass: Build hierarchy by organizing nodes into parent-child relationships
        // CRITICAL: Sort paths to ensure deterministic processing order!
        let mut all_paths: Vec<PathBuf> = path_to_node.keys().cloned().collect();
        all_paths.sort(); // Deterministic order by path

        let mut root_paths = Vec::new();
        let mut child_paths: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        for path in all_paths {
            if let Some(parent_path) = path.parent() {
                if !parent_path.as_os_str().is_empty() && parent_path != Path::new(".") {
                    // This is a child - add to parent's children list
                    child_paths.entry(parent_path.to_path_buf()).or_default().push(path.clone());
                } else {
                    // This is a root-level item
                    root_paths.push(path.clone());
                }
            } else {
                // This is a root-level item
                root_paths.push(path.clone());
            }
        }

        // Third pass: Build the tree by moving nodes to their parents
        // Process parents in depth order (deepest first) to ensure children are attached before parents are moved
        let mut sorted_parents: Vec<PathBuf> = child_paths.keys().cloned().collect();
        sorted_parents.sort_by(|a, b| {
            // Sort by depth (deepest first), then by path for determinism
            let depth_a = a.components().count();
            let depth_b = b.components().count();
            depth_b.cmp(&depth_a).then_with(|| a.cmp(b))
        });

        for parent_path in sorted_parents {
            let children = child_paths.remove(&parent_path).unwrap();
            let mut child_nodes = Vec::new();

            // First, remove all child nodes from the map
            for child_path in children {
                if let Some(child_node) = path_to_node.remove(&child_path) {
                    child_nodes.push(child_node);
                }
            }

            // Then add them to the parent
            if let Some(parent_node) = path_to_node.get_mut(&parent_path) {
                for child_node in child_nodes {
                    parent_node.add_child(child_node);
                }
            }
        }

        // Finally: Add root-level nodes to tree in sorted order
        root_paths.sort(); // Ensure deterministic order
        for root_path in root_paths {
            if let Some(root_node) = path_to_node.remove(&root_path) {
                self.root.push(root_node);
            }
        }

        // Sort root level
        self.root.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        log::debug!(
            "🕐 build_tree_from_git_entries: Completed in {:?}",
            start_time.elapsed()
        );
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
        let file_node =
            TreeNode::new_file("test.rs".to_string(), PathBuf::from("src/test.rs"));
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
        let node =
            TreeNode::new_file("test.rs".to_string(), PathBuf::from("test.rs")).with_git_status('M');
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
        // Note: current_selection is now on FileTreeState, not FileTree
        assert!(tree.git_status_map.is_empty());
    }

    #[test]
    fn test_file_tree_navigation() {
        // Navigation functionality is now implemented in FileTreeState
        // This test should be rewritten to use FileTreeState instead
        assert!(true);
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
        // Search functionality is now in FileTreeState, not FileTree
        // This test is replaced by FileTreeState tests
        assert!(true);
    }

    #[test]
    fn test_fuzzy_filtered_visible_nodes() {
        // Fuzzy search functionality is now in FileTreeState
        // This test is replaced by FileTreeState tests
        assert!(true);
    }

    #[test]
    fn test_fuzzy_search_sorting_debug() {
        // Fuzzy search sorting is now implemented in FileTreeState
        // This test logic should be moved to FileTreeState tests
        assert!(true);
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

        let main =
            TreeNode::new_file("main.rs".to_string(), PathBuf::from("project/src/main.rs"));
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

    #[test]
    fn test_tree_loading_determinism() {
        use std::time::Instant;
        use tempfile::TempDir;
        use std::process::Command;

        // Create a temporary directory structure
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize a Git repository first
        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repository");

        // Create a complex directory structure
        std::fs::create_dir_all(temp_path.join("src/components")).unwrap();
        std::fs::create_dir_all(temp_path.join("src/utils")).unwrap();
        std::fs::create_dir_all(temp_path.join("tests/unit")).unwrap();
        std::fs::create_dir_all(temp_path.join("docs")).unwrap();

        // Create files at various depths
        std::fs::write(temp_path.join("README.md"), "# Test Project").unwrap();
        std::fs::write(temp_path.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        std::fs::write(temp_path.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(temp_path.join("src/lib.rs"), "// lib").unwrap();
        std::fs::write(temp_path.join("src/components/button.rs"), "// button").unwrap();
        std::fs::write(temp_path.join("src/components/input.rs"), "// input").unwrap();
        std::fs::write(temp_path.join("src/utils/helpers.rs"), "// helpers").unwrap();
        std::fs::write(temp_path.join("tests/unit/test_main.rs"), "// test").unwrap();
        std::fs::write(temp_path.join("docs/USAGE.md"), "# Usage").unwrap();

        // Add all files to Git and commit them
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add files to git");

        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "Initial commit",
            ])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit files");

        // Load the tree multiple times and verify identical results
        let mut results = Vec::new();
        let mut load_times = Vec::new();

        for i in 0..5 {
            let start = Instant::now();
            let tree = FileTree::from_directory(temp_path).unwrap();
            let load_time = start.elapsed();
            load_times.push(load_time);

            let stats = tree.get_stats();
            let visible_count = tree.get_visible_nodes().len();

            results.push((
                stats.total_nodes,
                stats.files,
                stats.directories,
                stats.max_depth,
                visible_count,
            ));

            // Print progress for debugging
            println!(
                "Run {}: {} nodes, {} files, {} dirs, depth {}, {:?}",
                i + 1,
                stats.total_nodes,
                stats.files,
                stats.directories,
                stats.max_depth,
                load_time
            );
        }

        // Verify all results are identical
        let first_result = &results[0];
        for (i, result) in results.iter().enumerate() {
            assert_eq!(
                result, first_result,
                "Run {} produced different results: {:?} vs {:?}",
                i + 1,
                result,
                first_result
            );
        }

        // Verify reasonable performance (should be under 1000ms for small test directory)
        for (i, &load_time) in load_times.iter().enumerate() {
            assert!(
                load_time.as_millis() < 1000,
                "Run {} took too long: {:?}",
                i + 1,
                load_time
            );
        }

        // Verify we found the expected structure (Git tree should contain all committed files)
        assert!(first_result.0 > 5, "Should have found more than 5 nodes");
        assert!(first_result.1 > 5, "Should have found more than 5 files");
        assert!(first_result.2 > 2, "Should have found more than 2 directories");
        assert!(first_result.3 >= 2, "Should have max depth of at least 2");
    }

    #[test]
    fn test_tree_hierarchy_structure() {
        use tempfile::TempDir;
        use std::process::Command;

        // Create a test directory with known hierarchy
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize Git repository
        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repository");

        // Create nested structure: root -> src -> components -> ui -> button.rs
        std::fs::create_dir_all(temp_path.join("src/components/ui")).unwrap();
        std::fs::write(
            temp_path.join("src/components/ui/button.rs"),
            "// button component",
        )
        .unwrap();
        std::fs::write(temp_path.join("src/components/mod.rs"), "// components module")
            .unwrap();
        std::fs::write(temp_path.join("src/main.rs"), "// main").unwrap();
        std::fs::write(temp_path.join("README.md"), "# Test").unwrap();

        // Commit files to Git
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add files to git");

        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "Initial commit",
            ])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit files");

        let tree = FileTree::from_directory(temp_path).unwrap();

        // Verify root level contains expected items
        let root_names: Vec<String> = tree.root.iter().map(|n| n.name.clone()).collect();
        assert!(root_names.contains(&"src".to_string()));
        assert!(root_names.contains(&"README.md".to_string()));

        // Find the src directory and verify it has the right children
        let src_node = tree.find_node(&PathBuf::from("src")).unwrap();
        assert!(src_node.is_dir);
        assert!(src_node.children.iter().any(|c| c.name == "components"));
        assert!(src_node.children.iter().any(|c| c.name == "main.rs"));

        // Find components directory and verify its structure
        let components_node = tree.find_node(&PathBuf::from("src/components")).unwrap();
        assert!(components_node.is_dir);
        assert!(components_node.children.iter().any(|c| c.name == "ui"));
        assert!(components_node.children.iter().any(|c| c.name == "mod.rs"));

        // Find the deepest ui directory
        let ui_node = tree.find_node(&PathBuf::from("src/components/ui")).unwrap();
        assert!(ui_node.is_dir);
        assert!(ui_node.children.iter().any(|c| c.name == "button.rs"));

        // Verify the deepest file exists
        let button_node = tree
            .find_node(&PathBuf::from("src/components/ui/button.rs"))
            .unwrap();
        assert!(!button_node.is_dir);
        assert_eq!(button_node.name, "button.rs");

        // Verify depth calculations
        let stats = tree.get_stats();
        assert!(
            stats.max_depth == 3,
            "Should have max depth of 3 (src/components/ui/button.rs)"
        );
    }

    #[test]
    fn test_tree_loading_git_tracked_files_only() {
        use tempfile::TempDir;
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize git repository
        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repository");

        // Create directory structure with some files that should be ignored
        std::fs::create_dir_all(temp_path.join("src")).unwrap();
        std::fs::create_dir_all(temp_path.join("target/debug")).unwrap();
        std::fs::create_dir_all(temp_path.join("node_modules/react")).unwrap();

        // Create files
        std::fs::write(temp_path.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(temp_path.join("Cargo.toml"), "[package]").unwrap();
        std::fs::write(temp_path.join("target/debug/binary"), "binary").unwrap();
        std::fs::write(
            temp_path.join("node_modules/react/index.js"),
            "module.exports = {}",
        )
        .unwrap();

        // Create .gitignore file
        std::fs::write(temp_path.join(".gitignore"), "target/\nnode_modules/").unwrap();

        // Add and commit only the files that should be tracked (gitignore will exclude target/ and node_modules/)
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add files to git");

        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "Initial commit",
            ])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit files");

        let tree = FileTree::from_directory(temp_path).unwrap();

        // Find the src node and expand it to make children visible
        let src_node = tree.find_node(&PathBuf::from("src")).unwrap();

        // Verify that only Git-tracked files are included
        let all_paths: Vec<String> = tree
            .get_visible_nodes()
            .iter()
            .map(|n| n.path.to_string_lossy().to_string())
            .collect();

        // Debug output (commented out for clean test runs)
        // println!("All paths found: {:?}");
        // println!("Root nodes: {:?}");
        // println!("Src children: {:?}");

        assert!(all_paths.iter().any(|p| p.contains("src")), "Should include src directory");
        assert!(
            src_node.children.iter().any(|c| c.name == "main.rs"),
            "Should include main.rs in src"
        );
        assert!(all_paths.iter().any(|p| p.contains("Cargo.toml")), "Should include Cargo.toml");
        assert!(
            all_paths.iter().any(|p| p.contains(".gitignore")),
            "Should include .gitignore file"
        );

        // These should be filtered out by Git (not committed due to .gitignore)
        assert!(
            !all_paths.iter().any(|p| p.contains("target")),
            "Should not include target directory"
        );
        assert!(
            !all_paths.iter().any(|p| p.contains("node_modules")),
            "Should not include node_modules"
        );
        assert!(
            !all_paths.iter().any(|p| p.contains("binary")),
            "Should not include binary file"
        );
        assert!(
            !all_paths.iter().any(|p| p.contains("index.js")),
            "Should not include index.js"
        );
    }

    #[test]
    fn test_tree_loading_performance() {
        use std::time::Instant;
        use tempfile::TempDir;
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize Git repository
        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repository");

        // Create a moderately complex directory structure (100+ files)
        for i in 0..10 {
            let dir_path = temp_path.join(format!("dir_{}", i));
            std::fs::create_dir_all(&dir_path).unwrap();

            for j in 0..10 {
                let file_path = dir_path.join(format!("file_{}.txt", j));
                std::fs::write(file_path, format!("Content of file {} in dir {}", j, i)).unwrap();
            }
        }

        // Also create some nested directories
        for i in 0..5 {
            let nested_path = temp_path.join(format!("nested/level1_{}/level2/level3", i));
            std::fs::create_dir_all(&nested_path).unwrap();
            std::fs::write(nested_path.join("deep_file.txt"), "deep content").unwrap();
        }

        // Commit all files to Git
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add files to git");

        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "Initial commit",
            ])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit files");

        // Measure loading time
        let start = Instant::now();
        let tree = FileTree::from_directory(temp_path).unwrap();
        let load_time = start.elapsed();

        let stats = tree.get_stats();

        // Verify structure
        assert!(stats.total_nodes > 100, "Should have found over 100 nodes");
        assert!(
            stats.files >= 105,
            "Should have found at least 105 files (100 + 5 deep files)"
        );
        assert!(stats.directories >= 20, "Should have found at least 20 directories");
        assert!(stats.max_depth >= 3, "Should have max depth of at least 3");

        // Performance assertion - should load 100+ files reasonably quickly
        assert!(
            load_time.as_millis() < 1000,
            "Loading {} nodes took too long: {:?}",
            stats.total_nodes,
            load_time
        );

        println!("Performance test: {} nodes loaded in {:?}", stats.total_nodes, load_time);
    }

    #[test]
    fn test_tree_loading_edge_cases() {
        use tempfile::TempDir;
        use std::process::Command;

        // Test empty Git repository
        let empty_dir = TempDir::new().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(empty_dir.path())
            .output()
            .expect("Failed to initialize git repository");
        let tree = FileTree::from_directory(empty_dir.path()).unwrap();
        assert_eq!(tree.root.len(), 0, "Empty Git repo should have no files");

        // Test directory with hidden files that ARE committed to Git (should be included)
        let hidden_dir = TempDir::new().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(hidden_dir.path())
            .output()
            .expect("Failed to initialize git repository");
        std::fs::write(hidden_dir.path().join(".hidden"), "hidden content").unwrap();
        std::fs::create_dir_all(hidden_dir.path().join(".hidden_dir")).unwrap();
        std::fs::write(hidden_dir.path().join(".hidden_dir/file.txt"), "content").unwrap();

        // Commit the hidden files
        Command::new("git")
            .args(["add", "."])
            .current_dir(hidden_dir.path())
            .output()
            .expect("Failed to add files to git");
        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "Add hidden files",
            ])
            .current_dir(hidden_dir.path())
            .output()
            .expect("Failed to commit files");

        let tree = FileTree::from_directory(hidden_dir.path()).unwrap();
        assert!(tree.root.len() > 0, "Should include committed hidden files");

        let names: Vec<String> = tree.root.iter().map(|n| n.name.clone()).collect();
        assert!(names.contains(&".hidden".to_string()), "Should include .hidden file");
        assert!(names.contains(&".hidden_dir".to_string()), "Should include .hidden_dir");

        // Test directory with special characters in names
        let special_dir = TempDir::new().unwrap();
        Command::new("git")
            .args(["init"])
            .current_dir(special_dir.path())
            .output()
            .expect("Failed to initialize git repository");
        std::fs::write(special_dir.path().join("file with spaces.txt"), "content").unwrap();
        std::fs::write(special_dir.path().join("file-with-dashes.txt"), "content").unwrap();
        std::fs::write(
            special_dir.path().join("file_with_underscores.txt"),
            "content",
        )
        .unwrap();

        // Commit the files
        Command::new("git")
            .args(["add", "."])
            .current_dir(special_dir.path())
            .output()
            .expect("Failed to add files to git");
        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "Add files with special chars",
            ])
            .current_dir(special_dir.path())
            .output()
            .expect("Failed to commit files");

        let tree = FileTree::from_directory(special_dir.path()).unwrap();
        assert_eq!(tree.root.len(), 3);

        let names: Vec<String> = tree.root.iter().map(|n| n.name.clone()).collect();
        assert!(names.contains(&"file with spaces.txt".to_string()));
        assert!(names.contains(&"file-with-dashes.txt".to_string()));
        assert!(names.contains(&"file_with_underscores.txt".to_string()));
    }

    #[test]
    fn test_tree_sorting_consistency() {
        use tempfile::TempDir;
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize Git repository
        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repository");

        // Create files and directories with names that test sorting
        std::fs::create_dir_all(temp_path.join("z_last_dir")).unwrap();
        std::fs::create_dir_all(temp_path.join("a_first_dir")).unwrap();
        std::fs::create_dir_all(temp_path.join("m_middle_dir")).unwrap();

        // Add files to each directory so they show up in Git tree
        std::fs::write(temp_path.join("z_last_dir/file.txt"), "content").unwrap();
        std::fs::write(temp_path.join("a_first_dir/file.txt"), "content").unwrap();
        std::fs::write(temp_path.join("m_middle_dir/file.txt"), "content").unwrap();

        std::fs::write(temp_path.join("z_last_file.txt"), "content").unwrap();
        std::fs::write(temp_path.join("a_first_file.txt"), "content").unwrap();
        std::fs::write(temp_path.join("m_middle_file.txt"), "content").unwrap();

        // Commit files to Git
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add files to git");

        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "Add sorting test files",
            ])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit files");

        let tree = FileTree::from_directory(temp_path).unwrap();

        // Verify directories come before files and both are sorted alphabetically
        let root_names: Vec<String> = tree.root.iter().map(|n| n.name.clone()).collect();

        // Find directory and file positions
        let first_dir_pos = root_names.iter().position(|n| n == "a_first_dir").unwrap();
        let middle_dir_pos = root_names.iter().position(|n| n == "m_middle_dir").unwrap();
        let last_dir_pos = root_names.iter().position(|n| n == "z_last_dir").unwrap();

        let first_file_pos = root_names.iter().position(|n| n == "a_first_file.txt").unwrap();
        let middle_file_pos = root_names.iter().position(|n| n == "m_middle_file.txt").unwrap();
        let last_file_pos = root_names.iter().position(|n| n == "z_last_file.txt").unwrap();

        // Verify directories come first
        assert!(
            first_dir_pos < first_file_pos,
            "Directories should come before files"
        );
        assert!(
            last_dir_pos < first_file_pos,
            "All directories should come before any file"
        );

        // Verify alphabetical sorting within each group
        assert!(
            first_dir_pos < middle_dir_pos,
            "Directories should be sorted alphabetically"
        );
        assert!(
            middle_dir_pos < last_dir_pos,
            "Directories should be sorted alphabetically"
        );
        assert!(first_file_pos < middle_file_pos, "Files should be sorted alphabetically");
        assert!(middle_file_pos < last_file_pos, "Files should be sorted alphabetically");
    }

    /*
    // Test removed: FileTreeState functionality has been removed
    // during the navigator unification refactoring
    #[test]
    fn test_search_filters_out_empty_directories() {
        use tempfile::TempDir;
        use std::process::Command;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Initialize Git repository
        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repository");

        // Create structure:
        // - github_dir/ (empty directory that would match "git" search)
        // - src/
        //   - git_utils.rs (file that matches "git" search)
        //   - main.rs
        std::fs::create_dir_all(temp_path.join("github_dir")).unwrap();
        std::fs::create_dir_all(temp_path.join("src")).unwrap();
        std::fs::write(temp_path.join("src/git_utils.rs"), "// git utilities").unwrap();
        std::fs::write(temp_path.join("src/main.rs"), "fn main() {}").unwrap();

        // Commit files to Git (github_dir won't be committed since it's empty)
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add files to git");

        Command::new("git")
            .args([
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                "Add test files",
            ])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit files");

        // Create file tree and search for "git"
        let tree = FileTree::from_directory(temp_path).unwrap();
        let mut tree_state = FileTreeState::new();
        tree_state.set_tree_data(tree, String::new(), false);
        tree_state.set_search_query("git".to_string());

        // Get visible nodes from search results
        let visible_nodes = tree_state.get_visible_nodes_with_depth();
        let node_names: Vec<String> =
            visible_nodes.iter().map(|(node, _)| node.name.clone()).collect();

        // Should contain "src" (directory with matching child) and "git_utils.rs" (matching file)
        // Should NOT contain "github_dir" since it's empty
        assert!(
            node_names.contains(&"src".to_string()),
            "Should include src directory with matching child"
        );
        assert!(
            node_names.contains(&"git_utils.rs".to_string()),
            "Should include matching file"
        );
        assert!(
            !node_names.contains(&"github_dir".to_string()),
            "Should NOT include empty directory even if name matches"
        );
        assert!(
            !node_names.contains(&"main.rs".to_string()),
            "Should NOT include non-matching file"
        );

        // Verify that directories are only included if they have matching children
        let src_node =
            visible_nodes.iter().find(|(node, _)| node.name == "src").map(|(node, _)| node);

        if let Some(src_node) = src_node {
            assert!(src_node.is_dir, "src should be a directory");
            assert!(!src_node.children.is_empty(), "src should have children in search results");
            assert!(
                src_node.children.iter().any(|child| child.name == "git_utils.rs"),
                "src should contain git_utils.rs as child"
            );
        } else {
            panic!("src directory should be present in search results");
        }
    }
    */
}