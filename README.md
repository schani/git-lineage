# Git Lineage

A TUI (Terminal User Interface) application for exploring Git file history with line-level "time travel" capabilities.

[Loom](https://www.loom.com/share/f9a7db55217e4f21aa294de038cd08dc)

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

## Keybindings

### Global Navigation
- **Tab** / **Shift+Tab** - Cycle between panels (forward/backward)
- **1** - Focus File Navigator panel (left)
- **2** - Focus Commit History panel (middle)
- **3** - Focus Code Inspector panel (right)
- **[** / **]** - Navigate to older/younger commit (works from any panel)
- **q** / **Esc** - Quit application

### File Navigator Panel
- **↑** / **↓** - Navigate up/down through files
- **→** / **←** / **Enter** - Expand/collapse directories
- **Enter** on a file - Switch to Code Inspector panel
- **/** - Start search mode
- **Esc** - Exit search mode

### Commit History Panel
- **↑** / **↓** - Navigate through commit history
- **Enter** - Switch to Code Inspector panel

### Code Inspector Panel
- **↑** / **↓** / **PageUp** / **PageDown** - Navigate up/down
- **Home** / **End** - Go to first/last line
- **g** / **G** - Go to top/bottom of file

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

### Command Execution Mode (Automated Testing)

Execute commands against configurations and get the resulting state:

```bash
# Execute a command and save result
cargo run -- execute --config test_configs/default.json --command "next_panel" --output result.json

# Execute with screenshot generation
cargo run -- execute --config test_configs/default.json --command "toggle_diff" --screenshot --output result.json

# Available commands include:
# Panel navigation: next_panel, previous_panel
# File navigator: up, down, expand, collapse, select_file, start_search, search:a, end_search
# History: history_up, history_down, select_commit
# Inspector: inspector_up, inspector_down, page_up, page_down, toggle_diff, goto_top, goto_bottom
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

## Scripted UI Testing

The project includes a scripted testing system that allows you to write test scripts for complex user interactions and automatically verify UI behavior.

### Test Script Format

Test scripts use a simple command language:

```
# Comments start with #
# Commands are executed sequentially

# Send key events
key:/                   # Press the '/' key
key:s                   # Press the 's' key
key:Escape              # Press the Escape key
key:Enter               # Press Enter
key:Tab                 # Press Tab

# Take screenshots for verification
screenshot:before.txt   # Capture current UI state
screenshot:after.txt    # Capture UI state after interactions

# Wait for operations to complete
wait                    # Wait for async operations to settle
wait:500ms             # Wait for specific duration

# Assert application state
assert:active_panel:Navigator  # Verify current panel focus
```

### Running Scripted Tests

**Verify Mode (Default)** - Compares screenshots against existing files:
```bash
# Default behavior: verify screenshots match existing files
cargo run -- test --script tests/search_behavior.test

# Explicit verify mode (same as default)
cargo run -- test --script tests/search_behavior.test --verbose
```

**Overwrite Mode** - Creates/updates screenshot files:
```bash
# Generate new screenshots (use when creating new tests)
cargo run -- test --script tests/search_behavior.test --overwrite

# Update screenshots after UI changes
cargo run -- test --script tests/search_behavior.test --overwrite --verbose
```

### Test Modes Explained

1. **Verify Mode (Default)**
   - Compares current UI output with existing screenshot files
   - ✅ Passes if screenshots match exactly
   - ❌ Fails if content differs or files don't exist
   - Use for continuous integration and regression testing

2. **Overwrite Mode** (`--overwrite` flag)
   - Always writes new screenshot files
   - Use when creating new tests or updating after intentional UI changes
   - ⚠️ Only use when you've verified the UI changes are correct

### Example Test Script

```
# Test search functionality
# File: tests/search_label_immediate.test

# Start with normal view
screenshot:before_search.txt

# Press '/' to enter search mode - should immediately show "Search:" label
key:/
screenshot:after_slash.txt

# Type a character - should still show "Search:" label with content
key:s
screenshot:after_typing.txt

# Exit search mode
key:Escape
screenshot:after_escape.txt
```

### Integration with Continuous Integration

The scripted tests are designed to run automatically in CI environments:

```bash
# Run all scripted tests in verify mode (for CI)
find tests -name "*.test" -exec cargo run -- test --script {} \;

# Generate new screenshots during development
find tests -name "*.test" -exec cargo run -- test --script {} --overwrite \;
```

Since verify mode is the default, scripted tests can be easily integrated into your test suite and will fail if the UI behavior changes unexpectedly.

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

# Update rendering test expected outputs
./update_test_screenshots.sh
```

### Updating Rendering Test Screenshots

The project includes a script to regenerate all expected outputs for rendering tests:

```bash
./update_test_screenshots.sh
```

This script:
- 🔄 Finds all `*.json` test configuration files in `tests/rendering_tests/`
- 📸 Generates screenshots using the exact dimensions expected by tests (80x25)
- ✅ Updates all `*.expected.txt` files with the current UI output
- 🧪 Runs the rendering tests to verify they pass
- ✨ Reports success or failure

**When to use this script:**
- After making UI layout changes that affect test screenshots
- When adding new rendering test cases
- After updating the terminal rendering logic
- If rendering tests fail due to outdated expected outputs

**Important:** Always review the generated screenshots before committing to ensure the changes are intentional and correct.

### Pre-commit Hook

The repository includes a pre-commit hook that automatically runs `cargo test` before each commit to ensure code quality. The hook:

- Runs all tests with `cargo test --quiet`
- Prevents commits if any tests fail
- Shows clear success/failure messages

The hook is automatically installed at `.git/hooks/pre-commit` and is executable. If tests fail, fix them before committing:

```bash
# If tests fail during commit:
cargo test  # Fix any failing tests
git commit  # Try again
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
