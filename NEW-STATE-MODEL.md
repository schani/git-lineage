# New State Model Architecture

## 🎯 Problem Statement

The current search functionality has a fundamental bug where selection state gets corrupted when transitioning between search and normal modes. This document outlines a complete architectural solution.

## 🔍 Root Cause Analysis

### Current Architecture Problems

1. **Dual Tree Anti-Pattern**
   ```rust
   pub struct FileTreeState {
       original_tree: FileTree,  // Complete tree
       display_tree: FileTree,   // Filtered copy - DATA DUPLICATION!
   }
   ```

2. **Mixed Concerns** - `FileTreeState` manages:
   - Tree structure
   - Selection state  
   - View state
   - Search state
   - Navigation state

3. **Imperative State Management** - State changes scattered across files
4. **Index-Based Navigation** - Selection becomes meaningless when tree structure changes

### The Bug in Detail

```rust
// User has src/api/client.rs selected
current_selection = "src/api/client.rs"

// User starts search for "main"
display_tree = rebuild_filtered_tree("main")  // Now only contains src/main.rs
current_selection = "src/main.rs"             // Selection jumps!

// User exits search
display_tree = original_tree.clone()          // Back to full tree
current_selection = "src/main.rs"             // But user expects src/api/client.rs!
```

## 🏗️ New Architecture: State Machine + Single Tree

### Core Principles

1. **Single Source of Truth** - One immutable tree, multiple views
2. **State Machine** - Clear mode transitions with preserved context
3. **View Generation** - Filter on-demand, no data duplication
4. **Event-Driven** - Predictable state transitions

### Data Structure

```rust
pub struct NavigatorState {
    tree: FileTree,              // Single immutable tree - never changes
    mode: NavigatorMode,         // Current state with complete context
}

#[derive(Debug, Clone)]
pub enum NavigatorMode {
    Browsing {
        selection: Option<PathBuf>,      // Absolute path reference
        expanded: HashSet<PathBuf>,      // Which directories are open
        scroll_offset: usize,            // View scroll position
    },
    Searching {
        query: String,                   // Current search query
        results: Vec<PathBuf>,          // Filtered results
        selected_index: Option<usize>,   // Index in results
        saved_browsing: Box<NavigatorMode>, // COMPLETE preserved context
    },
}
```

### Key Insight: Views, Not Copies

Instead of maintaining two tree copies:

```rust
// ❌ Current approach - dual trees
struct FileTreeState {
    original_tree: FileTree,     // Data stored here
    display_tree: FileTree,      // Same data duplicated here
}

// ✅ New approach - single tree + dynamic views  
struct NavigatorState {
    tree: FileTree,                    // Data stored once
    cached_view: Option<Vec<PathBuf>>, // Generated view
}

impl NavigatorState {
    pub fn get_visible_items(&self) -> Vec<VisibleItem> {
        match &self.mode {
            NavigatorMode::Browsing { expanded, selection, .. } => {
                self.tree.flatten()
                    .filter(|node| self.is_visible_in_browsing(node, expanded))
                    .map(|node| VisibleItem {
                        path: node.path.clone(),
                        name: node.name.clone(),
                        depth: node.depth,
                        is_selected: Some(&node.path) == selection.as_ref(),
                    })
                    .collect()
            }
            NavigatorMode::Searching { query, results, selected_index, .. } => {
                results.iter().enumerate()
                    .map(|(i, path)| VisibleItem {
                        path: path.clone(),
                        name: path.file_name().unwrap().to_string(),
                        depth: 0, // Flat search results
                        is_selected: Some(i) == *selected_index,
                    })
                    .collect()
            }
        }
    }
}
```

## 🔄 State Transitions

### Event-Driven Architecture

```rust
#[derive(Debug)]
pub enum NavigatorEvent {
    SelectFile(PathBuf),
    StartSearch,
    UpdateSearchQuery(String),
    EndSearch,
    NavigateUp,
    NavigateDown,
    ToggleExpanded(PathBuf),
}

impl NavigatorState {
    pub fn handle_event(&mut self, event: NavigatorEvent) -> Result<()> {
        self.mode = match (&self.mode, event) {
            // Enter search mode - preserve complete context
            (NavigatorMode::Browsing(state), NavigatorEvent::StartSearch) => {
                NavigatorMode::Searching {
                    query: String::new(),
                    results: Vec::new(),
                    selected_index: None,
                    saved_browsing: Box::new(NavigatorMode::Browsing(state.clone())),
                }
            }
            
            // Exit search mode - restore complete context
            (NavigatorMode::Searching(state), NavigatorEvent::EndSearch) => {
                *state.saved_browsing.clone() // Perfect restoration!
            }
            
            // Update search query
            (NavigatorMode::Searching(state), NavigatorEvent::UpdateSearchQuery(query)) => {
                let results = self.tree.search(&query);
                NavigatorMode::Searching {
                    query,
                    results,
                    selected_index: if results.is_empty() { None } else { Some(0) },
                    saved_browsing: state.saved_browsing.clone(),
                }
            }
            
            // Navigation in browsing mode
            (NavigatorMode::Browsing(state), NavigatorEvent::NavigateDown) => {
                let visible_items = self.get_visible_items();
                let new_selection = self.find_next_item(&visible_items, &state.selection);
                NavigatorMode::Browsing {
                    selection: new_selection,
                    expanded: state.expanded.clone(),
                    scroll_offset: self.calculate_scroll_offset(&new_selection),
                }
            }
            
            // Navigation in search mode
            (NavigatorMode::Searching(state), NavigatorEvent::NavigateDown) => {
                let new_index = state.selected_index
                    .map(|i| (i + 1).min(state.results.len().saturating_sub(1)))
                    .or_else(|| if !state.results.is_empty() { Some(0) } else { None });
                
                NavigatorMode::Searching {
                    selected_index: new_index,
                    ..state.clone()
                }
            }
            
            // ... other transitions
        };
        Ok(())
    }
}
```

## 🎨 UI Integration

### View Model Generation

```rust
#[derive(Debug)]
pub struct VisibleItem {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_selected: bool,
    pub is_expanded: bool,
}

impl NavigatorState {
    pub fn build_view_model(&self) -> NavigatorViewModel {
        let items = self.get_visible_items();
        let scroll_offset = self.get_scroll_offset();
        let cursor_position = self.get_cursor_position();
        
        NavigatorViewModel {
            items,
            scroll_offset,
            cursor_position,
            search_query: self.get_search_query(),
            is_searching: matches!(self.mode, NavigatorMode::Searching { .. }),
        }
    }
}
```

### Rendering

```rust
// ui.rs
pub fn draw_file_navigator(f: &mut Frame, app: &App, area: Rect) {
    let view_model = app.navigator.build_view_model();
    
    let items: Vec<ListItem> = view_model.items
        .iter()
        .map(|item| {
            let style = if item.is_selected {
                Style::default().bg(Color::Blue)
            } else {
                Style::default()
            };
            
            let content = format!("{}{}", "  ".repeat(item.depth), item.name);
            ListItem::new(content).style(style)
        })
        .collect();
    
    let list = List::new(items)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(if view_model.is_searching {
                format!("Search: {}", view_model.search_query)
            } else {
                "Files".to_string()
            }));
    
    f.render_stateful_widget(list, area, &mut view_model.list_state);
}
```

## 🔧 Implementation Strategy

### Phase 1: Extract Current Logic
1. Create new `NavigatorState` with current logic
2. Implement basic state transitions
3. Replace `FileTreeState` usage gradually

### Phase 2: Event System
1. Define `NavigatorEvent` enum
2. Implement `handle_event` method
3. Update event handlers in `event.rs`

### Phase 3: View Generation
1. Implement `get_visible_items` method
2. Build view model for UI
3. Update rendering logic

### Phase 4: Testing & Migration
1. Add comprehensive state transition tests
2. Test search functionality end-to-end
3. Remove old dual-tree code

## 🚀 Benefits

### Bug Elimination
- **No more selection jumps** - Context preservation is automatic
- **No state corruption** - Atomic state transitions
- **Predictable behavior** - Clear state machine rules

### Performance Improvements
- **No tree duplication** - 50% less memory usage
- **Efficient filtering** - Generate views on-demand
- **Faster searches** - No tree rebuilding overhead

### Developer Experience
- **Easy testing** - Mock events, verify state transitions
- **Clear mental model** - Always know what mode you're in
- **Future-proof** - Adding diff/blame modes becomes trivial

### Code Quality
- **Single responsibility** - Each mode owns its logic
- **Immutable data** - No accidental mutations
- **Event-driven** - Predictable and debuggable

## 🧪 Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_preserves_browsing_context() {
        let mut navigator = NavigatorState::new(create_test_tree());
        
        // Set up browsing state
        navigator.handle_event(NavigatorEvent::SelectFile("src/api/client.rs".into())).unwrap();
        navigator.handle_event(NavigatorEvent::ToggleExpanded("src".into())).unwrap();
        navigator.handle_event(NavigatorEvent::ToggleExpanded("src/api".into())).unwrap();
        
        let original_state = navigator.mode.clone();
        
        // Enter search mode
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        navigator.handle_event(NavigatorEvent::UpdateSearchQuery("main".into())).unwrap();
        
        // Exit search mode
        navigator.handle_event(NavigatorEvent::EndSearch).unwrap();
        
        // Verify context is perfectly restored
        assert_eq!(navigator.mode, original_state);
    }
    
    #[test]
    fn test_search_results_navigation() {
        let mut navigator = NavigatorState::new(create_test_tree());
        
        navigator.handle_event(NavigatorEvent::StartSearch).unwrap();
        navigator.handle_event(NavigatorEvent::UpdateSearchQuery("test".into())).unwrap();
        
        if let NavigatorMode::Searching { selected_index, results, .. } = &navigator.mode {
            assert_eq!(*selected_index, Some(0));
            assert!(!results.is_empty());
        }
        
        navigator.handle_event(NavigatorEvent::NavigateDown).unwrap();
        
        if let NavigatorMode::Searching { selected_index, .. } = &navigator.mode {
            assert_eq!(*selected_index, Some(1));
        }
    }
}
```

## 🎉 Conclusion

This new state machine architecture eliminates the root causes of the search bug while providing a robust foundation for future features. The key insight is that we don't need two trees - we need one tree and different ways of viewing it.

The transition preserves complete context, making the user experience seamless and predictable. Performance improves due to eliminated data duplication, and the code becomes much easier to reason about and test.

**Next Steps**: Implement Phase 1 to extract current logic into the new structure, then gradually migrate the rest of the system.