use serde::{Deserialize, Serialize};
use crate::app::{PanelFocus, CommitInfo};
use crate::tree::{FileTree, TreeNode};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub active_panel: PanelFocus,
    pub file_tree: FileTree,
    pub search_query: String,
    pub in_search_mode: bool,
    pub commit_list: Vec<CommitInfo>,
    pub selected_commit_index: Option<usize>,
    pub current_content: Vec<String>,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub inspector_scroll_vertical: u16,
    pub inspector_scroll_horizontal: u16,
    pub show_diff_view: bool,
    pub status_message: String,
    pub is_loading: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        let mut file_tree = FileTree::new();
        
        // Create sample tree structure
        let mut src_dir = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        src_dir.expand(); // Make it expanded by default
        src_dir.add_child(TreeNode::new_file("main.rs".to_string(), PathBuf::from("src/main.rs"))
            .with_git_status('M'));
        src_dir.add_child(TreeNode::new_file("lib.rs".to_string(), PathBuf::from("src/lib.rs"))
            .with_git_status('A'));
        
        let cargo_toml = TreeNode::new_file("Cargo.toml".to_string(), PathBuf::from("Cargo.toml"))
            .with_git_status('M');
        
        file_tree.root.push(src_dir);
        file_tree.root.push(cargo_toml);
        
        // Select the main.rs file by default
        file_tree.select_node(&PathBuf::from("src/main.rs"));

        Self {
            active_panel: PanelFocus::Navigator,
            file_tree,
            search_query: String::new(),
            in_search_mode: false,
            commit_list: vec![
                CommitInfo {
                    hash: "a1b2c3d4e5f6789012345678901234567890abcd".to_string(),
                    short_hash: "a1b2c3d".to_string(),
                    author: "John Doe".to_string(),
                    date: "2 hours ago".to_string(),
                    subject: "Add new feature".to_string(),
                },
                CommitInfo {
                    hash: "b2c3d4e5f6789012345678901234567890abcdef".to_string(),
                    short_hash: "b2c3d4e".to_string(),
                    author: "Jane Smith".to_string(),
                    date: "1 day ago".to_string(),
                    subject: "Fix bug in parser".to_string(),
                },
                CommitInfo {
                    hash: "c3d4e5f6789012345678901234567890abcdef01".to_string(),
                    short_hash: "c3d4e5f".to_string(),
                    author: "Bob Johnson".to_string(),
                    date: "3 days ago".to_string(),
                    subject: "Initial commit".to_string(),
                },
            ],
            selected_commit_index: Some(0),
            current_content: vec![
                "use std::io;".to_string(),
                "".to_string(),
                "fn main() {".to_string(),
                "    println!(\"Hello, world!\");".to_string(),
                "    let mut input = String::new();".to_string(),
                "    io::stdin().read_line(&mut input).expect(\"Failed to read line\");".to_string(),
                "    println!(\"You entered: {}\", input.trim());".to_string(),
                "}".to_string(),
            ],
            cursor_line: 3,
            cursor_column: 4,
            inspector_scroll_vertical: 0,
            inspector_scroll_horizontal: 0,
            show_diff_view: false,
            status_message: "Ready".to_string(),
            is_loading: false,
        }
    }
}

impl TestConfig {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: TestConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}