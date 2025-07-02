use ratatui::{backend::Backend, buffer::Buffer, layout::{Position, Rect, Size}};
use std::io;

/// A headless backend for ratatui that doesn't render to any actual terminal
/// This is used for testing and automation where we want to run the app
/// logic without visual output
#[derive(Debug, Clone)]
pub struct HeadlessBackend {
    width: u16,
    height: u16,
    buffer: Buffer,
    cursor_position: Option<(u16, u16)>,
    cursor_visible: bool,
}

impl HeadlessBackend {
    /// Create a new headless backend with the specified dimensions
    pub fn new(width: u16, height: u16) -> Self {
        let area = Rect::new(0, 0, width, height);
        let mut buffer = Buffer::empty(area);
        buffer.reset();

        Self {
            width,
            height,
            buffer,
            cursor_position: None,
            cursor_visible: false,
        }
    }

    /// Get a copy of the current buffer content
    /// This can be used for testing or debugging
    pub fn get_buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// Get the text content of the buffer as a string
    /// Useful for assertions in tests
    pub fn get_content(&self) -> String {
        let mut content = String::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = &self.buffer[(x, y)];
                let mut symbol = cell.symbol().chars().next().unwrap_or(' ');
                
                // If cursor is visible and at this position, use cursor character
                if self.cursor_visible {
                    if let Some((cursor_x, cursor_y)) = self.cursor_position {
                        if x == cursor_x && y == cursor_y {
                            symbol = '█'; // Block cursor character
                        }
                    }
                }
                
                content.push(symbol);
            }
            if y < self.height - 1 {
                content.push('\n');
            }
        }
        content
    }

    /// Check if specific text appears in the buffer
    pub fn contains_text(&self, text: &str) -> bool {
        self.get_content().contains(text)
    }

    /// Get cursor information
    pub fn get_cursor(&self) -> (Option<(u16, u16)>, bool) {
        (self.cursor_position, self.cursor_visible)
    }

    /// Resize the backend
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        let area = Rect::new(0, 0, width, height);
        self.buffer = Buffer::empty(area);
        self.buffer.reset();
    }
}

impl Backend for HeadlessBackend {
    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a ratatui::buffer::Cell)>,
    {
        for (x, y, cell) in content {
            if x < self.width && y < self.height {
                self.buffer[(x, y)] = cell.clone();
            }
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.cursor_visible = false;
        Ok(())
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.cursor_visible = true;
        Ok(())
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        let (x, y) = self.cursor_position.unwrap_or((0, 0));
        Ok(Position::new(x, y))
    }

    fn set_cursor_position<P>(&mut self, position: P) -> io::Result<()>
    where
        P: Into<Position>,
    {
        let pos = position.into();
        self.cursor_position = Some((pos.x, pos.y));
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.buffer.reset();
        Ok(())
    }

    fn size(&self) -> io::Result<Size> {
        Ok(Size::new(self.width, self.height))
    }

    fn window_size(&mut self) -> io::Result<ratatui::backend::WindowSize> {
        Ok(ratatui::backend::WindowSize {
            columns_rows: Size::new(self.width, self.height),
            pixels: Size::new(0, 0), // Not relevant for headless
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        // No-op for headless backend
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        layout::{Constraint, Direction, Layout},
        widgets::{Block, Borders, Paragraph},
        Terminal,
    };

    #[test]
    fn test_headless_backend_creation() {
        let backend = HeadlessBackend::new(80, 24);
        assert_eq!(backend.width, 80);
        assert_eq!(backend.height, 24);
        assert!(!backend.cursor_visible);
        assert_eq!(backend.cursor_position, None);
    }

    #[test]
    fn test_headless_backend_size() {
        let backend = HeadlessBackend::new(120, 40);
        let size = backend.size().unwrap();
        assert_eq!(size.width, 120);
        assert_eq!(size.height, 40);
    }

    #[test]
    fn test_cursor_operations() {
        let mut backend = HeadlessBackend::new(80, 24);
        
        // Initially cursor is hidden and at default position
        assert!(!backend.cursor_visible);
        
        // Show cursor
        backend.show_cursor().unwrap();
        assert!(backend.cursor_visible);
        
        // Set cursor position
        backend.set_cursor_position(Position::new(10, 5)).unwrap();
        let (pos, visible) = backend.get_cursor();
        assert_eq!(pos, Some((10, 5)));
        assert!(visible);
        
        // Hide cursor
        backend.hide_cursor().unwrap();
        let (pos, visible) = backend.get_cursor();
        assert_eq!(pos, Some((10, 5))); // Position remains
        assert!(!visible);
    }

    #[test]
    fn test_buffer_operations() {
        let mut backend = HeadlessBackend::new(10, 3);
        
        // Clear should work
        backend.clear().unwrap();
        
        // Initial content should be empty/spaces
        let content = backend.get_content();
        assert_eq!(content.len(), 10 * 3 + 2); // 10*3 chars + 2 newlines
        assert!(content.chars().all(|c| c == ' ' || c == '\n'));
    }

    #[test]
    fn test_resize() {
        let mut backend = HeadlessBackend::new(80, 24);
        
        backend.resize(120, 40);
        assert_eq!(backend.width, 120);
        assert_eq!(backend.height, 40);
        
        let size = backend.size().unwrap();
        assert_eq!(size.width, 120);
        assert_eq!(size.height, 40);
    }

    #[test]
    fn test_rendering_simple_content() {
        let mut backend = HeadlessBackend::new(20, 5);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            let block = Block::default().title("Test").borders(Borders::ALL);
            f.render_widget(block, f.size());
        }).unwrap();

        let backend = terminal.backend();
        let content = backend.get_content();
        
        // Should contain the title and borders
        assert!(backend.contains_text("Test"));
        // The exact border characters depend on the terminal implementation
        // but there should be some structure
        assert!(content.len() > 0);
    }

    #[test]
    fn test_rendering_text_widget() {
        let mut backend = HeadlessBackend::new(30, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            let paragraph = Paragraph::new("Hello, World!");
            f.render_widget(paragraph, f.size());
        }).unwrap();

        let backend = terminal.backend();
        assert!(backend.contains_text("Hello, World!"));
    }

    #[test]
    fn test_complex_layout() {
        let mut backend = HeadlessBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let top_block = Block::default().title("Top").borders(Borders::ALL);
            let bottom_block = Block::default().title("Bottom").borders(Borders::ALL);

            f.render_widget(top_block, chunks[0]);
            f.render_widget(bottom_block, chunks[1]);
        }).unwrap();

        let backend = terminal.backend();
        assert!(backend.contains_text("Top"));
        assert!(backend.contains_text("Bottom"));
    }

    #[test]
    fn test_buffer_access() {
        let mut backend = HeadlessBackend::new(10, 3);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal.draw(|f| {
            let paragraph = Paragraph::new("ABC");
            f.render_widget(paragraph, f.size());
        }).unwrap();

        let backend = terminal.backend();
        let buffer = backend.get_buffer();
        
        // Should be able to access individual cells
        assert_eq!(buffer.area().width, 10);
        assert_eq!(buffer.area().height, 3);
    }
}