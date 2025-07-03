use crate::app::App;
use crate::async_task::{Task, TaskResult};
use crate::event::handle_event;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Test file format for headless testing
///
/// Format is a simple text file where each line represents a command:
/// - `key:<keyname>` - Send a key event (e.g., `key:tab`, `key:enter`, `key:q`)
/// - `char:<c>` - Send a character (e.g., `char:a`, `char:/`)
/// - `wait` - Wait for all async tasks to settle
/// - `wait:<ms>` - Wait for specific duration in milliseconds
/// - `settle` - Wait for background tasks to complete (same as `wait`)
/// - `assert:<property>:<value>` - Assert application state
/// - `# comment` - Comments (ignored)
/// - `immediate` - Set immediate mode (don't wait between commands)
/// - `settle_mode` - Set settle mode (wait between commands)
///
/// Examples:
/// ```text
/// # Navigate to a file and view its content
/// key:down
/// key:down
/// key:enter
/// assert:active_panel:Inspector
/// key:q
/// ```

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCommand {
    pub command_type: CommandType,
    pub value: String,
    pub immediate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandType {
    Key,
    Char,
    Wait,
    Assert,
    SetImmediate,
    SetSettle,
    Screenshot,
}

#[derive(Debug, Clone)]
pub struct TestScript {
    pub commands: Vec<TestCommand>,
    pub initial_settle: bool,
}

#[derive(Debug, Clone)]
pub struct TestRunner {
    pub script: TestScript,
    pub current_command: usize,
    pub immediate_mode: bool,
    pub max_settle_time: Duration,
    pub overwrite_mode: bool,
    pub screenshot_base_dir: Option<std::path::PathBuf>,
}

impl TestRunner {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Self::from_string(&content)
    }

    pub fn from_string(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut commands = Vec::new();
        let mut immediate_mode = false;
        let mut initial_settle = true;

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse special directives
            if line == "immediate" {
                immediate_mode = true;
                continue;
            }
            if line == "settle_mode" {
                immediate_mode = false;
                continue;
            }
            if line == "no_initial_settle" {
                initial_settle = false;
                continue;
            }

            // Parse commands
            let command = if line.starts_with("key:") {
                TestCommand {
                    command_type: CommandType::Key,
                    value: line[4..].to_string(),
                    immediate: immediate_mode,
                }
            } else if line.starts_with("char:") {
                TestCommand {
                    command_type: CommandType::Char,
                    value: line[5..].to_string(),
                    immediate: immediate_mode,
                }
            } else if line == "wait" || line == "settle" {
                TestCommand {
                    command_type: CommandType::Wait,
                    value: String::new(),
                    immediate: false, // Wait commands always wait
                }
            } else if line.starts_with("wait:") {
                TestCommand {
                    command_type: CommandType::Wait,
                    value: line[5..].to_string(),
                    immediate: false,
                }
            } else if line.starts_with("assert:") {
                TestCommand {
                    command_type: CommandType::Assert,
                    value: line[7..].to_string(),
                    immediate: immediate_mode,
                }
            } else if line.starts_with("screenshot:") {
                TestCommand {
                    command_type: CommandType::Screenshot,
                    value: line[11..].to_string(),
                    immediate: immediate_mode,
                }
            } else {
                return Err(
                    format!("Invalid command on line {}: {}", line_num + 1, line).into(),
                );
            };

            commands.push(command);
        }

        Ok(TestRunner {
            script: TestScript {
                commands,
                initial_settle,
            },
            current_command: 0,
            immediate_mode: false,
            max_settle_time: Duration::from_secs(5),
            overwrite_mode: false,
            screenshot_base_dir: None,
        })
    }

    pub async fn run(
        &mut self,
        app: &mut App,
        task_sender: &mpsc::Sender<Task>,
        mut task_receiver: mpsc::Receiver<TaskResult>,
    ) -> Result<TestResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let mut events_processed = 0;
        let mut assertions_passed = 0;
        let mut assertions_failed = 0;
        let mut errors = Vec::new();

        log::info!(
            "🧪 Starting test run with {} commands",
            self.script.commands.len()
        );

        // Initial settlement if requested
        if self.script.initial_settle {
            log::debug!("🧪 Waiting for initial settlement");
            if let Err(e) = self.wait_for_settlement(app, &mut task_receiver).await {
                errors.push(format!("Initial settlement failed: {}", e));
            }
        }

        // Execute commands
        for (index, command) in self.script.commands.iter().enumerate() {
            self.current_command = index;
            log::debug!("🧪 Executing command {}: {:?}", index, command);

            match &command.command_type {
                CommandType::Key => {
                    let event = self.parse_key_event(&command.value)?;
                    if let Err(e) = handle_event(event, app, task_sender) {
                        errors.push(format!("Key event failed: {}", e));
                    } else {
                        events_processed += 1;
                        app.navigator.build_view_model();
                    }
                }
                CommandType::Char => {
                    let char_val = command.value.chars().next().ok_or("Empty character command")?;
                    let event =
                        Event::Key(KeyEvent::new(KeyCode::Char(char_val), KeyModifiers::NONE));
                    if let Err(e) = handle_event(event, app, task_sender) {
                        errors.push(format!("Character event failed: {}", e));
                    } else {
                        events_processed += 1;
                        app.navigator.build_view_model();
                    }
                }
                CommandType::Wait => {
                    if command.value.is_empty() {
                        // Wait for settlement
                        if let Err(e) = self.wait_for_settlement(app, &mut task_receiver).await {
                            errors.push(format!("Settlement wait failed: {}", e));
                        }
                        app.navigator.build_view_model();
                    } else {
                        // Wait for specific duration
                        let ms: u64 = command
                            .value
                            .parse()
                            .map_err(|_| format!("Invalid wait duration: {}", command.value))?;
                        tokio::time::sleep(Duration::from_millis(ms)).await;
                    }
                }
                CommandType::Assert => {
                    app.navigator.build_view_model();
                    match self.evaluate_assertion(app, &command.value) {
                        Ok(true) => {
                            assertions_passed += 1;
                            log::debug!("🧪 Assertion passed: {}", command.value);
                        }
                        Ok(false) => {
                            assertions_failed += 1;
                            errors.push(format!("Assertion failed: {}", command.value));
                        }
                        Err(e) => {
                            assertions_failed += 1;
                            errors.push(format!("Assertion error: {}", e));
                        }
                    }
                }
                CommandType::Screenshot => {
                    // Take a screenshot and save to file
                    if let Err(e) = self.take_screenshot(app, &command.value) {
                        errors.push(format!("Screenshot failed: {}", e));
                    }
                }
                CommandType::SetImmediate => {
                    self.immediate_mode = true;
                }
                CommandType::SetSettle => {
                    self.immediate_mode = false;
                }
            }

            // Wait for settlement unless in immediate mode or this is a wait command
            if !command.immediate && !matches!(command.command_type, CommandType::Wait) {
                if let Err(e) = self.wait_for_settlement(app, &mut task_receiver).await {
                    errors.push(format!("Post-command settlement failed: {}", e));
                }
            }
        }

        let duration = start_time.elapsed();
        log::info!("🧪 Test run completed in {:?}", duration);

        let success = assertions_failed == 0 && errors.is_empty();
        Ok(TestResult {
            duration,
            events_processed,
            assertions_passed,
            assertions_failed,
            errors,
            success,
        })
    }

    fn parse_key_event(&self, key_str: &str) -> Result<Event, Box<dyn std::error::Error>> {
        let key_code = match key_str.to_lowercase().as_str() {
            "tab" => KeyCode::Tab,
            "enter" => KeyCode::Enter,
            "esc" | "escape" => KeyCode::Esc,
            "space" => KeyCode::Char(' '),
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            "backspace" => KeyCode::Backspace,
            "delete" => KeyCode::Delete,
            single_char if single_char.len() == 1 => {
                KeyCode::Char(single_char.chars().next().unwrap())
            }
            _ => return Err(format!("Unknown key: {}", key_str).into()),
        };

        Ok(Event::Key(KeyEvent::new(key_code, KeyModifiers::NONE)))
    }

    async fn wait_for_settlement(
        &self,
        app: &mut App,
        task_receiver: &mut mpsc::Receiver<TaskResult>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();

        // Wait for UI loading to complete and process any task results
        while app.ui.is_loading && start.elapsed() < self.max_settle_time {
            // Check for task results
            if let Ok(result) = timeout(Duration::from_millis(10), task_receiver.recv()).await {
                if let Some(task_result) = result {
                    log::debug!(
                        "🧪 Processing task result during settlement: {:?}",
                        std::mem::discriminant(&task_result)
                    );
                    crate::main_lib::handle_task_result(app, task_result);
                } else {
                    break; // Channel closed
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Process any remaining task results
        while let Ok(task_result) = task_receiver.try_recv() {
            log::debug!(
                "🧪 Processing remaining task result: {:?}",
                std::mem::discriminant(&task_result)
            );
            crate::main_lib::handle_task_result(app, task_result);
        }

        // Additional small delay to ensure everything settles
        tokio::time::sleep(Duration::from_millis(50)).await;

        if app.ui.is_loading {
            return Err("Settlement timeout: app is still loading".into());
        }

        Ok(())
    }

    fn evaluate_assertion(
        &self,
        app: &mut App,
        assertion: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = assertion.split(':').collect();
        if parts.len() != 2 {
            return Err("Assertion must be in format 'property:value'".into());
        }

        let property = parts[0];
        let expected = parts[1];

        match property {
            "active_panel" => {
                let actual = format!("{:?}", app.ui.active_panel);
                Ok(actual == expected)
            }
            "should_quit" => {
                let expected_bool =
                    expected.parse::<bool>().map_err(|_| "should_quit expects boolean value")?;
                Ok(app.should_quit == expected_bool)
            }
            "is_loading" => {
                let expected_bool =
                    expected.parse::<bool>().map_err(|_| "is_loading expects boolean value")?;
                Ok(app.ui.is_loading == expected_bool)
            }
            "status_contains" => Ok(app.ui.status_message.contains(expected)),
            "cursor_line" => {
                let expected_line =
                    expected.parse::<usize>().map_err(|_| "cursor_line expects numeric value")?;
                Ok(app.inspector.cursor_line == expected_line)
            }
            "content_lines" => {
                let expected_count = expected
                    .parse::<usize>()
                    .map_err(|_| "content_lines expects numeric value")?;
                Ok(app.inspector.current_content.len() == expected_count)
            }
            "has_file_selected" => {
                let expected_bool = expected
                    .parse::<bool>()
                    .map_err(|_| "has_file_selected expects boolean value")?;
                Ok(app.navigator.get_selection().is_some() == expected_bool)
            }
            "visible_files_count" => {
                let expected_count = expected
                    .parse::<usize>()
                    .map_err(|_| "visible_files_count expects numeric value")?;
                let view_model = app.navigator.build_view_model();
                Ok(view_model.items.len() == expected_count)
            }
            "is_searching" => {
                let expected_bool =
                    expected.parse::<bool>().map_err(|_| "is_searching expects boolean value")?;
                let view_model = app.navigator.build_view_model();
                Ok(view_model.is_searching == expected_bool)
            }
            "search_query" => {
                let view_model = app.navigator.build_view_model();
                Ok(view_model.search_query == expected)
            }
            "selected_file" => {
                let selection = app.navigator.get_selection();
                match selection {
                    Some(path) => {
                        let path_str = path.to_string_lossy();
                        Ok(path_str == expected)
                    }
                    None => Ok(expected == "none" || expected.is_empty()),
                }
            }
            _ => Err(format!("Unknown assertion property: {}", property).into()),
        }
    }

    fn take_screenshot(
        &self,
        app: &mut App,
        filename: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::headless_backend::HeadlessBackend;
        use crate::ui;
        use ratatui::Terminal;

        // Create a headless backend to capture the UI
        let backend = HeadlessBackend::new(120, 30);
        let mut terminal = Terminal::new(backend)?;

        // Render the app to capture the current state
        terminal.draw(|f| {
            ui::draw(f, app);
        })?;

        // Get the rendered content
        let content = terminal.backend().get_content();

        // Resolve the final screenshot path
        let final_path = if let Some(base_dir) = &self.screenshot_base_dir {
            base_dir.join(filename)
        } else {
            std::path::PathBuf::from(filename)
        };

        let final_filename = final_path.to_string_lossy();

        if self.overwrite_mode {
            // Overwrite mode: always write the file
            std::fs::write(&final_path, content)?;
            println!("📸 Screenshot saved to: {}", final_filename);
        } else {
            // Verify mode: compare with existing file
            match std::fs::read_to_string(&final_path) {
                Ok(existing_content) => {
                    if content == existing_content {
                        println!("✅ Screenshot verification passed: {}", final_filename);
                    } else {
                        return Err(format!(
                            "❌ Screenshot verification failed: {}. Content differs from expected. Use --overwrite to update.",
                            final_filename
                        )
                        .into());
                    }
                }
                Err(_) => {
                    return Err(format!(
                        "❌ Screenshot verification failed: {} does not exist. Use --overwrite to create.",
                        final_filename
                    )
                    .into());
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub duration: Duration,
    pub events_processed: usize,
    pub assertions_passed: usize,
    pub assertions_failed: usize,
    pub errors: Vec<String>,
    pub success: bool,
}

impl TestResult {
    pub fn print_summary(&self) {
        println!("🧪 Test Results:");
        println!("   Duration: {:?}", self.duration);
        println!("   Events processed: {}", self.events_processed);
        println!("   Assertions passed: {}", self.assertions_passed);
        println!("   Assertions failed: {}", self.assertions_failed);

        if !self.errors.is_empty() {
            println!("   Errors:");
            for error in &self.errors {
                println!("     - {}", error);
            }
        }

        if self.success {
            println!("   Status: ✅ PASSED");
        } else {
            println!("   Status: ❌ FAILED");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_utils;

    #[test]
    fn test_parse_simple_script() {
        let content = r#"
# Test script
key:down
char:a
wait:100
assert:active_panel:Navigator
"#;

        let runner = TestRunner::from_string(content).unwrap();
        assert_eq!(runner.script.commands.len(), 4);

        assert!(matches!(runner.script.commands[0].command_type, CommandType::Key));
        assert_eq!(runner.script.commands[0].value, "down");

        assert!(matches!(runner.script.commands[1].command_type, CommandType::Char));
        assert_eq!(runner.script.commands[1].value, "a");

        assert!(matches!(runner.script.commands[2].command_type, CommandType::Wait));
        assert_eq!(runner.script.commands[2].value, "100");

        assert!(matches!(runner.script.commands[3].command_type, CommandType::Assert));
        assert_eq!(runner.script.commands[3].value, "active_panel:Navigator");
    }

    #[test]
    fn test_parse_immediate_mode() {
        let content = r#"
key:down
immediate
key:up
key:down
settle_mode
key:enter
"#;

        let runner = TestRunner::from_string(content).unwrap();
        assert_eq!(runner.script.commands.len(), 4);

        assert!(!runner.script.commands[0].immediate); // Before immediate
        assert!(runner.script.commands[1].immediate); // After immediate
        assert!(runner.script.commands[2].immediate); // Still immediate
        assert!(!runner.script.commands[3].immediate); // After settle_mode
    }

    #[test]
    fn test_parse_key_events() {
        let runner = TestRunner::new();

        let tab_event = runner.parse_key_event("tab").unwrap();
        assert!(matches!(tab_event, Event::Key(KeyEvent { code: KeyCode::Tab, .. })));

        let enter_event = runner.parse_key_event("enter").unwrap();
        assert!(matches!(enter_event, Event::Key(KeyEvent { code: KeyCode::Enter, .. })));

        let char_event = runner.parse_key_event("a").unwrap();
        assert!(matches!(char_event, Event::Key(KeyEvent { code: KeyCode::Char('a'), .. })));
    }

    #[test]
    fn test_assertion_evaluation() {
        let repo = git_utils::open_repository(".").unwrap();
        let mut app = App::new(repo);
        app.ui.active_panel = crate::app::PanelFocus::History;
        app.should_quit = false;
        app.inspector.cursor_line = 5;

        let runner = TestRunner::new();

        assert!(runner.evaluate_assertion(&mut app, "active_panel:History").unwrap());
        assert!(!runner.evaluate_assertion(&mut app, "active_panel:Navigator").unwrap());

        assert!(runner.evaluate_assertion(&mut app, "should_quit:false").unwrap());
        assert!(!runner.evaluate_assertion(&mut app, "should_quit:true").unwrap());

        assert!(runner.evaluate_assertion(&mut app, "cursor_line:5").unwrap());
        assert!(!runner.evaluate_assertion(&mut app, "cursor_line:10").unwrap());
    }
}

impl TestRunner {
    pub fn new() -> Self {
        TestRunner {
            script: TestScript {
                commands: Vec::new(),
                initial_settle: true,
            },
            current_command: 0,
            immediate_mode: false,
            max_settle_time: Duration::from_secs(5),
            overwrite_mode: false,
            screenshot_base_dir: None,
        }
    }
}
