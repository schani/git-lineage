# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## PRIME DIRECTIVES

- Whenever you hit an important checkpoint and all the tests pass, commit all the changes with a meaningful commit description.
- Line coverage for each file must be at least 70%!
- ALL TESTS MUST PASS BEFORE COMMITTING! NO EXCEPTIONS!
- Use emojis.

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
The "Next Change" feature requires complex Git operations that must run asynchronously to prevent UI freezing. Use the `async_task.rs` worker pattern with `tokio::sync::mpsc` channels.

### Git Blame Integration
The Code Inspector panel shows blame information for each line. When viewing historical commits, use `blame.at_commit(<selected_commit_id>)` to get blame data for that specific point in time.

### State Management
All application state lives in the `App` struct in `app.rs`. Event handlers in `event.rs` either modify state directly (for fast operations) or send tasks to the async worker (for expensive operations).

## Module Responsibilities

- **git_utils.rs**: Only module that directly interacts with `gix` API
- **ui.rs**: Pure rendering - reads from App state but never modifies it
- **event.rs**: Translates user input into state changes or async tasks
- **async_task.rs**: Handles expensive Git operations without blocking UI
- **test_runner.rs**: Headless test execution with screenshot capture
- **tests/script_tests.rs**: Reusable script test driver and test definitions

The architecture enforces clear boundaries between Git operations, UI rendering, state management, and testing to maintain code clarity and testability.
