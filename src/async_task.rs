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
    FileTreeLoaded { files: Vec<crate::app::FileTreeNode> },
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

async fn load_file_tree(_repo_path: &str) -> Result<Vec<crate::app::FileTreeNode>, Box<dyn std::error::Error>> {
    // TODO: Implement using gix to get file tree with Git status
    // For now, return mock data
    Ok(vec![
        crate::app::FileTreeNode {
            name: "src".to_string(),
            path: "src".to_string(),
            is_dir: true,
            git_status: None,
            children: vec![
                crate::app::FileTreeNode {
                    name: "main.rs".to_string(),
                    path: "src/main.rs".to_string(),
                    is_dir: false,
                    git_status: Some('M'),
                    children: vec![],
                },
                crate::app::FileTreeNode {
                    name: "lib.rs".to_string(),
                    path: "src/lib.rs".to_string(),
                    is_dir: false,
                    git_status: Some('A'),
                    children: vec![],
                },
            ],
        },
        crate::app::FileTreeNode {
            name: "Cargo.toml".to_string(),
            path: "Cargo.toml".to_string(),
            is_dir: false,
            git_status: Some('M'),
            children: vec![],
        },
    ])
}

async fn load_commit_history(
    _repo_path: &str,
    _file_path: &str,
) -> Result<Vec<crate::app::CommitInfo>, Box<dyn std::error::Error>> {
    // TODO: Implement using gix rev-walk filtered by file path
    // For now, return mock data
    Ok(vec![
        crate::app::CommitInfo {
            hash: "a1b2c3d4e5f6789012345678901234567890abcd".to_string(),
            short_hash: "a1b2c3d".to_string(),
            author: "John Doe".to_string(),
            date: "2 hours ago".to_string(),
            subject: "Add new feature".to_string(),
        },
        crate::app::CommitInfo {
            hash: "b2c3d4e5f6789012345678901234567890abcdef".to_string(),
            short_hash: "b2c3d4e".to_string(),
            author: "Jane Smith".to_string(),
            date: "1 day ago".to_string(),
            subject: "Fix bug in parser".to_string(),
        },
        crate::app::CommitInfo {
            hash: "c3d4e5f6789012345678901234567890abcdef01".to_string(),
            short_hash: "c3d4e5f".to_string(),
            author: "Bob Johnson".to_string(),
            date: "3 days ago".to_string(),
            subject: "Initial commit".to_string(),
        },
    ])
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