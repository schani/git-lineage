use crossterm::event::{Event, KeyCode, KeyModifiers};
use tokio::sync::mpsc;

use crate::app::{App, PanelFocus};
use crate::async_task::Task;

pub mod code_inspector;
pub mod file_loader;
pub mod history;
pub mod inspector;
pub mod navigation;
pub mod navigator;

pub use code_inspector::*;
pub use file_loader::*;
pub use history::*;
pub use inspector::*;
pub use navigation::*;
pub use navigator::*;

pub fn handle_event(
    event: Event,
    app: &mut App,
    async_sender: &mpsc::Sender<Task>,
) -> Result<bool, Box<dyn std::error::Error>> { // Returns true if UI needs update
    let state_before = app.get_ui_state_hash();
    
    match event {
        Event::Key(key) => {
            // Global keybindings
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => {
                    if !is_in_search_mode(app) {
                        app.should_quit = true;
                        return Ok(false); // No render needed when quitting
                    }
                }
                KeyCode::Tab => {
                    if !is_in_search_mode(app) {
                        if key.modifiers.contains(KeyModifiers::SHIFT) {
                            app.previous_panel();
                        } else {
                            app.next_panel();
                        }
                        return Ok(true); // Panel change needs render
                    }
                }
                KeyCode::Char('1') => {
                    if !is_in_search_mode(app) {
                        app.ui.active_panel = PanelFocus::Navigator;
                        return Ok(true); // Panel change needs render
                    }
                }
                KeyCode::Char('2') => {
                    if !is_in_search_mode(app) {
                        app.ui.active_panel = PanelFocus::History;
                        return Ok(true); // Panel change needs render
                    }
                }
                KeyCode::Char('3') => {
                    if !is_in_search_mode(app) {
                        app.ui.active_panel = PanelFocus::Inspector;
                        return Ok(true); // Panel change needs render
                    }
                }
                KeyCode::Char('[') => {
                    if !is_in_search_mode(app) {
                        if navigate_to_older_commit(app) {
                            return Ok(true); // Commit navigation needs render
                        }
                    }
                }
                KeyCode::Char(']') => {
                    if !is_in_search_mode(app) {
                        if navigate_to_younger_commit(app) {
                            return Ok(true); // Commit navigation needs render
                        }
                    }
                }
                KeyCode::Char('l') => {
                    // Ctrl+L to force screen redraw
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.ui.force_redraw = true;
                        app.ui.status_message = "Screen refreshed".to_string();
                        return Ok(true); // Force redraw needs render
                    }
                }
                _ => {}
            }

            // Panel-specific keybindings
            match app.ui.active_panel {
                PanelFocus::Navigator => { handle_navigator_event(app, key.code, async_sender)?; },
                PanelFocus::History => { handle_history_event(app, key.code, async_sender)?; },
                PanelFocus::Inspector => { handle_inspector_event(app, key.code, async_sender)?; },
            }
        }
        Event::Resize(_, _) => {
            // Handle terminal resize if needed
        }
        _ => {}
    }

    // Check if UI state changed
    let state_after = app.get_ui_state_hash();
    Ok(state_before != state_after)
}

fn is_in_search_mode(app: &App) -> bool {
    app.navigator.file_tree_state.in_search_mode
        || (app.new_navigator.is_some()
            && app.new_navigator
                .as_ref()
                .unwrap()
                .is_searching())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
    use tokio::sync::mpsc;
    use crate::git_utils;
    
    #[test]
    fn test_ctrl_l_force_redraw() {
        // Setup
        let repo = git_utils::open_repository(".").expect("Should open test repo");
        let mut app = App::new(repo);
        let (task_sender, _task_receiver) = mpsc::channel::<Task>(32);
        
        // Initially force_redraw should be false
        assert!(!app.ui.force_redraw);
        
        // Create Ctrl+L key event
        let key_event = KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        };
        let event = Event::Key(key_event);
        
        // Handle the event
        handle_event(event, &mut app, &task_sender).expect("Should handle Ctrl+L");
        
        // Verify force_redraw is set and status message is updated
        assert!(app.ui.force_redraw, "force_redraw should be set to true");
        assert_eq!(app.ui.status_message, "Screen refreshed");
    }

    // Test utilities
    use std::path::PathBuf;
    
    fn create_test_app() -> App {
        let repo = crate::git_utils::open_repository(".")
            .unwrap_or_else(|_| panic!("Failed to open test repository"));
        let mut app = App::new(repo);

        // Set up a basic file tree for testing by building nodes manually
        use crate::tree::TreeNode;

        // Create root level nodes
        let src_main = TreeNode::new_file("main.rs".to_string(), "src/main.rs".into());
        let src_lib = TreeNode::new_file("lib.rs".to_string(), "src/lib.rs".into());
        let mut tests_dir = TreeNode::new_dir("tests".to_string(), "tests".into());
        let test_file = TreeNode::new_file("test.rs".to_string(), "tests/test.rs".into());

        // Add test file to tests directory
        tests_dir.add_child(test_file);

        // Add nodes to the file tree root
        let mut tree = crate::tree::FileTree::new();
        tree.root.push(src_main);
        tree.root.push(src_lib);
        tree.root.push(tests_dir);
        app.navigator.file_tree_state.set_tree_data(tree, String::new(), false);

        // Add some commits for testing
        app.history.commit_list = vec![
            crate::app::CommitInfo {
                hash: "abc123".to_string(),
                short_hash: "abc123".to_string(),
                author: "Test Author".to_string(),
                date: "2023-01-01".to_string(),
                subject: "Test commit".to_string(),
            },
            crate::app::CommitInfo {
                hash: "def456".to_string(),
                short_hash: "def456".to_string(),
                author: "Another Author".to_string(),
                date: "2023-01-02".to_string(),
                subject: "Another commit".to_string(),
            },
        ];

        app
    }

    fn create_key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn create_key_event_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    async fn create_test_channel() -> (mpsc::Sender<Task>, mpsc::Receiver<Task>) {
        mpsc::channel(100)
    }

    mod global_keybindings {
        use super::*;

        #[tokio::test]
        async fn test_quit_on_q() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Test 'q' key to quit
            let event = create_key_event(KeyCode::Char('q'));
            let result = handle_event(event, &mut app, &task_sender);

            assert!(result.is_ok());
            assert!(app.should_quit);
        }

        #[tokio::test]
        async fn test_quit_on_esc() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Test Esc key to quit
            let event = create_key_event(KeyCode::Esc);
            let result = handle_event(event, &mut app, &task_sender);

            assert!(result.is_ok());
            assert!(app.should_quit);
        }

        #[tokio::test]
        async fn test_tab_navigation() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Start at Navigator panel
            app.ui.active_panel = crate::app::PanelFocus::Navigator;

            // Test Tab key
            let event = create_key_event(KeyCode::Tab);
            let result = handle_event(event, &mut app, &task_sender);

            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, crate::app::PanelFocus::History);

            // Test Tab again
            let event = create_key_event(KeyCode::Tab);
            let result = handle_event(event, &mut app, &task_sender);

            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, crate::app::PanelFocus::Inspector);

            // Test Tab again to wrap around
            let event = create_key_event(KeyCode::Tab);
            let result = handle_event(event, &mut app, &task_sender);

            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, crate::app::PanelFocus::Navigator);
        }

        #[tokio::test]
        async fn test_shift_tab_navigation() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Start at Inspector panel
            app.ui.active_panel = crate::app::PanelFocus::Inspector;

            // Test Shift+Tab key
            let event = create_key_event_with_modifiers(KeyCode::Tab, KeyModifiers::SHIFT);
            let result = handle_event(event, &mut app, &task_sender);

            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, crate::app::PanelFocus::History);
        }

        #[tokio::test]
        async fn test_number_key_panel_switching() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Test '1' key
            let event = create_key_event(KeyCode::Char('1'));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, crate::app::PanelFocus::Navigator);

            // Test '2' key
            let event = create_key_event(KeyCode::Char('2'));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, crate::app::PanelFocus::History);

            // Test '3' key
            let event = create_key_event(KeyCode::Char('3'));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, crate::app::PanelFocus::Inspector);
        }

        #[tokio::test]
        async fn test_commit_navigation_keys() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Set up commit history with selection
            app.history.list_state.select(Some(0));

            // Test '[' key (older commit)
            let event = create_key_event(KeyCode::Char('['));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test ']' key (younger commit)
            let event = create_key_event(KeyCode::Char(']'));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
        }
    }

    mod navigator_events {
        use super::*;

        #[tokio::test]
        async fn test_navigator_up_down() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Set focus to Navigator
            app.ui.active_panel = crate::app::PanelFocus::Navigator;

            // Test Down key
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test Up key
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_navigator_left_right() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            app.ui.active_panel = crate::app::PanelFocus::Navigator;

            // Test Right key (expand)
            let event = create_key_event(KeyCode::Right);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test Left key (collapse)
            let event = create_key_event(KeyCode::Left);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_navigator_search_mode() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            app.ui.active_panel = crate::app::PanelFocus::Navigator;

            // Test '/' key to enter search mode
            let event = create_key_event(KeyCode::Char('/'));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
            assert!(app.navigator.file_tree_state.in_search_mode);

            // Test typing in search mode
            let event = create_key_event(KeyCode::Char('m'));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test Backspace in search mode
            let event = create_key_event(KeyCode::Backspace);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test Esc to exit search mode
            let event = create_key_event(KeyCode::Esc);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
            assert!(!app.navigator.file_tree_state.in_search_mode);
        }
    }

    mod history_events {
        use super::*;

        #[tokio::test]
        async fn test_history_navigation() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            app.ui.active_panel = crate::app::PanelFocus::History;

            // Test Down key
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test Up key
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
        }
    }

    mod inspector_events {
        use super::*;

        #[tokio::test]
        async fn test_inspector_navigation() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            app.ui.active_panel = crate::app::PanelFocus::Inspector;

            // Add some content to navigate
            app.inspector.current_content = vec![
                "Line 1".to_string(),
                "Line 2".to_string(),
                "Line 3".to_string(),
            ];

            // Test Down key
            let event = create_key_event(KeyCode::Down);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test Up key
            let event = create_key_event(KeyCode::Up);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test PageDown key
            let event = create_key_event(KeyCode::PageDown);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());

            // Test PageUp key
            let event = create_key_event(KeyCode::PageUp);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_inspector_diff_toggle() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            app.ui.active_panel = crate::app::PanelFocus::Inspector;

            // Test 'd' key to toggle diff view
            let event = create_key_event(KeyCode::Char('d'));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
        }
    }

    mod helper_functions {
        use super::*;

        #[test]
        fn test_is_in_search_mode_old_navigator() {
            let mut app = create_test_app();
            
            // Test with old navigator not in search mode
            app.navigator.file_tree_state.in_search_mode = false;
            assert!(!is_in_search_mode(&app));
            
            // Test with old navigator in search mode
            app.navigator.file_tree_state.in_search_mode = true;
            assert!(is_in_search_mode(&app));
        }
    }

    mod file_selection {
        use super::*;

        #[tokio::test]
        async fn test_file_selection_change_triggers_history_update() {
            let mut app = create_test_app();
            let (task_sender, mut task_receiver) = create_test_channel().await;

            // Simulate file selection
            app.navigator.file_tree_state.current_selection = Some(PathBuf::from("src/main.rs"));
            app.active_file_context = Some(PathBuf::from("src/main.rs"));

            // Call file selection change handler
            handle_file_selection_change(&mut app, &task_sender);

            // Check that a task was sent for loading commit history
            let task = task_receiver.try_recv();
            assert!(task.is_ok());
        }
    }

    mod navigation {
        use super::*;

        #[test]
        fn test_navigate_to_older_commit() {
            let mut app = create_test_app();
            
            // Set active file context (required for navigation)
            app.active_file_context = Some(PathBuf::from("src/main.rs"));
            
            // Set initial selection to first commit (index 0)
            app.history.list_state.select(Some(0));
            
            // Navigate to older commit (should move to index 1)
            let result = navigate_to_older_commit(&mut app);
            assert!(result);
            assert_eq!(app.history.list_state.selected(), Some(1));
        }

        #[test]
        fn test_navigate_to_younger_commit() {
            let mut app = create_test_app();
            
            // Set active file context (required for navigation)
            app.active_file_context = Some(PathBuf::from("src/main.rs"));
            
            // Set initial selection to second commit (index 1)
            app.history.list_state.select(Some(1));
            
            // Navigate to younger commit (should move to index 0)
            let result = navigate_to_younger_commit(&mut app);
            assert!(result);
            assert_eq!(app.history.list_state.selected(), Some(0));
        }

        #[test]
        fn test_navigate_to_older_commit_at_end() {
            let mut app = create_test_app();
            
            // Set active file context (required for navigation)
            app.active_file_context = Some(PathBuf::from("src/main.rs"));
            
            // Set selection to last commit
            let last_index = app.history.commit_list.len() - 1;
            app.history.list_state.select(Some(last_index));
            
            // Try to navigate to older commit (should not change)
            let result = navigate_to_older_commit(&mut app);
            assert!(!result);
            assert_eq!(app.history.list_state.selected(), Some(last_index));
        }

        #[test]
        fn test_navigate_to_younger_commit_at_beginning() {
            let mut app = create_test_app();
            
            // Set active file context (required for navigation)
            app.active_file_context = Some(PathBuf::from("src/main.rs"));
            
            // Set selection to first commit (index 0)
            app.history.list_state.select(Some(0));
            
            // Try to navigate to younger commit (should not change)
            let result = navigate_to_younger_commit(&mut app);
            assert!(!result);
            assert_eq!(app.history.list_state.selected(), Some(0));
        }
    }

    mod edge_cases {
        use super::*;

        #[tokio::test]
        async fn test_events_during_search_mode() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Enable search mode
            app.navigator.file_tree_state.in_search_mode = true;

            // Test that quit keys don't work in search mode
            let event = create_key_event(KeyCode::Char('q'));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
            assert!(!app.should_quit); // Should not quit

            // Test that tab navigation doesn't work in search mode
            let original_panel = app.ui.active_panel;
            let event = create_key_event(KeyCode::Tab);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
            assert_eq!(app.ui.active_panel, original_panel); // Should not change
        }

        #[tokio::test]
        async fn test_resize_event() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            let event = Event::Resize(80, 24);
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_unknown_key_event() {
            let mut app = create_test_app();
            let (task_sender, _task_receiver) = create_test_channel().await;

            // Test with an unmapped key
            let event = create_key_event(KeyCode::F(1));
            let result = handle_event(event, &mut app, &task_sender);
            assert!(result.is_ok());
        }
    }
}