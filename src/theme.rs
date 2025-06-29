use ratatui::style::Color;

/// Theme data structure containing all colors used in the application
#[derive(Debug, Clone)]
pub struct Theme {
    // Panel borders
    pub active_border: Color,
    pub inactive_border: Color,

    // File navigator
    pub file_selected_bg: Color,
    pub file_selected_fg: Color,
    pub file_directory: Color,
    pub file_git_modified: Color,
    pub file_git_added: Color,
    pub file_git_deleted: Color,
    pub file_git_untracked: Color,
    pub file_default: Color,
    pub search_text: Color,

    // Commit history
    pub commit_hash: Color,
    pub commit_author: Color,
    pub commit_date: Color,
    pub commit_selected_bg: Color,
    pub commit_selected_fg: Color,

    // Code inspector
    pub line_numbers: Color,
    pub line_numbers_current: Color,
    pub code_background_current: Color,
    pub code_foreground_current: Color,
    pub syntax_keyword: Color,
    pub syntax_string: Color,
    pub syntax_comment: Color,
    pub code_default: Color,

    // Status bar
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub status_help_text: Color,

    // General UI
    pub panel_title: Color,
    pub text_default: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Panel borders
            active_border: Color::Yellow,
            inactive_border: Color::DarkGray,

            // File navigator
            file_selected_bg: Color::White,
            file_selected_fg: Color::Black,
            file_directory: Color::Blue,
            file_git_modified: Color::Yellow,
            file_git_added: Color::Green,
            file_git_deleted: Color::Red,
            file_git_untracked: Color::Magenta,
            file_default: Color::Reset,
            search_text: Color::Gray,

            // Commit history
            commit_hash: Color::Yellow,
            commit_author: Color::Green,
            commit_date: Color::Blue,
            commit_selected_bg: Color::White,
            commit_selected_fg: Color::Black,

            // Code inspector
            line_numbers: Color::Blue,
            line_numbers_current: Color::Yellow,
            code_background_current: Color::White,
            code_foreground_current: Color::Black,
            syntax_keyword: Color::Magenta,
            syntax_string: Color::Green,
            syntax_comment: Color::Yellow,
            code_default: Color::Reset,

            // Status bar
            status_bar_bg: Color::DarkGray,
            status_bar_fg: Color::White,
            status_help_text: Color::Gray,

            // General UI
            panel_title: Color::Gray,
            text_default: Color::Reset,
        }
    }
}

/// Get the current theme
/// This function returns the theme configuration for the application
pub fn get_theme() -> Theme {
    // For now, return the default theme
    // In the future, this could load from configuration files,
    // environment variables, or user preferences
    Theme::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_theme_returns_valid_theme() {
        let theme = get_theme();

        // Test that we get a valid theme with expected colors
        assert_eq!(theme.active_border, Color::Yellow);
        assert_eq!(theme.inactive_border, Color::DarkGray);
        assert_eq!(theme.file_git_added, Color::Green);
        assert_eq!(theme.status_bar_bg, Color::DarkGray);
    }

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();

        // Verify some key colors are set correctly
        assert_eq!(theme.commit_hash, Color::Yellow);
        assert_eq!(theme.commit_author, Color::Green);
        assert_eq!(theme.line_numbers, Color::Blue);
    }

    #[test]
    fn test_theme_clone() {
        let theme1 = get_theme();
        let theme2 = theme1.clone();

        assert_eq!(theme1.active_border, theme2.active_border);
        assert_eq!(theme1.status_bar_bg, theme2.status_bar_bg);
    }
}
