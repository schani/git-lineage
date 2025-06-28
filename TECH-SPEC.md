## **Specification: Git Lineage TUI Utility (v3 - Architectural Blueprint)**

### 1. Project Overview

*   **Project Name:** Git Lineage
*   **Elevator Pitch:** An interactive TUI for exploring a file's complete history within a Git repository, enabling line-level "time travel" to see exactly when and how each line of code evolved.
*   **Core Philosophy:** Provide maximum context in a single, persistent view. The application state is interconnected across panels, creating a fluid and intuitive exploration experience.

### 2. Core Architecture & Module Organization

The project will be organized into distinct modules within the `src/` directory to enforce separation of concerns.

*   `main.rs`: Application entry point and main event loop orchestrator.
*   `app.rs`: The central state model ("single source of truth").
*   `ui.rs`: The rendering logic (The "View").
*   `event.rs`: User input and event handling (The "Controller").
*   `git_utils.rs`: A facade module for all `gix` interactions.
*   `async_task.rs`: Logic for the background worker thread.
*   `config.rs`: Application configuration (colors, keybindings).
*   `error.rs`: Custom error types and `Result` aliases.

### 3. Module Specifications

#### **3.1. `main.rs` - The Orchestrator**
*   **Responsibilities:**
    *   Initialize the `tokio` async runtime.
    *   Set up and restore the terminal using `crossterm`.
    *   Instantiate the central `App` struct from `app.rs`.
    *   Create `tokio::sync::mpsc` channels for communication between the UI thread and the async worker.
    *   Spawn the background worker task defined in `async_task.rs`.
    *   Run the main application loop, which will:
        1.  Render the current `App` state by calling `ui::draw()`.
        2.  Poll for user input (`crossterm::event`) and messages from the async worker channel.
        3.  Delegate event handling to `event::handle_event()`.
        4.  Update the `App` state with results received from the async worker.

#### **3.2. `app.rs` - The State Model**
*   **Responsibilities:** Define all application state. This module contains no logic, only data structures.
*   **Key Structs/Enums:**
    *   `pub struct App`: The central state container holding an instance of `gix::Repository`, the active panel enum, and the state for each panel.
    *   `pub enum PanelFocus { Navigator, History, Inspector }`.
    *   State for Panel 1: `tui_tree_widget::TreeState`, `Vec<FileTreeNode>`.
    *   State for Panel 2: `ratatui::widgets::ListState`, `Vec<CommitInfo>`.
    *   State for Panel 3: Scroll positions, cursor position, `Option<gix::Blame>`, `Vec<String>` for content.
    *   UI State: `status_message: String`, `is_loading: bool`.

#### **3.3. `ui.rs` - The View**
*   **Responsibilities:** All rendering logic. Reads from the `App` state but does not modify it.
*   **Key Functions:**
    *   `pub fn draw(frame: &mut Frame, app: &App)`: Main entry point for drawing. Sets up the main layout and calls sub-routines for each panel.
    *   Helper functions (`draw_file_navigator`, etc.) for each panel, which use widgets from `ratatui`, `tui-tree-widget`, and styled text from `syntect` to render the view.

#### **3.4. `event.rs` - The Controller**
*   **Responsibilities:** Handle all user input and translate it into state changes or async tasks.
*   **Key Functions:**
    *   `pub fn handle_event(event: Event, app: &mut App, async_sender: mpsc::Sender<Task>)`: Dispatches events based on the active panel.
    *   For fast operations (e.g., moving a cursor), it modifies the `app` struct directly.
    *   For slow operations (e.g., "Next Change"), it constructs a `Task` enum and sends it to the background worker via the `async_sender`.

#### **3.5. `git_utils.rs` - The Git Facade**
*   **Responsibilities:** Abstract all `gix` API calls into a clean, high-level interface for the rest of the application. This is the only module that should directly depend on `gix`.
*   **Key Functions:**
    *   `get_file_tree_with_status(...)`: Fetches the file list, respecting `.gitignore`.
    *   `get_commit_history_for_file(...)`: Implements the filtered `rev-walk`.
    *   `get_blame_at_commit(...)`: Retrieves `gix::Blame` for a specific file at a specific commit.
    *   `find_next_change(...)`: The core algorithm for forward-blame.

#### **3.6. `async_task.rs` - The Background Worker**
*   **Responsibilities:** Handle long-running operations without freezing the UI.
*   **Key Structs/Enums:**
    *   `pub enum Task`: Defines jobs for the worker (e.g., `FindNextChange { ... }`).
    *   `pub enum TaskResult`: Defines results sent back to the UI (e.g., `NextChangeFound { ... }`).
*   **Key Functions:**
    *   `pub async fn run_worker(...)`: An `async fn` that runs in a `tokio` task. It loops, receives `Task` messages, calls the appropriate function in `git_utils.rs`, and sends a `TaskResult` back to the UI thread.

### 4. Technical Stack & Dependencies

| Purpose                      | Crate(s)                                    | Module(s) where primarily used                               |
| ---------------------------- | ------------------------------------------- | ------------------------------------------------------------ |
| **TUI Rendering & Layout**   | `ratatui`                                   | `ui.rs`, `main.rs`                                           |
| **Terminal Backend & Input** | `crossterm`                                 | `main.rs`, `event.rs`                                        |
| **Core Git Operations**      | `gix`                                       | `git_utils.rs` (and passed around in `App`)                  |
| **File Tree Widget**         | `tui-tree-widget`                           | `app.rs` (state), `ui.rs` (rendering)                        |
| **Syntax Highlighting**      | `syntect`                                   | `ui.rs` (for styling text before rendering)                  |
| **Text Diffing**             | `similar`                                   | `git_utils.rs` (in `find_next_change`), `ui.rs` (for diff view) |
| **Async Runtime & Channels** | `tokio` + `tokio::sync::mpsc`               | `main.rs`, `async_task.rs`, `event.rs`                       |
| **Fuzzy Finding Logic**      | `fuzzy-matcher`                             | `event.rs` (for filtering logic)                             |

### 5. Data Flow Example: "Next Change"

1.  **User Action:** User presses `n` in the Code Inspector.
2.  **Event Handling (`event.rs`):** `handle_event` receives the key press. It creates a `Task::FindNextChange` variant with context from `app` and sends it to the async worker via the `mpsc::Sender`. It also sets `app.is_loading = true`.
3.  **UI Update (`ui.rs`):** On the next tick, `draw` sees `app.is_loading` is true and renders a "Searching..." message in the status bar.
4.  **Background Work (`async_task.rs`):** The `run_worker` task receives the `Task`. It calls `git_utils::find_next_change()`. This function performs the expensive `gix` operations and diffing with `similar`.
5.  **Result (`async_task.rs`):** When the operation completes, the worker creates a `TaskResult::NextChangeFound { commit_id }` and sends it back to the UI thread.
6.  **State Update (`main.rs`):** The main event loop receives the `TaskResult`. It updates `app.is_loading = false`, finds the commit in Panel 2's data, updates `app.commit_list_state` to select it, and sets `app.active_panel` to `PanelFocus::History`.
7.  **Final Render (`ui.rs`):** On the final tick, `draw` renders the new state: the "Searching..." message is gone, and the selection in the Commit History panel has moved to the correct commit.
