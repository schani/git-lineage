# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## PRIME DIRECTIVES

- Whenever you hit an important checkpoint and all the tests pass, commit all the changes with a meaningful commit description.
- Line coverage for each file must be at least 70%!
- ALL TESTS MUST PASS BEFORE COMMITTING! NO EXCEPTIONS!
- Use emojis.

## Project Overview

Git Lineage is a TUI (Terminal User Interface) application for exploring Git file history with line-level "time travel" capabilities. The application provides an interactive three-panel layout for navigating files, viewing commit history, and inspecting code.

## Architecture

The project follows a clean separation of concerns with these core modules:

- `main.rs`: Application orchestrator and main event loop
- `app.rs`: Central state model (single source of truth)
- `ui.rs`: Rendering logic (View layer)
- `event/`: Input handling and event processing module (Controller layer)
  - `mod.rs`: Module root
  - `code_inspector.rs`: Code inspector event handling
  - `file_loader.rs`: File loading events
  - `history.rs`: History panel events
  - `navigator.rs`: Navigator panel events
- `git_utils.rs`: Git operations facade using `gix`
- `async_task.rs`: Background worker for expensive operations
- `error.rs`: Error types and Result aliases
- `cli.rs`: Command-line interface handling
- `test_runner.rs`: Headless test execution with screenshot capture
- `theme.rs`: UI theming and styling

## Key Technical Decisions

### UI Layout
Three-panel persistent layout:
- Panel 1 (left top): File Navigator - tree view of Git repository files with search functionality (/)
- Panel 2 (left bottom): Commit History - chronological commits for selected file
- Panel 3 (right): Code Inspector - file content with syntax highlighting and diff view toggle (d)

### Data Flow
- Panel 1 selection drives Panel 2 and Panel 3 content
- Panel 2 selection drives Panel 3 content
- Focus cycles between panels with Tab/Shift+Tab

## Testing Infrastructure

### Overview
The project has a comprehensive testing infrastructure with multiple test types:

- **Unit Tests**: Standard Rust unit tests in `src/` modules
- **Integration Tests**: Complex scenario tests in `tests/`
- **Script Tests**: UI behavior verification with screenshot comparison
- **Rendering Tests**: Visual output verification

### Script Testing System
Script tests provide automated UI testing with screenshot-based verification:

#### Directory Structure
```
tests/scripts/
├── {test_name}/
│   ├── script           # Test commands (key presses, assertions, screenshots)
│   ├── before_X.txt     # Expected screenshot files
│   ├── after_Y.txt
│   └── ...
```

#### Test Script Format
Test scripts use a simple text format:
```text
# Comments start with #
key:down              # Send key press
key:enter             # Send key press
char:a                # Send character
screenshot:file.txt   # Take/verify screenshot
wait                  # Wait for async operations
wait:500              # Wait specific milliseconds
assert:property:value # Assert application state
immediate             # Set immediate mode (no delays)
settle_mode           # Set settle mode (with delays)
```

#### Creating New Script Tests
1. **Create test directory**: `tests/scripts/my_new_test/`
2. **Write script file**: `tests/scripts/my_new_test/script`
3. **Add to script_tests.rs**:
   ```rust
   script_test!(test_my_new_test, "my_new_test");
   ```
4. **Generate screenshots** (first time):
   ```bash
   cd tests/test-repo
   cargo run --bin git-lineage -- test --script ../../tests/scripts/my_new_test/script --overwrite
   cp *.txt ../../tests/scripts/my_new_test/
   rm *.txt
   ```

#### Running Script Tests
```bash
# Run all script tests
cargo test --test script_tests

# Run specific script test
cargo test test_search_label_immediate

# Update screenshots (development only)
cd tests/test-repo
cargo run --bin git-lineage -- test --script ../../tests/scripts/test_name/script --overwrite
```

#### Rebuilding All Script Screenshots
The `tools/rebuild-screenshots` script automates the process of regenerating all script test screenshots:

```bash
# Rebuild all script test screenshots
./tools/rebuild-screenshots
```

This tool:
- Automatically finds all script tests in `tests/scripts/`
- Runs each test with the `--overwrite` flag
- Removes old screenshots and replaces them with new ones
- Shows progress and handles errors gracefully
- Useful when UI changes affect multiple tests

#### ScriptTestDriver API
The `ScriptTestDriver` provides a reusable testing interface:

```rust
// Create driver
let driver = ScriptTestDriver::new()?;

// Run test in verify mode (CI/standard testing)
driver.run_script_test("test_name").await?;

// Run test in update mode (development)
driver.update_script_test("test_name").await?;
```

### Test Environment
- **Controlled Repository**: Tests run in `tests/test-repo` submodule
- **No Test Artifacts**: Screenshots and temporary files never committed to test-repo

### Key Testing Principles
1. **Never commit to test-repo**: Only use it as execution environment
2. **Screenshot-based verification**: Visual regression testing for UI behavior
3. **Organized test structure**: Each test is self-contained in its directory
4. **Reusable infrastructure**: ScriptTestDriver handles all boilerplate

## Critical Implementation Notes

### Async Operations
All expensive Git operations run asynchronously to prevent UI freezing, using the `async_task.rs` worker pattern with `tokio::sync::mpsc` channels. The "Next Change" feature (keyboard shortcuts 'n'/'p') is currently a TODO placeholder that will require complex Git operations when implemented.

### Git Blame Integration (TODO)
Git blame functionality is planned but not yet implemented. The codebase includes placeholder functions (`get_blame_at_commit()` in `git_utils.rs`) and data structures (`current_blame` in `InspectorState`) for future blame support. When implemented, the Code Inspector panel will show blame information for each line, with the ability to view blame data at specific commits using `blame.at_commit(<selected_commit_id>)`.

### State Management
All application state lives in the `App` struct in `app.rs`. Event handlers in `event.rs` either modify state directly (for fast operations) or send tasks to the async worker (for expensive operations).

## Module Responsibilities

- **git_utils.rs**: Only module that directly interacts with `gix` API
- **ui.rs**: Pure rendering - reads from App state but never modifies it
- **event/**: Module directory that translates user input into state changes or async tasks
- **async_task.rs**: Handles expensive Git operations without blocking UI
- **test_runner.rs**: Headless test execution with screenshot capture
- **tests/script_tests.rs**: Reusable script test driver and test definitions
- **cli.rs**: Command-line argument parsing and command dispatch
- **theme.rs**: Syntax highlighting and UI theming

The architecture enforces clear boundaries between Git operations, UI rendering, state management, and testing to maintain code clarity and testability.
