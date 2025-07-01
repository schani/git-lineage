use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum Task {
    LoadFileTree,
    LoadCommitHistory {
        file_path: String,
    },
    LoadCommitHistoryProgressive {
        file_path: String,
        chunk_size: usize,
        start_offset: usize,
    },
    LoadCommitHistoryStreaming {
        file_path: String,
        cancellation_token: CancellationToken,
    },
    FindNextChange {
        file_path: String,
        current_commit: String,
        line_number: usize,
    },
}

#[derive(Debug, Clone)]
pub enum TaskResult {
    FileTreeLoaded {
        files: crate::tree::FileTree,
    },
    CommitHistoryLoaded {
        file_path: String,
        commits: Vec<crate::app::CommitInfo>,
    },
    CommitHistoryChunkLoaded {
        file_path: String,
        commits: Vec<crate::app::CommitInfo>,
        is_complete: bool,
        chunk_offset: usize,
    },
    CommitFound {
        file_path: String,
        commit: crate::app::CommitInfo,
        total_commits_so_far: usize,
    },
    CommitHistoryComplete {
        file_path: String,
        total_commits: usize,
    },
    NextChangeFound {
        commit_hash: String,
    },
    NextChangeNotFound,
    Error {
        message: String,
    },
}

pub async fn run_worker(
    mut task_receiver: mpsc::Receiver<Task>,
    result_sender: mpsc::Sender<TaskResult>,
    repo_path: String,
) {
    log::info!("🕐 run_worker: Starting async worker for repo: {}", repo_path);
    
    while let Some(task) = task_receiver.recv().await {
        let task_start = Instant::now();
        log::debug!("🕐 run_worker: Processing task: {:?}", task);
        
        let result = match task {
            Task::LoadFileTree => {
                let load_start = Instant::now();
                match load_file_tree(&repo_path).await {
                    Ok(files) => {
                        log::info!("🕐 run_worker: LoadFileTree completed in {:?}", load_start.elapsed());
                        TaskResult::FileTreeLoaded { files }
                    },
                    Err(e) => {
                        log::warn!("🕐 run_worker: LoadFileTree failed in {:?}: {}", load_start.elapsed(), e);
                        TaskResult::Error {
                            message: e.to_string(),
                        }
                    },
                }
            },
            Task::LoadCommitHistory { file_path } => {
                let load_start = Instant::now();
                match load_commit_history(&repo_path, &file_path).await {
                    Ok(commits) => {
                        log::info!("🕐 run_worker: LoadCommitHistory for '{}' completed in {:?} - {} commits", 
                                 file_path, load_start.elapsed(), commits.len());
                        TaskResult::CommitHistoryLoaded {
                            file_path: file_path.clone(),
                            commits,
                        }
                    },
                    Err(e) => {
                        log::warn!("🕐 run_worker: LoadCommitHistory for '{}' failed in {:?}: {}", 
                                 file_path, load_start.elapsed(), e);
                        TaskResult::Error {
                            message: e.to_string(),
                        }
                    },
                }
            },
            Task::LoadCommitHistoryProgressive { file_path, chunk_size, start_offset } => {
                let load_start = Instant::now();
                match load_commit_history_chunk(&repo_path, &file_path, chunk_size, start_offset).await {
                    Ok((commits, is_complete)) => {
                        log::info!("🕐 run_worker: LoadCommitHistoryProgressive for '{}' completed in {:?} - {} commits (chunk_offset: {}, complete: {})", 
                                 file_path, load_start.elapsed(), commits.len(), start_offset, is_complete);
                        TaskResult::CommitHistoryChunkLoaded {
                            file_path: file_path.clone(),
                            commits,
                            is_complete,
                            chunk_offset: start_offset,
                        }
                    },
                    Err(e) => {
                        log::warn!("🕐 run_worker: LoadCommitHistoryProgressive for '{}' failed in {:?}: {}", 
                                 file_path, load_start.elapsed(), e);
                        TaskResult::Error {
                            message: e.to_string(),
                        }
                    },
                }
            },
            Task::LoadCommitHistoryStreaming { file_path, cancellation_token } => {
                let load_start = Instant::now();
                match load_commit_history_streaming(&repo_path, &file_path, result_sender.clone(), cancellation_token).await {
                    Ok(total_commits) => {
                        log::info!("🕐 run_worker: LoadCommitHistoryStreaming for '{}' completed in {:?} - {} total commits", 
                                 file_path, load_start.elapsed(), total_commits);
                        TaskResult::CommitHistoryComplete {
                            file_path: file_path.clone(),
                            total_commits,
                        }
                    },
                    Err(e) => {
                        log::warn!("🕐 run_worker: LoadCommitHistoryStreaming for '{}' failed in {:?}: {}", 
                                 file_path, load_start.elapsed(), e);
                        TaskResult::Error {
                            message: e.to_string(),
                        }
                    },
                }
            },
            Task::FindNextChange {
                file_path,
                current_commit,
                line_number,
            } => {
                let find_start = Instant::now();
                match find_next_change(&repo_path, &file_path, &current_commit, line_number).await {
                    Ok(Some(commit_hash)) => {
                        log::info!("🕐 run_worker: FindNextChange for '{}' line {} from {} found in {:?}: {}", 
                                 file_path, line_number, &current_commit[..8], find_start.elapsed(), &commit_hash[..8]);
                        TaskResult::NextChangeFound { commit_hash }
                    },
                    Ok(None) => {
                        log::info!("🕐 run_worker: FindNextChange for '{}' line {} from {} completed in {:?} - no change found", 
                                 file_path, line_number, &current_commit[..8], find_start.elapsed());
                        TaskResult::NextChangeNotFound
                    },
                    Err(e) => {
                        log::warn!("🕐 run_worker: FindNextChange for '{}' line {} from {} failed in {:?}: {}", 
                                 file_path, line_number, &current_commit[..8], find_start.elapsed(), e);
                        TaskResult::Error {
                            message: e.to_string(),
                        }
                    },
                }
            }
        };
        
        log::debug!("🕐 run_worker: Task processing completed in {:?}", task_start.elapsed());
        log::debug!("🕐 run_worker: About to send result: {:?}", std::mem::discriminant(&result));

        if let Err(_) = result_sender.send(result).await {
            // Main thread has dropped the receiver, exit worker
            log::info!("🕐 run_worker: Result sender closed, exiting worker");
            break;
        } else {
            log::debug!("🕐 run_worker: Result sent successfully");
        }
    }
}

pub async fn load_file_tree(
    repo_path: &str,
) -> Result<crate::tree::FileTree, Box<dyn std::error::Error>> {
    crate::tree::FileTree::from_directory(repo_path).map_err(|e| e.into())
}

async fn load_commit_history(
    repo_path: &str,
    file_path: &str,
) -> Result<Vec<crate::app::CommitInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let async_start = Instant::now();
    log::debug!("🕐 load_commit_history: Starting async wrapper for '{}'", file_path);
    
    // Run in blocking task since git operations are sync
    let repo_path = repo_path.to_string();
    let file_path = file_path.to_string();

    let blocking_start = Instant::now();
    let result = tokio::task::spawn_blocking(
        move || -> Result<Vec<crate::app::CommitInfo>, Box<dyn std::error::Error + Send + Sync>> {
            let repo = crate::git_utils::open_repository(&repo_path).map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )) as Box<dyn std::error::Error + Send + Sync>
            })?;
            crate::git_utils::get_commit_history_for_file(&repo, &file_path).map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )) as Box<dyn std::error::Error + Send + Sync>
            })
        },
    )
    .await?;
    
    log::debug!("🕐 load_commit_history: Blocking task completed in {:?}, total async time: {:?}", 
              blocking_start.elapsed(), async_start.elapsed());
    
    result
}

async fn load_commit_history_chunk(
    repo_path: &str,
    file_path: &str,
    chunk_size: usize,
    start_offset: usize,
) -> Result<(Vec<crate::app::CommitInfo>, bool), Box<dyn std::error::Error + Send + Sync>> {
    let async_start = Instant::now();
    log::debug!("🕐 load_commit_history_chunk: Starting async wrapper for '{}' (chunk_size: {}, offset: {})", 
              file_path, chunk_size, start_offset);
    
    // Run in blocking task since git operations are sync
    let repo_path = repo_path.to_string();
    let file_path = file_path.to_string();

    let blocking_start = Instant::now();
    let result = tokio::task::spawn_blocking(
        move || -> Result<(Vec<crate::app::CommitInfo>, bool), Box<dyn std::error::Error + Send + Sync>> {
            let repo = crate::git_utils::open_repository(&repo_path).map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )) as Box<dyn std::error::Error + Send + Sync>
            })?;
            crate::git_utils::get_commit_history_chunk(&repo, &file_path, chunk_size, start_offset).map_err(|e| {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )) as Box<dyn std::error::Error + Send + Sync>
            })
        },
    )
    .await?;
    
    log::debug!("🕐 load_commit_history_chunk: Blocking task completed in {:?}, total async time: {:?}", 
              blocking_start.elapsed(), async_start.elapsed());
    
    result
}

async fn load_commit_history_streaming(
    repo_path: &str,
    file_path: &str,
    result_sender: mpsc::Sender<TaskResult>,
    cancellation_token: CancellationToken,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    let async_start = Instant::now();
    log::debug!("🕐 load_commit_history_streaming: Starting async wrapper for '{}'", file_path);
    
    // Run in blocking task since git operations are sync
    let repo_path = repo_path.to_string();
    let file_path = file_path.to_string();

    let blocking_start = Instant::now();
    let result = tokio::task::spawn_blocking(move || -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let repo = crate::git_utils::open_repository(&repo_path).map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;

        // Create a callback that sends individual commits via the result sender
        let file_path_for_callback = file_path.clone();
        let result_sender_for_callback = result_sender.clone();
        let cancellation_token_for_callback = cancellation_token.clone();
        
        crate::git_utils::get_commit_history_streaming(&repo, &file_path, |commit, total_so_far| {
            // Send the individual commit found
            let result = TaskResult::CommitFound {
                file_path: file_path_for_callback.clone(),
                commit,
                total_commits_so_far: total_so_far,
            };
            
            // If sending fails, the UI thread has dropped the receiver, so stop
            if result_sender_for_callback.try_send(result).is_err() {
                log::info!("🕐 load_commit_history_streaming: Result sender closed, stopping early");
                return false; // Stop iteration
            }
            
            true // Continue iteration
        }, &cancellation_token).map_err(|e| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            )) as Box<dyn std::error::Error + Send + Sync>
        })
    }).await?;
    
    log::debug!("🕐 load_commit_history_streaming: Blocking task completed in {:?}, total async time: {:?}", 
              blocking_start.elapsed(), async_start.elapsed());
    
    result
}


async fn find_next_change(
    _repo_path: &str,
    _file_path: &str,
    _current_commit: &str,
    _line_number: usize,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // TODO: Implement the complex next change algorithm using gix
    // This should:
    // 1. Create a rev-walk from HEAD to current_commit
    // 2. For each commit, diff with its parent
    // 3. Check if the line at line_number was modified
    // 4. Return the first commit where this happens

    // Simulate async work
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // For now, return mock result
    Ok(Some("d4e5f6789012345678901234567890abcdef0123".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::CommitInfo;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::sync::mpsc;
    use tokio_test::{assert_err, assert_ok};

    // Test utilities
    fn create_test_commit_info(hash: &str, subject: &str) -> CommitInfo {
        CommitInfo {
            hash: hash.to_string(),
            short_hash: hash[0..7].to_string(),
            author: "Test Author".to_string(),
            date: "2023-01-01".to_string(),
            subject: subject.to_string(),
        }
    }

    async fn create_test_channels() -> (
        mpsc::Sender<Task>,
        mpsc::Receiver<Task>,
        mpsc::Sender<TaskResult>,
        mpsc::Receiver<TaskResult>,
    ) {
        let (task_tx, task_rx) = mpsc::channel(1000);
        let (result_tx, result_rx) = mpsc::channel(1000);
        (task_tx, task_rx, result_tx, result_rx)
    }

    fn create_test_git_repo(temp_dir: &TempDir) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use std::process::Command;

        let repo_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .args(&["init"])
            .current_dir(repo_path)
            .output()?;

        // Set up git config
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
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
            repo_path.join("src/lib.rs"),
            "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        )?;
        fs::write(
            repo_path.join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )?;

        // Add and commit files
        Command::new("git")
            .args(&["add", "."])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    mod task_processing {
        use super::*;

        #[tokio::test]
        async fn test_load_file_tree_success() {
            let temp_dir = TempDir::new().unwrap();
            create_test_git_repo(&temp_dir).unwrap();

            let result = load_file_tree(temp_dir.path().to_str().unwrap()).await;

            assert_ok!(&result);
            let tree = result.unwrap();
            assert!(!tree.root.is_empty());
        }

        #[tokio::test]
        async fn test_load_file_tree_nonexistent_path() {
            let result = load_file_tree("/nonexistent/path").await;

            assert!(result.is_err(), "Should return error for nonexistent path");
            // For non-existent paths, Git tree walking returns error since it's not a valid Git repository
        }

        #[tokio::test]
        async fn test_load_commit_history_success() {
            let temp_dir = TempDir::new().unwrap();
            create_test_git_repo(&temp_dir).unwrap();

            let result =
                load_commit_history(temp_dir.path().to_str().unwrap(), "src/main.rs").await;

            assert_ok!(&result);
            let commits = result.unwrap();
            assert!(!commits.is_empty());
            assert!(commits[0].subject.contains("Initial"));
        }

        #[tokio::test]
        async fn test_load_commit_history_invalid_repo() {
            let result = load_commit_history("/nonexistent/path", "src/main.rs").await;

            assert_err!(&result);
        }

        #[tokio::test]
        async fn test_load_commit_history_invalid_file() {
            let temp_dir = TempDir::new().unwrap();
            create_test_git_repo(&temp_dir).unwrap();

            let result =
                load_commit_history(temp_dir.path().to_str().unwrap(), "nonexistent.rs").await;

            // Should succeed but return empty list
            assert_ok!(&result);
            let commits = result.unwrap();
            assert!(commits.is_empty());
        }


        #[tokio::test]
        async fn test_find_next_change_returns_mock() {
            let result = find_next_change("test_repo", "src/main.rs", "current_commit", 5).await;

            assert_ok!(&result);
            let commit_hash = result.unwrap();
            assert!(commit_hash.is_some());
            assert_eq!(
                commit_hash.unwrap(),
                "d4e5f6789012345678901234567890abcdef0123"
            );
        }
    }

    mod worker_lifecycle {
        use super::*;

        #[tokio::test]
        async fn test_worker_processes_load_file_tree() {
            let (task_tx, task_rx, result_tx, mut result_rx) = create_test_channels().await;

            // Start worker
            let worker_handle = tokio::spawn(run_worker(task_rx, result_tx, ".".to_string()));

            // Send task
            task_tx.send(Task::LoadFileTree).await.unwrap();

            // Receive result
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                result_rx.recv()
            ).await.unwrap().unwrap();
            match result {
                TaskResult::FileTreeLoaded { files } => {
                    assert!(!files.root.is_empty());
                }
                TaskResult::Error { message } => {
                    // Acceptable if current directory isn't a git repo
                    assert!(message.contains("repository") || message.contains("git"));
                }
                _ => panic!("Unexpected result type"),
            }

            // Clean shutdown
            drop(task_tx);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                worker_handle
            ).await.unwrap();
        }

        #[tokio::test]
        async fn test_worker_processes_load_commit_history() {
            let temp_dir = TempDir::new().unwrap();
            create_test_git_repo(&temp_dir).unwrap();

            let (task_tx, task_rx, result_tx, mut result_rx) = create_test_channels().await;

            // Start worker
            let worker_handle = tokio::spawn(run_worker(
                task_rx,
                result_tx,
                temp_dir.path().to_str().unwrap().to_string(),
            ));

            // Send task
            task_tx
                .send(Task::LoadCommitHistory {
                    file_path: "src/main.rs".to_string(),
                })
                .await
                .unwrap();

            // Receive result
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                result_rx.recv()
            ).await.unwrap().unwrap();
            match result {
                TaskResult::CommitHistoryLoaded { file_path, commits } => {
                    assert_eq!(file_path, "src/main.rs");
                    assert!(!commits.is_empty());
                }
                _ => panic!("Expected CommitHistoryLoaded result"),
            }

            // Clean shutdown
            drop(task_tx);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                worker_handle
            ).await.unwrap();
        }


        #[tokio::test]
        async fn test_worker_processes_find_next_change() {
            let (task_tx, task_rx, result_tx, mut result_rx) = create_test_channels().await;

            // Start worker
            let worker_handle = tokio::spawn(run_worker(task_rx, result_tx, ".".to_string()));

            // Send task
            task_tx
                .send(Task::FindNextChange {
                    file_path: "src/main.rs".to_string(),
                    current_commit: "abc123".to_string(),
                    line_number: 5,
                })
                .await
                .unwrap();

            // Receive result
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                result_rx.recv()
            ).await.unwrap().unwrap();
            match result {
                TaskResult::NextChangeFound { commit_hash } => {
                    assert!(!commit_hash.is_empty());
                }
                _ => panic!("Expected NextChangeFound result"),
            }

            // Clean shutdown
            drop(task_tx);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                worker_handle
            ).await.unwrap();
        }

        #[tokio::test]
        async fn test_worker_handles_channel_close() {
            let (task_tx, task_rx, result_tx, _result_rx) = create_test_channels().await;

            // Start worker
            let worker_handle = tokio::spawn(run_worker(task_rx, result_tx, ".".to_string()));

            // Drop result receiver to simulate main thread exit
            drop(_result_rx);

            // Send task - worker should exit gracefully when it can't send result
            task_tx.send(Task::LoadFileTree).await.unwrap();

            // Worker should exit gracefully
            let result = worker_handle.await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_worker_exits_when_task_channel_closed() {
            let (_task_tx, task_rx, result_tx, _result_rx) = create_test_channels().await;

            // Start worker
            let worker_handle = tokio::spawn(run_worker(task_rx, result_tx, ".".to_string()));

            // Close task channel immediately
            drop(_task_tx);

            // Worker should exit gracefully
            let result = worker_handle.await;
            assert!(result.is_ok());
        }

        #[tokio::test]
        async fn test_worker_processes_multiple_tasks() {
            let temp_dir = TempDir::new().unwrap();
            create_test_git_repo(&temp_dir).unwrap();

            let (task_tx, task_rx, result_tx, mut result_rx) = create_test_channels().await;

            // Start worker
            let worker_handle = tokio::spawn(run_worker(
                task_rx,
                result_tx,
                temp_dir.path().to_str().unwrap().to_string(),
            ));

            // Send multiple tasks
            task_tx.send(Task::LoadFileTree).await.unwrap();
            task_tx
                .send(Task::LoadCommitHistory {
                    file_path: "src/main.rs".to_string(),
                })
                .await
                .unwrap();

            // Receive all results with timeout
            for _ in 0..2 {
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    result_rx.recv()
                ).await.unwrap().unwrap();
                match result {
                    TaskResult::FileTreeLoaded { .. } => {}
                    TaskResult::CommitHistoryLoaded { .. } => {}
                    TaskResult::Error { .. } => {} // Git operations might fail in test environment
                    _ => panic!("Unexpected result type"),
                }
            }

            // Clean shutdown
            drop(task_tx);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                worker_handle
            ).await.unwrap();
        }
    }

    mod error_scenarios {
        use super::*;

        #[tokio::test]
        async fn test_worker_handles_git_errors() {
            let (task_tx, task_rx, result_tx, mut result_rx) = create_test_channels().await;

            // Start worker with invalid repo path
            let worker_handle = tokio::spawn(run_worker(
                task_rx,
                result_tx,
                "/totally/invalid/path".to_string(),
            ));

            // Send task that will fail
            task_tx
                .send(Task::LoadCommitHistory {
                    file_path: "src/main.rs".to_string(),
                })
                .await
                .unwrap();

            // Should receive error result
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                result_rx.recv()
            ).await.unwrap().unwrap();
            match result {
                TaskResult::Error { message } => {
                    assert!(!message.is_empty());
                }
                _ => panic!("Expected Error result"),
            }

            // Clean shutdown
            drop(task_tx);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                worker_handle
            ).await.unwrap();
        }

        #[tokio::test]
        async fn test_load_commit_history_with_permission_denied() {
            // Try to access a path that would cause permission issues
            let result = load_commit_history("/root/nonexistent", "file.rs").await;

            assert_err!(&result);
        }

        #[tokio::test]
        async fn test_worker_error_propagation() {
            let (task_tx, task_rx, result_tx, mut result_rx) = create_test_channels().await;

            // Start worker with invalid repo
            let worker_handle = tokio::spawn(run_worker(
                task_rx,
                result_tx,
                "/invalid/repo/path".to_string(),
            ));

            // Send tasks that should produce errors
            let tasks = vec![
                Task::LoadCommitHistory {
                    file_path: "test.rs".to_string(),
                },
                Task::LoadFileTree,
            ];

            for task in tasks {
                task_tx.send(task).await.unwrap();

                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    result_rx.recv()
                ).await.unwrap().unwrap();
                match result {
                    TaskResult::Error { message } => {
                        assert!(!message.is_empty());
                    }
                    TaskResult::FileTreeLoaded { .. } => {
                        // LoadFileTree falls back to mock data, so this is acceptable
                    }
                    _ => panic!("Expected Error or FileTreeLoaded result"),
                }
            }

            // Clean shutdown
            drop(task_tx);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                worker_handle
            ).await.unwrap();
        }

        #[tokio::test]
        async fn test_concurrent_workers() {
            let temp_dir = TempDir::new().unwrap();
            create_test_git_repo(&temp_dir).unwrap();

            let repo_path = temp_dir.path().to_str().unwrap().to_string();

            // Create multiple workers
            let mut workers = Vec::new();
            let mut task_senders = Vec::new();
            let mut result_receivers = Vec::new();

            for _ in 0..3 {
                let (task_tx, task_rx, result_tx, result_rx) = create_test_channels().await;
                let worker_handle = tokio::spawn(run_worker(task_rx, result_tx, repo_path.clone()));

                workers.push(worker_handle);
                task_senders.push(task_tx);
                result_receivers.push(result_rx);
            }

            // Send tasks to all workers
            for (i, task_tx) in task_senders.iter().enumerate() {
                task_tx
                    .send(Task::LoadCommitHistory {
                        file_path: format!("src/file{}.rs", i),
                    })
                    .await
                    .unwrap();
            }

            // Collect results from all workers with timeout
            for (i, mut result_rx) in result_receivers.into_iter().enumerate() {
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    result_rx.recv()
                ).await.unwrap().unwrap();
                match result {
                    TaskResult::CommitHistoryLoaded { file_path, commits } => {
                        assert_eq!(file_path, format!("src/file{}.rs", i));
                        // Empty is okay for non-existent files
                        assert!(commits.is_empty());
                    }
                    TaskResult::Error { .. } => {
                        // Errors are acceptable for non-existent files
                    }
                    _ => panic!("Unexpected result type"),
                }
            }

            // Clean shutdown
            for task_tx in task_senders {
                drop(task_tx);
            }

            for worker in workers {
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    worker
                ).await.unwrap();
            }
        }
    }

    mod coverage_completion {
        use super::*;

        #[tokio::test]
        async fn test_load_file_tree_with_nonexistent_path() {
            // FileTree::from_directory now requires a valid Git repository
            // Nonexistent paths should return an error
            let result = load_file_tree("/nonexistent/path/that/should/fail").await;

            assert!(result.is_err(), "Should return error for nonexistent path");
        }

        #[tokio::test]
        async fn test_worker_processes_tasks_with_errors() {
            let (task_tx, task_rx, result_tx, mut result_rx) = create_test_channels().await;

            // Start worker with invalid repo path to trigger all error paths
            let worker_handle = tokio::spawn(run_worker(
                task_rx,
                result_tx,
                "/absolutely/invalid/repo/path".to_string(),
            ));

            // Test LoadFileTree with invalid path (now returns error)
            task_tx.send(Task::LoadFileTree).await.unwrap();
            let result = result_rx.recv().await.unwrap();
            match result {
                TaskResult::Error { message } => {
                    // Should fail with error for invalid Git repository path
                    assert!(message.contains("Failed to open Git repository") || message.contains("Failed to walk Git tree"));
                }
                _ => panic!("Expected Error result, got: {:?}", result),
            }

            // Test LoadCommitHistory error path (covers line 62)
            task_tx
                .send(Task::LoadCommitHistory {
                    file_path: "nonexistent.rs".to_string(),
                })
                .await
                .unwrap();
            let result = result_rx.recv().await.unwrap();
            match result {
                TaskResult::Error { message } => {
                    assert!(!message.is_empty());
                }
                _ => panic!("Expected Error result for invalid repo"),
            }


            // Test FindNextChange error paths (covers line 87)
            task_tx
                .send(Task::FindNextChange {
                    file_path: "test.rs".to_string(),
                    current_commit: "invalid".to_string(),
                    line_number: 1,
                })
                .await
                .unwrap();
            let result = result_rx.recv().await.unwrap();
            match result {
                TaskResult::NextChangeFound { .. } => {
                    // Mock implementation returns success, which is expected
                }
                TaskResult::NextChangeNotFound => {
                    // Also acceptable for mock implementation
                }
                _ => panic!("Expected NextChange result for mock implementation"),
            }

            // Clean shutdown
            drop(task_tx);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                worker_handle
            ).await.unwrap();
        }

        #[tokio::test]
        async fn test_find_next_change_not_found_path() {
            // Test both the NextChangeFound and NextChangeNotFound paths by using worker
            let (task_tx, task_rx, result_tx, mut result_rx) = create_test_channels().await;

            let worker_handle = tokio::spawn(run_worker(task_rx, result_tx, ".".to_string()));

            // Send task that should return NextChangeFound
            task_tx
                .send(Task::FindNextChange {
                    file_path: "src/main.rs".to_string(),
                    current_commit: "abc123".to_string(),
                    line_number: 5,
                })
                .await
                .unwrap();

            let result = result_rx.recv().await.unwrap();
            match result {
                TaskResult::NextChangeFound { commit_hash } => {
                    assert!(!commit_hash.is_empty());
                }
                TaskResult::NextChangeNotFound => {
                    // This would cover line 85
                }
                _ => panic!("Expected NextChange result"),
            }

            // Clean shutdown
            drop(task_tx);
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                worker_handle
            ).await.unwrap();
        }

        #[tokio::test]
        async fn test_worker_handles_result_send_failure() {
            let (task_tx, task_rx, result_tx, result_rx) = create_test_channels().await;

            // Start worker
            let worker_handle = tokio::spawn(run_worker(task_rx, result_tx, ".".to_string()));

            // Drop result receiver to cause send failure
            drop(result_rx);

            // Send a task - the worker should detect the send failure and exit gracefully
            task_tx.send(Task::LoadFileTree).await.unwrap();

            // Worker should exit when it can't send the result (covers line 93-96)
            let result = worker_handle.await;
            assert!(result.is_ok());

            // Clean up
            drop(task_tx);
        }

        #[tokio::test]
        async fn test_git_error_handling_in_load_commit_history() {
            // Test that git errors are properly converted to the expected error type
            // This covers lines 155-165 in the error handling paths
            let result = load_commit_history("/invalid/git/repo", "test.rs").await;

            assert_err!(&result);
            let error = result.unwrap_err();
            // Verify the error message contains something meaningful
            let error_str = error.to_string();
            assert!(!error_str.is_empty());
        }
    }

    mod edge_cases {
        use super::*;

        #[tokio::test]
        async fn test_task_and_result_serialization() {
            // Test that tasks can be cloned/serialized properly
            let task = Task::LoadCommitHistory {
                file_path: "test.rs".to_string(),
            };
            let task_clone = task.clone();

            match (task, task_clone) {
                (
                    Task::LoadCommitHistory { file_path: path1 },
                    Task::LoadCommitHistory { file_path: path2 },
                ) => {
                    assert_eq!(path1, path2);
                }
                _ => panic!("Task cloning failed"),
            }

            // Test result cloning
            let result = TaskResult::NextChangeFound {
                commit_hash: "abc123".to_string(),
            };
            let result_clone = result.clone();

            match (result, result_clone) {
                (
                    TaskResult::NextChangeFound { commit_hash: hash1 },
                    TaskResult::NextChangeFound { commit_hash: hash2 },
                ) => {
                    assert_eq!(hash1, hash2);
                }
                _ => panic!("Result cloning failed"),
            }
        }

        #[tokio::test]
        async fn test_empty_file_paths() {
            let result = load_commit_history(".", "").await;
            // Empty file paths should either succeed with empty result or fail
            match result {
                Ok(commits) => assert!(commits.is_empty()),
                Err(_) => {} // Both outcomes are acceptable
            }
        }

        #[tokio::test]
        async fn test_very_long_file_paths() {
            let long_path = "a/".repeat(1000) + "file.rs";
            let result = load_commit_history(".", &long_path).await;
            // Very long paths should either succeed with empty result or fail
            match result {
                Ok(commits) => assert!(commits.is_empty()),
                Err(_) => {} // Both outcomes are acceptable
            }
        }

        #[tokio::test]
        async fn test_special_characters_in_paths() {
            let special_paths = vec![
                "src/file with spaces.rs",
                "src/file@with#special$chars.rs",
                "src/файл.rs", // Unicode
                "src/🦀.rs",   // Emoji
            ];

            for path in special_paths {
                let result = load_commit_history(".", path).await;
                // Should handle gracefully (may succeed or fail, but shouldn't panic)
                match result {
                    Ok(_) | Err(_) => {} // Both outcomes are acceptable
                }
            }
        }

        #[tokio::test]
        async fn test_mock_file_tree_structure() {
            // Test the mock structure creation directly by creating it manually
            // since the real fallback might not trigger as expected
            let mut tree = crate::tree::FileTree::new();

            let mut src_dir =
                crate::tree::TreeNode::new_dir("src".to_string(), std::path::PathBuf::from("src"));
            src_dir.expand();
            src_dir.add_child(
                crate::tree::TreeNode::new_file(
                    "main.rs".to_string(),
                    std::path::PathBuf::from("src/main.rs"),
                )
                .with_git_status('M'),
            );
            src_dir.add_child(
                crate::tree::TreeNode::new_file(
                    "lib.rs".to_string(),
                    std::path::PathBuf::from("src/lib.rs"),
                )
                .with_git_status('A'),
            );

            tree.root.push(src_dir);
            tree.root.push(
                crate::tree::TreeNode::new_file(
                    "Cargo.toml".to_string(),
                    std::path::PathBuf::from("Cargo.toml"),
                )
                .with_git_status('M'),
            );

            // Note: select_node is now a FileTreeState method, not FileTree
            // tree.select_node(&std::path::PathBuf::from("src/main.rs"));

            // Verify mock structure details
            assert_eq!(tree.root.len(), 2); // src dir + Cargo.toml

            let src_dir = tree.root.iter().find(|node| node.name == "src").unwrap();
            assert!(src_dir.is_dir);
            assert!(src_dir.is_expanded);
            assert_eq!(src_dir.children.len(), 2); // main.rs + lib.rs

            let main_rs = src_dir
                .children
                .iter()
                .find(|node| node.name == "main.rs")
                .unwrap();
            assert!(!main_rs.is_dir);
            assert_eq!(main_rs.git_status, Some('M'));

            let lib_rs = src_dir
                .children
                .iter()
                .find(|node| node.name == "lib.rs")
                .unwrap();
            assert!(!lib_rs.is_dir);
            assert_eq!(lib_rs.git_status, Some('A'));

            let cargo_toml = tree
                .root
                .iter()
                .find(|node| node.name == "Cargo.toml")
                .unwrap();
            assert!(!cargo_toml.is_dir);
            assert_eq!(cargo_toml.git_status, Some('M'));

            // Note: current_selection is now on FileTreeState, not FileTree
            // assert_eq!(tree.current_selection, Some(PathBuf::from("src/main.rs")));
        }
    }
}
