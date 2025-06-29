## Keep the cursor on “the same” line across commits

Line numbers mean nothing outside the version you’re looking at; all that survives is text.
So we:

1. **Diff the two blobs once.**
2. **Turn the edit script into an `old line → new line` table.**
3. **Look up (or approximate) the new position of the original line.**

Below is a compact, end‑to‑end Rust sketch that uses the `gix` crate’s built‑in diff (it re‑exports **imara‑diff**):

```rust
use gix::prelude::*;
use imara_diff::{diff, Algorithm, ChangeTag};

fn map_lines_between(comm_a: &str, comm_b: &str, path: &str) -> anyhow::Result<Vec<Option<usize>>> {
    // 1. open repo and load both blobs
    let repo   = gix::open(".")?;
    let blob_a = repo.find_object(comm_a)?.peel_to_tree()?.lookup_entry(path)?.object()?.peel_to_blob()?;
    let blob_b = repo.find_object(comm_b)?.peel_to_tree()?.lookup_entry(path)?.object()?.peel_to_blob()?;
    let old    : Vec<&str> = std::str::from_utf8(blob_a.data())?.lines().collect();
    let new_   : Vec<&str> = std::str::from_utf8(blob_b.data())?.lines().collect();

    // 2. diff once
    let ops = diff(Algorithm::Histogram, &old, &new_);

    // 3. build `old index -> new index` map
    let mut map = vec![None; old.len()];
    let (mut o, mut n) = (0usize, 0usize);

    for op in ops {
        match op.tag() {
            ChangeTag::Equal => {
                for _ in 0..op.len() {
                    map[o] = Some(n);
                    o += 1; n += 1;
                }
            }
            ChangeTag::Delete =>  o += op.len(),
            ChangeTag::Insert =>  n += op.len(),
        }
    }
    Ok(map)
}
```

### Using the map

```rust
let map = map_lines_between("HEAD", "HEAD~1", "src/lib.rs")?;
match map[42] {
    Some(l) => println!("Line 43 moved to {}", l + 1),      // exact match
    None    => {
        // fall back to nearest neighbour that survived
        let new_line = (1..)
            .find_map(|d| map.get(42 + d).copied().flatten()
                           .or_else(|| map.get(42 - d).copied().flatten()));
        println!("Original line vanished; nearest is {:?}", new_line);
    }
}
```

### Crates that can help

| Purpose                                       | Crate          | Comment                                                          |
| --------------------------------------------- | -------------- | ---------------------------------------------------------------- |
| Fast Git‑style diff                           | **imara‑diff** | Already a dependency of `gix`; implements Myers, Histogram, etc. |
| Friendly “text diff” API                      | **similar**    | `TextDiff::from_lines` gives you hunks and change tags.          |
| Follow a range through history (`git log ‑L`) | **gix‑blame**  | Experimental but works for viewers.                              |
| libgit2 bindings                              | **git2**       | `Diff`, `DiffHunk`, `DiffLine` deliver line offset info.         |

---

**Edge cases**

* **Deleted or radically changed lines** – walk up/down to the closest surviving mapping, or highlight that the line disappeared.
* **CR/LF differences** – normalise first or line counts drift.
* **Very large files** – cache the map; diff dominates runtime.

That’s it: one diff, one lookup table, and the cursor faithfully tracks code as it moves around.


---

## Implementation Plan

### Architecture Analysis

**Current State**: The git-lineage TUI has a solid foundation but currently resets cursor position to line 0 on every commit switch. Key components:

- **Commit switching**: `update_code_inspector_for_commit()` in `src/event.rs:270-331`
- **Cursor management**: `cursor_line` field in `App` struct, managed by `handle_inspector_event()`
- **Content loading**: `get_file_content_at_commit()` in `src/git_utils.rs` using gix library
- **No caching**: Every commit switch triggers fresh git operations

### Detailed Implementation Plan

> **Note**: Originally planned to include a caching layer in Phase 2, but we're deferring this optimization until we can measure actual performance. Better to implement the core functionality first and see if caching is actually needed.

#### Phase 1: Core Line Mapping Infrastructure ✅ COMPLETED

**1.1 Create line mapping module** (`src/line_mapping.rs`)
- `LineMapping` struct to represent old_line → new_line mappings
- Core function `map_lines_between_commits(repo, commit_a, commit_b, file_path)`
- Use `imara_diff` (already available via gix) with Histogram algorithm
- Return `Vec<Option<usize>>` where index = old line, value = new line (0-based)
- Handle edge cases: binary files, missing files, empty files

**1.2 Add error handling**
- Custom error types for mapping failures
- Graceful degradation when diff fails or file doesn't exist in one commit
- Log warnings for performance issues (very large diffs)

#### Phase 2: Position Tracking System

**2.1 Extend App state**
- `per_commit_cursor_positions: HashMap<(String, PathBuf), usize>` to remember cursor per (commit, file)
- `last_commit_for_mapping: Option<String>` to track previous commit for diff
- Helper methods: `save_cursor_position()`, `restore_cursor_position()`, `get_mapped_line()`

**2.2 Smart cursor positioning**
- When switching commits, calculate line mapping from previous → new commit
- Apply mapping to current cursor position with fallback strategies:
  1. **Exact match**: Direct mapping available
  2. **Nearest neighbor**: Search ±N lines for closest surviving line
  3. **Proportional**: Map using `(old_line / old_file_length) * new_file_length`
  4. **Fallback**: Default to saved position or top of file

#### Phase 3: Integration with Existing Code

**3.1 Modify commit switching logic**
- Update `update_code_inspector_for_commit()` in `src/event.rs`
- Before loading new content: save current cursor position
- After loading new content: calculate mapping and restore cursor position
- Handle first-time commit selection (no previous commit to map from)

**3.2 Update cursor management**
- Enhance `ensure_inspector_cursor_visible()` to handle mapped positions
- Add bounds checking for mapped positions exceeding file length
- Update viewport scrolling to center mapped line when possible

#### Phase 4: Edge Case Handling & User Experience

**4.1 Robust fallback strategies**
- Binary files: Disable line mapping, maintain basic line number
- File renames/moves: Detect via git and maintain mapping
- Very large files (>10k lines): Show warning and offer simpler mapping
- Completely rewritten files: Fall back to proportional mapping

**4.2 User feedback**
- Status messages when exact mapping unavailable
- Visual indicators when cursor position is approximated
- Option to disable feature if performance becomes problematic

#### Phase 5: Testing and Validation

**5.1 Unit tests for line mapping**
- Test cases for various diff scenarios: additions, deletions, moves, complex edits
- Edge cases: empty files, binary files, identical files
- Performance tests with large files

**5.2 Integration tests**
- Real git repositories with actual commit histories
- User interaction scenarios via existing test infrastructure
- Regression tests for cursor positioning accuracy

#### Future: Performance Optimizations (If Needed)

**Caching Layer (Deferred)**
- Will implement caching if performance testing reveals it's necessary
- Planned: `LineMappingCache` with LRU eviction policy
- Async line mapping for expensive operations
- Intelligent prefetching for adjacent commits

### Implementation Details

#### Key Data Structures

```rust
// src/line_mapping.rs
pub struct LineMapping {
    pub mapping: Vec<Option<usize>>,  // old_line -> new_line
    pub reverse_mapping: Vec<Option<usize>>, // new_line -> old_line
    pub old_file_size: usize,
    pub new_file_size: usize,
}

// src/app.rs additions
pub struct App {
    // ... existing fields ...
    pub per_commit_cursor_positions: HashMap<(String, PathBuf), usize>,
    pub last_commit_for_mapping: Option<String>,
    // Note: Caching will be added later if performance testing shows it's needed
}
```

#### Integration Points

1. **Event handling**: Modify `update_code_inspector_for_commit()` to use line mapping
2. **Git operations**: Extend `src/git_utils.rs` with diff utilities  
3. **UI feedback**: Add mapping status to status bar
4. **Performance monitoring**: Track mapping computation time to determine if async/caching optimizations are needed

#### Fallback Strategy Priority

1. **Exact mapping**: Line content unchanged between commits
2. **Nearest neighbor**: Within ±5 lines of exact position
3. **Contextual mapping**: Search for similar code patterns nearby
4. **Proportional mapping**: Scale line number by file size ratio
5. **Saved position**: Previously recorded cursor position for this commit
6. **Default**: Top of file (current behavior)

This plan provides a robust, performant solution that gracefully handles edge cases while maintaining the responsive feel of the TUI application.
