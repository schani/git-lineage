use crate::{app::PanelFocus, command::Command, test_config::TestConfig};

/// Result of executing a command
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub config: TestConfig,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

/// Executes commands against test configurations
pub struct Executor;

impl Executor {
    /// Execute a command against a configuration and return the resulting state
    pub fn execute(config: &TestConfig, command: Command) -> ExecutionResult {
        let mut new_config = config.clone();
        let mut status_message = None;
        let mut should_quit = false;

        match command {
            Command::Quit => {
                should_quit = true;
                status_message = Some("Goodbye!".to_string());
            }

            Command::NextPanel => {
                new_config.active_panel = match new_config.active_panel {
                    PanelFocus::Navigator => PanelFocus::History,
                    PanelFocus::History => PanelFocus::Inspector,
                    PanelFocus::Inspector => PanelFocus::Navigator,
                };
                status_message = Some(format!("Switched to {:?} panel", new_config.active_panel));
            }

            Command::PreviousPanel => {
                new_config.active_panel = match new_config.active_panel {
                    PanelFocus::Navigator => PanelFocus::Inspector,
                    PanelFocus::History => PanelFocus::Navigator,
                    PanelFocus::Inspector => PanelFocus::History,
                };
                status_message = Some(format!("Switched to {:?} panel", new_config.active_panel));
            }

            // File Navigator commands
            Command::NavigateUp => {
                if new_config.active_panel == PanelFocus::Navigator {
                    Self::execute_navigate_up(&mut new_config, &mut status_message);
                }
            }

            Command::NavigateDown => {
                if new_config.active_panel == PanelFocus::Navigator {
                    Self::execute_navigate_down(&mut new_config, &mut status_message);
                }
            }

            Command::ExpandNode => {
                if new_config.active_panel == PanelFocus::Navigator {
                    Self::execute_expand_node(&mut new_config, &mut status_message);
                }
            }

            Command::CollapseNode => {
                if new_config.active_panel == PanelFocus::Navigator {
                    Self::execute_collapse_node(&mut new_config, &mut status_message);
                }
            }

            Command::SelectFile => {
                if new_config.active_panel == PanelFocus::Navigator {
                    Self::execute_select_file(&mut new_config, &mut status_message);
                }
            }

            Command::StartSearch => {
                if new_config.active_panel == PanelFocus::Navigator {
                    new_config.in_search_mode = true;
                    new_config.search_query.clear();
                    status_message = Some("Search mode activated".to_string());
                }
            }

            Command::EndSearch => {
                if new_config.active_panel == PanelFocus::Navigator && new_config.in_search_mode {
                    new_config.in_search_mode = false;
                    new_config.search_query.clear();
                    status_message = Some("Search mode deactivated".to_string());
                }
            }

            Command::SearchInput(ch) => {
                if new_config.active_panel == PanelFocus::Navigator && new_config.in_search_mode {
                    new_config.search_query.push(ch);
                    status_message = Some(format!("Search: {}", new_config.search_query));
                }
            }

            Command::SearchBackspace => {
                if new_config.active_panel == PanelFocus::Navigator && new_config.in_search_mode {
                    new_config.search_query.pop();
                    status_message = Some(format!("Search: {}", new_config.search_query));
                }
            }

            // Commit History commands
            Command::HistoryUp => {
                if new_config.active_panel == PanelFocus::History {
                    Self::execute_history_up(&mut new_config, &mut status_message);
                }
            }

            Command::HistoryDown => {
                if new_config.active_panel == PanelFocus::History {
                    Self::execute_history_down(&mut new_config, &mut status_message);
                }
            }

            Command::SelectCommit => {
                if new_config.active_panel == PanelFocus::History {
                    Self::execute_select_commit(&mut new_config, &mut status_message);
                }
            }

            // Code Inspector commands
            Command::InspectorUp => {
                if new_config.active_panel == PanelFocus::Inspector {
                    Self::execute_inspector_up(&mut new_config, &mut status_message);
                }
            }

            Command::InspectorDown => {
                if new_config.active_panel == PanelFocus::Inspector {
                    Self::execute_inspector_down(&mut new_config, &mut status_message);
                }
            }

            Command::InspectorPageUp => {
                if new_config.active_panel == PanelFocus::Inspector {
                    Self::execute_inspector_page_up(&mut new_config, &mut status_message);
                }
            }

            Command::InspectorPageDown => {
                if new_config.active_panel == PanelFocus::Inspector {
                    Self::execute_inspector_page_down(&mut new_config, &mut status_message);
                }
            }

            Command::InspectorHome => {
                if new_config.active_panel == PanelFocus::Inspector {
                    new_config.cursor_line = 0;
                    new_config.inspector_scroll_vertical = 0;
                    status_message = Some("Moved to beginning of file".to_string());
                }
            }

            Command::InspectorEnd => {
                if new_config.active_panel == PanelFocus::Inspector {
                    new_config.cursor_line = new_config.current_content.len().saturating_sub(1);
                    status_message = Some("Moved to end of file".to_string());
                }
            }

            Command::InspectorLeft => {
                if new_config.active_panel == PanelFocus::Inspector {
                    new_config.cursor_column = new_config.cursor_column.saturating_sub(1);
                    status_message = Some(format!("Column: {}", new_config.cursor_column));
                }
            }

            Command::InspectorRight => {
                if new_config.active_panel == PanelFocus::Inspector {
                    new_config.cursor_column += 1;
                    status_message = Some(format!("Column: {}", new_config.cursor_column));
                }
            }

            Command::GoToTop => {
                if new_config.active_panel == PanelFocus::Inspector {
                    new_config.cursor_line = 0;
                    new_config.inspector_scroll_vertical = 0;
                    status_message = Some("Moved to top".to_string());
                }
            }

            Command::GoToBottom => {
                if new_config.active_panel == PanelFocus::Inspector {
                    new_config.cursor_line = new_config.current_content.len().saturating_sub(1);
                    status_message = Some("Moved to bottom".to_string());
                }
            }

            Command::PreviousChange => {
                if new_config.active_panel == PanelFocus::Inspector {
                    // Simulate finding previous change (would use Git blame in real implementation)
                    status_message = Some(format!(
                        "Previous change for line {}",
                        new_config.cursor_line + 1
                    ));
                }
            }

            Command::NextChange => {
                if new_config.active_panel == PanelFocus::Inspector {
                    new_config.is_loading = true;
                    status_message = Some("Searching for next change...".to_string());
                }
            }

            Command::ToggleDiff => {
                if new_config.active_panel == PanelFocus::Inspector {
                    new_config.show_diff_view = !new_config.show_diff_view;
                    status_message = Some(if new_config.show_diff_view {
                        "Switched to diff view".to_string()
                    } else {
                        "Switched to full file view".to_string()
                    });
                }
            }

            Command::Sequence(commands) => {
                // Execute commands in sequence
                for cmd in commands {
                    let result = Self::execute(&new_config, cmd);
                    new_config = result.config;
                    if let Some(msg) = result.status_message {
                        status_message = Some(msg);
                    }
                    if result.should_quit {
                        should_quit = true;
                        break;
                    }
                }
            }
        }

        // Update the final status message if one was set
        if let Some(msg) = &status_message {
            new_config.status_message = msg.clone();
        }

        ExecutionResult {
            config: new_config,
            status_message,
            should_quit,
        }
    }
}

// Implementation of specific command handlers
impl Executor {
    fn execute_navigate_up(config: &mut TestConfig, status_message: &mut Option<String>) {
        if config.file_tree.navigate_up() {
            *status_message = Some("Navigated up in file tree".to_string());
        } else {
            *status_message = Some("Already at top".to_string());
        }
    }

    fn execute_navigate_down(config: &mut TestConfig, status_message: &mut Option<String>) {
        if config.file_tree.navigate_down() {
            *status_message = Some("Navigated down in file tree".to_string());
        } else {
            *status_message = Some("Already at bottom".to_string());
        }
    }

    fn execute_expand_node(config: &mut TestConfig, status_message: &mut Option<String>) {
        if let Some(selected_path) = config.file_tree.current_selection.clone() {
            if config.file_tree.expand_node(&selected_path) {
                *status_message = Some("Expanded directory node".to_string());
            } else {
                *status_message = Some("Cannot expand this node".to_string());
            }
        }
    }

    fn execute_collapse_node(config: &mut TestConfig, status_message: &mut Option<String>) {
        if let Some(selected_path) = config.file_tree.current_selection.clone() {
            if config.file_tree.collapse_node(&selected_path) {
                *status_message = Some("Collapsed directory node".to_string());
            } else {
                *status_message = Some("Cannot collapse this node".to_string());
            }
        }
    }

    fn execute_select_file(config: &mut TestConfig, status_message: &mut Option<String>) {
        if let Some(selected_path) = config.file_tree.current_selection.clone() {
            let is_dir = config
                .file_tree
                .find_node(&selected_path)
                .map(|node| node.is_dir)
                .unwrap_or(false);

            if !is_dir {
                *status_message = Some(format!("Selected file: {}", selected_path.display()));
            } else {
                // Toggle directory expansion
                let was_expanded = config
                    .file_tree
                    .find_node(&selected_path)
                    .map(|n| n.is_expanded)
                    .unwrap_or(false);

                config.file_tree.toggle_node(&selected_path);
                *status_message = Some(if was_expanded {
                    "Collapsed directory".to_string()
                } else {
                    "Expanded directory".to_string()
                });
            }
        } else {
            *status_message = Some("No file selected".to_string());
        }
    }

    fn execute_history_up(config: &mut TestConfig, status_message: &mut Option<String>) {
        if let Some(current) = config.selected_commit_index {
            if current > 0 {
                config.selected_commit_index = Some(current - 1);
                if let Some(commit) = config.commit_list.get(current - 1) {
                    *status_message = Some(format!("Selected commit: {}", commit.short_hash));
                }
            }
        } else if !config.commit_list.is_empty() {
            config.selected_commit_index = Some(0);
            if let Some(commit) = config.commit_list.first() {
                *status_message = Some(format!("Selected commit: {}", commit.short_hash));
            }
        }
    }

    fn execute_history_down(config: &mut TestConfig, status_message: &mut Option<String>) {
        if let Some(current) = config.selected_commit_index {
            if current < config.commit_list.len().saturating_sub(1) {
                config.selected_commit_index = Some(current + 1);
                if let Some(commit) = config.commit_list.get(current + 1) {
                    *status_message = Some(format!("Selected commit: {}", commit.short_hash));
                }
            }
        } else if !config.commit_list.is_empty() {
            config.selected_commit_index = Some(0);
            if let Some(commit) = config.commit_list.first() {
                *status_message = Some(format!("Selected commit: {}", commit.short_hash));
            }
        }
    }

    fn execute_select_commit(config: &mut TestConfig, status_message: &mut Option<String>) {
        if let Some(index) = config.selected_commit_index {
            if let Some(commit) = config.commit_list.get(index) {
                *status_message = Some(format!("Viewing commit: {}", commit.short_hash));
                // In real implementation, this would load file content for the commit
            }
        }
    }

    fn execute_inspector_up(config: &mut TestConfig, status_message: &mut Option<String>) {
        if config.cursor_line > 0 {
            config.cursor_line -= 1;
            if config.cursor_line < config.inspector_scroll_vertical as usize {
                config.inspector_scroll_vertical = config.cursor_line as u16;
            }
            *status_message = Some(format!("Line: {}", config.cursor_line + 1));
        }
    }

    fn execute_inspector_down(config: &mut TestConfig, status_message: &mut Option<String>) {
        if config.cursor_line < config.current_content.len().saturating_sub(1) {
            config.cursor_line += 1;
            *status_message = Some(format!("Line: {}", config.cursor_line + 1));
        }
    }

    fn execute_inspector_page_up(config: &mut TestConfig, status_message: &mut Option<String>) {
        config.cursor_line = config.cursor_line.saturating_sub(10);
        config.inspector_scroll_vertical = config.cursor_line as u16;
        *status_message = Some(format!("Page up - Line: {}", config.cursor_line + 1));
    }

    fn execute_inspector_page_down(config: &mut TestConfig, status_message: &mut Option<String>) {
        config.cursor_line =
            (config.cursor_line + 10).min(config.current_content.len().saturating_sub(1));
        *status_message = Some(format!("Page down - Line: {}", config.cursor_line + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::CommitInfo;
    use crate::tree::{FileTree, TreeNode};
    use std::path::PathBuf;

    // Test utilities
    fn create_test_config_with_tree() -> TestConfig {
        let mut tree = FileTree::new();

        // Create a test directory structure
        let mut src_dir = TreeNode::new_dir("src".to_string(), PathBuf::from("src"));
        src_dir.expand();
        src_dir.add_child(TreeNode::new_file(
            "main.rs".to_string(),
            PathBuf::from("src/main.rs"),
        ));
        src_dir.add_child(TreeNode::new_file(
            "lib.rs".to_string(),
            PathBuf::from("src/lib.rs"),
        ));

        tree.root.push(src_dir);
        tree.root.push(TreeNode::new_file(
            "Cargo.toml".to_string(),
            PathBuf::from("Cargo.toml"),
        ));

        // Select the first item (src directory)
        tree.navigate_to_first();

        TestConfig {
            file_tree: tree,
            active_panel: PanelFocus::Navigator,
            ..TestConfig::default()
        }
    }

    fn create_test_config_with_commits() -> TestConfig {
        let commits = vec![
            CommitInfo {
                hash: "abc123def456".to_string(),
                short_hash: "abc123".to_string(),
                author: "Alice Smith".to_string(),
                date: "2023-01-01".to_string(),
                subject: "Initial commit".to_string(),
            },
            CommitInfo {
                hash: "def456ghi789".to_string(),
                short_hash: "def456".to_string(),
                author: "Bob Jones".to_string(),
                date: "2023-01-02".to_string(),
                subject: "Add feature".to_string(),
            },
            CommitInfo {
                hash: "ghi789jkl012".to_string(),
                short_hash: "ghi789".to_string(),
                author: "Carol Davis".to_string(),
                date: "2023-01-03".to_string(),
                subject: "Fix bug".to_string(),
            },
        ];

        TestConfig {
            commit_list: commits,
            selected_commit_index: Some(0),
            active_panel: PanelFocus::History,
            ..TestConfig::default()
        }
    }

    fn create_test_config_with_content() -> TestConfig {
        let content = vec![
            "use std::io;".to_string(),
            "".to_string(),
            "fn main() {".to_string(),
            "    println!(\"Hello, world!\");".to_string(),
            "    let mut input = String::new();".to_string(),
            "    io::stdin().read_line(&mut input).unwrap();".to_string(),
            "    println!(\"You entered: {}\", input.trim());".to_string(),
            "}".to_string(),
        ];

        TestConfig {
            current_content: content,
            cursor_line: 3,
            cursor_column: 4,
            active_panel: PanelFocus::Inspector,
            ..TestConfig::default()
        }
    }

    mod panel_navigation {
        use super::*;

        #[test]
        fn test_next_panel_complete_cycle() {
            let mut config = TestConfig {
                active_panel: PanelFocus::Navigator,
                ..TestConfig::default()
            };

            // Navigator -> History
            let result = Executor::execute(&config, Command::NextPanel);
            config = result.config;
            assert_eq!(config.active_panel, PanelFocus::History);
            assert!(result.status_message.unwrap().contains("History"));
            assert!(!result.should_quit);

            // History -> Inspector
            let result = Executor::execute(&config, Command::NextPanel);
            config = result.config;
            assert_eq!(config.active_panel, PanelFocus::Inspector);
            assert!(result.status_message.unwrap().contains("Inspector"));

            // Inspector -> Navigator (complete cycle)
            let result = Executor::execute(&config, Command::NextPanel);
            config = result.config;
            assert_eq!(config.active_panel, PanelFocus::Navigator);
            assert!(result.status_message.unwrap().contains("Navigator"));
        }

        #[test]
        fn test_previous_panel_complete_cycle() {
            let mut config = TestConfig {
                active_panel: PanelFocus::Navigator,
                ..TestConfig::default()
            };

            // Navigator -> Inspector (reverse direction)
            let result = Executor::execute(&config, Command::PreviousPanel);
            config = result.config;
            assert_eq!(config.active_panel, PanelFocus::Inspector);
            assert!(result.status_message.unwrap().contains("Inspector"));

            // Inspector -> History
            let result = Executor::execute(&config, Command::PreviousPanel);
            config = result.config;
            assert_eq!(config.active_panel, PanelFocus::History);
            assert!(result.status_message.unwrap().contains("History"));

            // History -> Navigator (complete reverse cycle)
            let result = Executor::execute(&config, Command::PreviousPanel);
            config = result.config;
            assert_eq!(config.active_panel, PanelFocus::Navigator);
            assert!(result.status_message.unwrap().contains("Navigator"));
        }

        #[test]
        fn test_quit_command() {
            let config = TestConfig::default();

            let result = Executor::execute(&config, Command::Quit);
            assert!(result.should_quit);
            assert_eq!(result.status_message, Some("Goodbye!".to_string()));
        }
    }

    mod file_navigator_commands {
        use super::*;

        #[test]
        fn test_navigate_up_down() {
            let mut config = create_test_config_with_tree();

            // Navigate down to main.rs
            let result = Executor::execute(&config, Command::NavigateDown);
            config = result.config;
            assert!(result.status_message.unwrap().contains("Navigated down"));

            // Navigate back up
            let result = Executor::execute(&config, Command::NavigateUp);
            config = result.config;
            assert!(result.status_message.unwrap().contains("Navigated up"));
        }

        #[test]
        fn test_navigate_up_at_top() {
            let mut config = create_test_config_with_tree();

            // Try to navigate up when already at top
            let result = Executor::execute(&config, Command::NavigateUp);
            assert!(result.status_message.unwrap().contains("Already at top"));
        }

        #[test]
        fn test_navigate_down_at_bottom() {
            let mut config = create_test_config_with_tree();

            // Navigate to the bottom
            while config.file_tree.navigate_down() {
                let result = Executor::execute(&config, Command::NavigateDown);
                config = result.config;
            }

            // Try to navigate down when at bottom
            let result = Executor::execute(&config, Command::NavigateDown);
            assert!(result.status_message.unwrap().contains("Already at bottom"));
        }

        #[test]
        fn test_expand_collapse_directory() {
            let mut config = create_test_config_with_tree();

            // Expand the currently selected directory (src)
            let result = Executor::execute(&config, Command::ExpandNode);
            config = result.config;
            assert!(result
                .status_message
                .unwrap()
                .contains("Expanded directory"));

            // Collapse the directory
            let result = Executor::execute(&config, Command::CollapseNode);
            config = result.config;
            assert!(result
                .status_message
                .unwrap()
                .contains("Collapsed directory"));
        }

        #[test]
        fn test_expand_collapse_file() {
            let mut config = create_test_config_with_tree();

            // Navigate to a file (Cargo.toml)
            config.file_tree.select_node(&PathBuf::from("Cargo.toml"));

            // Try to expand a file (should fail)
            let result = Executor::execute(&config, Command::ExpandNode);
            assert!(result.status_message.unwrap().contains("Cannot expand"));

            // Try to collapse a file (should fail)
            let result = Executor::execute(&config, Command::CollapseNode);
            assert!(result.status_message.unwrap().contains("Cannot collapse"));
        }

        #[test]
        fn test_select_file() {
            let mut config = create_test_config_with_tree();

            // Select a file
            config.file_tree.select_node(&PathBuf::from("Cargo.toml"));
            let result = Executor::execute(&config, Command::SelectFile);
            assert!(result
                .status_message
                .unwrap()
                .contains("Selected file: Cargo.toml"));
        }

        #[test]
        fn test_select_directory_toggles_expansion() {
            let mut config = create_test_config_with_tree();

            // Start with collapsed src directory
            let src_path = PathBuf::from("src");
            config.file_tree.collapse_node(&src_path);
            config.file_tree.select_node(&src_path);

            // Select directory should expand it
            let result = Executor::execute(&config, Command::SelectFile);
            config = result.config;
            assert!(result
                .status_message
                .unwrap()
                .contains("Expanded directory"));

            // Select again should collapse it
            let result = Executor::execute(&config, Command::SelectFile);
            config = result.config;
            assert!(result
                .status_message
                .unwrap()
                .contains("Collapsed directory"));
        }

        #[test]
        fn test_select_file_no_selection() {
            let mut config = TestConfig {
                active_panel: PanelFocus::Navigator,
                file_tree: FileTree::new(), // Truly empty file tree
                ..TestConfig::default()
            };
            // Ensure no selection
            config.file_tree.current_selection = None;

            let result = Executor::execute(&config, Command::SelectFile);
            assert!(result.status_message.unwrap().contains("No file selected"));
        }

        #[test]
        fn test_navigator_commands_ignored_on_other_panels() {
            let config = TestConfig {
                active_panel: PanelFocus::History,
                ..TestConfig::default()
            };

            // Navigator commands should be ignored when not on Navigator panel
            let result = Executor::execute(&config, Command::NavigateUp);
            assert!(result.status_message.is_none());

            let result = Executor::execute(&config, Command::NavigateDown);
            assert!(result.status_message.is_none());

            let result = Executor::execute(&config, Command::ExpandNode);
            assert!(result.status_message.is_none());
        }
    }

    mod search_functionality {
        use super::*;

        #[test]
        fn test_search_full_workflow() {
            let mut config = TestConfig {
                active_panel: PanelFocus::Navigator,
                in_search_mode: false,
                search_query: String::new(),
                ..TestConfig::default()
            };

            // Start search
            let result = Executor::execute(&config, Command::StartSearch);
            config = result.config;
            assert!(config.in_search_mode);
            assert_eq!(
                result.status_message,
                Some("Search mode activated".to_string())
            );

            // Add search input
            let result = Executor::execute(&config, Command::SearchInput('t'));
            config = result.config;
            assert_eq!(config.search_query, "t");
            assert_eq!(result.status_message, Some("Search: t".to_string()));

            // Add more input
            let result = Executor::execute(&config, Command::SearchInput('e'));
            config = result.config;
            assert_eq!(config.search_query, "te");
            assert_eq!(result.status_message, Some("Search: te".to_string()));

            let result = Executor::execute(&config, Command::SearchInput('s'));
            config = result.config;
            assert_eq!(config.search_query, "tes");
            assert_eq!(result.status_message, Some("Search: tes".to_string()));

            let result = Executor::execute(&config, Command::SearchInput('t'));
            config = result.config;
            assert_eq!(config.search_query, "test");
            assert_eq!(result.status_message, Some("Search: test".to_string()));

            // Backspace
            let result = Executor::execute(&config, Command::SearchBackspace);
            config = result.config;
            assert_eq!(config.search_query, "tes");
            assert_eq!(result.status_message, Some("Search: tes".to_string()));

            let result = Executor::execute(&config, Command::SearchBackspace);
            config = result.config;
            assert_eq!(config.search_query, "te");

            // End search
            let result = Executor::execute(&config, Command::EndSearch);
            config = result.config;
            assert!(!config.in_search_mode);
            assert!(config.search_query.is_empty());
            assert_eq!(
                result.status_message,
                Some("Search mode deactivated".to_string())
            );
        }

        #[test]
        fn test_search_backspace_empty_query() {
            let mut config = TestConfig {
                active_panel: PanelFocus::Navigator,
                in_search_mode: true,
                search_query: String::new(),
                ..TestConfig::default()
            };

            // Backspace on empty query
            let result = Executor::execute(&config, Command::SearchBackspace);
            config = result.config;
            assert!(config.search_query.is_empty());
            assert_eq!(result.status_message, Some("Search: ".to_string()));
        }

        #[test]
        fn test_search_commands_ignored_outside_navigator() {
            let config = TestConfig {
                active_panel: PanelFocus::History,
                in_search_mode: false,
                ..TestConfig::default()
            };

            // Search commands should be ignored when not on Navigator panel
            let result = Executor::execute(&config, Command::StartSearch);
            assert!(!result.config.in_search_mode);
            assert!(result.status_message.is_none());
        }

        #[test]
        fn test_search_input_ignored_when_not_in_search_mode() {
            let config = TestConfig {
                active_panel: PanelFocus::Navigator,
                in_search_mode: false,
                search_query: String::new(),
                ..TestConfig::default()
            };

            // Search input should be ignored when not in search mode
            let result = Executor::execute(&config, Command::SearchInput('x'));
            assert!(result.config.search_query.is_empty());
            assert!(result.status_message.is_none());
        }

        #[test]
        fn test_end_search_when_not_searching() {
            let config = TestConfig {
                active_panel: PanelFocus::Navigator,
                in_search_mode: false,
                ..TestConfig::default()
            };

            // End search when not searching should do nothing
            let result = Executor::execute(&config, Command::EndSearch);
            assert!(!result.config.in_search_mode);
            assert!(result.status_message.is_none());
        }
    }

    mod history_commands {
        use super::*;

        #[test]
        fn test_history_navigation() {
            let mut config = create_test_config_with_commits();

            // Start at first commit (index 0)
            assert_eq!(config.selected_commit_index, Some(0));

            // Navigate down to next commit
            let result = Executor::execute(&config, Command::HistoryDown);
            config = result.config;
            assert_eq!(config.selected_commit_index, Some(1));
            assert!(result.status_message.unwrap().contains("def456"));

            // Navigate down again
            let result = Executor::execute(&config, Command::HistoryDown);
            config = result.config;
            assert_eq!(config.selected_commit_index, Some(2));
            assert!(result.status_message.unwrap().contains("ghi789"));

            // Try to navigate down at bottom (should stay at last commit)
            let result = Executor::execute(&config, Command::HistoryDown);
            config = result.config;
            assert_eq!(config.selected_commit_index, Some(2));

            // Navigate back up
            let result = Executor::execute(&config, Command::HistoryUp);
            config = result.config;
            assert_eq!(config.selected_commit_index, Some(1));
            assert!(result.status_message.unwrap().contains("def456"));

            // Navigate up again
            let result = Executor::execute(&config, Command::HistoryUp);
            config = result.config;
            assert_eq!(config.selected_commit_index, Some(0));
            assert!(result.status_message.unwrap().contains("abc123"));
        }

        #[test]
        fn test_history_navigation_no_selection() {
            let mut config = TestConfig {
                commit_list: vec![CommitInfo {
                    hash: "abc123".to_string(),
                    short_hash: "abc123".to_string(),
                    author: "Test".to_string(),
                    date: "2023-01-01".to_string(),
                    subject: "Test commit".to_string(),
                }],
                selected_commit_index: None,
                active_panel: PanelFocus::History,
                ..TestConfig::default()
            };

            // Navigate up when no selection should select first commit
            let result = Executor::execute(&config, Command::HistoryUp);
            config = result.config;
            assert_eq!(config.selected_commit_index, Some(0));
            assert!(result.status_message.unwrap().contains("abc123"));

            // Reset to no selection
            config.selected_commit_index = None;

            // Navigate down when no selection should also select first commit
            let result = Executor::execute(&config, Command::HistoryDown);
            config = result.config;
            assert_eq!(config.selected_commit_index, Some(0));
            assert!(result.status_message.unwrap().contains("abc123"));
        }

        #[test]
        fn test_history_navigation_empty_list() {
            let config = TestConfig {
                commit_list: vec![],
                selected_commit_index: None,
                active_panel: PanelFocus::History,
                ..TestConfig::default()
            };

            // Navigation on empty list should do nothing
            let result = Executor::execute(&config, Command::HistoryUp);
            assert_eq!(result.config.selected_commit_index, None);
            assert!(result.status_message.is_none());

            let result = Executor::execute(&config, Command::HistoryDown);
            assert_eq!(result.config.selected_commit_index, None);
            assert!(result.status_message.is_none());
        }

        #[test]
        fn test_select_commit() {
            let config = create_test_config_with_commits();

            let result = Executor::execute(&config, Command::SelectCommit);
            assert!(result
                .status_message
                .unwrap()
                .contains("Viewing commit: abc123"));
        }

        #[test]
        fn test_select_commit_no_selection() {
            let config = TestConfig {
                commit_list: vec![],
                selected_commit_index: None,
                active_panel: PanelFocus::History,
                ..TestConfig::default()
            };

            let result = Executor::execute(&config, Command::SelectCommit);
            assert!(result.status_message.is_none());
        }

        #[test]
        fn test_history_commands_ignored_on_other_panels() {
            let config = TestConfig {
                active_panel: PanelFocus::Navigator,
                ..TestConfig::default()
            };

            // History commands should be ignored when not on History panel
            let result = Executor::execute(&config, Command::HistoryUp);
            assert!(result.status_message.is_none());

            let result = Executor::execute(&config, Command::HistoryDown);
            assert!(result.status_message.is_none());

            let result = Executor::execute(&config, Command::SelectCommit);
            assert!(result.status_message.is_none());
        }
    }

    mod inspector_commands {
        use super::*;

        #[test]
        fn test_inspector_cursor_movement() {
            let mut config = create_test_config_with_content();

            // Start at line 3, column 4
            assert_eq!(config.cursor_line, 3);
            assert_eq!(config.cursor_column, 4);

            // Move up
            let result = Executor::execute(&config, Command::InspectorUp);
            config = result.config;
            assert_eq!(config.cursor_line, 2);
            assert!(result.status_message.unwrap().contains("Line: 3")); // 1-indexed display

            // Move down
            let result = Executor::execute(&config, Command::InspectorDown);
            config = result.config;
            assert_eq!(config.cursor_line, 3);
            assert!(result.status_message.unwrap().contains("Line: 4"));

            // Move down again
            let result = Executor::execute(&config, Command::InspectorDown);
            config = result.config;
            assert_eq!(config.cursor_line, 4);
            assert!(result.status_message.unwrap().contains("Line: 5"));

            // Move left
            let result = Executor::execute(&config, Command::InspectorLeft);
            config = result.config;
            assert_eq!(config.cursor_column, 3);
            assert!(result.status_message.unwrap().contains("Column: 3"));

            // Move right
            let result = Executor::execute(&config, Command::InspectorRight);
            config = result.config;
            assert_eq!(config.cursor_column, 4);
            assert!(result.status_message.unwrap().contains("Column: 4"));

            // Move right again
            let result = Executor::execute(&config, Command::InspectorRight);
            config = result.config;
            assert_eq!(config.cursor_column, 5);
            assert!(result.status_message.unwrap().contains("Column: 5"));
        }

        #[test]
        fn test_inspector_cursor_boundaries() {
            let mut config = create_test_config_with_content();
            config.cursor_line = 0;
            config.cursor_column = 0;

            // Try to move up at top
            let result = Executor::execute(&config, Command::InspectorUp);
            config = result.config;
            assert_eq!(config.cursor_line, 0);
            assert!(result.status_message.is_none());

            // Move to bottom
            config.cursor_line = config.current_content.len() - 1;

            // Try to move down at bottom
            let result = Executor::execute(&config, Command::InspectorDown);
            config = result.config;
            assert_eq!(config.cursor_line, config.current_content.len() - 1);
            assert!(result.status_message.is_none());

            // Try to move left at column 0
            config.cursor_column = 0;
            let result = Executor::execute(&config, Command::InspectorLeft);
            config = result.config;
            assert_eq!(config.cursor_column, 0);
            assert!(result.status_message.unwrap().contains("Column: 0"));
        }

        #[test]
        fn test_inspector_page_movement() {
            let mut config = create_test_config_with_content();
            config.cursor_line = 5;

            // Page up
            let result = Executor::execute(&config, Command::InspectorPageUp);
            config = result.config;
            assert_eq!(config.cursor_line, 0); // max(5-10, 0) = 0
            assert_eq!(config.inspector_scroll_vertical, 0);
            assert!(result.status_message.unwrap().contains("Page up - Line: 1"));

            // Set cursor to middle again
            config.cursor_line = 4;

            // Page down
            let result = Executor::execute(&config, Command::InspectorPageDown);
            config = result.config;
            let expected_line = std::cmp::min(4 + 10, config.current_content.len() - 1);
            assert_eq!(config.cursor_line, expected_line);
            assert!(result.status_message.unwrap().contains("Page down"));
        }

        #[test]
        fn test_inspector_home_end() {
            let mut config = create_test_config_with_content();
            config.cursor_line = 4;
            config.cursor_column = 10;
            config.inspector_scroll_vertical = 2;

            // Home
            let result = Executor::execute(&config, Command::InspectorHome);
            config = result.config;
            assert_eq!(config.cursor_line, 0);
            assert_eq!(config.inspector_scroll_vertical, 0);
            assert!(result.status_message.unwrap().contains("beginning of file"));

            // End
            let result = Executor::execute(&config, Command::InspectorEnd);
            config = result.config;
            assert_eq!(config.cursor_line, config.current_content.len() - 1);
            assert!(result.status_message.unwrap().contains("end of file"));
        }

        #[test]
        fn test_inspector_go_to_commands() {
            let mut config = create_test_config_with_content();
            config.cursor_line = 4;
            config.inspector_scroll_vertical = 2;

            // Go to top
            let result = Executor::execute(&config, Command::GoToTop);
            config = result.config;
            assert_eq!(config.cursor_line, 0);
            assert_eq!(config.inspector_scroll_vertical, 0);
            assert!(result.status_message.unwrap().contains("top"));

            // Go to bottom
            let result = Executor::execute(&config, Command::GoToBottom);
            config = result.config;
            assert_eq!(config.cursor_line, config.current_content.len() - 1);
            assert!(result.status_message.unwrap().contains("bottom"));
        }

        #[test]
        fn test_inspector_change_navigation() {
            let config = create_test_config_with_content();

            // Previous change
            let result = Executor::execute(&config, Command::PreviousChange);
            assert!(result
                .status_message
                .unwrap()
                .contains("Previous change for line 4"));

            // Next change
            let result = Executor::execute(&config, Command::NextChange);
            assert!(result.config.is_loading);
            assert!(result
                .status_message
                .unwrap()
                .contains("Searching for next change"));
        }

        #[test]
        fn test_inspector_toggle_diff() {
            let mut config = create_test_config_with_content();
            assert!(!config.show_diff_view);

            // Toggle to diff view
            let result = Executor::execute(&config, Command::ToggleDiff);
            config = result.config;
            assert!(config.show_diff_view);
            assert!(result
                .status_message
                .unwrap()
                .contains("Switched to diff view"));

            // Toggle back to full file view
            let result = Executor::execute(&config, Command::ToggleDiff);
            config = result.config;
            assert!(!config.show_diff_view);
            assert!(result
                .status_message
                .unwrap()
                .contains("Switched to full file view"));
        }

        #[test]
        fn test_inspector_commands_ignored_on_other_panels() {
            let config = TestConfig {
                active_panel: PanelFocus::Navigator,
                ..TestConfig::default()
            };

            // Inspector commands should be ignored when not on Inspector panel
            let result = Executor::execute(&config, Command::InspectorUp);
            assert!(result.status_message.is_none());

            let result = Executor::execute(&config, Command::InspectorDown);
            assert!(result.status_message.is_none());

            let result = Executor::execute(&config, Command::ToggleDiff);
            assert!(result.status_message.is_none());
        }
    }

    mod sequence_commands {
        use super::*;

        #[test]
        fn test_simple_sequence() {
            let config = TestConfig {
                active_panel: PanelFocus::Navigator,
                ..TestConfig::default()
            };

            let sequence = Command::Sequence(vec![Command::NextPanel, Command::NextPanel]);

            let result = Executor::execute(&config, sequence);
            assert_eq!(result.config.active_panel, PanelFocus::Inspector);
            assert!(result.status_message.unwrap().contains("Inspector"));
        }

        #[test]
        fn test_complex_sequence() {
            let config = create_test_config_with_tree();

            let sequence = Command::Sequence(vec![
                Command::NavigateDown,  // Move down in tree
                Command::ExpandNode,    // Expand if it's a directory
                Command::NextPanel,     // Switch to History panel
                Command::PreviousPanel, // Switch back to Navigator
            ]);

            let result = Executor::execute(&config, sequence);
            assert_eq!(result.config.active_panel, PanelFocus::Navigator);
        }

        #[test]
        fn test_sequence_with_quit() {
            let config = TestConfig::default();

            let sequence = Command::Sequence(vec![
                Command::NextPanel,
                Command::Quit,
                Command::NextPanel, // This should not execute
            ]);

            let result = Executor::execute(&config, sequence);
            assert!(result.should_quit);
            assert_eq!(result.status_message, Some("Goodbye!".to_string()));
        }

        #[test]
        fn test_search_sequence() {
            let config = TestConfig {
                active_panel: PanelFocus::Navigator,
                ..TestConfig::default()
            };

            let sequence = Command::Sequence(vec![
                Command::StartSearch,
                Command::SearchInput('h'),
                Command::SearchInput('e'),
                Command::SearchInput('l'),
                Command::SearchInput('l'),
                Command::SearchInput('o'),
                Command::SearchBackspace, // Remove 'o'
                Command::SearchBackspace, // Remove 'l'
                Command::EndSearch,
            ]);

            let result = Executor::execute(&config, sequence);
            assert!(!result.config.in_search_mode);
            assert!(result.config.search_query.is_empty());
            assert_eq!(
                result.status_message,
                Some("Search mode deactivated".to_string())
            );
        }

        #[test]
        fn test_empty_sequence() {
            let config = TestConfig::default();

            let sequence = Command::Sequence(vec![]);

            let result = Executor::execute(&config, sequence);
            assert_eq!(result.config.active_panel, config.active_panel);
            assert!(!result.should_quit);
            assert!(result.status_message.is_none());
        }
    }

    mod edge_cases_and_error_handling {
        use super::*;

        #[test]
        fn test_all_commands_preserve_config_integrity() {
            let original_config = create_test_config_with_content();

            let commands = vec![
                Command::NextPanel,
                Command::PreviousPanel,
                Command::NavigateUp,
                Command::NavigateDown,
                Command::ExpandNode,
                Command::CollapseNode,
                Command::SelectFile,
                Command::StartSearch,
                Command::EndSearch,
                Command::SearchInput('x'),
                Command::SearchBackspace,
                Command::HistoryUp,
                Command::HistoryDown,
                Command::SelectCommit,
                Command::InspectorUp,
                Command::InspectorDown,
                Command::InspectorPageUp,
                Command::InspectorPageDown,
                Command::InspectorHome,
                Command::InspectorEnd,
                Command::InspectorLeft,
                Command::InspectorRight,
                Command::GoToTop,
                Command::GoToBottom,
                Command::PreviousChange,
                Command::NextChange,
                Command::ToggleDiff,
            ];

            for command in commands {
                let is_quit_command = matches!(command, Command::Quit);
                let result = Executor::execute(&original_config, command);

                // Config should always be valid after execution
                assert!(!result.config.current_content.is_empty());
                assert!(result.config.cursor_line < result.config.current_content.len());

                // Only quit command should set should_quit
                if !is_quit_command {
                    assert!(!result.should_quit);
                }
            }
        }

        #[test]
        fn test_status_message_propagation() {
            let config = TestConfig::default();

            let result = Executor::execute(&config, Command::NextPanel);

            // Status message should be set in both result and config
            assert!(result.status_message.is_some());
            assert_eq!(result.config.status_message, result.status_message.unwrap());
        }

        #[test]
        fn test_execution_result_structure() {
            let config = TestConfig::default();

            let result = Executor::execute(&config, Command::Quit);

            // Verify ExecutionResult fields
            assert!(result.should_quit);
            assert!(result.status_message.is_some());
            assert_eq!(result.status_message, Some("Goodbye!".to_string()));
            assert_eq!(result.config.status_message, "Goodbye!");
        }

        #[test]
        fn test_panel_focus_consistency() {
            let config = TestConfig {
                active_panel: PanelFocus::Inspector,
                ..TestConfig::default()
            };

            // Navigator commands should be ignored
            let result = Executor::execute(&config, Command::NavigateUp);
            assert_eq!(result.config.active_panel, PanelFocus::Inspector);
            assert!(result.status_message.is_none());

            // History commands should be ignored
            let result = Executor::execute(&config, Command::HistoryUp);
            assert_eq!(result.config.active_panel, PanelFocus::Inspector);
            assert!(result.status_message.is_none());

            // Inspector commands should work
            let result = Executor::execute(&config, Command::InspectorUp);
            assert_eq!(result.config.active_panel, PanelFocus::Inspector);
            // Status message may or may not be set depending on cursor position
        }

        #[test]
        fn test_cursor_scroll_synchronization() {
            let mut config = create_test_config_with_content();
            config.cursor_line = 5;
            config.inspector_scroll_vertical = 10;

            // Moving up should adjust scroll if cursor goes above viewport
            let result = Executor::execute(&config, Command::InspectorUp);
            let final_config = result.config;

            if final_config.cursor_line < final_config.inspector_scroll_vertical as usize {
                assert_eq!(
                    final_config.inspector_scroll_vertical,
                    final_config.cursor_line as u16
                );
            }
        }
    }
}
