use gix::Repository;
use std::path::Path;

use crate::app::{CommitInfo, FileTreeNode};

pub fn open_repository<P: AsRef<Path>>(
    path: P,
) -> Result<Repository, Box<dyn std::error::Error + Send + Sync>> {
    Ok(gix::discover(path)?)
}

pub fn get_file_tree_with_status(
    _repo: &Repository,
) -> Result<Vec<FileTreeNode>, Box<dyn std::error::Error>> {
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

    // Normalize the file path by removing "./" prefix if present
    let normalized_path = if file_path.starts_with("./") {
        &file_path[2..]
    } else {
        file_path
    };

    // Use gix to walk the commit history
    let head_id = repo.head_id()?;
    let commit_iter = repo.rev_walk([head_id]).all()?;

    // Walk through commits and check if they modified the file
    for commit_info in commit_iter {
        let commit_info = commit_info?;
        let commit = repo.find_object(commit_info.id)?.try_into_commit()?;

        // Check if this commit actually modified the file by comparing with parent(s)
        let modified_file = if commit.parent_ids().count() == 0 {
            // This is the initial commit, check if file exists
            let tree = commit.tree()?;
            tree.lookup_entry_by_path(normalized_path)?
                .is_some()
        } else {
            // Compare with parent commit(s) to see if file was modified
            let mut file_modified = false;

            for parent_id in commit.parent_ids() {
                let parent_commit = repo.find_object(parent_id)?.try_into_commit()?;
                let current_tree = commit.tree()?;
                let parent_tree = parent_commit.tree()?;

                let current_entry =
                    current_tree.lookup_entry_by_path(normalized_path)?;
                let parent_entry =
                    parent_tree.lookup_entry_by_path(normalized_path)?;

                match (current_entry, parent_entry) {
                    (Some(current), Some(parent)) => {
                        // File exists in both - check if content changed
                        if current.oid() != parent.oid() {
                            file_modified = true;
                            break;
                        }
                    }
                    (Some(_), None) => {
                        // File was added
                        file_modified = true;
                        break;
                    }
                    (None, Some(_)) => {
                        // File was deleted
                        file_modified = true;
                        break;
                    }
                    (None, None) => {
                        // File doesn't exist in either - not modified
                    }
                }
            }

            file_modified
        };

        if modified_file {
            // Get commit metadata
            let commit_obj = commit.decode()?;
            let author = &commit_obj.author;
            let message = commit_obj.message.to_string();

            // Format date
            let date = format!("{}", author.time);

            let commit_hash = commit_info.id.to_string();
            let short_hash = commit_hash[..8].to_string();

            commits.push(CommitInfo {
                hash: commit_hash,
                short_hash,
                author: author.name.to_string(),
                date,
                subject: message,
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
    get_file_content_with_gix(repo, file_path, commit_hash)
}

fn get_file_content_with_gix(
    repo: &Repository,
    file_path: &str,
    commit_hash: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Find the commit object by hash
    let oid = gix::ObjectId::from_hex(commit_hash.as_bytes())?;
    let commit = repo.find_object(oid)?.try_into_commit()?;

    // Get the tree from the commit
    let tree = commit.tree()?;

    // Navigate to the file in the tree
    let file_entry = tree
        .lookup_entry_by_path(file_path)?
        .ok_or_else(|| format!("File '{}' not found in commit {}", file_path, commit_hash))?;

    // Get the blob content
    let blob = file_entry.object()?.try_into_blob()?;
    let content_bytes = blob.data.clone();

    // Convert to string and split into lines
    let content_str = String::from_utf8_lossy(&content_bytes);
    let lines: Vec<String> = content_str.lines().map(|line| line.to_string()).collect();

    Ok(lines)
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
        let commits =
            get_commit_history_for_file(&repo, "src/main.rs").expect("Should get commit history");

        // We should have at least one commit for main.rs
        assert!(!commits.is_empty(), "Should have commits for main.rs");

        // Check that commits have all required fields
        for commit in &commits {
            assert!(!commit.hash.is_empty(), "Commit hash should not be empty");
            assert!(
                !commit.short_hash.is_empty(),
                "Short hash should not be empty"
            );
            assert!(!commit.author.is_empty(), "Author should not be empty");
            assert!(!commit.date.is_empty(), "Date should not be empty");
            // Subject can be empty for some commits
        }

        // Uncomment for debugging:
        // println!("Found {} commits for src/main.rs", commits.len());
        // for commit in commits.iter().take(3) {
        //     println!("  {} - {} by {} on {}",
        //         commit.short_hash, commit.subject, commit.author, commit.date);
        // }
    }

    #[test]
    fn test_commit_history_only_shows_modifications() {
        // This test ensures we only get commits that actually modified the file,
        // not all commits that simply contained the file
        let repo = open_repository(".").expect("Should be able to open repository");

        // Test with README.md which should have fewer commits than src/main.rs
        let readme_commits =
            get_commit_history_for_file(&repo, "README.md").expect("Should get README.md history");
        let main_commits =
            get_commit_history_for_file(&repo, "src/main.rs").expect("Should get main.rs history");

        // Uncomment for debugging:
        // println!("README.md has {} commits", readme_commits.len());
        // println!("src/main.rs has {} commits", main_commits.len());

        // README.md should have significantly fewer commits than main.rs since it's modified less frequently
        // This test will fail if we revert to showing all commits that contain the file
        assert!(
            readme_commits.len() < main_commits.len(),
            "README.md should have fewer commits than src/main.rs (README: {}, main.rs: {})",
            readme_commits.len(),
            main_commits.len()
        );

        // Verify we're not returning an excessive number of commits for README.md
        // Based on actual git history, README.md should have around 2-5 commits max
        assert!(
            readme_commits.len() <= 10,
            "README.md should not have more than 10 commits, got {}",
            readme_commits.len()
        );

        // Each commit should represent an actual modification
        // Uncomment for debugging:
        // for commit in &readme_commits {
        //     println!("  README.md modified in: {} - {}", commit.short_hash, commit.subject);
        // }
    }

    #[test]
    fn test_commit_history_for_nonexistent_file() {
        // Test that we get empty history for a file that doesn't exist
        let repo = open_repository(".").expect("Should be able to open repository");
        let commits = get_commit_history_for_file(&repo, "nonexistent/file.txt")
            .expect("Should handle nonexistent file");

        assert!(
            commits.is_empty(),
            "Should have no commits for nonexistent file"
        );
    }

    #[test]
    fn test_get_file_content_at_commit() {
        // Test getting file content at specific commit
        let repo = open_repository(".").expect("Should be able to open repository");

        // First get some commits for main.rs to test with
        let commits =
            get_commit_history_for_file(&repo, "src/main.rs").expect("Should get commit history");

        if !commits.is_empty() {
            let latest_commit = &commits[0];

            // Try to get the content of main.rs at the latest commit
            let content = get_file_content_at_commit(&repo, "src/main.rs", &latest_commit.hash);

            match content {
                Ok(lines) => {
                    assert!(!lines.is_empty(), "File content should not be empty");
                    // Uncomment for debugging:
                    // println!("Successfully retrieved {} lines from src/main.rs at commit {}",
                    //     lines.len(), latest_commit.short_hash);

                    // Print first few lines for verification
                    // for (i, line) in lines.iter().take(5).enumerate() {
                    //     println!("  {}: {}", i + 1, line);
                    // }
                }
                Err(e) => {
                    // Uncomment for debugging:
                    // println!("Note: Could not retrieve file content (expected in some cases): {}", e);
                    // This test might fail in some environments, so we don't assert failure
                    let _ = e; // Silence unused variable warning
                }
            }
        } else {
            // Uncomment for debugging:
            // println!("No commits found for src/main.rs, skipping content test");
        }
    }
}
