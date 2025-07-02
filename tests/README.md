# Headless Testing for Git Lineage

This directory contains example test scripts for the headless testing facility.

## Usage

Run a test script:
```bash
cargo run -- test --script tests/basic_navigation.test --verbose
```

Run with a custom initial configuration:
```bash
cargo run -- test --script tests/search_functionality.test --config config.json
```

Set custom settlement timeout:
```bash
cargo run -- test --script tests/timing_control.test --settle-timeout 10
```

## Test File Format

Test files use a simple text format where each line represents a command:

### Commands

- `key:<keyname>` - Send a key event
  - Examples: `key:tab`, `key:enter`, `key:up`, `key:down`, `key:esc`
- `char:<c>` - Send a character
  - Examples: `char:a`, `char:/`, `char:1`
- `wait` or `settle` - Wait for all async tasks to settle
- `wait:<ms>` - Wait for specific duration in milliseconds
- `assert:<property>:<value>` - Assert application state
- `# comment` - Comments (ignored)
- `immediate` - Set immediate mode (don't wait between commands)
- `settle_mode` - Set settle mode (wait between commands)
- `no_initial_settle` - Don't wait for initial settlement before first command

### Supported Key Names

- `tab`, `enter`, `esc`/`escape`, `space`
- `up`, `down`, `left`, `right`
- `home`, `end`, `pageup`, `pagedown`
- `backspace`, `delete`
- Single characters: `a`, `z`, `1`, `/`, etc.

### Assertion Properties

- `active_panel` - Current active panel (`Navigator`, `History`, `Inspector`)
- `should_quit` - Whether app should quit (`true`/`false`)
- `is_loading` - Whether app is loading (`true`/`false`)
- `status_contains` - Whether status message contains text
- `cursor_line` - Current cursor line number (0-based)
- `content_lines` - Number of lines in inspector content
- `has_file_selected` - Whether a file is selected (`true`/`false`)
- `visible_files_count` - Number of files visible in navigator
- `is_searching` - Whether navigator is in search mode (`true`/`false`)
- `search_query` - Current search query string

## Example Tests

### Basic Navigation (`basic_navigation.test`)
Tests fundamental navigation and panel switching.

### Search Functionality (`search_functionality.test`)
Tests search mode entry, typing, navigation, and exit.

### Timing Control (`timing_control.test`)
Demonstrates immediate mode, custom waits, and settlement control.

## Settlement Behavior

By default, the test runner waits for async operations to complete after each command. This ensures the application reaches a stable state before proceeding.

- **Settle Mode** (default): Wait for operations to complete after each command
- **Immediate Mode**: Execute commands without waiting (useful for testing rapid input)
- **Manual Control**: Use `wait` or `settle` commands to control timing explicitly

## Creating Test Scripts

1. Start with basic navigation to set up the application state
2. Use assertions to verify expected behavior
3. Use `immediate` mode for testing rapid input sequences
4. Use explicit `wait` commands when you need precise timing control
5. Always end with `key:q` and `assert:should_quit:true` for clean exit

## Debugging

Use the `--verbose` flag to see detailed logging of test execution:

```bash
cargo run -- test --script tests/debug.test --verbose
```

This will show:
- Each command being executed
- Async task results being processed
- Assertion results
- Settlement timing information