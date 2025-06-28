# Git Lineage

A TUI (Terminal User Interface) application for exploring Git file history with line-level "time travel" capabilities.

## Features

- Interactive three-panel layout for navigating files, viewing commit history, and inspecting code
- Line-level blame information with jump-to-commit functionality
- "Next Change" feature to trace line evolution through history
- Syntax highlighting for code inspection
- Fuzzy file search
- Async operations to prevent UI blocking

## Usage

### Interactive Mode (Default)

```bash
cargo run
# or
git-lineage
```

### Screenshot Mode (Visual Testing)

Generate text-based screenshots of UI configurations for testing and documentation:

```bash
# Generate screenshot from JSON config
cargo run -- screenshot --config test_configs/default.json --output screenshot.txt

# Specify terminal dimensions
cargo run -- screenshot --config test_configs/default.json --width 100 --height 30

# Output to stdout
cargo run -- screenshot --config test_configs/default.json
```

## Visual Testing System

The project includes a comprehensive visual testing system that allows you to:

1. **Configure UI states via JSON** - Define file trees, commit histories, panel focus, and more
2. **Generate text screenshots** - Render any configuration to a text file for inspection
3. **Test different scenarios** - Validate UI behavior across various states

### Example JSON Configuration

```json
{
  "active_panel": "History",
  "file_tree": [
    {
      "name": "src",
      "path": "src",
      "is_dir": true,
      "git_status": null,
      "children": [
        {
          "name": "main.rs",
          "path": "src/main.rs",
          "is_dir": false,
          "git_status": "M",
          "children": []
        }
      ]
    }
  ],
  "selected_file_path": "src/main.rs",
  "commit_list": [
    {
      "hash": "a1b2c3d4e5f6789012345678901234567890abcd",
      "short_hash": "a1b2c3d",
      "author": "John Doe",
      "date": "2 hours ago",
      "subject": "Add new feature"
    }
  ],
  "selected_commit_index": 0,
  "current_content": [
    "fn main() {",
    "    println!(\"Hello, world!\");",
    "}"
  ],
  "cursor_line": 1,
  "status_message": "Ready",
  "is_loading": false
}
```

### Available Test Configurations

- `test_configs/default.json` - Basic three-panel layout with file navigator focused
- `test_configs/history_panel.json` - History panel focused with multiple commits
- `test_configs/search_mode.json` - File navigator in search mode
- `test_configs/loading_state.json` - Loading state during async operations

## Architecture

The project follows clean architecture principles with clear separation of concerns:

- **main.rs** - Application orchestrator and CLI handling
- **app.rs** - Central state model (single source of truth)
- **ui.rs** - Pure rendering logic (View layer)
- **event.rs** - Input handling and event processing (Controller)
- **async_task.rs** - Background worker for expensive Git operations
- **git_utils.rs** - Git operations facade using `gix`
- **screenshot.rs** - Visual testing system
- **test_config.rs** - JSON configuration structures

## Development

```bash
# Build the project
cargo build

# Run tests
cargo test

# Check code
cargo check

# Generate screenshots for all test configs
for config in test_configs/*.json; do
    cargo run -- screenshot --config "$config" --output "screenshots/$(basename "$config" .json).txt"
done
```

## Dependencies

- **ratatui** - TUI framework
- **crossterm** - Terminal backend
- **gix** - Pure Rust Git implementation
- **tokio** - Async runtime
- **clap** - Command line argument parsing
- **serde** - JSON serialization/deserialization
- **syntect** - Syntax highlighting
- **tui-tree-widget** - Tree view widget
- **similar** - Text diffing
- **fuzzy-matcher** - Fuzzy search