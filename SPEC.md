## **Specification: Git Lineage TUI Utility**

### 1. Project Overview

*   **Project Name:** Git Lineage
*   **Elevator Pitch:** An interactive TUI for exploring a file's complete history within a Git repository, enabling line-level "time travel" to see exactly when and how each line of code evolved.
*   **Core Philosophy:** Provide maximum context in a single, persistent view. The state of all panels is interconnected, creating a fluid and intuitive exploration experience without modal dialogs or screen switching.

### 2. UI Layout & Core Functionality

The application will use a persistent three-panel layout rendered in the terminal.

```
+------------------+------------------------------------------+
|  [PANEL 1]       |                                          |
|  File Navigator  |                                          |
|  (Focus: 1)      |                                          |
|                  |                                          |
|                  |          [PANEL 3]                       |
+------------------+          Code Inspector                  |
|  [PANEL 2]       |          (File Content + Blame)          |
|  Commit History  |                                          |
|  (Focus: 2)      |          (Focus: 3)                      |
|                  |                                          |
|                  |                                          |
+------------------+------------------------------------------+
| [STATUS BAR] Help text, current line info, async status     |
+-------------------------------------------------------------+
```

*   **Layout:** Left side is vertically split (approx. 50/50 or configurable), taking up ~35% of the screen width. The right side is a single large panel taking up ~65% of the width.
*   **Focus Management:** The user can cycle focus between the three panels using `Tab` and `Shift+Tab`. The active panel will have a highlighted border.
*   **Data Flow:**
    *   Selection in **Panel 1** (File Navigator) drives the content of **Panel 2** (Commit History) and **Panel 3** (Code Inspector).
    *   Selection in **Panel 2** (Commit History) drives the content of **Panel 3** (Code Inspector).

### 3. Panel Specifications

#### **Panel 1: File Navigator**

*   **Content:** A tree view of all files and directories tracked by the Git repository.
    *   Displays file/directory names.
    *   Indicates Git status (e.g., `M` for modified, `A` for added, `?` for untracked).
    *   Untracked files should be visually distinct (e.g., different color).
*   **Data Source:** `gix::Repository::worktree()` combined with status information from `gix::Repository::status()`.
*   **Interactivity:**
    *   `↑`/`↓` arrows: Navigate the list.
    *   `→`/`←` arrows: Expand/collapse directories in the tree view.
    *   `/`: Enter fuzzy-find mode to filter the file list.
    *   `Enter`: Can be used to confirm selection (though navigation itself is sufficient to trigger updates).
*   **State Tracking:**
    *   A tree structure representing the file system.
    *   `tui-tree-widget::TreeState` or similar to manage selection and expansion.
    *   Current search/filter query.

##### Search

- When I press `/` it should go into search string entry mode (that's already implemented)
- When I start typing it should filter down the tree with fuzzy search
- When I press Enter, it should focus back on the panel, but the search should stay active
- When I press Esc when entering the search, it should clear the search string and focus back on the panel

#### **Panel 2: Commit History**

*   **Content:** A chronological list (newest first) of all commits that have modified the file currently selected in Panel 1.
    *   **Per-commit line format:** `[short_hash] [relative_date] [author_name] <commit_subject>`
*   **Data Source:** A `gix` rev-walk filtered by the selected file path.
    *   `repo.rev_walk().all().path_filter(|path, is_dir| ...)` provides an efficient way to get the relevant commit history.
*   **Interactivity:**
    *   `↑`/`↓` arrows: Navigate the commit list.
    *   As the selection changes, Panel 3 updates instantly.
*   **State Tracking:**
    *   `Vec<CommitInfo>` holding the data for the current file's history.
    *   `ratatui::widgets::ListState` to manage the selection.

#### **Panel 3: Code Inspector**

*   **Content:** The central view showing the file's content.
    *   **Syntax Highlighting:** Applied based on the file extension.
    *   **Gutter:** A column on the left displays `git blame` information for each line.
        *   Format: `[short_hash] [author] [date]`
    *   The content and blame info always correspond to the commit selected in Panel 2. If no commit is selected, it shows the `HEAD` version.
*   **Data Source:**
    *   **Blame:** `gix::Repository::blame()` for the selected file path. To view a historical state, use `blame.at_commit(<selected_commit_id>)`.
    *   **File Content:** Retrieve the blob object from the `gix` tree corresponding to the selected commit and file path.
*   **Interactivity:**
    *   Standard text navigation (`↑`/`↓`, `PageUp`/`PageDown`, `Home`/`End`, `g`/`G`).
    *   **`p` (Previous Change):** Triggers the "jump to blame commit" action.
    *   **`n` (Next Change):** Triggers the "find next modification" action.
    *   **`d` (Toggle Diff):** Toggles the view between full file content and a diff view showing only the changes from the selected commit.
*   **State Tracking:**
    *   `gix::Blame` result for the current view.
    *   `Vec<String>` for the file content.
    *   Vertical and horizontal scroll state.
    *   Cursor position (line and column).

### 4. Key Feature Implementation Algorithms

#### **4.1. "Previous Change" (Jump to Blame Commit)**

1.  **Trigger:** User presses `p` in Panel 3.
2.  **Input:** The current cursor line number in Panel 3.
3.  **Action:**
    a. Look up the line in the cached `gix::Blame` result to get the blame `hunk`.
    b. Extract the commit ID (`C_blame`) from the hunk.
    c. Find the index of `C_blame` in the `Vec<CommitInfo>` currently displayed in Panel 2.
    d. Update Panel 2's `ListState` to select that index.
    e. Set the active focus to Panel 2.

#### **4.2. "Next Change" (Forward Blame)**

This is the most complex operation and may be asynchronous to avoid UI lockup.

1.  **Trigger:** User presses `n` in Panel 3.
2.  **Input:** The current cursor line number (`L`) and its blame commit ID (`C_blame`).
3.  **Action (Async Task):**
    a. Use `gix` to create a reversed rev-walk starting from `HEAD` and ending just after `C_blame`: `repo.rev_walk().all().from(repo.head_id()?).to(C_blame)`. This gives you a list of commits to check, from newest to oldest.
    b. Iterate backwards through this list of commits (i.e., forward in chronological time from `C_blame`).
    c. For each commit `C_next` in the sequence:
        i. Find its parent commit, `C_parent`.
        ii. Get the tree objects for the file path from both `C_parent` and `C_next`.
        iii. Use a diffing algorithm (e.g., using the `similar` crate) on the content of the two file blobs.
        iv. Analyze the diff changes to see if the original line `L` (accounting for line number shifts from previous changes in the file) is part of a modification or deletion hunk.
        v. If it is, `C_next` is the target commit. Break the loop.
4.  **Completion:**
    a. If a target commit is found, find its index in Panel 2's list, update the selection, and switch focus to Panel 2.
    b. If the loop completes with no match, display a message in the status bar: "No subsequent changes to this line found."

### 5. Technical Specification & Dependencies

*   **Language:** Rust (Latest Stable Edition)
*   **TUI Framework:** `ratatui` - The community-maintained fork of `tui-rs`.
*   **Terminal Backend:** `crossterm` - For terminal manipulation, input handling.
*   **Git Implementation:** `gix` - The pure Rust, high-performance Git implementation.
*   **Syntax Highlighting:** `syntect` - For parsing sublime-syntax definitions and highlighting code.
*   **Fuzzy Finding:** `fuzzy-matcher` or similar for file searching.
*   **Async Runtime:** `tokio` or `async-std` to run expensive Git operations (like "Next Change") in the background without freezing the UI.

### 6. Application State Management

A central `App` struct will manage the entire application state.

```rust
// Simplified Example
use ratatui::widgets::{ListState, TreeState};
use gix::Repository;

struct App {
    repo: Repository,
    active_panel: PanelFocus,

    // Panel 1 State
    file_tree: Vec<FileTreeNode>,
    file_tree_state: TreeState,

    // Panel 2 State
    commit_list: Vec<CommitInfo>,
    commit_list_state: ListState,

    // Panel 3 State
    current_blame: Option<gix::Blame<'repo>>,
    current_content: Vec<String>,
    inspector_scroll_state: (u16, u16),

    // Other state
    status_message: String,
    is_loading: bool,
}
```

### 7. Event Loop

The main loop will follow a standard `ratatui` pattern:

1.  Start a background thread for any long-running (async) tasks.
2.  Enter the main loop:
    a. `draw()` the current `App` state to the terminal buffer.
    b. `crossterm::event::poll()` for a user input event with a short timeout.
    c. If an event occurs, `handle_event(app, event)`.
    d. Check for results from the async task channel and update `App` state accordingly.
    e. If `app.should_quit` is true, break the loop.
3.  Restore the terminal.

### 8. V2 / Future Enhancements

*   **Diff View:** A fully-featured, side-by-side or unified diff view in Panel 3.
*   **Branch Navigation:** An additional pop-up or panel to view and check out other branches.
*   **Staging:** Ability to stage/unstage hunks or lines directly from the inspector view.
*   **Configuration File:** Allow users to customize colors, layout splits, and keybindings.
*   **Search:** In-file text search in Panel 3.
