use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Config {
    pub colors: ColorConfig,
    pub layout: LayoutConfig,
    pub keybindings: KeybindingConfig,
}

#[derive(Debug, Clone)]
pub struct ColorConfig {
    pub active_border: Color,
    pub inactive_border: Color,
    pub selected_item: Color,
    pub line_numbers: Color,
    pub git_added: Color,
    pub git_modified: Color,
    pub git_deleted: Color,
    pub commit_hash: Color,
    pub commit_author: Color,
    pub commit_date: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
}

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub left_panel_width: u16,
    pub file_navigator_height: u16,
    pub show_line_numbers: bool,
    pub tab_size: usize,
}

#[derive(Debug, Clone)]
pub struct KeybindingConfig {
    pub quit: char,
    pub next_panel: char,
    pub previous_change: char,
    pub next_change: char,
    pub toggle_diff: char,
    pub search: char,
    pub goto_top: char,
    pub goto_bottom: char,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            colors: ColorConfig::default(),
            layout: LayoutConfig::default(),
            keybindings: KeybindingConfig::default(),
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            active_border: Color::Yellow,
            inactive_border: Color::White,
            selected_item: Color::DarkGray,
            line_numbers: Color::Blue,
            git_added: Color::Green,
            git_modified: Color::Yellow,
            git_deleted: Color::Red,
            commit_hash: Color::Yellow,
            commit_author: Color::Green,
            commit_date: Color::Blue,
            status_bar_bg: Color::DarkGray,
            status_bar_fg: Color::White,
        }
    }
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            left_panel_width: 35,
            file_navigator_height: 50,
            show_line_numbers: true,
            tab_size: 4,
        }
    }
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            quit: 'q',
            next_panel: '\t', // Tab
            previous_change: 'p',
            next_change: 'n',
            toggle_diff: 'd',
            search: '/',
            goto_top: 'g',
            goto_bottom: 'G',
        }
    }
}

impl Config {
    pub fn load() -> Self {
        // TODO: Load configuration from file
        // For now, return default configuration
        Self::default()
    }
    
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Save configuration to file
        Ok(())
    }
}