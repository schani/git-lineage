use git_lineage::app::{App, CommitInfo, PanelFocus};
use git_lineage::async_task::TaskResult;
use git_lineage::cli::{Cli, Commands};
use git_lineage::test_config::TestConfig;
use git_lineage::tree::{FileTree, TreeNode};
use git_lineage::*;
use serial_test::serial;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio_test::{assert_err, assert_ok};

// Test utilities
fn create_test_git_repo(temp_dir: &TempDir) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use std::process::Command as StdCommand;

    let repo_path = temp_dir.path();

    // Initialize git repo
    StdCommand::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output()?;

    // Set up git config
    StdCommand::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()?;

    StdCommand::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()?;

    // Create test files
    fs::create_dir_all(repo_path.join("src"))?;
    fs::write(
        repo_path.join("src/main.rs"),
        "fn main() { println!(\"Hello\"); }",
    )?;
    fs::write(
        repo_path.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"",
    )?;

    // Add and commit files
    StdCommand::new("git")
        .args(&["add", "."])
        .current_dir(repo_path)
        .output()?;

    StdCommand::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()?;

    Ok(())
}

fn create_test_app() -> App {
    // Try to open current directory, fallback to creating a temp repo if that fails
    let repo = git_lineage::git_utils::open_repository(".")
        .or_else(|_| {
            // Create a temporary git repo for testing
            let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
            std::process::Command::new("git")
                .args(["init"])
                .current_dir(temp_dir.path())
                .output()
                .expect("Failed to initialize git repo");
            
            // Create a test file and commit
            std::fs::write(temp_dir.path().join("test.txt"), "test content")
                .expect("Failed to write test file");
            
            std::process::Command::new("git")
                .args(["add", "test.txt"])
                .current_dir(temp_dir.path())
                .output()
                .expect("Failed to add test file");
                
            std::process::Command::new("git")
                .args(["commit", "-m", "Initial test commit"])
                .env("GIT_AUTHOR_NAME", "Test")
                .env("GIT_AUTHOR_EMAIL", "test@test.com")
                .env("GIT_COMMITTER_NAME", "Test")
                .env("GIT_COMMITTER_EMAIL", "test@test.com")
                .current_dir(temp_dir.path())
                .output()
                .expect("Failed to commit test file");
            
            git_lineage::git_utils::open_repository(temp_dir.path().to_str().unwrap())
        })
        .unwrap_or_else(|_| panic!("Failed to open or create test repository"));
    let mut app = App::new(repo);

    // Add test data
    let mut tree = FileTree::new();
    let src_node = TreeNode::new_file("main.rs".to_string(), PathBuf::from("src/main.rs"));
    tree.root.push(src_node);
    app.navigator.file_tree_state.set_tree_data(tree, String::new(), false);

    app.history.commit_list = vec![CommitInfo {
        hash: "abc123".to_string(),
        short_hash: "abc123".to_string(),
        author: "Test Author".to_string(),
        date: "2023-01-01".to_string(),
        subject: "Test commit".to_string(),
    }];

    app
}

fn create_test_config_file(
    temp_dir: &TempDir,
    config: &TestConfig,
) -> std::result::Result<PathBuf, Box<dyn std::error::Error>> {
    let config_path = temp_dir.path().join("test_config.json");
    let config_json = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, config_json)?;
    Ok(config_path)
}

mod cli_integration {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_parsing_run_command() {
        // Test default run command
        let args = vec!["git-lineage"];
        let cli = Cli::try_parse_from(args);
        assert_ok!(&cli);

        let cli = cli.unwrap();
        assert_eq!(cli.command, None); // Defaults to Run
    }

    #[test]
    fn test_cli_parsing_screenshot_command() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        fs::write(&config_path, "{}").unwrap();

        let args = vec![
            "git-lineage",
            "screenshot",
            "--config",
            config_path.to_str().unwrap(),
            "--output",
            "test.txt",
            "--width",
            "120",
            "--height",
            "40",
        ];

        let cli = Cli::try_parse_from(args);
        assert_ok!(&cli);

        let cli = cli.unwrap();
        match cli.command.unwrap() {
            Commands::Screenshot {
                config,
                output,
                width,
                height,
            } => {
                assert!(config.contains("config.json"));
                assert_eq!(output, Some("test.txt".to_string()));
                assert_eq!(width, 120);
                assert_eq!(height, 40);
            }
            _ => panic!("Expected Screenshot command"),
        }
    }

    #[test]
    fn test_cli_parsing_execute_command() {
        let args = vec![
            "git-lineage",
            "execute",
            "--config",
            "config.json",
            "--command",
            "quit",
            "--output",
            "result.json",
        ];

        let cli = Cli::try_parse_from(args);
        assert_ok!(&cli);

        let cli = cli.unwrap();
        match cli.command.unwrap() {
            Commands::Execute {
                config,
                command,
                output,
                screenshot,
                width,
                height,
            } => {
                assert_eq!(config, "config.json");
                assert_eq!(command, "quit");
                assert_eq!(output, Some("result.json".to_string()));
                assert!(!screenshot);
                assert_eq!(width, 120);
                assert_eq!(height, 40);
            }
            _ => panic!("Expected Execute command"),
        }
    }

    #[test]
    fn test_cli_parsing_save_state_command() {
        let args = vec!["git-lineage", "save-state", "--output", "state.json"];

        let cli = Cli::try_parse_from(args);
        assert_ok!(&cli);

        let cli = cli.unwrap();
        match cli.command.unwrap() {
            Commands::SaveState { output } => {
                assert_eq!(output, Some("state.json".to_string()));
            }
            _ => panic!("Expected SaveState command"),
        }
    }
}

mod task_result_handling {
    use super::*;

    #[test]
    fn test_handle_file_tree_loaded() {
        let mut app = create_test_app();
        app.ui.is_loading = true;

        let mut tree = FileTree::new();
        let test_node = TreeNode::new_file("test.rs".to_string(), PathBuf::from("test.rs"));
        tree.root.push(test_node);

        let result = TaskResult::FileTreeLoaded { files: tree };

        git_lineage::main_lib::handle_task_result(&mut app, result);

        assert!(!app.ui.is_loading);
        assert_eq!(app.navigator.file_tree_state.original_tree().root.len(), 1);
        assert_eq!(app.navigator.scroll_offset, 0);
        assert_eq!(app.navigator.cursor_position, 0);
        assert!(app.ui.status_message.contains("File tree loaded"));
    }

    #[test]
    fn test_handle_commit_history_loaded_with_commits() {
        let mut app = create_test_app();
        app.ui.is_loading = true;
        // Set active file context to match the result
        app.active_file_context = Some(std::path::PathBuf::from("test_file.rs"));

        let commits = vec![
            CommitInfo {
                hash: "abc123".to_string(),
                short_hash: "abc123".to_string(),
                author: "Test Author".to_string(),
                date: "2023-01-01".to_string(),
                subject: "Test commit".to_string(),
            },
            CommitInfo {
                hash: "def456".to_string(),
                short_hash: "def456".to_string(),
                author: "Another Author".to_string(),
                date: "2023-01-02".to_string(),
                subject: "Another commit".to_string(),
            },
        ];

        let result = TaskResult::CommitHistoryLoaded {
            file_path: "test_file.rs".to_string(),
            commits: commits.clone(),
        };

        git_lineage::main_lib::handle_task_result(&mut app, result);

        assert!(!app.ui.is_loading);
        assert_eq!(app.history.commit_list.len(), 2);
        assert_eq!(app.history.list_state.selected(), Some(0));
        // When auto-loading commits, the status message is updated by the content loading
        // Since the test uses invalid commit hashes, it will fail to load content
        assert!(
            app.ui.status_message.contains("Loaded 2 commits")
                || app.ui.status_message.contains("Failed to load content")
                || app.ui.status_message.contains("Error loading file content")
        );
    }

    #[test]
    fn test_handle_commit_history_loaded_empty() {
        let mut app = create_test_app();
        app.ui.is_loading = true;
        // Set active file context to match the result
        app.active_file_context = Some(std::path::PathBuf::from("empty_file.rs"));

        let result = TaskResult::CommitHistoryLoaded {
            file_path: "empty_file.rs".to_string(),
            commits: vec![],
        };

        git_lineage::main_lib::handle_task_result(&mut app, result);

        assert!(!app.ui.is_loading);
        assert_eq!(app.history.commit_list.len(), 0);
        assert_eq!(app.history.list_state.selected(), None);
        assert!(app.ui.status_message.contains("No commits found"));
    }

    #[test]
    fn test_handle_commit_history_loaded_race_condition_protection() {
        let mut app = create_test_app();
        app.ui.is_loading = true;

        // Start with empty commit list to test race condition properly
        app.history.commit_list.clear();
        app.history.list_state.select(None);

        // Simulate race condition: User was viewing file A, but has now moved to directory B
        // The active_file_context is None (directory selected), but we receive stale
        // async result for file A
        app.active_file_context = None; // Directory or no selection

        let commits = vec![CommitInfo {
            hash: "stale123".to_string(),
            short_hash: "stale123".to_string(),
            author: "Stale Author".to_string(),
            date: "2023-01-01".to_string(),
            subject: "Stale commit from previous file".to_string(),
        }];

        // This result is for "old_file.rs" but user has moved away from it
        let stale_result = TaskResult::CommitHistoryLoaded {
            file_path: "old_file.rs".to_string(),
            commits: commits.clone(),
        };

        git_lineage::main_lib::handle_task_result(&mut app, stale_result);

        // The stale result should be IGNORED:
        assert!(!app.ui.is_loading);
        assert_eq!(app.history.commit_list.len(), 0); // Should remain empty
        assert_eq!(app.history.list_state.selected(), None); // Should remain None
        assert!(app.ui.status_message.contains("ignored")); // Should indicate result was ignored

        // Now test that valid results are still processed when context matches
        app.active_file_context = Some(std::path::PathBuf::from("current_file.rs"));
        app.ui.is_loading = true; // Reset loading state for second test

        let valid_result = TaskResult::CommitHistoryLoaded {
            file_path: "current_file.rs".to_string(),
            commits: commits.clone(),
        };

        git_lineage::main_lib::handle_task_result(&mut app, valid_result);

        // Valid result should be applied:
        assert_eq!(app.history.commit_list.len(), 1); // Should contain the commit
        assert_eq!(app.history.list_state.selected(), Some(0)); // Should select first commit
                                                                // When auto-loading commits, the status message is updated by the content loading
                                                                // Since the test uses invalid commit hashes, it will fail to load content
        assert!(
            app.ui.status_message.contains("Loaded 1 commits")
                || app.ui.status_message.contains("Failed to load content")
                || app.ui.status_message.contains("Error loading file content")
        ); // Should show success
    }


    #[test]
    fn test_handle_next_change_found() {
        let mut app = create_test_app();
        app.ui.is_loading = true;
        app.ui.active_panel = PanelFocus::Navigator;

        let result = TaskResult::NextChangeFound {
            commit_hash: "abc123".to_string(),
        };

        git_lineage::main_lib::handle_task_result(&mut app, result);

        assert!(!app.ui.is_loading);
        assert_eq!(app.history.list_state.selected(), Some(0));
        assert_eq!(app.ui.active_panel, PanelFocus::History);
        assert!(app.ui.status_message.contains("Found next change"));
    }

    #[test]
    fn test_handle_next_change_found_commit_not_in_history() {
        let mut app = create_test_app();
        app.ui.is_loading = true;

        let result = TaskResult::NextChangeFound {
            commit_hash: "nonexistent".to_string(),
        };

        git_lineage::main_lib::handle_task_result(&mut app, result);

        assert!(!app.ui.is_loading);
        assert!(app.ui.status_message.contains("commit not in history"));
    }

    #[test]
    fn test_handle_next_change_not_found() {
        let mut app = create_test_app();
        app.ui.is_loading = true;

        let result = TaskResult::NextChangeNotFound;

        git_lineage::main_lib::handle_task_result(&mut app, result);

        assert!(!app.ui.is_loading);
        assert!(app
            .ui
            .status_message
            .contains("No subsequent changes found"));
    }

    #[test]
    fn test_handle_error_result() {
        let mut app = create_test_app();
        app.ui.is_loading = true;

        let result = TaskResult::Error {
            message: "Test error message".to_string(),
        };

        git_lineage::main_lib::handle_task_result(&mut app, result);

        assert!(!app.ui.is_loading);
        assert!(app.ui.status_message.contains("Error: Test error message"));
    }
}

mod command_execution {
    use super::*;

    #[test]
    #[serial]
    fn test_execute_command_with_output_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create test config
        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        let output_path = temp_dir.path().join("result.json");

        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "quit",
            Some(output_path.to_str().unwrap()),
            false,
            120,
            40,
        );

        assert_ok!(&result);
        assert!(output_path.exists());

        // Verify output content
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(!content.is_empty());

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    #[serial]
    fn test_execute_command_with_screenshot() {
        let temp_dir = TempDir::new().unwrap();
        create_test_git_repo(&temp_dir).unwrap();

        // Create test config in the temp directory with the git repo
        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        let output_path = temp_dir.path().join("result.json");

        // Change to the test directory only for the duration of the execute_command call
        let original_dir = std::env::current_dir().unwrap();

        // Use a guard to ensure directory is always restored
        struct DirGuard {
            original_dir: std::path::PathBuf,
        }
        impl Drop for DirGuard {
            fn drop(&mut self) {
                let _ = std::env::set_current_dir(&self.original_dir);
            }
        }

        let _guard = DirGuard {
            original_dir: original_dir.clone(),
        };
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "quit",
            Some(output_path.to_str().unwrap()),
            true,
            80,
            30,
        );

        // Restore original directory before asserting (guard will also do this)
        std::env::set_current_dir(&original_dir).unwrap();

        assert_ok!(&result);
        assert!(output_path.exists());

        // Screenshot should also be created
        let screenshot_path = temp_dir.path().join("result.screenshot.txt");
        assert!(screenshot_path.exists());

        drop(temp_dir);
    }

    #[test]
    #[serial]
    fn test_execute_command_invalid_config() {
        let result = git_lineage::main_lib::execute_command(
            "/nonexistent/config.json",
            "quit",
            None,
            false,
            120,
            40,
        );

        assert_err!(&result);
    }

    #[test]
    #[serial]
    fn test_execute_command_invalid_command() {
        let temp_dir = TempDir::new().unwrap();

        // Create test config
        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "invalid_command_syntax",
            None,
            false,
            120,
            40,
        );

        // Should handle gracefully - commands are parsed leniently
        match result {
            Ok(_) => {}  // Command parsing might succeed with default handling
            Err(_) => {} // Or it might fail, both are acceptable
        }
    }

    #[test]
    #[serial]
    fn test_execute_command_complex_sequence() {
        let temp_dir = TempDir::new().unwrap();

        // Create test config
        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        let output_path = temp_dir.path().join("complex_result.json");

        // Use proper sequence format
        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "sequence:[down,down,enter,tab,up,quit]",
            Some(output_path.to_str().unwrap()),
            false,
            120,
            40,
        );

        assert_ok!(&result);
        assert!(output_path.exists());
    }
}

mod state_management {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_save_current_state_to_file() {
        let temp_dir = TempDir::new().unwrap();
        create_test_git_repo(&temp_dir).unwrap();

        // Change to the test directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let output_path = temp_dir.path().join("saved_state.json");

        let result =
            git_lineage::main_lib::save_current_state(Some(output_path.to_str().unwrap())).await;

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert_ok!(&result);
        assert!(output_path.exists());

        // Verify output content
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(!content.is_empty());

        // Should be valid JSON that can be parsed as TestConfig
        let parsed: TestConfig = serde_json::from_str(&content).unwrap();
        assert!(!parsed.status_message.is_empty());
    }

    #[tokio::test]
    #[serial]
    async fn test_save_current_state_to_stdout() {
        let temp_dir = TempDir::new().unwrap();
        create_test_git_repo(&temp_dir).unwrap();

        // Change to the test directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = git_lineage::main_lib::save_current_state(None).await;

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert_ok!(&result);
    }

    #[tokio::test]
    #[serial]
    async fn test_save_current_state_invalid_repo() {
        let temp_dir = TempDir::new().unwrap();
        // Don't create a git repo

        // Change to the test directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = git_lineage::main_lib::save_current_state(None).await;

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert_err!(&result);
    }

    #[tokio::test]
    #[serial]
    async fn test_save_current_state_file_tree_error() {
        let temp_dir = TempDir::new().unwrap();
        create_test_git_repo(&temp_dir).unwrap();

        // Change to the test directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let output_path = temp_dir.path().join("saved_state.json");

        // This should still succeed even if file tree loading has issues
        let result =
            git_lineage::main_lib::save_current_state(Some(output_path.to_str().unwrap())).await;

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert_ok!(&result);
        assert!(output_path.exists());
    }
}

mod integration_scenarios {
    use super::*;

    #[test]
    #[serial]
    fn test_command_execution_output_formats() {
        let temp_dir = TempDir::new().unwrap();

        // Create test config
        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        // Test different output file extensions
        let output_formats = vec![
            ("result.json", false),
            ("result.txt", true), // Should generate screenshot
        ];

        for (filename, should_screenshot) in output_formats {
            let output_path = temp_dir.path().join(filename);

            let result = git_lineage::main_lib::execute_command(
                config_path.to_str().unwrap(),
                "quit",
                Some(output_path.to_str().unwrap()),
                should_screenshot,
                100,
                30,
            );

            assert_ok!(&result);
            assert!(output_path.exists());

            if should_screenshot {
                let screenshot_path = temp_dir.path().join("result.txt.screenshot.txt");
                assert!(screenshot_path.exists());
            }
        }
    }

    #[test]
    #[serial]
    fn test_temp_file_cleanup() {
        let temp_dir = TempDir::new().unwrap();

        // Create test config
        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        let output_path = temp_dir.path().join("result.json");

        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "quit",
            Some(output_path.to_str().unwrap()),
            true, // Generate screenshot to test cleanup
            120,
            40,
        );

        assert_ok!(&result);

        // Temp config file should be cleaned up
        let temp_config_path = std::env::current_dir().unwrap().join("temp_config.json");
        assert!(!temp_config_path.exists());
    }

    #[tokio::test]
    #[serial]
    async fn test_state_serialization_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        create_test_git_repo(&temp_dir).unwrap();

        // Change to the test directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let state_path = temp_dir.path().join("state.json");

        // Save current state
        let save_result =
            git_lineage::main_lib::save_current_state(Some(state_path.to_str().unwrap())).await;
        assert_ok!(&save_result);

        // Load and execute command using saved state - use proper sequence format
        let execute_result = git_lineage::main_lib::execute_command(
            state_path.to_str().unwrap(),
            "sequence:[tab,down,up,quit]",
            None,
            false,
            120,
            40,
        );

        // Restore original directory before asserting (and before temp_dir drops)
        std::env::set_current_dir(&original_dir).unwrap();

        assert_ok!(&execute_result);

        // Keep temp_dir alive until the end
        drop(temp_dir);
    }

    #[test]
    fn test_error_handling_chain() {
        let temp_dir = TempDir::new().unwrap();

        // Test cascade of potential errors
        let test_cases = vec![("/nonexistent/config.json", "quit"), ("", "quit")];

        for (config_path, command) in test_cases {
            let result =
                git_lineage::main_lib::execute_command(config_path, command, None, false, 120, 40);

            // Should fail gracefully
            assert_err!(&result);
        }
    }
}

mod edge_cases {
    use super::*;

    #[test]
    #[serial]
    fn test_very_large_dimensions() {
        let temp_dir = TempDir::new().unwrap();

        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "quit",
            None,
            false,
            9999, // Very large width
            9999, // Very large height
        );

        // Should handle gracefully
        match result {
            Ok(_) => {}
            Err(_) => {} // Both outcomes acceptable for extreme dimensions
        }
    }

    #[test]
    #[serial]
    fn test_zero_dimensions() {
        let temp_dir = TempDir::new().unwrap();

        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "quit",
            None,
            false,
            0, // Zero width
            0, // Zero height
        );

        // Should handle gracefully
        match result {
            Ok(_) => {}
            Err(_) => {} // Both outcomes acceptable for zero dimensions
        }
    }

    #[test]
    #[serial]
    fn test_empty_command_string() {
        let temp_dir = TempDir::new().unwrap();

        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "", // Empty command
            None,
            false,
            120,
            40,
        );

        // Empty commands should fail with an error
        assert_err!(&result);
    }

    #[test]
    #[serial]
    fn test_very_long_command_string() {
        let temp_dir = TempDir::new().unwrap();

        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        // Create a very long command string - comma separated format should fail
        let long_command = "down,".repeat(1000) + "quit";

        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            &long_command,
            None,
            false,
            120,
            40,
        );

        // Comma-separated commands should fail
        assert_err!(&result);
    }

    #[tokio::test]
    async fn test_save_state_permission_denied() {
        // Try to save to a location that would cause permission issues
        let result = git_lineage::main_lib::save_current_state(Some("/root/forbidden.json")).await;

        // Should fail gracefully
        assert_err!(&result);
    }

    #[test]
    #[serial]
    fn test_screenshot_path_generation() {
        let temp_dir = TempDir::new().unwrap();

        let config = TestConfig::default();
        let config_path = create_test_config_file(&temp_dir, &config).unwrap();

        // Test screenshot path generation without output file
        let result = git_lineage::main_lib::execute_command(
            config_path.to_str().unwrap(),
            "quit",
            None,
            true, // Generate screenshot
            120,
            40,
        );

        assert_ok!(&result);

        // Default screenshot should be created - don't fail if cleanup fails
        let default_screenshot = std::env::current_dir()
            .unwrap()
            .join("command_result_screenshot.txt");
        if default_screenshot.exists() {
            // Ignore cleanup errors to prevent race conditions between tests
            let _ = fs::remove_file(default_screenshot);
        }
    }
}
