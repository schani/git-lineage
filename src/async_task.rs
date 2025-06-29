use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Task {
    LoadFileTree,
    LoadCommitHistory { file_path: String },
    LoadFileContent { file_path: String, commit_hash: String },
    FindNextChange {
        file_path: String,
        current_commit: String,
        line_number: usize,
    },
}

#[derive(Debug, Clone)]
pub enum TaskResult {
    FileTreeLoaded { files: crate::tree::FileTree },
    CommitHistoryLoaded { commits: Vec<crate::app::CommitInfo> },
    FileContentLoaded { content: Vec<String>, blame_info: Option<String> },
    NextChangeFound { commit_hash: String },
    NextChangeNotFound,
    Error { message: String },
}

pub async fn run_worker(
    mut task_receiver: mpsc::Receiver<Task>,
    result_sender: mpsc::Sender<TaskResult>,
    repo_path: String,
) {
    while let Some(task) = task_receiver.recv().await {
        let result = match task {
            Task::LoadFileTree => {
                match load_file_tree(&repo_path).await {
                    Ok(files) => TaskResult::FileTreeLoaded { files },
                    Err(e) => TaskResult::Error { message: e.to_string() },
                }
            }
            Task::LoadCommitHistory { file_path } => {
                match load_commit_history(&repo_path, &file_path).await {
                    Ok(commits) => TaskResult::CommitHistoryLoaded { commits },
                    Err(e) => TaskResult::Error { message: e.to_string() },
                }
            }
            Task::LoadFileContent { file_path, commit_hash } => {
                match load_file_content(&repo_path, &file_path, &commit_hash).await {
                    Ok((content, blame_info)) => TaskResult::FileContentLoaded { content, blame_info },
                    Err(e) => TaskResult::Error { message: e.to_string() },
                }
            }
            Task::FindNextChange { file_path, current_commit, line_number } => {
                match find_next_change(&repo_path, &file_path, &current_commit, line_number).await {
                    Ok(Some(commit_hash)) => TaskResult::NextChangeFound { commit_hash },
                    Ok(None) => TaskResult::NextChangeNotFound,
                    Err(e) => TaskResult::Error { message: e.to_string() },
                }
            }
        };

        if let Err(_) = result_sender.send(result).await {
            // Main thread has dropped the receiver, exit worker
            break;
        }
    }
}

pub async fn load_file_tree(repo_path: &str) -> Result<crate::tree::FileTree, Box<dyn std::error::Error>> {
    // Try to load from the actual directory, fallback to mock data
    match crate::tree::FileTree::from_directory(repo_path) {
        Ok(tree) => Ok(tree),
        Err(_) => {
            // Create mock data using the new FileTree structure
            let mut tree = crate::tree::FileTree::new();
            
            let mut src_dir = crate::tree::TreeNode::new_dir("src".to_string(), std::path::PathBuf::from("src"));
            src_dir.expand();
            src_dir.add_child(crate::tree::TreeNode::new_file("main.rs".to_string(), std::path::PathBuf::from("src/main.rs"))
                .with_git_status('M'));
            src_dir.add_child(crate::tree::TreeNode::new_file("lib.rs".to_string(), std::path::PathBuf::from("src/lib.rs"))
                .with_git_status('A'));
            
            tree.root.push(src_dir);
            tree.root.push(crate::tree::TreeNode::new_file("Cargo.toml".to_string(), std::path::PathBuf::from("Cargo.toml"))
                .with_git_status('M'));
            
            // Select first file by default
            tree.select_node(&std::path::PathBuf::from("src/main.rs"));
            
            Ok(tree)
        }
    }
}

async fn load_commit_history(
    repo_path: &str,
    file_path: &str,
) -> Result<Vec<crate::app::CommitInfo>, Box<dyn std::error::Error + Send + Sync>> {
    // Run in blocking task since git operations are sync
    let repo_path = repo_path.to_string();
    let file_path = file_path.to_string();
    
    tokio::task::spawn_blocking(move || -> Result<Vec<crate::app::CommitInfo>, Box<dyn std::error::Error + Send + Sync>> {
        let repo = crate::git_utils::open_repository(&repo_path)
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)?;
        crate::git_utils::get_commit_history_for_file(&repo, &file_path)
            .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error + Send + Sync>)
    }).await?
}

async fn load_file_content(
    _repo_path: &str,
    _file_path: &str,
    _commit_hash: &str,
) -> Result<(Vec<String>, Option<String>), Box<dyn std::error::Error>> {
    // TODO: Implement using gix to get file content and blame at specific commit
    // For now, return mock data
    let content = vec![
        "use std::io;".to_string(),
        "".to_string(),
        "fn main() {".to_string(),
        "    println!(\"Hello, world!\");".to_string(),
        "    let mut input = String::new();".to_string(),
        "    io::stdin().read_line(&mut input).expect(\"Failed to read line\");".to_string(),
        "    println!(\"You entered: {}\", input.trim());".to_string(),
        "}".to_string(),
    ];
    
    let blame_info = Some("Mock blame info".to_string());
    
    Ok((content, blame_info))
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