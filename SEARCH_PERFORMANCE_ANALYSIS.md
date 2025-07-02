# Search Performance Analysis

## Problem Summary

When searching for a single character like `s` in a large repository, the search display generation takes ~8 seconds. The profiling reveals:

- **48,491 search results** are found quickly (54ms)
- **3,607 directories** need to be expanded
- The bottleneck is in `directory_contains_matches_fast` which:
  - Is called for **every directory** in the tree
  - For each directory, it checks **all 48k results** to see if any are children
  - This is O(directories × results) = potentially millions of string comparisons

## Root Cause

The "fast" directory checking function is actually O(n) where n is the number of search results:

```rust
fn directory_contains_matches_fast(&self, dir_node: &TreeNode, results_set: &HashSet<&PathBuf>) -> bool {
    let dir_path_str = dir_node.path.to_string_lossy();
    let dir_prefix = format!("{}/", dir_path_str);
    
    // This iterates through ALL search results for EACH directory!
    results_set.iter().any(|result_path| {
        result_path.to_string_lossy().starts_with(&dir_prefix)
    })
}
```

## Performance Impact

With the profiling added, you'll see logs like:
- Total directory checks (including recursive): potentially thousands
- Each check does up to 48k string comparisons
- String conversion overhead for each comparison

## Optimization Strategies

### 1. Pre-build Parent-Child Index (Recommended)
Instead of checking if a directory contains results, build an index during the search phase:

```rust
// During search result collection
let mut parent_contains_match: HashSet<PathBuf> = HashSet::new();
for result in &results {
    let mut parent = result.parent();
    while let Some(p) = parent {
        parent_contains_match.insert(p.to_path_buf());
        parent = p.parent();
    }
}

// Then checking becomes O(1)
fn directory_contains_matches(&self, dir_path: &Path, index: &HashSet<PathBuf>) -> bool {
    index.contains(dir_path)
}
```

### 2. Use Path-Based Trie Structure
Build a trie from search results for efficient prefix matching.

### 3. Incremental Search Updates
Cache the tree structure and only update changed portions when search query changes.

### 4. Lazy Evaluation
Only compute visible items for the current viewport instead of all 52k items.

## Testing the Performance

Run the provided script to see the detailed profiling:

```bash
./test_search_perf.sh
```

Or manually:
```bash
RUST_LOG=debug cargo run --release
# Press '/' to search, then type 's'
```

The logs will show:
- Phase timings
- Number of directory checks
- Warnings when a single directory check does >10k comparisons