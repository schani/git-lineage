use std::fs;
use std::path::Path;
use ratatui::{backend::TestBackend, Terminal};
use git_lineage::{
    app::App,
    test_config::TestConfig,
    ui,
    git_utils,
    screenshot::buffer_to_string,
};

/// Test structure to hold test case information
#[derive(Debug)]
struct RenderingTest {
    name: String,
    config_path: String,
    expected_path: String,
}

/// Discover all rendering tests in the tests/rendering_tests directory
fn discover_rendering_tests() -> Vec<RenderingTest> {
    let test_dir = "tests/rendering_tests";
    let mut tests = Vec::new();
    
    if let Ok(entries) = fs::read_dir(test_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name.ends_with(".json") && !file_name.contains(".expected") {
                    let name = file_name.trim_end_matches(".json").to_string();
                    let config_path = path.to_string_lossy().to_string();
                    let expected_path = format!("{}/{}.expected.txt", test_dir, name);
                    
                    // Only include tests where the expected file exists
                    if Path::new(&expected_path).exists() {
                        tests.push(RenderingTest {
                            name,
                            config_path,
                            expected_path,
                        });
                    }
                }
            }
        }
    }
    
    tests.sort_by(|a, b| a.name.cmp(&b.name));
    tests
}

/// Generate a screenshot from a test configuration
fn generate_test_screenshot(config: &TestConfig) -> Result<String, Box<dyn std::error::Error>> {
    // Create a dummy repository (we won't use it for screenshots)
    let repo = git_utils::open_repository(".").map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())) as Box<dyn std::error::Error>)?;
    
    // Create app from test config
    let app = App::from_test_config(config, repo);
    
    // Create TestBackend with fixed dimensions (80x25 to match expected screenshots)
    let backend = TestBackend::new(80, 25);
    let mut terminal = Terminal::new(backend)?;
    
    // Render the UI once
    terminal.draw(|frame| {
        ui::draw(frame, &app);
    })?;
    
    // Get the buffer content and convert to string
    let buffer = terminal.backend().buffer().clone();
    Ok(buffer_to_string(&buffer))
}

/// Compare two screenshot strings, ignoring minor whitespace differences
fn compare_screenshots(actual: &str, expected: &str) -> bool {
    // Normalize line endings and trailing whitespace
    let normalize = |s: &str| -> String {
        s.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    };
    
    normalize(actual) == normalize(expected)
}

/// Generate a diff between two screenshots for debugging
fn generate_diff(actual: &str, expected: &str) -> String {
    let actual_lines: Vec<&str> = actual.lines().collect();
    let expected_lines: Vec<&str> = expected.lines().collect();
    let max_lines = actual_lines.len().max(expected_lines.len());
    
    let mut diff = String::new();
    diff.push_str("Screenshot comparison failed:\n");
    diff.push_str("--- Expected\n");
    diff.push_str("+++ Actual\n");
    
    for i in 0..max_lines {
        let expected_line = expected_lines.get(i).unwrap_or(&"");
        let actual_line = actual_lines.get(i).unwrap_or(&"");
        
        if expected_line != actual_line {
            diff.push_str(&format!("@@ Line {} @@\n", i + 1));
            diff.push_str(&format!("-{}\n", expected_line));
            diff.push_str(&format!("+{}\n", actual_line));
        }
    }
    
    diff
}

/// Run a single rendering test
fn run_rendering_test(test: &RenderingTest) -> Result<(), String> {
    // Load the test configuration
    let config = TestConfig::load_from_file(&test.config_path)
        .map_err(|e| format!("Failed to load config {}: {}", test.config_path, e))?;
    
    // Generate the screenshot
    let actual_screenshot = generate_test_screenshot(&config)
        .map_err(|e| format!("Failed to generate screenshot for {}: {}", test.name, e))?;
    
    // Load the expected screenshot
    let expected_screenshot = fs::read_to_string(&test.expected_path)
        .map_err(|e| format!("Failed to load expected screenshot {}: {}", test.expected_path, e))?;
    
    // Compare the screenshots
    if !compare_screenshots(&actual_screenshot, &expected_screenshot) {
        let diff = generate_diff(&actual_screenshot, &expected_screenshot);
        return Err(format!("Screenshot mismatch for test '{}':\n{}", test.name, diff));
    }
    
    Ok(())
}

/// Test that all rendering tests pass
#[test]
fn test_all_rendering() {
    let tests = discover_rendering_tests();
    
    if tests.is_empty() {
        panic!("No rendering tests found in tests/rendering_tests/");
    }
    
    let mut failures = Vec::new();
    
    for test in &tests {
        if let Err(error) = run_rendering_test(test) {
            failures.push(format!("❌ {}: {}", test.name, error));
        } else {
            println!("✅ {}: PASSED", test.name);
        }
    }
    
    if !failures.is_empty() {
        panic!("\n{} rendering test(s) failed:\n{}", failures.len(), failures.join("\n"));
    }
    
    println!("🎉 All {} rendering tests passed!", tests.len());
}

/// Individual test functions for each specific test case
/// These allow running specific tests with `cargo test test_rendering_default_navigator`

#[test]
fn test_rendering_default_navigator() {
    let test = RenderingTest {
        name: "default_navigator".to_string(),
        config_path: "tests/rendering_tests/default_navigator.json".to_string(),
        expected_path: "tests/rendering_tests/default_navigator.expected.txt".to_string(),
    };
    
    if let Err(error) = run_rendering_test(&test) {
        panic!("Rendering test failed: {}", error);
    }
}

#[test]
fn test_rendering_history_focused() {
    let test = RenderingTest {
        name: "history_focused".to_string(),
        config_path: "tests/rendering_tests/history_focused.json".to_string(),
        expected_path: "tests/rendering_tests/history_focused.expected.txt".to_string(),
    };
    
    if let Err(error) = run_rendering_test(&test) {
        panic!("Rendering test failed: {}", error);
    }
}

#[test]
fn test_rendering_search_active() {
    let test = RenderingTest {
        name: "search_active".to_string(),
        config_path: "tests/rendering_tests/search_active.json".to_string(),
        expected_path: "tests/rendering_tests/search_active.expected.txt".to_string(),
    };
    
    if let Err(error) = run_rendering_test(&test) {
        panic!("Rendering test failed: {}", error);
    }
}

#[test]
fn test_rendering_inspector_diff() {
    let test = RenderingTest {
        name: "inspector_diff".to_string(),
        config_path: "tests/rendering_tests/inspector_diff.json".to_string(),
        expected_path: "tests/rendering_tests/inspector_diff.expected.txt".to_string(),
    };
    
    if let Err(error) = run_rendering_test(&test) {
        panic!("Rendering test failed: {}", error);
    }
}

#[test]
fn test_rendering_loading_state() {
    let test = RenderingTest {
        name: "loading_state".to_string(),
        config_path: "tests/rendering_tests/loading_state.json".to_string(),
        expected_path: "tests/rendering_tests/loading_state.expected.txt".to_string(),
    };
    
    if let Err(error) = run_rendering_test(&test) {
        panic!("Rendering test failed: {}", error);
    }
}

#[cfg(test)]
mod helper_tests {
    use super::*;
    
    #[test]
    fn test_discover_rendering_tests() {
        let tests = discover_rendering_tests();
        assert!(!tests.is_empty(), "Should discover at least one rendering test");
        
        // Verify test discovery includes expected tests
        let test_names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
        assert!(test_names.contains(&"default_navigator"));
        assert!(test_names.contains(&"history_focused"));
    }
    
    #[test]
    fn test_compare_screenshots() {
        let screenshot1 = "line1\nline2\nline3";
        let screenshot2 = "line1\nline2\nline3";
        let screenshot3 = "line1\nline2\nDIFFERENT";
        
        assert!(compare_screenshots(screenshot1, screenshot2));
        assert!(!compare_screenshots(screenshot1, screenshot3));
        
        // Test whitespace normalization
        let with_trailing = "line1  \nline2\t\nline3   ";
        let without_trailing = "line1\nline2\nline3";
        assert!(compare_screenshots(with_trailing, without_trailing));
    }
    
    #[test]
    fn test_generate_diff() {
        let actual = "line1\nDIFFERENT\nline3";
        let expected = "line1\nline2\nline3";
        
        let diff = generate_diff(actual, expected);
        assert!(diff.contains("Screenshot comparison failed"));
        assert!(diff.contains("Line 2"));
        assert!(diff.contains("-line2"));
        assert!(diff.contains("+DIFFERENT"));
    }
}