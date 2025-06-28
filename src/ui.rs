use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, PanelFocus};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(frame.area());

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let status_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    // Draw panels
    draw_file_navigator(frame, app, left_chunks[0]);
    draw_commit_history(frame, app, left_chunks[1]);
    draw_code_inspector(frame, app, chunks[1]);
    draw_status_bar(frame, app, status_chunks[1]);
}

fn draw_file_navigator(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == PanelFocus::Navigator;
    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let title = if app.in_search_mode {
        format!(" File Navigator (Search: {}) ", app.search_query)
    } else {
        " File Navigator ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .padding(ratatui::widgets::Padding::new(0, 0, 0, 0));

    if app.file_tree.root.is_empty() {
        let paragraph = Paragraph::new("No files found")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(paragraph, area);
        return;
    }

    // Get visible nodes with their display depths from the file tree
    let visible_nodes_with_depth = app.file_tree.get_visible_nodes_with_depth();
    
    // Convert visible nodes to list items with proper highlighting
    let items: Vec<ListItem> = visible_nodes_with_depth
        .iter()
        .enumerate()
        .map(|(_i, (node, display_depth))| {
            let status_char = match node.git_status {
                Some('M') => 'M',
                Some('A') => 'A', 
                Some('D') => 'D',
                Some('?') => '?',
                _ => ' ',
            };
            
            // Use display depth for indentation (how deep in the currently visible tree)
            let display_name = if node.is_dir {
                let expand_char = if node.is_expanded { "▼" } else { "▶" };
                if *display_depth == 0 {
                    format!("{} {}", expand_char, node.name)
                } else {
                    format!("{}{} {}", " ".repeat(display_depth * 2), expand_char, node.name)
                }
            } else {
                if *display_depth == 0 {
                    // Root level files - align with directory names (after expand char + space)
                    if status_char == ' ' {
                        format!("  {}", node.name)
                    } else {
                        format!("{} {}", status_char, node.name)
                    }
                } else {
                    // Nested files - align with nested directory names
                    if status_char == ' ' {
                        format!("{}  {}", " ".repeat(display_depth * 2), node.name)
                    } else {
                        format!("{}{} {}", " ".repeat(display_depth * 2), status_char, node.name)
                    }
                }
            };
            
            
            // Check if this node is selected
            let is_selected = Some(&node.path) == app.file_tree.current_selection.as_ref();
            
            let line = if is_selected {
                // Highlight selected item with high contrast
                Line::from(vec![
                    Span::styled(display_name, Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD))
                ])
            } else {
                // Style based on git status and type with moderate, readable colors
                let style = if node.is_dir {
                    Style::default().fg(Color::Blue).add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    match node.git_status {
                        Some('M') => Style::default().fg(Color::Yellow),
                        Some('A') => Style::default().fg(Color::Green), 
                        Some('D') => Style::default().fg(Color::Red),
                        Some('?') => Style::default().fg(Color::Magenta),
                        _ => Style::default().fg(Color::Reset), // Default terminal color
                    }
                };
                Line::from(vec![Span::styled(display_name, style)])
            };
            
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::White).fg(Color::Black).add_modifier(ratatui::style::Modifier::BOLD))
        .highlight_symbol("");

    // Find the selected index for the list state
    let selected_index = if let Some(ref current_selection) = app.file_tree.current_selection {
        visible_nodes_with_depth.iter().position(|(node, _)| &node.path == current_selection)
    } else {
        None
    };

    let mut list_state = ListState::default();
    list_state.select(selected_index);
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_commit_history(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == PanelFocus::History;
    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let title = if let Some(ref path) = app.file_tree.current_selection {
        format!(" Commit History ({}) ", path.display())
    } else {
        " Commit History ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.commit_list.is_empty() {
        let paragraph = Paragraph::new("Select a file to view its history")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(paragraph, area);
        return;
    }

    let items: Vec<ListItem> = app.commit_list
        .iter()
        .map(|commit| {
            let line = Line::from(vec![
                Span::styled(&commit.short_hash, Style::default().fg(Color::Yellow)),
                Span::raw(" "),
                Span::styled(&commit.date, Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(&commit.author, Style::default().fg(Color::Green)),
                Span::raw(" "),
                Span::raw(&commit.subject),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">> ");

    let mut list_state = app.commit_list_state.clone();
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_code_inspector(frame: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_panel == PanelFocus::Inspector;
    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let title = if app.show_diff_view {
        " Code Inspector (Diff View) "
    } else {
        " Code Inspector "
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.current_content.is_empty() {
        let paragraph = Paragraph::new("Select a file and commit to view content")
            .block(block)
            .style(Style::default().fg(Color::Gray));
        frame.render_widget(paragraph, area);
        return;
    }

    // Simple content display for now
    let content_lines: Vec<Line> = app.current_content
        .iter()
        .enumerate()
        .skip(app.inspector_scroll_vertical as usize)
        .take((area.height - 2) as usize) // Account for borders
        .map(|(line_num, line)| {
            let line_number = format!("{:4} ", line_num + 1);
            if line_num == app.cursor_line {
                Line::from(vec![
                    Span::styled(line_number, Style::default().fg(Color::Yellow)),
                    Span::styled(line, Style::default().bg(Color::DarkGray)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(line_number, Style::default().fg(Color::Blue)),
                    Span::raw(line),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .scroll((0, app.inspector_scroll_horizontal));

    frame.render_widget(paragraph, area);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status_text = if app.is_loading {
        format!("Loading... | {}", app.status_message)
    } else {
        app.status_message.clone()
    };

    let help_text = match app.active_panel {
        PanelFocus::Navigator => "Tab: Switch panel | /: Search | ↑↓: Navigate | →←: Expand/Collapse",
        PanelFocus::History => "Tab: Switch panel | ↑↓: Navigate | Enter: Select commit",
        PanelFocus::Inspector => "Tab: Switch panel | ↑↓: Navigate | p: Previous change | n: Next change | d: Toggle diff",
    };

    let status_line = Line::from(vec![
        Span::styled(status_text, Style::default().fg(Color::White)),
        Span::raw(" | "),
        Span::styled(help_text, Style::default().fg(Color::Gray)),
    ]);

    let paragraph = Paragraph::new(status_line)
        .style(Style::default().bg(Color::DarkGray));

    frame.render_widget(paragraph, area);
}