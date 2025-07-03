# Refactoring Plan: Fixing This Shit

This document outlines the necessary steps to fix the core architectural problems in the codebase. The primary goal is to eliminate state duplication and remove obsolete code, leading to a simpler, more maintainable application. This plan focuses only on essential fixes.

## The Core Problem: Dual State Management

The application currently maintains two separate implementations for the file navigator:
1.  A legacy, complex implementation within `src/app.rs`.
2.  A newer, cleaner, state-machine-based implementation in `src/navigator.rs`.

This has resulted in two sources of truth, a cascade of conditional logic to switch between them, and a significant amount of "cruft" and dead code.

---

## Phase 1: Unify the Navigator

The highest priority is to rip out the old navigator and make the new one the single source of truth.

### Source of Truth

-   **`navigator::NavigatorState` will be the *only* source of truth for the file tree, selection state, expansion state, and search state.**
-   The main `App` struct in `src/app.rs` will own this state directly, not as an `Option`.

### Data Structure Changes

The `App` struct will be simplified.

**FROM (in `src/app.rs`):**
```rust
pub struct App {
    // ...
    pub navigator: NavigatorState, // Legacy navigator
    pub new_navigator: Option<NewNavigatorState>, // New state machine navigator
    pub cached_navigator_view_model: Option<crate::navigator::NavigatorViewModel>,
    // ...
}
```

**TO (in `src/app.rs`):**
```rust
pub struct App {
    // ...
    pub navigator: crate::navigator::NavigatorState, // The one and only navigator
    // ...
}
```

### Deletions (What to Rip Out)

The following components are obsolete and **must be deleted**:

1.  **Structs:**
    -   The `app::NavigatorState` struct in `src/app.rs`.
    -   The `app::FileTreeNode` struct in `src/app.rs` (it's redundant; `tree::TreeNode` is used by the new navigator).

2.  **Fields in `App`:**
    -   `App.navigator` (the instance of the old `app::NavigatorState`).
    -   `App.new_navigator` will be renamed to `App.navigator`.
    -   `App.cached_navigator_view_model`. The view model should be built on-demand by the UI during the draw call. Caching it on the `App` struct is another form of state duplication.

3.  **Methods in `App`:**
    -   `is_using_new_navigator()`
    -   `navigate_tree_up()` (and its underlying `navigate_file_navigator_up()`)
    -   `navigate_tree_down()` (and its underlying `navigate_file_navigator_down()`)
    -   `expand_selected_node()`
    -   `collapse_selected_node()`
    -   `toggle_selected_node()`
    -   `update_file_navigator_list_state()`
    -   `set_file_navigator_viewport_height()`
    -   `set_file_tree_from_directory()`
    -   `get_navigator_search_query()` (the version with branching logic)
    -   `is_navigator_searching()` (the version with branching logic)
    -   `refresh_navigator_view_model()`

4.  **Conditional Logic:**
    -   All `if self.is_using_new_navigator()` blocks and their `else` counterparts throughout the entire codebase.

5.  **Test Code:**
    -   The parts of `App::from_test_config` that initialize the old navigator state.
    -   Any tests that specifically target the old navigator's methods.

---

## Phase 2: Simplify State & Remove Cruft

With the navigator unified, the next step is to simplify the remaining state management and remove other sources of redundancy and inefficiency.

### Source of Truth

-   **Core application state lives in `App` and its sub-structs (`HistoryState`, `InspectorState`).**
-   **UI widget state (e.g., `ratatui::widgets::ListState`) does not belong in the application state.** It should be created transiently within the `draw` functions in `src/ui.rs`. The application state should only store the *data* needed to render (e.g., the list of commits and the index of the selected one), not the UI widget's internal state.

### Data Structure Changes

The `HistoryState` struct will be simplified to remove its dependency on `ratatui`.

**FROM (in `src/app.rs`):**
```rust
pub struct HistoryState {
    pub commit_list: Vec<CommitInfo>,
    pub list_state: ListState, // <-- DELETE
    pub selected_commit_hash: Option<String>,
    // ...
}
```

**TO (in `src/app.rs`):**
```rust
pub struct HistoryState {
    pub commit_list: Vec<CommitInfo>,
    pub selected_commit_index: Option<usize>, // <-- ADD THIS
    pub selected_commit_hash: Option<String>,
    // ...
}
```
The selected index is the source of truth. The `selected_commit_hash` can be derived from it.

### Deletions (What to Rip Out)

1.  **The `get_ui_state_hash()` method in `app.rs`:** This is a major piece of technical debt. It's an inefficient, brute-force way to detect changes. It should be replaced by a simple `dirty` flag or by letting the new `navigator`'s view model caching handle it.

2.  **`ListState` from `HistoryState`:** The `list_state` field in `app::HistoryState` must be deleted.

3.  **Duplicated `update_code_inspector_for_commit` function:** This function exists in both `src/event/history.rs` and `src/event/mod.rs`. The one in `mod.rs` should be deleted.

4.  **Unused `Config` system:** The entire `src/config.rs` module is unused. It should be deleted to reduce noise. The theme can remain hardcoded in `src/theme.rs` for now.

---

## What to Absolutely Avoid

-   **DO NOT store UI widget state (like `ListState`) in the main application state.** This creates a tight coupling between your application logic and the `ratatui` library and is a primary source of state synchronization bugs. Derive UI state at render time.
-   **DO NOT write complex change-detection logic (like `get_ui_state_hash`).** This is a sign of flawed state management. A simple boolean flag set when state changes is sufficient.
-   **DO NOT leave old code paths behind conditional flags.** When a feature is refactored, the old code must be deleted. Leaving it in creates confusion and technical debt.
