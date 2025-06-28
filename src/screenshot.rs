use ratatui::{
    backend::TestBackend,
    buffer::Buffer,
    Terminal,
};
use std::fs;

use crate::{app::App, test_config::TestConfig, ui, git_utils};

pub fn generate_screenshot(
    config_path: &str,
    output_path: Option<&str>,
    width: u16,
    height: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load the test configuration
    let config = TestConfig::load_from_file(config_path)?;
    
    // Create a dummy repository (we won't use it for screenshots)
    let repo = git_utils::open_repository(".")?;
    
    // Create app from test config
    let app = App::from_test_config(&config, repo);
    
    // Create TestBackend with specified dimensions
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend)?;
    
    // Render the UI once
    terminal.draw(|frame| {
        ui::draw(frame, &app);
    })?;
    
    // Get the buffer content
    let buffer = terminal.backend().buffer().clone();
    
    // Convert buffer to string
    let screenshot = buffer_to_string(&buffer);
    
    // Output the screenshot
    match output_path {
        Some(path) => {
            fs::write(path, screenshot)?;
            println!("Screenshot saved to: {}", path);
        }
        None => {
            print!("{}", screenshot);
        }
    }
    
    Ok(())
}

pub fn buffer_to_string(buffer: &Buffer) -> String {
    let mut result = String::new();
    
    for y in 0..buffer.area().height {
        for x in 0..buffer.area().width {
            let cell = &buffer[(x, y)];
            let sym = cell.symbol();
            
            // Use a space for empty cells to make output more readable
            if sym.is_empty() {
                result.push(' ');
            } else {
                result.push_str(sym);
            }
        }
        result.push('\n');
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_buffer_to_string() {
        let backend = TestBackend::new(10, 3);
        let mut terminal = Terminal::new(backend).unwrap();
        
        terminal.draw(|frame| {
            use ratatui::{
                widgets::{Block, Borders, Paragraph},
                text::Text,
            };
            
            let paragraph = Paragraph::new(Text::from("Test"))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(paragraph, frame.area());
        }).unwrap();
        
        let buffer = terminal.backend().buffer().clone();
        let result = buffer_to_string(&buffer);
        
        // Should contain border characters and the text "Test"
        assert!(result.contains("Test"));
        assert!(result.len() > 0);
    }
}