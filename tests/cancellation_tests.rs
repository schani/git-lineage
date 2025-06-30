use git_lineage::async_task::{Task, TaskResult, run_worker};
use git_lineage::app::App;
use git_lineage::git_utils::get_commit_history_streaming;
use serial_test::serial;
use std::fs;
use std::process::Command as StdCommand;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// Create a test git repository with many commits to test cancellation
fn create_large_test_git_repo(temp_dir: &TempDir) -> std::result::Result<(), Box<dyn std::error::Error>> {
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

    // Create initial file
    fs::create_dir_all(repo_path.join("src"))?;
    fs::write(
        repo_path.join("src/test.rs"),
        "fn main() { println!(\"Initial\"); }",
    )?;

    // Add and commit initial file
    StdCommand::new("git")
        .args(&["add", "."])
        .current_dir(repo_path)
        .output()?;

    StdCommand::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()?;

    // Create many more commits with changes to the target file
    for i in 1..=20 {
        fs::write(
            repo_path.join("src/test.rs"),
            format!("fn main() {{ println!(\"Version {}\"); }}", i),
        )?;

        StdCommand::new("git")
            .args(&["add", "src/test.rs"])
            .current_dir(repo_path)
            .output()?;

        StdCommand::new("git")
            .args(&["commit", "-m", &format!("Update version {}", i)])
            .current_dir(repo_path)
            .output()?;
    }

    // Create many commits that DON'T modify the target file
    // These will be processed but won't trigger the callback
    for i in 1..=50 {
        fs::write(
            repo_path.join(format!("other{}.txt", i)),
            format!("Other file content {}", i),
        )?;

        StdCommand::new("git")
            .args(&["add", &format!("other{}.txt", i)])
            .current_dir(repo_path)
            .output()?;

        StdCommand::new("git")
            .args(&["commit", "-m", &format!("Add other file {}", i)])
            .current_dir(repo_path)
            .output()?;
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_streaming_task_cancellation_immediate() {
    // Test that cancellation works immediately when token is cancelled before task starts
    let temp_dir = TempDir::new().unwrap();
    create_large_test_git_repo(&temp_dir).unwrap();

    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    let file_path = "src/test.rs".to_string();

    // Create and immediately cancel the token
    let cancellation_token = CancellationToken::new();
    cancellation_token.cancel();

    let repo = git_lineage::git_utils::open_repository(&repo_path).unwrap();
    
    let start_time = Instant::now();
    let mut callback_count = 0;
    
    let result = get_commit_history_streaming(
        &repo,
        &file_path,
        |_commit, _total| {
            callback_count += 1;
            true
        },
        &cancellation_token,
    );

    let elapsed = start_time.elapsed();
    
    // Should complete very quickly due to immediate cancellation
    assert!(elapsed < Duration::from_millis(100), "Should cancel immediately");
    assert_eq!(callback_count, 0, "Callback should not be called when cancelled immediately");
    assert!(result.is_ok(), "Function should return Ok even when cancelled");
}

#[tokio::test]
#[serial] 
async fn test_streaming_task_cancellation_during_processing() {
    // Test that cancellation works while processing commits
    let temp_dir = TempDir::new().unwrap();
    create_large_test_git_repo(&temp_dir).unwrap();

    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    let file_path = "src/test.rs".to_string();

    let cancellation_token = CancellationToken::new();
    let cancel_token_clone = cancellation_token.clone();

    // Cancel the token after a short delay
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel_token_clone.cancel();
    });

    let repo = git_lineage::git_utils::open_repository(&repo_path).unwrap();
    
    let start_time = Instant::now();
    let mut callback_count = 0;
    
    let result = get_commit_history_streaming(
        &repo,
        &file_path,
        |_commit, _total| {
            callback_count += 1;
            true
        },
        &cancellation_token,
    );

    let elapsed = start_time.elapsed();
    
    // Should complete reasonably quickly due to cancellation
    assert!(elapsed < Duration::from_secs(2), "Should cancel within reasonable time");
    assert!(result.is_ok(), "Function should return Ok even when cancelled");
    
    // May have found some commits before cancellation
    println!("Found {} commits before cancellation", callback_count);
}

#[tokio::test]
#[serial]
async fn test_async_worker_streaming_task_cancellation() {
    // Test cancellation through the full async worker pipeline
    let temp_dir = TempDir::new().unwrap();
    create_large_test_git_repo(&temp_dir).unwrap();

    let repo_path = temp_dir.path().to_str().unwrap().to_string();
    let file_path = "src/test.rs".to_string();

    let (task_sender, task_receiver) = mpsc::channel(32);
    let (result_sender, mut result_receiver) = mpsc::channel(32);

    // Start the worker
    let worker_repo_path = repo_path.clone();
    let worker_handle = tokio::spawn(async move {
        run_worker(task_receiver, result_sender, worker_repo_path).await;
    });

    // Create cancellation token and send streaming task
    let cancellation_token = CancellationToken::new();
    let task = Task::LoadCommitHistoryStreaming {
        file_path: file_path.clone(),
        cancellation_token: cancellation_token.clone(),
    };

    task_sender.send(task).await.unwrap();

    // Cancel after a short delay
    let cancel_token_clone = cancellation_token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        cancel_token_clone.cancel();
    });

    let start_time = Instant::now();
    let mut commit_found_count = 0;
    let mut completion_received = false;

    // Collect results with timeout
    let timeout_duration = Duration::from_secs(3);
    while let Ok(Some(result)) = timeout(timeout_duration, result_receiver.recv()).await {
        match result {
            TaskResult::CommitFound { file_path: path, .. } => {
                assert_eq!(path, file_path);
                commit_found_count += 1;
            }
            TaskResult::CommitHistoryComplete { file_path: path, total_commits } => {
                assert_eq!(path, file_path);
                completion_received = true;
                println!("Completed with {} total commits", total_commits);
                break;
            }
            TaskResult::Error { message } => {
                println!("Task error: {}", message);
                break;
            }
            _ => {
                panic!("Unexpected task result: {:?}", result);
            }
        }
    }

    let elapsed = start_time.elapsed();
    
    // Should complete due to cancellation within reasonable time
    assert!(elapsed < Duration::from_secs(2), "Should cancel within reasonable time");
    assert!(completion_received || commit_found_count > 0, "Should receive some results before cancellation");

    // Clean shutdown
    drop(task_sender);
    let _ = timeout(Duration::from_secs(1), worker_handle).await;
}

#[tokio::test]
#[serial]
async fn test_app_state_cancellation_on_file_switch() {
    // Test that switching files in app state properly cancels previous streaming tasks
    let temp_dir = TempDir::new().unwrap();
    create_large_test_git_repo(&temp_dir).unwrap();

    let repo = git_lineage::git_utils::open_repository(temp_dir.path()).unwrap();
    let mut app = App::new(repo);

    // Simulate setting up a streaming task
    let cancellation_token1 = CancellationToken::new();
    app.history.streaming_cancellation_token = Some(cancellation_token1.clone());
    
    // Verify token is not cancelled initially
    assert!(!cancellation_token1.is_cancelled());

    // Switch to a new file - this should cancel the previous token
    app.history.reset_for_new_file();

    // Verify the old token was cancelled
    assert!(cancellation_token1.is_cancelled());
    
    // Verify the token was cleared from app state
    assert!(app.history.streaming_cancellation_token.is_none());
    
    // Set up a new streaming task
    let cancellation_token2 = CancellationToken::new();
    app.history.streaming_cancellation_token = Some(cancellation_token2.clone());
    
    // Switch files again
    app.history.reset_for_new_file();
    
    // Verify the second token was also cancelled
    assert!(cancellation_token2.is_cancelled());
}

#[tokio::test]
#[serial]
async fn test_multiple_rapid_file_switches() {
    // Test that rapid file switching properly cancels all previous tasks
    let temp_dir = TempDir::new().unwrap();
    create_large_test_git_repo(&temp_dir).unwrap();

    let repo = git_lineage::git_utils::open_repository(temp_dir.path()).unwrap();
    let mut app = App::new(repo);

    let mut tokens = Vec::new();

    // Simulate rapid file switching
    for _i in 0..10 {
        // Set up a streaming task
        let cancellation_token = CancellationToken::new();
        app.history.streaming_cancellation_token = Some(cancellation_token.clone());
        tokens.push(cancellation_token);
        
        // Immediately switch to next file
        app.history.reset_for_new_file();
        
        // Small delay to make it more realistic
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    // All tokens should be cancelled
    for (i, token) in tokens.iter().enumerate() {
        assert!(token.is_cancelled(), "Token {} should be cancelled", i);
    }
    
    // App state should be clean
    assert!(app.history.streaming_cancellation_token.is_none());
}

#[tokio::test]
#[serial]
async fn test_cancellation_stops_processing_non_matching_commits() {
    // Test that cancellation stops processing even when iterating through commits
    // that don't modify the target file (the main fix)
    let temp_dir = TempDir::new().unwrap();
    
    // Create repo with LOTS of non-matching commits
    let repo_path = temp_dir.path();
    
    // Initialize git repo
    StdCommand::new("git")
        .args(&["init"])
        .current_dir(repo_path)
        .output().unwrap();

    StdCommand::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output().unwrap();

    StdCommand::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output().unwrap();

    // Create target file once
    fs::write(repo_path.join("target.rs"), "fn main() {}").unwrap();
    StdCommand::new("git")
        .args(&["add", "target.rs"])
        .current_dir(repo_path)
        .output().unwrap();
    StdCommand::new("git")
        .args(&["commit", "-m", "Add target file"])
        .current_dir(repo_path)
        .output().unwrap();

    // Create many commits that DON'T modify target.rs
    // These commits will be processed but won't trigger the callback
    for i in 1..=100 {
        fs::write(repo_path.join(format!("unrelated{}.txt", i)), "content").unwrap();
        StdCommand::new("git")
            .args(&["add", &format!("unrelated{}.txt", i)])
            .current_dir(repo_path)
            .output().unwrap();
        StdCommand::new("git")
            .args(&["commit", "-m", &format!("Unrelated commit {}", i)])
            .current_dir(repo_path)
            .output().unwrap();
    }

    let repo = git_lineage::git_utils::open_repository(repo_path).unwrap();
    let cancellation_token = CancellationToken::new();
    
    // Cancel after very short delay
    let cancel_token_clone = cancellation_token.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancel_token_clone.cancel();
    });

    let start_time = Instant::now();
    let mut callback_count = 0;
    
    let result = get_commit_history_streaming(
        &repo,
        "target.rs",
        |_commit, _total| {
            callback_count += 1;
            true
        },
        &cancellation_token,
    );

    let elapsed = start_time.elapsed();
    
    // Should stop quickly due to cancellation, not process all 100+ commits
    assert!(elapsed < Duration::from_millis(500), "Should cancel quickly even with many non-matching commits");
    assert!(result.is_ok());
    
    // Should find the one matching commit or stop before finding it
    assert!(callback_count <= 1, "Should find at most 1 matching commit");
    
    println!("Processed for {:?}, found {} commits", elapsed, callback_count);
}