# New Navigator State Model

## Overview

The navigator has been simplified from a complex state machine with multiple modes to a single, unified state structure. This eliminates the dual-tree anti-pattern and ensures consistent filtering behavior.

## Core Structure

```rust
pub struct NavigatorState {
    tree: FileTree,
    selection: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    scroll_offset: usize,
    query: String,  // Empty string means show all files, non-empty means filter
    editing_search: bool,  // UI state for showing search cursor
}
```

## Key Principles

1. **Single Source of Truth**: One state struct, one way to filter, one way to render
2. **Consistent Filtering**: The same filtering logic applies whether typing or not
3. **Simple UI State**: `editing_search` only controls cursor display, not behavior
4. **No State Machine**: No complex transitions or saved states

## Behavior Logic

### Search Activation
- **Input**: `/` key press
- **Action**: `editing_search = true`
- **Result**: Search cursor appears, current query remains active

### Character Input
- **Condition**: `editing_search == true`
- **Input**: Any character
- **Action**: Update `query` string
- **Result**: Tree immediately filters to match query

### Search Completion (Enter)
- **Input**: Enter key while `editing_search == true`
- **Action**: `editing_search = false`
- **Result**: 
  - Search cursor disappears
  - Query remains active (tree stays filtered)
  - User can navigate filtered results

### Search Cancellation (Escape)
- **Input**: Escape key while `editing_search == true`
- **Action**: 
  - `editing_search = false`
  - `query = ""`
- **Result**:
  - Search cursor disappears
  - Filter is cleared (tree shows all files)

## Rendering Logic

### Tree Display
```rust
fn build_view_model(&self) -> NavigatorViewModel {
    let items = if self.query.is_empty() {
        // Show full tree
        self.get_browsing_visible_items(&self.expanded, &self.selection)
    } else {
        // Show filtered tree (same logic always)
        let results = self.search_files(&self.query);
        self.get_search_visible_items(&results, &None)
    };
    
    NavigatorViewModel {
        items,
        scroll_offset: self.scroll_offset,
        cursor_position: self.calculate_cursor_position(),
        search_query: self.query.clone(),
        is_searching: self.editing_search,  // Only for UI cursor
    }
}
```

### UI Elements
- **Title Bar**: 
  - If `editing_search`: "File Navigator (Search: {query}█"
  - Else: "File Navigator"
- **Status Bar**:
  - If `editing_search`: "Search mode activated"
  - Else if `!query.is_empty()`: "Filtered results - {query}"
  - Else: Normal status

## Event Handling

### Simplified Event Processing
```rust
pub fn handle_event(&mut self, event: NavigatorEvent) {
    match event {
        NavigatorEvent::StartSearch => {
            self.editing_search = true;
        }
        
        NavigatorEvent::UpdateSearchQuery(new_query) => {
            if self.editing_search {
                self.query = new_query;
            }
        }
        
        NavigatorEvent::EndSearch => {
            self.editing_search = false;
            self.query.clear();
        }
        
        NavigatorEvent::EndSearchKeepQuery => {
            self.editing_search = false;
            // query stays as-is
        }
        
        // Navigation events work the same regardless of search state
        NavigatorEvent::NavigateUp => {
            let visible_items = self.get_current_visible_items();
            self.selection = self.find_previous_item(&visible_items, &self.selection);
        }
        
        // ... other navigation events
    }
}
```

## Benefits

### 1. Eliminates Inconsistency
- **Before**: Search mode and browsing-with-query used different filtering logic
- **After**: One filtering function, always consistent results

### 2. Simplifies State Management
- **Before**: Complex state machine with transitions and saved states
- **After**: Simple boolean flag for UI state

### 3. Removes Anti-Patterns
- **Before**: Dual trees, separate result vectors, complex transitions
- **After**: Single tree, single filtering path

### 4. Improves Maintainability
- **Before**: Multiple code paths to maintain and test
- **After**: One clear, linear code path

### 5. Fixes User Experience
- **Before**: Tree changes unexpectedly after Enter
- **After**: Tree stays consistent, only cursor disappears

## Migration Strategy

1. **Replace NavigatorMode enum** with simple struct fields
2. **Consolidate filtering logic** into single function
3. **Update event handlers** to modify fields directly
4. **Simplify view model building** to one path
5. **Update tests** to verify consistent behavior

## Key Insight

The fundamental insight is that **search is not a mode, it's a filter**. The "mode" is just whether the user is currently editing that filter. This mental model maps perfectly to the simplified state structure and eliminates all the complexity that caused the original bug.

## Implementation Details

### Core State Structure
```rust
pub struct NavigatorState {
    tree: FileTree,
    selection: Option<PathBuf>,
    expanded: HashSet<PathBuf>,
    scroll_offset: usize,
    query: String,
    editing_search: bool,
}

impl NavigatorState {
    pub fn new(tree: FileTree) -> Self {
        let expanded = Self::extract_expanded_paths(&tree);
        Self {
            tree,
            selection: None,
            expanded,
            scroll_offset: 0,
            query: String::new(),
            editing_search: false,
        }
    }
}
```

### Event Handling
```rust
pub fn handle_event(&mut self, event: NavigatorEvent) {
    match event {
        NavigatorEvent::StartSearch => {
            self.editing_search = true;
        }
        
        NavigatorEvent::UpdateSearchQuery(new_query) => {
            if self.editing_search {
                self.query = new_query;
            }
        }
        
        NavigatorEvent::EndSearch => {
            self.editing_search = false;
            self.query.clear();
        }
        
        NavigatorEvent::EndSearchKeepQuery => {
            self.editing_search = false;
        }
        
        NavigatorEvent::NavigateUp => {
            let visible_items = self.get_current_visible_items();
            self.selection = self.find_previous_item(&visible_items, &self.selection);
        }
        
        NavigatorEvent::NavigateDown => {
            let visible_items = self.get_current_visible_items();
            self.selection = self.find_next_item(&visible_items, &self.selection);
        }
        
        NavigatorEvent::ToggleExpanded(path) => {
            if self.expanded.contains(&path) {
                self.expanded.remove(&path);
            } else {
                self.expanded.insert(path);
            }
        }
        
        NavigatorEvent::SelectFile(path) => {
            self.selection = Some(path);
        }
        
        // ... other events
    }
}
```

### View Model Generation
```rust
pub fn build_view_model(&self) -> NavigatorViewModel {
    let items = if self.query.is_empty() {
        self.get_browsing_visible_items(&self.expanded, &self.selection)
    } else {
        let results = self.search_files(&self.query);
        self.get_search_visible_items(&results, &self.selection)
    };
    
    let cursor_position = self.selection
        .as_ref()
        .and_then(|sel| items.iter().position(|item| &item.path == sel))
        .unwrap_or(0);
    
    NavigatorViewModel {
        items,
        scroll_offset: self.scroll_offset,
        cursor_position,
        search_query: self.query.clone(),
        is_searching: self.editing_search,
    }
}
```

This architecture eliminates the state machine complexity while ensuring consistent behavior across all interaction patterns.