use gix::Repository;
use ratatui::widgets::ListState;
use tui_tree_widget::TreeState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Navigator,
    History,
    Inspector,
}

#[derive(Debug, Clone)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub git_status: Option<char>,
    pub children: Vec<FileTreeNode>,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
}

pub struct App {
    pub repo: Repository,
    pub active_panel: PanelFocus,
    pub should_quit: bool,

    // Panel 1 State - File Navigator
    pub file_tree: Vec<FileTreeNode>,
    pub file_tree_state: TreeState<usize>,
    pub selected_file_path: Option<String>,
    pub search_query: String,
    pub in_search_mode: bool,

    // Panel 2 State - Commit History
    pub commit_list: Vec<CommitInfo>,
    pub commit_list_state: ListState,
    pub selected_commit_hash: Option<String>,

    // Panel 3 State - Code Inspector
    pub current_content: Vec<String>,
    pub current_blame: Option<String>, // Simplified for now
    pub inspector_scroll_vertical: u16,
    pub inspector_scroll_horizontal: u16,
    pub cursor_line: usize,
    pub cursor_column: usize,
    pub show_diff_view: bool,

    // UI State
    pub status_message: String,
    pub is_loading: bool,
}

impl App {
    pub fn new(repo: Repository) -> Self {
        Self {
            repo,
            active_panel: PanelFocus::Navigator,
            should_quit: false,

            file_tree: Vec::new(),
            file_tree_state: TreeState::default(),
            selected_file_path: None,
            search_query: String::new(),
            in_search_mode: false,

            commit_list: Vec::new(),
            commit_list_state: ListState::default(),
            selected_commit_hash: None,

            current_content: Vec::new(),
            current_blame: None,
            inspector_scroll_vertical: 0,
            inspector_scroll_horizontal: 0,
            cursor_line: 0,
            cursor_column: 0,
            show_diff_view: false,

            status_message: "Ready".to_string(),
            is_loading: false,
        }
    }

    pub fn next_panel(&mut self) {
        self.active_panel = match self.active_panel {
            PanelFocus::Navigator => PanelFocus::History,
            PanelFocus::History => PanelFocus::Inspector,
            PanelFocus::Inspector => PanelFocus::Navigator,
        };
    }

    pub fn previous_panel(&mut self) {
        self.active_panel = match self.active_panel {
            PanelFocus::Navigator => PanelFocus::Inspector,
            PanelFocus::History => PanelFocus::Navigator,
            PanelFocus::Inspector => PanelFocus::History,
        };
    }
}