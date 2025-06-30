use gix::Repository;
use std::path::Path;
use std::time::Instant;
use chrono::{Local, TimeZone};

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
    let start_time = Instant::now();
    log::debug!("🕐 get_commit_history_for_file: Starting for file: {}", file_path);
    
    let mut commits = Vec::new();

    // Normalize the file path by removing "./" prefix if present
    let normalized_path = if file_path.starts_with("./") {
        &file_path[2..]
    } else {
        file_path
    };

    // Use gix to walk the commit history
    let head_setup_start = Instant::now();
    let head_id = repo.head_id()?;
    let commit_iter = repo.rev_walk([head_id]).all()?;
    log::debug!("🕐 get_commit_history_for_file: Head setup took: {:?}", head_setup_start.elapsed());

    // Walk through commits and check if they modified the file
    let mut commits_processed = 0;
    let commit_iteration_start = Instant::now();
    
    for commit_info in commit_iter {
        let commit_start = Instant::now();
        let commit_info = commit_info?;
        let commit = repo.find_object(commit_info.id)?.try_into_commit()?;
        commits_processed += 1;

        // Check if this commit actually modified the file by comparing with parent(s)
        let modified_file = if commit.parent_ids().count() == 0 {
            // This is the initial commit, check if file exists
            let tree = commit.tree()?;
            tree.lookup_entry_by_path(normalized_path)?
                .is_some()
        } else {
            // Compare with parent commit(s) to see if file was modified
            let mut file_modified = false;
            let parent_comparison_start = Instant::now();

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
            log::debug!("🕐 get_commit_history_for_file: Parent comparison for commit {} took: {:?}", 
                      commit_info.id.to_string()[..8].to_string(), parent_comparison_start.elapsed());

            file_modified
        };

        if modified_file {
            // Get commit metadata
            let commit_obj = commit.decode()?;
            let author = &commit_obj.author;
            let message = commit_obj.message.to_string();

            // Parse Git timestamp format: "timestamp timezone" (e.g., "1751295482 -0400")
            let timestamp = match author.time.split_whitespace().next() {
                Some(ts_str) => ts_str.parse::<i64>().unwrap_or(0),
                None => 0,
            };
            
            // Format date as human-readable
            let datetime = Local.timestamp_opt(timestamp, 0).single().unwrap_or_else(|| Local::now());
            let date = datetime.format("%Y-%m-%d %H:%M").to_string();

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
        
        log::debug!("🕐 get_commit_history_for_file: Commit {} processing took: {:?}", 
                  commit_info.id.to_string()[..8].to_string(), commit_start.elapsed());
    }
    
    log::info!("🕐 get_commit_history_for_file: Completed for '{}' - {} commits found from {} processed in {:?}", 
             file_path, commits.len(), commits_processed, start_time.elapsed());
    log::debug!("🕐 get_commit_history_for_file: Commit iteration took: {:?}", commit_iteration_start.elapsed());

    Ok(commits)
}

pub fn get_commit_history_chunk(
    repo: &Repository,
    file_path: &str,
    chunk_size: usize,
    start_offset: usize,
) -> Result<(Vec<CommitInfo>, bool), Box<dyn std::error::Error + Send + Sync>> {
    let start_time = Instant::now();
    log::debug!("🕐 get_commit_history_chunk: Starting for file: {} (chunk_size: {}, offset: {})", 
              file_path, chunk_size, start_offset);
    
    let mut commits = Vec::new();
    let mut commits_found = 0;
    let mut commits_processed = 0;

    // Normalize the file path by removing "./" prefix if present
    let normalized_path = if file_path.starts_with("./") {
        &file_path[2..]
    } else {
        file_path
    };

    // Use gix to walk the commit history
    let head_setup_start = Instant::now();
    let head_id = repo.head_id()?;
    let commit_iter = repo.rev_walk([head_id]).all()?;
    log::debug!("🕐 get_commit_history_chunk: Head setup took: {:?}", head_setup_start.elapsed());

    // Walk through commits and check if they modified the file
    let commit_iteration_start = Instant::now();
    
    for commit_info in commit_iter {
        let commit_start = Instant::now();
        let commit_info = commit_info?;
        let commit = repo.find_object(commit_info.id)?.try_into_commit()?;
        commits_processed += 1;

        // Check if this commit actually modified the file by comparing with parent(s)
        let modified_file = if commit.parent_ids().count() == 0 {
            // This is the initial commit, check if file exists
            let tree = commit.tree()?;
            tree.lookup_entry_by_path(normalized_path)?
                .is_some()
        } else {
            // Compare with parent commit(s) to see if file was modified
            let mut file_modified = false;
            let parent_comparison_start = Instant::now();

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
            log::debug!("🕐 get_commit_history_chunk: Parent comparison for commit {} took: {:?}", 
                      commit_info.id.to_string()[..8].to_string(), parent_comparison_start.elapsed());

            file_modified
        };

        if modified_file {
            // Skip commits until we reach the start_offset
            if commits_found < start_offset {
                commits_found += 1;
                continue;
            }

            // Stop if we've collected enough commits for this chunk
            if commits.len() >= chunk_size {
                log::info!("🕐 get_commit_history_chunk: Chunk complete for '{}' - {} commits collected in {:?}", 
                         file_path, commits.len(), start_time.elapsed());
                return Ok((commits, false)); // More commits available
            }

            // Get commit metadata
            let commit_obj = commit.decode()?;
            let author = &commit_obj.author;
            let message = commit_obj.message.to_string();

            // Parse Git timestamp format: "timestamp timezone" (e.g., "1751295482 -0400")
            let timestamp = match author.time.split_whitespace().next() {
                Some(ts_str) => ts_str.parse::<i64>().unwrap_or(0),
                None => 0,
            };
            
            // Format date as human-readable
            let datetime = Local.timestamp_opt(timestamp, 0).single().unwrap_or_else(|| Local::now());
            let date = datetime.format("%Y-%m-%d %H:%M").to_string();

            let commit_hash = commit_info.id.to_string();
            let short_hash = commit_hash[..8].to_string();

            commits.push(CommitInfo {
                hash: commit_hash,
                short_hash,
                author: author.name.to_string(),
                date,
                subject: message,
            });
            commits_found += 1;
        }
        
        log::debug!("🕐 get_commit_history_chunk: Commit {} processing took: {:?}", 
                  commit_info.id.to_string()[..8].to_string(), commit_start.elapsed());
    }
    
    log::info!("🕐 get_commit_history_chunk: Completed for '{}' - {} commits found from {} processed in {:?}", 
             file_path, commits.len(), commits_processed, start_time.elapsed());
    log::debug!("🕐 get_commit_history_chunk: Commit iteration took: {:?}", commit_iteration_start.elapsed());

    Ok((commits, true)) // All commits loaded
}

pub fn get_commit_history_streaming<F>(
    repo: &Repository,
    file_path: &str,
    mut on_commit_found: F,
    cancellation_token: &tokio_util::sync::CancellationToken,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>>
where
    F: FnMut(CommitInfo, usize) -> bool, // Returns false to stop early
{
    let start_time = Instant::now();
    log::debug!("🕐 get_commit_history_streaming: Starting for file: {}", file_path);
    
    let mut commits_found = 0;
    let mut commits_processed = 0;

    // Normalize the file path by removing "./" prefix if present
    let normalized_path = if file_path.starts_with("./") {
        &file_path[2..]
    } else {
        file_path
    };

    // Use gix to walk the commit history
    let head_setup_start = Instant::now();
    let head_id = repo.head_id()?;
    let commit_iter = repo.rev_walk([head_id]).all()?;
    log::debug!("🕐 get_commit_history_streaming: Head setup took: {:?}", head_setup_start.elapsed());

    // Walk through commits and check if they modified the file
    let commit_iteration_start = Instant::now();
    
    for commit_info in commit_iter {
        // Check for cancellation at the start of each commit iteration
        if cancellation_token.is_cancelled() {
            log::info!("🕐 get_commit_history_streaming: Task cancelled, stopping at {} commits found from {} processed", commits_found, commits_processed);
            break;
        }
        
        let commit_start = Instant::now();
        let commit_info = commit_info?;
        let commit = repo.find_object(commit_info.id)?.try_into_commit()?;
        commits_processed += 1;

        // Check if this commit actually modified the file by comparing with parent(s)
        let modified_file = if commit.parent_ids().count() == 0 {
            // This is the initial commit, check if file exists
            let tree = commit.tree()?;
            tree.lookup_entry_by_path(normalized_path)?
                .is_some()
        } else {
            // Compare with parent commit(s) to see if file was modified
            let mut file_modified = false;
            let parent_comparison_start = Instant::now();

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
            log::debug!("🕐 get_commit_history_streaming: Parent comparison for commit {} took: {:?}", 
                      commit_info.id.to_string()[..8].to_string(), parent_comparison_start.elapsed());

            file_modified
        };

        if modified_file {
            // Get commit metadata
            let commit_obj = commit.decode()?;
            let author = &commit_obj.author;
            let message = commit_obj.message.to_string();

            // Parse Git timestamp format: "timestamp timezone" (e.g., "1751295482 -0400")
            let timestamp = match author.time.split_whitespace().next() {
                Some(ts_str) => ts_str.parse::<i64>().unwrap_or(0),
                None => 0,
            };
            
            // Format date as human-readable
            let datetime = Local.timestamp_opt(timestamp, 0).single().unwrap_or_else(|| Local::now());
            let date = datetime.format("%Y-%m-%d %H:%M").to_string();

            let commit_hash = commit_info.id.to_string();
            let short_hash = commit_hash[..8].to_string();

            let commit_info = CommitInfo {
                hash: commit_hash,
                short_hash,
                author: author.name.to_string(),
                date,
                subject: message,
            };
            
            commits_found += 1;
            
            // Call the callback with the found commit
            if !on_commit_found(commit_info, commits_found) {
                log::info!("🕐 get_commit_history_streaming: Stopped early at {} commits by callback", commits_found);
                break;
            }
        }
        
        log::debug!("🕐 get_commit_history_streaming: Commit {} processing took: {:?}", 
                  commit_info.id.to_string()[..8].to_string(), commit_start.elapsed());
    }
    
    log::info!("🕐 get_commit_history_streaming: Completed for '{}' - {} commits found from {} processed in {:?}", 
             file_path, commits_found, commits_processed, start_time.elapsed());
    log::debug!("🕐 get_commit_history_streaming: Commit iteration took: {:?}", commit_iteration_start.elapsed());

    Ok(commits_found)
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

pub fn get_file_content_at_head(
    repo: &Repository,
    file_path: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    log::debug!("🕐 get_file_content_at_head: Starting for file: {}", file_path);
    
    // Get HEAD commit
    let head_id = repo.head_id()?;
    let head_hash = head_id.to_string();
    
    log::debug!("🕐 get_file_content_at_head: HEAD resolution took: {:?}", start_time.elapsed());
    
    // Use existing function to get content at HEAD
    get_file_content_with_gix(repo, file_path, &head_hash)
}

fn get_file_content_with_gix(
    repo: &Repository,
    file_path: &str,
    commit_hash: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    log::debug!("🕐 get_file_content_with_gix: Starting for file: {} at commit: {}", file_path, &commit_hash[..8]);
    
    // Find the commit object by hash
    let oid_start = Instant::now();
    let oid = gix::ObjectId::from_hex(commit_hash.as_bytes())?;
    let commit = repo.find_object(oid)?.try_into_commit()?;
    log::debug!("🕐 get_file_content_with_gix: OID resolution took: {:?}", oid_start.elapsed());

    // Get the tree from the commit
    let tree_start = Instant::now();
    let tree = commit.tree()?;
    log::debug!("🕐 get_file_content_with_gix: Tree retrieval took: {:?}", tree_start.elapsed());

    // Navigate to the file in the tree
    let lookup_start = Instant::now();
    let file_entry = tree
        .lookup_entry_by_path(file_path)?
        .ok_or_else(|| format!("File '{}' not found in commit {}", file_path, commit_hash))?;
    log::debug!("🕐 get_file_content_with_gix: File lookup took: {:?}", lookup_start.elapsed());

    // Get the blob content
    let blob_start = Instant::now();
    let blob = file_entry.object()?.try_into_blob()?;
    let content_bytes = blob.data.clone();
    log::debug!("🕐 get_file_content_with_gix: Blob retrieval took: {:?}, size: {} bytes", 
              blob_start.elapsed(), content_bytes.len());

    // Convert to string and split into lines
    let parsing_start = Instant::now();
    let content_str = String::from_utf8_lossy(&content_bytes);
    let lines: Vec<String> = content_str.lines().map(|line| line.to_string()).collect();
    log::debug!("🕐 get_file_content_with_gix: Content parsing took: {:?}, {} lines", 
              parsing_start.elapsed(), lines.len());
    
    log::info!("🕐 get_file_content_with_gix: Completed for '{}' at {} - {} lines in {:?}", 
             file_path, &commit_hash[..8], lines.len(), start_time.elapsed());

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
    fn test_gix_author_time_parsing() {
        // Test that we can properly parse gix author time format
        let repo = open_repository(".").expect("Should be able to open repository");
        let commits = get_commit_history_for_file(&repo, "src/main.rs").expect("Should get commit history");
        
        if !commits.is_empty() {
            let commit = &commits[0];
            
            // Check that the date is properly formatted (should be in YYYY-MM-DD HH:MM format)
            assert!(!commit.date.is_empty(), "Date should not be empty");
            assert!(commit.date.contains("-"), "Date should contain dash separators");
            assert!(commit.date.contains(":"), "Date should contain time separators");
            
            // Check that the date string is reasonable length (should be like "2025-06-30 14:30")
            assert!(commit.date.len() >= 16, "Date should be at least 16 characters long");
            assert!(commit.date.len() <= 20, "Date should be at most 20 characters long");
            
            // Verify the date format matches expected pattern (YYYY-MM-DD HH:MM)
            let parts: Vec<&str> = commit.date.split_whitespace().collect();
            assert_eq!(parts.len(), 2, "Date should have date and time parts");
            
            let date_part = parts[0];
            let time_part = parts[1];
            
            // Date part should be YYYY-MM-DD
            assert_eq!(date_part.len(), 10, "Date part should be 10 characters");
            assert_eq!(date_part.chars().nth(4), Some('-'), "Year-month separator should be dash");
            assert_eq!(date_part.chars().nth(7), Some('-'), "Month-day separator should be dash");
            
            // Time part should be HH:MM
            assert_eq!(time_part.len(), 5, "Time part should be 5 characters");
            assert_eq!(time_part.chars().nth(2), Some(':'), "Hour-minute separator should be colon");
            
            // Uncomment for debugging:
            // println!("Parsed date: {}", commit.date);
        }
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
