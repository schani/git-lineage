use serde::{Deserialize, Serialize};
use crate::app::{PanelFocus, FileTreeNode, CommitInfo};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub active_panel: PanelFocus,
    pub file_tree: Vec<FileTreeNode>,
    pub selected_file_path: Option<String>,
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
        Self {
            active_panel: PanelFocus::Navigator,
            file_tree: vec![
                FileTreeNode {
                    name: "src".to_string(),
                    path: "src".to_string(),
                    is_dir: true,
                    git_status: None,
                    children: vec![
                        FileTreeNode {
                            name: "main.rs".to_string(),
                            path: "src/main.rs".to_string(),
                            is_dir: false,
                            git_status: Some('M'),
                            children: vec![],
                        },
                        FileTreeNode {
                            name: "lib.rs".to_string(),
                            path: "src/lib.rs".to_string(),
                            is_dir: false,
                            git_status: Some('A'),
                            children: vec![],
                        },
                    ],
                },
                FileTreeNode {
                    name: "Cargo.toml".to_string(),
                    path: "Cargo.toml".to_string(),
                    is_dir: false,
                    git_status: Some('M'),
                    children: vec![],
                },
            ],
            selected_file_path: Some("src/main.rs".to_string()),
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