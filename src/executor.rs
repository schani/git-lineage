use crate::{
    command::Command,
    test_config::TestConfig,
    app::PanelFocus,
};

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
                    status_message = Some(format!("Previous change for line {}", new_config.cursor_line + 1));
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
            let is_dir = config.file_tree.find_node(&selected_path)
                .map(|node| node.is_dir)
                .unwrap_or(false);
                
            if !is_dir {
                *status_message = Some(format!("Selected file: {}", selected_path.display()));
            } else {
                // Toggle directory expansion
                let was_expanded = config.file_tree.find_node(&selected_path)
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
        config.cursor_line = (config.cursor_line + 10).min(config.current_content.len().saturating_sub(1));
        *status_message = Some(format!("Page down - Line: {}", config.cursor_line + 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_panel() {
        let config = TestConfig {
            active_panel: PanelFocus::Navigator,
            ..TestConfig::default()
        };
        
        let result = Executor::execute(&config, Command::NextPanel);
        assert_eq!(result.config.active_panel, PanelFocus::History);
        assert!(result.status_message.is_some());
        assert!(!result.should_quit);
    }
    
    #[test]
    fn test_quit_command() {
        let config = TestConfig::default();
        
        let result = Executor::execute(&config, Command::Quit);
        assert!(result.should_quit);
        assert!(result.status_message.is_some());
    }
    
    #[test]
    fn test_search_commands() {
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
        
        // Add search input
        let result = Executor::execute(&config, Command::SearchInput('t'));
        config = result.config;
        assert_eq!(config.search_query, "t");
        
        // Add more input
        let result = Executor::execute(&config, Command::SearchInput('e'));
        config = result.config;
        assert_eq!(config.search_query, "te");
        
        // Backspace
        let result = Executor::execute(&config, Command::SearchBackspace);
        config = result.config;
        assert_eq!(config.search_query, "t");
        
        // End search
        let result = Executor::execute(&config, Command::EndSearch);
        config = result.config;
        assert!(!config.in_search_mode);
        assert!(config.search_query.is_empty());
    }
    
    #[test]
    fn test_sequence_command() {
        let config = TestConfig {
            active_panel: PanelFocus::Navigator,
            ..TestConfig::default()
        };
        
        let sequence = Command::Sequence(vec![
            Command::NextPanel,
            Command::NextPanel,
        ]);
        
        let result = Executor::execute(&config, sequence);
        assert_eq!(result.config.active_panel, PanelFocus::Inspector);
    }
}