# File Tree Architecture

## Current Problem

The file tree implementation is fundamentally broken because it mixes two different data structures and operations:

1. **UI rendering** uses filtered/searched nodes from `get_fuzzy_filtered_visible_nodes()`
2. **Navigation and selection** operates on the original full tree structure via `current_selection`
3. **Result**: Complete disconnect between what the user sees and what gets selected

### The Bug in Action

```
User searches for "c" → UI shows: [src, config.toml]
User navigates to position 1 → cursor_position = 1 (pointing to config.toml in UI)
BUT current_selection still points to old tree position!
get_selected_file_path() returns wrong file!
Code Inspector loads wrong file content!
```

## Required Architecture

We need **TWO SEPARATE TREE STRUCTURES**:

### 1. Original Tree
- **Purpose**: Complete, unfiltered file tree structure
- **Source**: Loaded from filesystem/git
- **Never changes** except on filesystem reload
- **Used for**: Search matching only

### 2. Display Tree  
- **Purpose**: Current filtered/expanded view that user sees
- **Source**: Derived from Original Tree based on:
  - Search filters
  - Expansion state
  - User navigation
- **Changes when**: Search query changes, directories expand/collapse
- **Used for**: ALL user interactions (navigation, selection, rendering)

### 3. Display List
- **Purpose**: Linear list for UI rendering
- **Source**: Derived from Display Tree by flattening expanded nodes
- **Used for**: UI rendering only

## Data Flow

```
Original Tree (filesystem) 
    ↓
    ↓ [search/filter/expand operations]
    ↓
Display Tree (user's current view)
    ↓
    ↓ [flatten expanded nodes]
    ↓
Display List (linear for UI rendering)
```

## Operations by Data Structure

### Original Tree
- **READ ONLY** after initial load
- Used for fuzzy search matching
- Filesystem reload updates

### Display Tree  
- **All navigation** (up/down/left/right)
- **All selection** (`current_selection` points here)
- **All expansion/collapse**
- **Cursor position tracking**
- **File selection** (`get_selected_file_path()` uses this)

### Display List
- **UI rendering only**
- **Scrolling/viewport calculations**
- **Visual highlighting**

## Implementation Changes Required

### Current Broken Code
```rust
// UI uses filtered nodes
let all_visible_nodes = if !app.navigator.search_query.is_empty() {
    app.navigator.file_tree.get_fuzzy_filtered_visible_nodes(&query)
} else {
    app.navigator.file_tree.get_visible_nodes_with_depth()
};

// But selection uses original tree!
pub fn get_selected_file_path(&self) -> Option<PathBuf> {
    self.navigator.file_tree.current_selection.clone()  // WRONG TREE!
}
```

### Required New Code Structure
```rust
pub struct FileTreeState {
    /// Complete filesystem tree (read-only after load)
    original_tree: FileTree,
    
    /// Current user view (filtered/expanded)
    display_tree: FileTree,
    
    /// Current selection in display tree
    current_selection: Option<PathBuf>,
    
    /// Current search query
    search_query: String,
}

impl FileTreeState {
    /// Rebuild display tree from original tree + search query
    fn update_display_tree(&mut self) {
        if self.search_query.is_empty() {
            self.display_tree = self.original_tree.clone();
        } else {
            self.display_tree = self.create_filtered_tree(&self.search_query);
        }
        // Preserve or reset selection as appropriate
        self.update_selection_after_display_change();
    }
    
    /// All navigation operates on display tree
    fn navigate_up(&mut self) -> bool {
        // Navigate in display_tree, update current_selection
    }
    
    /// File selection uses display tree
    fn get_selected_file_path(&self) -> Option<PathBuf> {
        self.current_selection.clone()  // Points to display_tree!
    }
}
```

## Key Principles

1. **Everything (!!!!!) must operate on the display tree, except for search**
2. **Search operates on original tree to find matches**
3. **Display tree is rebuilt when search query changes**
4. **All user interactions (navigation, selection, expansion) work on display tree**
5. **Display list is derived from display tree for UI rendering**
6. **No mixing of tree structures in any operation**

## Benefits

- **Consistent selection**: What you see is what you select
- **Proper search interaction**: Navigation works correctly in filtered results
- **Clean separation**: Search logic vs navigation logic
- **Expansion state**: Can be properly maintained per tree state
- **Performance**: Display operations don't need to re-filter
- **Debuggability**: Clear data flow and responsibilities

## Migration Strategy

1. **Add `original_tree` and `display_tree` fields**
2. **Update search to rebuild `display_tree`**  
3. **Move all navigation to operate on `display_tree`**
4. **Update selection tracking to use `display_tree`**
5. **Update UI rendering to use `display_tree` consistently**
6. **Remove all mixed-tree operations**
7. **Add comprehensive tests for tree synchronization**

This architecture fixes the fundamental design flaw that causes the file selection bugs during search operations.