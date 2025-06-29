use gix::Repository;
use std::path::Path;

use crate::app::{FileTreeNode, CommitInfo};

pub fn open_repository<P: AsRef<Path>>(path: P) -> Result<Repository, Box<dyn std::error::Error + Send + Sync>> {
    Ok(gix::discover(path)?)
}

pub fn get_file_tree_with_status(_repo: &Repository) -> Result<Vec<FileTreeNode>, Box<dyn std::error::Error>> {
    // TODO: Implement using gix::Repository::worktree() and status()
    // This should:
    // 1. Get all tracked files from the worktree
    // 2. Get status information for each file
    // 3. Build a tree structure respecting .gitignore
    
    // For now, return empty vec as placeholder
    Ok(vec![])
}

pub fn get_commit_history_for_file(
    repo: &Repository,
    file_path: &str,
) -> Result<Vec<CommitInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let mut commits = Vec::new();
    
    // For now, let's use a simpler approach with git command line until we get the gix API right
    // This will be replaced with proper gix implementation later
    let output = std::process::Command::new("git")
        .arg("log")
        .arg("--pretty=format:%H|%h|%an|%ad|%s")
        .arg("--date=short")
        .arg("--")
        .arg(file_path)
        .current_dir(repo.work_dir().unwrap_or(std::path::Path::new(".")))
        .output()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    
    for line in output_str.lines() {
        if line.trim().is_empty() {
            continue;
        }
        
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 5 {
            commits.push(CommitInfo {
                hash: parts[0].to_string(),
                short_hash: parts[1].to_string(),
                author: parts[2].to_string(),
                date: parts[3].to_string(),
                subject: parts[4..].join("|"), // In case subject contains |
            });
        }
    }
    
    Ok(commits)
}

pub fn get_blame_at_commit(
    repo: &Repository,
    file_path: &str,
    commit_hash: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // TODO: Implement using gix::Repository::blame()
    // For historical states, use blame.at_commit(<selected_commit_id>)
    
    // For now, return placeholder
    let _ = (repo, file_path, commit_hash);
    Ok("Mock blame data".to_string())
}

pub fn get_file_content_at_commit(
    repo: &Repository,
    file_path: &str,
    commit_hash: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // TODO: Implement by:
    // 1. Getting the commit object by hash
    // 2. Getting the tree from the commit
    // 3. Finding the blob for the file path
    // 4. Reading the blob content and splitting into lines
    
    // For now, return placeholder content
    let _ = (repo, file_path, commit_hash);
    Ok(vec![
        "// Placeholder content".to_string(),
        "// TODO: Load from Git".to_string(),
    ])
}

pub fn find_next_change_for_line(
    repo: &Repository,
    file_path: &str,
    current_commit: &str,
    line_number: usize,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    // TODO: Implement the complex next change algorithm
    // This is the most complex operation and should:
    // 1. Create a rev-walk from HEAD to current_commit
    // 2. Iterate through commits chronologically forward from current_commit
    // 3. For each commit, diff with its parent
    // 4. Use the `similar` crate to analyze if the target line was modified
    // 5. Return the first commit where the line was changed
    
    // For now, return None as placeholder
    let _ = (repo, file_path, current_commit, line_number);
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_open_repository() {
        // Test opening the current repository
        let repo = open_repository(".");
        assert!(repo.is_ok(), "Should be able to open current repository");
    }

    #[test]
    fn test_get_commit_history_for_file() {
        // Test getting commit history for a file that exists in this repo
        let repo = open_repository(".").expect("Should be able to open repository");
        let commits = get_commit_history_for_file(&repo, "src/main.rs").expect("Should get commit history");
        
        // We should have at least one commit for main.rs
        assert!(!commits.is_empty(), "Should have commits for main.rs");
        
        // Check that commits have all required fields
        for commit in &commits {
            assert!(!commit.hash.is_empty(), "Commit hash should not be empty");
            assert!(!commit.short_hash.is_empty(), "Short hash should not be empty");
            assert!(!commit.author.is_empty(), "Author should not be empty");
            assert!(!commit.date.is_empty(), "Date should not be empty");
            // Subject can be empty for some commits
        }
        
        println!("Found {} commits for src/main.rs", commits.len());
        for commit in commits.iter().take(3) {
            println!("  {} - {} by {} on {}", 
                commit.short_hash, commit.subject, commit.author, commit.date);
        }
    }
}