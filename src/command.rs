use serde::{Deserialize, Serialize};

/// Represents all possible user commands that can be executed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    // Global commands
    NextPanel,
    PreviousPanel,
    Quit,

    // File Navigator commands
    NavigateUp,
    NavigateDown,
    ExpandNode,
    CollapseNode,
    SelectFile,
    StartSearch,
    EndSearch,
    SearchInput(char),
    SearchBackspace,

    // Commit History commands
    HistoryUp,
    HistoryDown,
    SelectCommit,

    // Code Inspector commands
    InspectorUp,
    InspectorDown,
    InspectorPageUp,
    InspectorPageDown,
    InspectorHome,
    InspectorEnd,
    InspectorLeft,
    InspectorRight,
    GoToTop,
    GoToBottom,
    PreviousChange,
    NextChange,
    ToggleDiff,

    // Multi-step commands for testing
    Sequence(Vec<Command>),
}

impl Command {
    /// Parse a command from a string representation
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "next_panel" | "tab" => Ok(Command::NextPanel),
            "previous_panel" | "shift_tab" => Ok(Command::PreviousPanel),
            "quit" | "q" => Ok(Command::Quit),
            
            "navigate_up" | "up" => Ok(Command::NavigateUp),
            "navigate_down" | "down" => Ok(Command::NavigateDown),
            "expand" | "right" => Ok(Command::ExpandNode),
            "collapse" | "left" => Ok(Command::CollapseNode),
            "select_file" | "enter" => Ok(Command::SelectFile),
            "start_search" | "/" => Ok(Command::StartSearch),
            "end_search" | "escape" => Ok(Command::EndSearch),
            "search_backspace" | "backspace" => Ok(Command::SearchBackspace),
            
            "history_up" => Ok(Command::HistoryUp),
            "history_down" => Ok(Command::HistoryDown),
            "select_commit" => Ok(Command::SelectCommit),
            
            "inspector_up" => Ok(Command::InspectorUp),
            "inspector_down" => Ok(Command::InspectorDown),
            "inspector_page_up" | "page_up" => Ok(Command::InspectorPageUp),
            "inspector_page_down" | "page_down" => Ok(Command::InspectorPageDown),
            "inspector_home" | "home" => Ok(Command::InspectorHome),
            "inspector_end" | "end" => Ok(Command::InspectorEnd),
            "inspector_left" => Ok(Command::InspectorLeft),
            "inspector_right" => Ok(Command::InspectorRight),
            "goto_top" | "g" => Ok(Command::GoToTop),
            "goto_bottom" | "shift_g" => Ok(Command::GoToBottom),
            "previous_change" | "p" => Ok(Command::PreviousChange),
            "next_change" | "n" => Ok(Command::NextChange),
            "toggle_diff" | "d" => Ok(Command::ToggleDiff),
            
            _ => {
                if s.starts_with("search:") {
                    if let Some(char_str) = s.strip_prefix("search:") {
                        if let Some(ch) = char_str.chars().next() {
                            return Ok(Command::SearchInput(ch));
                        }
                    }
                }
                
                if s.starts_with("sequence:[") && s.ends_with("]") {
                    // Parse sequence: sequence:[cmd1,cmd2,cmd3]
                    let inner = &s[10..s.len()-1]; // Remove "sequence:[" and "]"
                    if inner.is_empty() {
                        return Ok(Command::Sequence(vec![]));
                    }
                    
                    let command_strings: Vec<&str> = inner.split(',').collect();
                    let mut commands = Vec::new();
                    
                    for cmd_str in command_strings {
                        let cmd_str = cmd_str.trim();
                        match Command::from_string(cmd_str) {
                            Ok(cmd) => commands.push(cmd),
                            Err(e) => return Err(format!("Invalid command in sequence '{}': {}", cmd_str, e)),
                        }
                    }
                    
                    return Ok(Command::Sequence(commands));
                }
                
                Err(format!("Unknown command: {}", s))
            }
        }
    }

    /// Convert command to string representation
    pub fn to_string(&self) -> String {
        match self {
            Command::NextPanel => "next_panel".to_string(),
            Command::PreviousPanel => "previous_panel".to_string(),
            Command::Quit => "quit".to_string(),
            
            Command::NavigateUp => "navigate_up".to_string(),
            Command::NavigateDown => "navigate_down".to_string(),
            Command::ExpandNode => "expand".to_string(),
            Command::CollapseNode => "collapse".to_string(),
            Command::SelectFile => "select_file".to_string(),
            Command::StartSearch => "start_search".to_string(),
            Command::EndSearch => "end_search".to_string(),
            Command::SearchInput(ch) => format!("search:{}", ch),
            Command::SearchBackspace => "search_backspace".to_string(),
            
            Command::HistoryUp => "history_up".to_string(),
            Command::HistoryDown => "history_down".to_string(),
            Command::SelectCommit => "select_commit".to_string(),
            
            Command::InspectorUp => "inspector_up".to_string(),
            Command::InspectorDown => "inspector_down".to_string(),
            Command::InspectorPageUp => "inspector_page_up".to_string(),
            Command::InspectorPageDown => "inspector_page_down".to_string(),
            Command::InspectorHome => "inspector_home".to_string(),
            Command::InspectorEnd => "inspector_end".to_string(),
            Command::InspectorLeft => "inspector_left".to_string(),
            Command::InspectorRight => "inspector_right".to_string(),
            Command::GoToTop => "goto_top".to_string(),
            Command::GoToBottom => "goto_bottom".to_string(),
            Command::PreviousChange => "previous_change".to_string(),
            Command::NextChange => "next_change".to_string(),
            Command::ToggleDiff => "toggle_diff".to_string(),
            
            Command::Sequence(commands) => {
                format!("sequence:[{}]", 
                    commands.iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(","))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        assert_eq!(Command::from_string("next_panel").unwrap(), Command::NextPanel);
        assert_eq!(Command::from_string("tab").unwrap(), Command::NextPanel);
        assert_eq!(Command::from_string("up").unwrap(), Command::NavigateUp);
        assert_eq!(Command::from_string("search:a").unwrap(), Command::SearchInput('a'));
        
        assert!(Command::from_string("invalid").is_err());
        assert!(Command::from_string("").is_err()); // Empty string should fail
        assert!(Command::from_string("down,up,quit").is_err()); // Comma separated should fail
    }

    #[test]
    fn test_command_to_string() {
        assert_eq!(Command::NextPanel.to_string(), "next_panel");
        assert_eq!(Command::SearchInput('x').to_string(), "search:x");
        assert_eq!(Command::ToggleDiff.to_string(), "toggle_diff");
    }
}