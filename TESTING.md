# Testing Strategy for Git Lineage

**Goal**: Achieve minimum 70% line coverage for every source file

**Current Coverage**: 31.25% (401/1283 lines)  
**Target Coverage**: 70%+ (900+ lines)  
**Gap to Close**: ~500 additional lines of coverage needed

## Executive Summary

This document outlines a comprehensive testing strategy to achieve 70%+ line coverage across all modules. Our approach focuses on three testing layers:

1. **Unit Tests** - Individual function and struct testing
2. **Integration Tests** - Component interaction testing  
3. **Property-Based Tests** - Edge case and state validation

## Coverage Analysis by Priority

### 🔴 Critical Priority (0% Coverage - Must Fix)

#### `src/event.rs` (289 lines, 0% coverage → Target: 70% = 202 lines)
**Challenge**: Complex event handling with async channels and state mutations  
**Strategy**: Mock-based testing with controlled input simulation

#### `src/async_task.rs` (149 lines, 0% coverage → Target: 70% = 104 lines)  
**Challenge**: Async operations, external git commands, and channel communication  
**Strategy**: Integration tests with real git repos + mocked external calls

#### `src/main.rs` (285 lines, 0% coverage → Target: 70% = 200 lines)
**Challenge**: Application lifecycle, terminal setup, and async event loops  
**Strategy**: Integration tests with headless terminal backends

### 🟡 High Priority (Low Coverage - Needs Major Work)

#### `src/app.rs` (335 lines, 18.5% coverage → Target: 70% = 234 lines)
**Gap**: +170 lines needed  
**Strategy**: State machine testing and property-based validation

#### `src/executor.rs` (462 lines, 21.0% coverage → Target: 70% = 323 lines)  
**Gap**: +290 lines needed  
**Strategy**: Command execution simulation with comprehensive error scenarios

### 🟡 Medium Priority (Moderate Coverage - Focused Improvements)

#### `src/git_utils.rs` (146 lines, 54.8% coverage → Target: 70% = 102 lines)
**Gap**: +85 lines needed  
**Strategy**: Mock git repositories and edge case testing

#### `src/command.rs` (181 lines, 48.7% coverage → Target: 70% = 127 lines)
**Gap**: +89 lines needed  
**Strategy**: Parser validation and command generation testing

### 🟢 Lower Priority (Good Coverage - Minor Gaps)

#### `src/tree.rs` (957 lines, 45.3% coverage → Target: 70% = 670 lines)
**Gap**: +536 lines needed  
**Note**: Large file, but solid foundation exists

#### `src/ui.rs` (299 lines, 73.7% coverage → Target: 70% = 209 lines)  
**Status**: ✅ Already exceeds target

## Testing Architecture & Tools

### Required Dependencies

Add to `Cargo.toml`:

```toml
[dev-dependencies]
# Existing
tokio = { version = "1.0", features = ["full", "test-util"] }
tokio-test = "0.4"

# New additions
mockall = "0.12"           # Mocking framework
proptest = "1.4"           # Property-based testing  
temp_testdir = "0.2"       # Temporary test directories
assert_matches = "1.5"     # Enhanced pattern matching
serial_test = "3.0"        # Serial test execution
fake = "2.9"               # Fake data generation
maplit = "1.0"             # Collection macros
```

### Testing Patterns

#### 1. Event Testing Pattern
```rust
#[cfg(test)]
mod event_tests {
    use super::*;
    use tokio::sync::mpsc;
    use crate::app::App;
    
    async fn create_test_app() -> (App, mpsc::Receiver<Task>) {
        let (tx, rx) = mpsc::channel(100);
        let repo = create_test_repo();
        let app = App::new(repo);
        (app, rx)
    }
    
    #[tokio::test]
    async fn test_quit_key_sets_should_quit() {
        let (mut app, _rx) = create_test_app().await;
        let event = Event::Key(KeyCode::Char('q').into());
        
        handle_event(event, &mut app, &tx).await.unwrap();
        
        assert!(app.should_quit);
    }
}
```

#### 2. Async Task Testing Pattern
```rust
#[cfg(test)]
mod async_task_tests {
    use super::*;
    use temp_testdir::TempDir;
    
    #[tokio::test]
    async fn test_commit_history_loading() {
        let temp_dir = TempDir::new().unwrap();
        setup_test_git_repo(&temp_dir);
        
        let (tx, mut rx) = mpsc::channel(10);
        let (result_tx, result_rx) = mpsc::channel(10);
        
        tokio::spawn(run_worker(rx, result_tx, temp_dir.to_string()));
        
        tx.send(Task::LoadCommitHistory { 
            file_path: "test.txt".to_string() 
        }).await.unwrap();
        
        let result = result_rx.recv().await.unwrap();
        match result {
            TaskResult::CommitHistoryLoaded { commits } => {
                assert!(!commits.is_empty());
            }
            _ => panic!("Expected CommitHistoryLoaded"),
        }
    }
}
```

#### 3. State Machine Testing Pattern  
```rust
#[cfg(test)]
mod app_state_tests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn test_panel_navigation_invariants(
            initial_panel in panel_strategy(),
            operations in vec(navigation_op_strategy(), 1..10)
        ) {
            let mut app = create_test_app();
            app.active_panel = initial_panel;
            
            for op in operations {
                apply_navigation_op(&mut app, op);
                
                // Invariants that must always hold
                prop_assert!(app.is_valid_state());
                prop_assert!(app.active_panel.is_valid());
            }
        }
    }
}
```

## Module-Specific Testing Plans

### `src/event.rs` - Event Handling (289 lines → 202 lines coverage)

**Testing Approach**: Comprehensive input simulation

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    
    mod global_keybindings {
        #[tokio::test] async fn test_quit_on_q() { /* ... */ }
        #[tokio::test] async fn test_quit_on_esc() { /* ... */ }
        #[tokio::test] async fn test_tab_next_panel() { /* ... */ }
        #[tokio::test] async fn test_shift_tab_prev_panel() { /* ... */ }
        #[tokio::test] async fn test_resize_event_handling() { /* ... */ }
    }
    
    mod navigator_events {
        #[tokio::test] async fn test_up_down_navigation() { /* ... */ }
        #[tokio::test] async fn test_expand_collapse_directories() { /* ... */ }
        #[tokio::test] async fn test_file_selection_triggers_history() { /* ... */ }
        #[tokio::test] async fn test_search_mode_activation() { /* ... */ }
        #[tokio::test] async fn test_search_input_handling() { /* ... */ }
        #[tokio::test] async fn test_search_escape() { /* ... */ }
    }
    
    mod history_events {
        #[tokio::test] async fn test_commit_navigation() { /* ... */ }
        #[tokio::test] async fn test_commit_selection() { /* ... */ }
        #[tokio::test] async fn test_empty_history_handling() { /* ... */ }
    }
    
    mod inspector_events {
        #[tokio::test] async fn test_scroll_up_down() { /* ... */ }
        #[tokio::test] async fn test_home_end_navigation() { /* ... */ }
        #[tokio::test] async fn test_previous_change_navigation() { /* ... */ }
        #[tokio::test] async fn test_cursor_bounds_validation() { /* ... */ }
    }
}
```

**Coverage Target**: 35 test functions × 6 lines avg = 210 lines covered

### `src/async_task.rs` - Async Operations (149 lines → 104 lines coverage)

**Testing Approach**: Real git repos + controlled async testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use temp_testdir::TempDir;
    use tokio_test::{assert_ok, assert_err};
    
    mod task_processing {
        #[tokio::test] async fn test_load_file_tree_success() { /* ... */ }
        #[tokio::test] async fn test_load_file_tree_invalid_path() { /* ... */ }
        #[tokio::test] async fn test_load_commit_history_success() { /* ... */ }
        #[tokio::test] async fn test_load_commit_history_no_commits() { /* ... */ }
        #[tokio::test] async fn test_load_file_content_success() { /* ... */ }
        #[tokio::test] async fn test_find_next_change_found() { /* ... */ }
        #[tokio::test] async fn test_find_next_change_not_found() { /* ... */ }
    }
    
    mod worker_lifecycle {
        #[tokio::test] async fn test_worker_startup_shutdown() { /* ... */ }
        #[tokio::test] async fn test_worker_handles_channel_close() { /* ... */ }
        #[tokio::test] async fn test_worker_error_propagation() { /* ... */ }
    }
    
    mod error_scenarios {
        #[tokio::test] async fn test_git_command_failure() { /* ... */ }
        #[tokio::test] async fn test_repository_not_found() { /* ... */ }
        #[tokio::test] async fn test_permission_denied() { /* ... */ }
        #[tokio::test] async fn test_network_timeout() { /* ... */ }
    }
}
```

**Coverage Target**: 15 test functions × 7 lines avg = 105 lines covered

### `src/main.rs` - Application Lifecycle (285 lines → 200 lines coverage)

**Testing Approach**: Headless terminal testing and lifecycle simulation

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use tokio_test::assert_ok;
    
    mod application_lifecycle {
        #[tokio::test] async fn test_app_initialization() { /* ... */ }
        #[tokio::test] async fn test_terminal_setup_teardown() { /* ... */ }
        #[tokio::test] async fn test_graceful_shutdown() { /* ... */ }
        #[tokio::test] async fn test_error_recovery() { /* ... */ }
    }
    
    mod task_result_handling {
        #[tokio::test] async fn test_file_tree_loaded_result() { /* ... */ }
        #[tokio::test] async fn test_commit_history_loaded_result() { /* ... */ }
        #[tokio::test] async fn test_file_content_loaded_result() { /* ... */ }
        #[tokio::test] async fn test_next_change_found_result() { /* ... */ }
        #[tokio::test] async fn test_error_result_handling() { /* ... */ }
    }
    
    mod cli_integration {
        #[tokio::test] async fn test_cli_arg_parsing() { /* ... */ }
        #[tokio::test] async fn test_invalid_repository_path() { /* ... */ }
        #[tokio::test] async fn test_help_command() { /* ... */ }
    }
}
```

**Coverage Target**: 12 test functions × 17 lines avg = 204 lines covered

### `src/app.rs` - Application State (335 lines → 234 lines coverage)

**Testing Approach**: State machine validation + property-based testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    mod state_management {
        #[test] fn test_app_creation() { /* ... */ }
        #[test] fn test_panel_navigation() { /* ... */ }
        #[test] fn test_search_mode_toggle() { /* ... */ }
        #[test] fn test_file_selection() { /* ... */ }
        #[test] fn test_commit_selection() { /* ... */ }
    }
    
    mod file_navigator {
        #[test] fn test_update_list_state() { /* ... */ }
        #[test] fn test_scroll_offset_bounds() { /* ... */ }
        #[test] fn test_cursor_position_validation() { /* ... */ }
        #[test] fn test_viewport_height_setting() { /* ... */ }
    }
    
    mod git_integration {
        #[test] fn test_selected_file_path() { /* ... */ }
        #[test] fn test_file_tree_from_directory() { /* ... */ }
        #[test] fn test_navigation_with_empty_tree() { /* ... */ }
    }
    
    proptest! {
        #[test] fn test_navigation_invariants(ops in vec(nav_op(), 1..20)) {
            let mut app = App::new(test_repo());
            for op in ops {
                apply_op(&mut app, op);
                prop_assert!(app.is_valid_state());
            }
        }
    }
}
```

**Coverage Target**: 15 test functions × 12 lines avg = 180 lines covered

### `src/executor.rs` - Command Execution (462 lines → 323 lines coverage)

**Testing Approach**: Command simulation and comprehensive error testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use fake::{Fake, Faker};
    
    mod command_execution {
        #[test] fn test_execute_quit() { /* ... */ }
        #[test] fn test_execute_navigation() { /* ... */ }
        #[test] fn test_execute_search() { /* ... */ }
        #[test] fn test_execute_sequence() { /* ... */ }
        #[test] fn test_execute_invalid_command() { /* ... */ }
    }
    
    mod state_mutations {
        #[test] fn test_panel_changes() { /* ... */ }
        #[test] fn test_search_state_changes() { /* ... */ }
        #[test] fn test_cursor_movements() { /* ... */ }
        #[test] fn test_scroll_operations() { /* ... */ }
    }
    
    mod error_handling {
        #[test] fn test_invalid_direction() { /* ... */ }
        #[test] fn test_out_of_bounds_navigation() { /* ... */ }
        #[test] fn test_empty_file_tree() { /* ... */ }
        #[test] fn test_malformed_commands() { /* ... */ }
    }
    
    proptest! {
        #[test] fn test_command_execution_safety(
            cmd in command_strategy(),
            state in app_state_strategy()
        ) {
            let mut app = state;
            let result = execute_command(&mut app, cmd);
            prop_assert!(app.is_valid_state());
        }
    }
}
```

**Coverage Target**: 20 test functions × 15 lines avg = 300 lines covered

### Additional Modules

#### `src/git_utils.rs` (+85 lines needed)
- Mock git repositories with various states
- Edge case testing (empty repos, corrupted history, etc.)
- Error condition simulation (permission denied, network issues)

#### `src/command.rs` (+89 lines needed)  
- Parser edge cases and malformed input
- Command generation and serialization
- Complex command sequence validation

#### `src/tree.rs` (+536 lines needed)
- Property-based tree operations
- Git status integration scenarios
- Large file tree performance testing

## Implementation Timeline

### Phase 1: Foundation (Week 1)
1. Set up testing dependencies and infrastructure
2. Create test utilities and helper functions
3. Implement event testing framework

### Phase 2: Critical Coverage (Week 2)  
1. Complete `src/event.rs` testing (0% → 70%)
2. Complete `src/async_task.rs` testing (0% → 70%)
3. Complete `src/main.rs` testing (0% → 70%)

### Phase 3: High Priority (Week 3)
1. Expand `src/app.rs` testing (18% → 70%)
2. Expand `src/executor.rs` testing (21% → 70%)

### Phase 4: Remaining Modules (Week 4)
1. Complete remaining modules to 70%
2. Property-based testing implementation
3. Performance and integration testing

## Success Metrics

- **Line Coverage**: 70%+ per file (measured with `cargo tarpaulin`)
- **Branch Coverage**: 60%+ overall
- **Test Execution Time**: <30 seconds for full suite
- **Flakiness**: <1% test failure rate on clean runs
- **Maintainability**: New features require tests before merge

## Continuous Integration

```yaml
# .github/workflows/test.yml
- name: Run tests with coverage
  run: |
    cargo tarpaulin --all-features --workspace --timeout 120 \
    --out Html --output-dir coverage \
    --fail-under 70 --per-file
```

## Tools and Commands

```bash
# Generate coverage report
cargo tarpaulin --all-features --workspace --out Html

# Run tests for specific module  
cargo test app::tests

# Run property-based tests with more cases
PROPTEST_CASES=10000 cargo test

# Generate documentation for test helpers
cargo doc --document-private-items --open
```

---

**Next Steps**: 
1. Add testing dependencies to `Cargo.toml`
2. Create test infrastructure and utilities
3. Begin with `src/event.rs` as highest impact/lowest complexity
4. Implement coverage monitoring in CI/CD pipeline

This strategy will systematically bring every module to 70%+ coverage while building a robust, maintainable test suite that catches regressions and validates correctness.