# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## PRIME DIRECTIVES

- Whenever you hit an important checkpoint and all the tests pass, commit all the changes with a meaningful commit description.  Use emojis.

## Project Overview

Git Lineage is a TUI (Terminal User Interface) application for exploring Git file history with line-level "time travel" capabilities. The application provides an interactive three-panel layout for navigating files, viewing commit history, and inspecting code with blame information.

## Architecture

The project follows a clean separation of concerns with these core modules:

- `main.rs`: Application orchestrator and main event loop
- `app.rs`: Central state model (single source of truth)
- `ui.rs`: Rendering logic (View layer)
- `event.rs`: Input handling and event processing (Controller layer)
- `git_utils.rs`: Git operations facade using `gix`
- `async_task.rs`: Background worker for expensive operations
- `config.rs`: Application configuration
- `error.rs`: Error types and Result aliases

## Key Technical Decisions

### UI Layout
Three-panel persistent layout:
- Panel 1 (left top): File Navigator - tree view of Git repository files
- Panel 2 (left bottom): Commit History - chronological commits for selected file
- Panel 3 (right): Code Inspector - file content with Git blame gutter

### Data Flow
- Panel 1 selection drives Panel 2 and Panel 3 content
- Panel 2 selection drives Panel 3 content
- Focus cycles between panels with Tab/Shift+Tab

### Core Features
- **Previous Change (`p` key)**: Jump to the blame commit for current line
- **Next Change (`n` key)**: Find next modification to current line (async operation)
- **Diff Toggle (`d` key)**: Switch between full file and diff view

### Technology Stack
- **TUI Framework**: `ratatui` with `crossterm` backend
- **Git Operations**: `gix` (pure Rust Git implementation)
- **Async Runtime**: `tokio` with `mpsc` channels for UI/worker communication
- **Syntax Highlighting**: `syntect`
- **Tree Widget**: `tui-tree-widget`
- **Text Diffing**: `similar` crate
- **Fuzzy Finding**: `fuzzy-matcher`

## Development Commands

Since this is a new Rust project, standard Cargo commands will apply:

```bash
# Build the project
cargo build

# Run the application
cargo run

# Run tests
cargo test

# Run a specific test
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

## Critical Implementation Notes

### Async Operations
The "Next Change" feature requires complex Git operations that must run asynchronously to prevent UI freezing. Use the `async_task.rs` worker pattern with `tokio::sync::mpsc` channels.

### Git Blame Integration
The Code Inspector panel shows blame information for each line. When viewing historical commits, use `blame.at_commit(<selected_commit_id>)` to get blame data for that specific point in time.

### State Management
All application state lives in the `App` struct in `app.rs`. Event handlers in `event.rs` either modify state directly (for fast operations) or send tasks to the async worker (for expensive operations).

### Performance Considerations
- Use `gix::Repository::rev_walk().all().path_filter()` for efficient commit history filtering
- Cache blame results to avoid repeated Git operations
- Implement proper scroll state management for large files and long commit histories

## Module Responsibilities

- **git_utils.rs**: Only module that directly interacts with `gix` API
- **ui.rs**: Pure rendering - reads from App state but never modifies it
- **event.rs**: Translates user input into state changes or async tasks
- **async_task.rs**: Handles expensive Git operations without blocking UI

The architecture enforces clear boundaries between Git operations, UI rendering, and state management to maintain code clarity and testability.
