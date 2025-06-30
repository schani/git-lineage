use std::path::Path;
use gix::Repository;

/// Represents the mapping of lines from one version of a file to another
#[derive(Debug, Clone, PartialEq)]
pub struct LineMapping {
    /// Maps old line numbers to new line numbers (0-based indexing)
    /// None means the line was deleted in the new version
    pub mapping: Vec<Option<usize>>,
    
    /// Maps new line numbers to old line numbers (0-based indexing)  
    /// None means the line was added in the new version
    pub reverse_mapping: Vec<Option<usize>>,
    
    /// Number of lines in the old version
    pub old_file_size: usize,
    
    /// Number of lines in the new version  
    pub new_file_size: usize,
}

impl LineMapping {
    /// Create a new empty line mapping
    pub fn new(old_size: usize, new_size: usize) -> Self {
        Self {
            mapping: vec![None; old_size],
            reverse_mapping: vec![None; new_size],
            old_file_size: old_size,
            new_file_size: new_size,
        }
    }

    /// Create an identity mapping for identical files
    pub fn identity(file_size: usize) -> Self {
        let mapping = (0..file_size).map(Some).collect();
        let reverse_mapping = (0..file_size).map(Some).collect();
        
        Self {
            mapping,
            reverse_mapping,
            old_file_size: file_size,
            new_file_size: file_size,
        }
    }

    /// Map a line number from old to new version
    pub fn map_line(&self, old_line: usize) -> Option<usize> {
        self.mapping.get(old_line).copied().flatten()
    }

    /// Map a line number from new to old version
    pub fn reverse_map_line(&self, new_line: usize) -> Option<usize> {
        self.reverse_mapping.get(new_line).copied().flatten()
    }

    /// Find the nearest mapped line within a given range
    pub fn find_nearest_mapped_line(&self, old_line: usize, search_radius: usize) -> Option<usize> {
        // First try exact mapping
        if let Some(mapped) = self.map_line(old_line) {
            return Some(mapped);
        }

        // Search in expanding radius
        for radius in 1..=search_radius {
            // Try lines before
            if old_line >= radius {
                if let Some(mapped) = self.map_line(old_line - radius) {
                    return Some(mapped);
                }
            }
            
            // Try lines after
            if old_line + radius < self.old_file_size {
                if let Some(mapped) = self.map_line(old_line + radius) {
                    return Some(mapped);
                }
            }
        }

        None
    }

    /// Get proportional mapping when exact mapping fails
    pub fn proportional_map(&self, old_line: usize) -> usize {
        if self.old_file_size == 0 {
            return 0;
        }
        
        let ratio = old_line as f64 / self.old_file_size as f64;
        let new_line = (ratio * self.new_file_size as f64).round() as usize;
        
        // Ensure we don't exceed bounds
        new_line.min(self.new_file_size.saturating_sub(1))
    }

    /// Find exact content match for a line between commits
    /// Returns Some(line_number) if exactly one match is found, None otherwise
    pub fn find_exact_content_match(
        &self,
        old_line: usize,
        repo: &Repository,
        from_commit: &str,
        to_commit: &str,
        file_path: &Path,
    ) -> Result<Option<usize>, LineMappingError> {
        // Get the content of the line we're trying to map
        let old_content = get_file_content_at_commit(repo, from_commit, file_path)?;
        let old_lines: Vec<&str> = old_content.lines().collect();
        
        if old_line >= old_lines.len() {
            return Ok(None);
        }
        
        let target_line_content = old_lines[old_line];
        
        // Get the content of the target commit
        let new_content = get_file_content_at_commit(repo, to_commit, file_path)?;
        let new_lines: Vec<&str> = new_content.lines().collect();
        
        // Find all matching lines
        let matches: Vec<usize> = new_lines
            .iter()
            .enumerate()
            .filter_map(|(idx, &line)| {
                if line == target_line_content {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        
        // Return the match only if there's exactly one
        match matches.len() {
            1 => Ok(Some(matches[0])),
            _ => Ok(None), // 0 matches or multiple matches (ambiguous)
        }
    }

    /// Enhanced version of find_nearest_mapped_line that includes exact content fallback
    pub fn find_nearest_mapped_line_with_content_fallback(
        &self,
        old_line: usize,
        search_radius: usize,
        repo: &Repository,
        from_commit: &str,
        to_commit: &str,
        file_path: &Path,
    ) -> Result<Option<usize>, LineMappingError> {
        // First try the standard nearest mapping
        if let Some(mapped) = self.find_nearest_mapped_line(old_line, search_radius) {
            return Ok(Some(mapped));
        }
        
        // If that fails, try exact content fallback
        self.find_exact_content_match(old_line, repo, from_commit, to_commit, file_path)
    }
}

/// Error types for line mapping operations
#[derive(Debug, thiserror::Error)]
pub enum LineMappingError {
    #[error("Git error: {0}")]
    Git(#[from] gix::diff::tree::changes::Error),
    
    #[error("Object not found: {0}")]
    ObjectNotFound(String),
    
    #[error("File not found in commit: {path}")]
    FileNotFound { path: String },
    
    #[error("Binary file cannot be mapped: {path}")]
    BinaryFile { path: String },
    
    #[error("Diff computation failed: {reason}")]
    DiffFailed { reason: String },
}

/// Compute line mapping between two commits for a specific file
pub fn map_lines_between_commits(
    repo: &Repository,
    from_commit: &str,
    to_commit: &str,
    file_path: &Path,
) -> std::result::Result<LineMapping, LineMappingError> {
    // Handle same commit case
    if from_commit == to_commit {
        let content = get_file_content_at_commit(repo, from_commit, file_path)?;
        let line_count = content.lines().count();
        return Ok(LineMapping::identity(line_count));
    }

    // Get file content at both commits
    let old_content = get_file_content_at_commit(repo, from_commit, file_path)?;
    let new_content = get_file_content_at_commit(repo, to_commit, file_path)?;

    // Use similar crate for diffing (already in dependencies)
    let diff = similar::TextDiff::from_lines(&old_content, &new_content);
    
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();
    
    let mut mapping = LineMapping::new(old_lines.len(), new_lines.len());
    
    let mut old_line_idx = 0;
    let mut new_line_idx = 0;

    // Process diff operations to build mapping
    for change in diff.iter_all_changes() {
        match change.tag() {
            similar::ChangeTag::Equal => {
                // Lines are identical - create bidirectional mapping
                mapping.mapping[old_line_idx] = Some(new_line_idx);
                mapping.reverse_mapping[new_line_idx] = Some(old_line_idx);
                old_line_idx += 1;
                new_line_idx += 1;
            }
            similar::ChangeTag::Delete => {
                // Line was deleted - no mapping for this old line
                // mapping[old_line_idx] remains None
                old_line_idx += 1;
            }
            similar::ChangeTag::Insert => {
                // Line was inserted - no reverse mapping for this new line  
                // reverse_mapping[new_line_idx] remains None
                new_line_idx += 1;
            }
        }
    }

    Ok(mapping)
}

/// Get file content at a specific commit
fn get_file_content_at_commit(
    repo: &Repository,
    commit_hash: &str,
    file_path: &Path,
) -> std::result::Result<String, LineMappingError> {
    // Find the commit object by hash
    let oid = gix::ObjectId::from_hex(commit_hash.as_bytes())
        .map_err(|_| LineMappingError::ObjectNotFound(commit_hash.to_string()))?;
    
    let commit = repo.find_object(oid)
        .map_err(|_| LineMappingError::ObjectNotFound(commit_hash.to_string()))?
        .try_into_commit()
        .map_err(|_| LineMappingError::ObjectNotFound(commit_hash.to_string()))?;

    // Get the tree from the commit
    let tree = commit.tree()
        .map_err(|e| LineMappingError::DiffFailed { reason: e.to_string() })?;

    // Navigate to the file in the tree
    let mut buf = Vec::new();
    let file_entry = tree
        .lookup_entry_by_path(file_path, &mut buf)
        .map_err(|_| LineMappingError::FileNotFound { 
            path: file_path.to_string_lossy().to_string() 
        })?
        .ok_or_else(|| LineMappingError::FileNotFound { 
            path: file_path.to_string_lossy().to_string() 
        })?;

    // Get the blob content
    let blob = file_entry.object()
        .map_err(|e| LineMappingError::DiffFailed { reason: e.to_string() })?
        .try_into_blob()
        .map_err(|_| LineMappingError::BinaryFile { 
            path: file_path.to_string_lossy().to_string() 
        })?;
    
    let content_bytes = blob.data.clone();

    // Check if file is binary (contains null bytes)
    if content_bytes.contains(&0) {
        return Err(LineMappingError::BinaryFile { 
            path: file_path.to_string_lossy().to_string() 
        });
    }

    // Convert to string
    String::from_utf8(content_bytes)
        .map_err(|_| LineMappingError::BinaryFile { 
            path: file_path.to_string_lossy().to_string() 
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use std::process::Command;

    fn create_test_repo() -> (tempfile::TempDir, Repository) {
        let temp_dir = tempdir().unwrap();
        let repo_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Configure git user (required for commits)
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let repo = gix::discover(repo_path).unwrap();
        (temp_dir, repo)
    }

    fn commit_file(repo_path: &Path, filename: &str, content: &str, message: &str) -> String {
        let file_path = repo_path.join(filename);
        fs::write(&file_path, content).unwrap();

        Command::new("git")
            .args(["add", filename])
            .current_dir(repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo_path)
            .output()
            .unwrap();

        // Get the commit hash
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .unwrap();

        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    #[test]
    fn test_identity_mapping() {
        let mapping = LineMapping::identity(5);
        
        assert_eq!(mapping.old_file_size, 5);
        assert_eq!(mapping.new_file_size, 5);
        
        for i in 0..5 {
            assert_eq!(mapping.map_line(i), Some(i));
            assert_eq!(mapping.reverse_map_line(i), Some(i));
        }
    }

    #[test]
    fn test_proportional_mapping() {
        let mapping = LineMapping::new(10, 20);
        
        assert_eq!(mapping.proportional_map(0), 0);
        assert_eq!(mapping.proportional_map(5), 10);
        assert_eq!(mapping.proportional_map(9), 18);
    }

    #[test]
    fn test_nearest_mapped_line() {
        let mut mapping = LineMapping::new(10, 10);
        
        // Set up some mappings with gaps
        mapping.mapping[0] = Some(0);
        mapping.mapping[1] = None; // deleted
        mapping.mapping[2] = None; // deleted  
        mapping.mapping[3] = Some(1);
        mapping.mapping[4] = Some(2);
        
        // Test finding nearest for deleted line
        assert_eq!(mapping.find_nearest_mapped_line(1, 3), Some(0)); // nearest is line 0 -> 0
        assert_eq!(mapping.find_nearest_mapped_line(2, 3), Some(1)); // nearest is line 3 -> 1
    }

    #[test]
    fn test_same_commit_mapping() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        let content = "line 1\nline 2\nline 3\n";
        let commit_hash = commit_file(repo_path, "test.txt", content, "Initial commit");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit_hash,
            &commit_hash,
            Path::new("test.txt")
        ).unwrap();
        
        // Should be identity mapping
        assert_eq!(mapping.old_file_size, 3);
        assert_eq!(mapping.new_file_size, 3);
        
        for i in 0..3 {
            assert_eq!(mapping.map_line(i), Some(i));
        }
    }

    #[test]
    fn test_simple_line_addition() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // First commit
        let content1 = "line 1\nline 2\nline 3\n";
        let commit1 = commit_file(repo_path, "test.txt", content1, "Initial commit");
        
        // Second commit - add line in middle
        let content2 = "line 1\nNEW LINE\nline 2\nline 3\n";
        let commit2 = commit_file(repo_path, "test.txt", content2, "Add line");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("test.txt")
        ).unwrap();
        
        assert_eq!(mapping.old_file_size, 3);
        assert_eq!(mapping.new_file_size, 4);
        
        // line 0 -> 0 (unchanged)
        assert_eq!(mapping.map_line(0), Some(0));
        // line 1 -> 2 (shifted down by insertion)
        assert_eq!(mapping.map_line(1), Some(2));
        // line 2 -> 3 (shifted down by insertion)
        assert_eq!(mapping.map_line(2), Some(3));
        
        // Reverse mapping
        assert_eq!(mapping.reverse_map_line(0), Some(0)); // new line 0 from old line 0
        assert_eq!(mapping.reverse_map_line(1), None);    // new line 1 is inserted
        assert_eq!(mapping.reverse_map_line(2), Some(1)); // new line 2 from old line 1
        assert_eq!(mapping.reverse_map_line(3), Some(2)); // new line 3 from old line 2
    }

    #[test]
    fn test_line_deletion() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // First commit
        let content1 = "line 1\nDELETE ME\nline 2\nline 3\n";
        let commit1 = commit_file(repo_path, "test.txt", content1, "Initial commit");
        
        // Second commit - remove line
        let content2 = "line 1\nline 2\nline 3\n";
        let commit2 = commit_file(repo_path, "test.txt", content2, "Delete line");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("test.txt")
        ).unwrap();
        
        assert_eq!(mapping.old_file_size, 4);
        assert_eq!(mapping.new_file_size, 3);
        
        // line 0 -> 0 (unchanged)
        assert_eq!(mapping.map_line(0), Some(0));
        // line 1 -> None (deleted)
        assert_eq!(mapping.map_line(1), None);
        // line 2 -> 1 (shifted up)
        assert_eq!(mapping.map_line(2), Some(1));
        // line 3 -> 2 (shifted up)
        assert_eq!(mapping.map_line(3), Some(2));
    }

    #[test]
    fn test_file_not_found_error() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        let commit_hash = commit_file(repo_path, "test.txt", "content", "Initial commit");
        
        let result = map_lines_between_commits(
            &repo,
            &commit_hash,
            &commit_hash,
            Path::new("nonexistent.txt")
        );
        
        assert!(matches!(result, Err(LineMappingError::FileNotFound { .. })));
    }

    #[test] 
    fn test_empty_file_handling() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // First commit - empty file
        let commit1 = commit_file(repo_path, "empty.txt", "", "Empty file");
        
        // Second commit - add content
        let content2 = "line 1\nline 2\n";
        let commit2 = commit_file(repo_path, "empty.txt", content2, "Add content");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("empty.txt")
        ).unwrap();
        
        assert_eq!(mapping.old_file_size, 0);
        assert_eq!(mapping.new_file_size, 2);
        
        // All new lines should have no reverse mapping
        assert_eq!(mapping.reverse_map_line(0), None);
        assert_eq!(mapping.reverse_map_line(1), None);
    }

    #[test]
    fn test_exact_content_fallback_single_match() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // First commit - original content
        let content1 = "line 1\nUNIQUE_LINE\nline 2\nline 3\n";
        let commit1 = commit_file(repo_path, "test.txt", content1, "Initial commit");
        
        // Second commit - rearrange lines (UNIQUE_LINE moved to different position)
        let content2 = "line 1\nline 2\nUNIQUE_LINE\nline 3\n";
        let commit2 = commit_file(repo_path, "test.txt", content2, "Rearrange lines");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("test.txt")
        ).unwrap();
        
        // The UNIQUE_LINE should be mappable via exact content fallback
        // Line 1 (UNIQUE_LINE) in old commit should map to line 2 in new commit
        let result = mapping.find_exact_content_match(1, &repo, &commit1, &commit2, Path::new("test.txt"));
        assert_eq!(result.unwrap(), Some(2));
    }

    #[test]
    fn test_exact_content_fallback_no_match() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // First commit - original content
        let content1 = "line 1\nDELETED_LINE\nline 2\n";
        let commit1 = commit_file(repo_path, "test.txt", content1, "Initial commit");
        
        // Second commit - line completely removed
        let content2 = "line 1\nline 2\n";
        let commit2 = commit_file(repo_path, "test.txt", content2, "Remove line");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("test.txt")
        ).unwrap();
        
        // The deleted line should not be found
        let result = mapping.find_exact_content_match(1, &repo, &commit1, &commit2, Path::new("test.txt"));
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_exact_content_fallback_multiple_matches() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // First commit - original content with duplicate line
        let content1 = "line 1\nDUPLICATE\nline 2\n";
        let commit1 = commit_file(repo_path, "test.txt", content1, "Initial commit");
        
        // Second commit - multiple instances of the same line
        let content2 = "line 1\nDUPLICATE\nDUPLICATE\nline 2\n";
        let commit2 = commit_file(repo_path, "test.txt", content2, "Add duplicate");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("test.txt")
        ).unwrap();
        
        // When there are multiple matches, should return None (ambiguous)
        let result = mapping.find_exact_content_match(1, &repo, &commit1, &commit2, Path::new("test.txt"));
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_exact_content_fallback_whitespace_sensitivity() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // First commit - line with trailing space
        let content1 = "line 1\nLINE_WITH_SPACE \nline 2\n";
        let commit1 = commit_file(repo_path, "test.txt", content1, "Initial commit");
        
        // Second commit - same line but without trailing space
        let content2 = "line 1\nLINE_WITH_SPACE\nline 2\n";
        let commit2 = commit_file(repo_path, "test.txt", content2, "Remove trailing space");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("test.txt")
        ).unwrap();
        
        // Should not match because of whitespace difference
        let result = mapping.find_exact_content_match(1, &repo, &commit1, &commit2, Path::new("test.txt"));
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_exact_content_fallback_integration() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // First commit - original structure
        let content1 = "function foo() {\n  return 42;\n}\nfunction bar() {\n  return 24;\n}\n";
        let commit1 = commit_file(repo_path, "code.js", content1, "Initial functions");
        
        // Second commit - functions reordered and middle content changed
        let content2 = "function bar() {\n  return 24;\n}\nfunction foo() {\n  return 99;\n}\n";
        let commit2 = commit_file(repo_path, "code.js", content2, "Reorder and modify");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("code.js")
        ).unwrap();
        
        // Line 3 "function bar() {" should map to line 0 via exact content match
        let result = mapping.find_exact_content_match(3, &repo, &commit1, &commit2, Path::new("code.js"));
        assert_eq!(result.unwrap(), Some(0));
        
        // Line 4 "  return 24;" should map to line 1 via exact content match  
        let result = mapping.find_exact_content_match(4, &repo, &commit1, &commit2, Path::new("code.js"));
        assert_eq!(result.unwrap(), Some(1));
    }

    #[test]
    fn test_find_nearest_with_exact_content_fallback_integration() {
        let (_temp_dir, repo) = create_test_repo();
        let repo_path = _temp_dir.path();
        
        // Create a scenario where we can test the fallback logic
        let content1 = "line_1\nline_2\nline_3\nMOVED_LINE\nline_4\nline_5\n";
        let commit1 = commit_file(repo_path, "test.txt", content1, "Initial commit");
        
        // Simulate a complex restructuring where MOVED_LINE goes to a different location
        let content2 = "new_1\nnew_2\nnew_3\nnew_4\nnew_5\nMOVED_LINE\nnew_6\n";
        let commit2 = commit_file(repo_path, "test.txt", content2, "Restructure completely");
        
        let mapping = map_lines_between_commits(
            &repo,
            &commit1,
            &commit2,
            Path::new("test.txt")
        ).unwrap();
        
        // Test that the enhanced method finds content matches when standard mapping fails
        // We'll test with line 3 (MOVED_LINE) from the first commit
        let result = mapping.find_nearest_mapped_line_with_content_fallback(
            3, 0, &repo, &commit1, &commit2, Path::new("test.txt")
        );
        
        // The exact content fallback should find the moved line, even if nearest neighbor fails
        match result {
            Ok(Some(new_line)) => {
                // Verify that the content actually matches
                let old_content = get_file_content_at_commit(&repo, &commit1, Path::new("test.txt")).unwrap();
                let new_content = get_file_content_at_commit(&repo, &commit2, Path::new("test.txt")).unwrap();
                let old_lines: Vec<&str> = old_content.lines().collect();
                let new_lines: Vec<&str> = new_content.lines().collect();
                assert_eq!(old_lines[3], new_lines[new_line]);
                assert_eq!(old_lines[3], "MOVED_LINE");
            }
            _ => panic!("Expected to find exact content match"),
        }
    }
}