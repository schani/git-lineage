use git_lineage::app::App;
use git_lineage::async_task::{Task, TaskResult};
use git_lineage::git_utils;
use git_lineage::test_runner::TestRunner;
use std::path::Path;
use std::env;
use tokio::sync::mpsc;

/// Test driver for running script-based UI tests as regular Rust tests
/// This provides a reusable interface for converting .test files into Rust test functions
pub struct ScriptTestDriver {
    test_repo_path: std::path::PathBuf,
    original_dir: std::path::PathBuf,
}

impl ScriptTestDriver {
    /// Create a new test driver that runs tests in the test-repo submodule
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let original_dir = env::current_dir()?;
        
        // Find the test repository by looking for the manifest directory
        // When tests run, they might be in various temporary directories
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let manifest_path = std::path::Path::new(manifest_dir);
        let test_repo_path = manifest_path.join("tests/test-repo");
        
        if !test_repo_path.exists() {
            return Err(format!("Test repository not found at: {}", test_repo_path.display()).into());
        }
        
        Ok(Self {
            test_repo_path,
            original_dir,
        })
    }
    
    /// Run a script test and verify all screenshots match expected results
    /// This is the "verify" mode - tests fail if screenshots don't match
    /// test_name should be the directory name under tests/scripts/ (e.g., "search_label_immediate")
    pub async fn run_script_test(&self, test_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Change to test repo directory for app execution
        env::set_current_dir(&self.test_repo_path)?;
        
        // Ensure we restore the original directory even if test fails
        let _guard = DirectoryGuard::new(&self.original_dir);
        
        // Set up the application and async task system
        // Use the absolute path to open the repository
        let repo = git_utils::open_repository(&self.test_repo_path).map_err(|e| format!("Failed to open repo: {}", e))?;
        let mut app = App::new(repo);
        
        // Set up task communication channels
        let (task_sender, task_receiver) = mpsc::channel::<Task>(100);
        let (result_sender, result_receiver) = mpsc::channel::<TaskResult>(100);
        
        // Start the async task worker
        let repo_path = self.test_repo_path.to_string_lossy().to_string();
        let async_worker = tokio::spawn(async move {
            git_lineage::async_task::run_worker(task_receiver, result_sender, repo_path).await;
        });
        
        // Load initial file tree (same as interactive/headless modes)
        if let Err(e) = task_sender.send(Task::LoadFileTree).await {
            return Err(format!("Failed to send LoadFileTree task: {}", e).into());
        }
        
        // Give some time for initial loading
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Create TestRunner in verify mode (overwrite_mode = false)
        // Resolve test directory and script file relative to the manifest directory
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let manifest_path = std::path::Path::new(manifest_dir);
        let test_dir = manifest_path.join("tests/scripts").join(test_name);
        let script_file = test_dir.join("script");
        let test_script = std::fs::read_to_string(&script_file)
            .map_err(|e| format!("Failed to read script file {:?}: {}", script_file, e))?;
        let mut test_runner = TestRunner::from_string(&test_script)?;
        test_runner.overwrite_mode = false;
        
        // IMPORTANT: Set screenshot base directory to the test's directory
        // The app runs in test-repo but screenshots should be saved/verified in the test dir
        test_runner.screenshot_base_dir = Some(test_dir);
        
        // Run the test - this will verify screenshots match expected results
        let result = test_runner.run(&mut app, &task_sender, result_receiver).await;
        
        // Clean up async worker
        async_worker.abort();
        
        match result {
            Ok(test_result) => {
                if test_result.success {
                    Ok(())
                } else {
                    // Build error message from test errors
                    let error_msg = if !test_result.errors.is_empty() {
                        format!("Test failed with errors: {}", test_result.errors.join(", "))
                    } else {
                        "Test failed".to_string()
                    };
                    Err(error_msg.into())
                }
            }
            Err(e) => Err(format!("Script test failed: {}", e).into()),
        }
    }
    
    /// Run a script test and overwrite/create screenshots (for test creation/updates)
    /// This should generally only be used during test development, not in CI
    /// test_name should be the directory name under tests/scripts/ (e.g., "search_label_immediate")
    pub async fn update_script_test(&self, test_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Change to test repo directory for app execution
        env::set_current_dir(&self.test_repo_path)?;
        
        // Ensure we restore the original directory even if test fails
        let _guard = DirectoryGuard::new(&self.original_dir);
        
        // Set up the application and async task system
        // Use the absolute path to open the repository
        let repo = git_utils::open_repository(&self.test_repo_path).map_err(|e| format!("Failed to open repo: {}", e))?;
        let mut app = App::new(repo);
        
        // Set up task communication channels
        let (task_sender, task_receiver) = mpsc::channel::<Task>(100);
        let (result_sender, result_receiver) = mpsc::channel::<TaskResult>(100);
        
        // Start the async task worker
        let repo_path = self.test_repo_path.to_string_lossy().to_string();
        let async_worker = tokio::spawn(async move {
            git_lineage::async_task::run_worker(task_receiver, result_sender, repo_path).await;
        });
        
        // Load initial file tree (same as interactive/headless modes)
        if let Err(e) = task_sender.send(Task::LoadFileTree).await {
            return Err(format!("Failed to send LoadFileTree task: {}", e).into());
        }
        
        // Give some time for initial loading
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        // Create TestRunner in overwrite mode (overwrite_mode = true)
        // Resolve test directory and script file relative to the manifest directory
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let manifest_path = std::path::Path::new(manifest_dir);
        let test_dir = manifest_path.join("tests/scripts").join(test_name);
        let script_file = test_dir.join("script");
        let test_script = std::fs::read_to_string(&script_file)
            .map_err(|e| format!("Failed to read script file {:?}: {}", script_file, e))?;
        let mut test_runner = TestRunner::from_string(&test_script)?;
        test_runner.overwrite_mode = true;
        
        // IMPORTANT: Set screenshot base directory to the test's directory
        // The app runs in test-repo but screenshots should be saved/verified in the test dir
        test_runner.screenshot_base_dir = Some(test_dir);
        
        // Run the test - this will create/update screenshots
        let result = test_runner.run(&mut app, &task_sender, result_receiver).await;
        
        // Clean up async worker
        async_worker.abort();
        
        match result {
            Ok(test_result) => {
                if test_result.success {
                    Ok(())
                } else {
                    // Build error message from test errors
                    let error_msg = if !test_result.errors.is_empty() {
                        format!("Test update failed with errors: {}", test_result.errors.join(", "))
                    } else {
                        "Test update failed".to_string()
                    };
                    Err(error_msg.into())
                }
            }
            Err(e) => Err(format!("Script test update failed: {}", e).into()),
        }
    }
}

/// RAII guard to ensure we always restore the original working directory
struct DirectoryGuard {
    original_dir: std::path::PathBuf,
}

impl DirectoryGuard {
    fn new(original_dir: &Path) -> Self {
        Self {
            original_dir: original_dir.to_path_buf(),
        }
    }
}

impl Drop for DirectoryGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.original_dir);
    }
}

// Macro to easily create script test functions
macro_rules! script_test {
    ($test_name:ident, $test_dir:expr) => {
        #[tokio::test]
        async fn $test_name() {
            let driver = ScriptTestDriver::new().expect("Failed to create test driver");
            driver.run_script_test($test_dir).await.expect("Script test failed");
        }
    };
}

// Define actual test functions using the macro
script_test!(test_search_label_immediate, "search_label_immediate");
script_test!(test_search_exit_with_enter, "test_search_exit_with_enter");
script_test!(test_search_f_enter_consistency, "test_search_f_enter_consistency");
script_test!(test_search_rea_selection, "test_search_rea_selection");
script_test!(test_enter_toggle_directory, "test_enter_toggle_directory");
script_test!(test_search_selection_movement, "test_search_selection_movement");
script_test!(test_direct_file_selection, "test_direct_file_selection");
script_test!(test_enter_inspector_focus, "test_enter_inspector_focus");
script_test!(test_search_q_doesnt_quit, "test_search_q_doesnt_quit");
script_test!(test_diff_view_persistence, "test_diff_view_persistence");
script_test!(test_diff_view_panel_switch, "test_diff_view_panel_switch");

// Additional script tests can be added here as they are created
// script_test!(test_basic_navigation, "basic_navigation");
// script_test!(test_search_functionality, "search_functionality");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_creation() {
        let driver = ScriptTestDriver::new();
        assert!(driver.is_ok(), "Should be able to create test driver");
    }

    // This test is no longer valid because we use CARGO_MANIFEST_DIR
    // which always resolves correctly regardless of current directory

    #[tokio::test]
    async fn test_nonexistent_script_file() {
        let driver = ScriptTestDriver::new().expect("Failed to create test driver");
        let result = driver.run_script_test("tests/nonexistent.test").await;
        assert!(result.is_err(), "Should fail for nonexistent test file");
    }
}