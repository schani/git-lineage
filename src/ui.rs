use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, PanelFocus};
use crate::theme::get_theme;

pub fn draw(frame: &mut Frame, app: &mut App) {
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
    // Use new navigator if available, otherwise fall back to old one
    if let Some(ref view_model) = app.cached_navigator_view_model {
        draw_file_navigator_new(frame, app, view_model, area);
    } else {
        draw_file_navigator_old(frame, app, area);
    }
}

fn draw_file_navigator_new(frame: &mut Frame, app: &App, view_model: &crate::navigator::NavigatorViewModel, area: Rect) {
    let theme = get_theme();
    let is_active = app.ui.active_panel == PanelFocus::Navigator;
    let border_style = if is_active {
        Style::default().fg(theme.active_border)
    } else {
        Style::default().fg(theme.inactive_border)
    };
    
    let title = if view_model.is_searching || !view_model.search_query.is_empty() {
        format!(" File Navigator (Search: {}) ", view_model.search_query)
    } else {
        " File Navigator ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .padding(ratatui::widgets::Padding::new(0, 0, 0, 0));

    // Position cursor when in search mode and navigator is focused
    if view_model.is_searching && is_active {
        let search_prefix = " File Navigator (Search: ";
        let cursor_x = area.x + search_prefix.len() as u16 + view_model.search_query.len() as u16 + 1;
        let cursor_y = area.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    if view_model.items.is_empty() {
        let paragraph = Paragraph::new("No files found")
            .block(block)
            .style(Style::default().fg(theme.panel_title));
        frame.render_widget(paragraph, area);
        return;
    }

    // Convert visible items to list items
    let items: Vec<ListItem> = view_model.items
        .iter()
        .map(|item| {
            let status_char = match item.git_status {
                Some('M') => 'M',
                Some('A') => 'A',
                Some('D') => 'D',
                Some('?') => '?',
                _ => ' ',
            };

            let display_name = if item.is_dir {
                let expand_char = if item.is_expanded { "▼" } else { "▶" };
                if item.depth == 0 {
                    format!("{} {}", expand_char, item.name)
                } else {
                    format!(
                        "{}{} {}",
                        " ".repeat(item.depth * 2),
                        expand_char,
                        item.name
                    )
                }
            } else {
                if item.depth == 0 {
                    if status_char == ' ' {
                        format!("  {}", item.name)
                    } else {
                        format!("{} {}", status_char, item.name)
                    }
                } else {
                    if status_char == ' ' {
                        format!("{}  {}", " ".repeat(item.depth * 2), item.name)
                    } else {
                        format!(
                            "{}{} {}",
                            " ".repeat(item.depth * 2),
                            status_char,
                            item.name
                        )
                    }
                }
            };

            let line = if item.is_selected {
                // Highlight selected item
                let content_width = (area.width as usize).saturating_sub(2);
                let display_len = display_name.chars().count();
                let padding_needed = content_width.saturating_sub(display_len);
                let padded_name = format!("{}{}", display_name, " ".repeat(padding_needed));
                Line::from(vec![Span::styled(
                    padded_name,
                    Style::default()
                        .fg(theme.file_selected_fg)
                        .bg(theme.file_selected_bg)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )])
            } else {
                let style = if item.is_dir {
                    Style::default()
                        .fg(theme.file_directory)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    match item.git_status {
                        Some('M') => Style::default().fg(theme.file_git_modified),
                        Some('A') => Style::default().fg(theme.file_git_added),
                        Some('D') => Style::default().fg(theme.file_git_deleted),
                        Some('?') => Style::default().fg(theme.file_git_untracked),
                        _ => Style::default().fg(theme.file_default),
                    }
                };
                Line::from(vec![Span::styled(display_name, style)])
            };

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    let mut list_state = ListState::default();
    list_state.select(None);
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_file_navigator_old(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme();
    let is_active = app.ui.active_panel == PanelFocus::Navigator;
    let border_style = if is_active {
        Style::default().fg(theme.active_border)
    } else {
        Style::default().fg(theme.inactive_border)
    };

    let title = if app.navigator.file_tree_state.in_search_mode {
        format!(" File Navigator (Search: {}) ", app.navigator.file_tree_state.search_query)
    } else if !app.navigator.file_tree_state.search_query.is_empty() {
        format!(" File Navigator (Search: {}) ", app.navigator.file_tree_state.search_query)
    } else {
        " File Navigator ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .padding(ratatui::widgets::Padding::new(0, 0, 0, 0));

    if app.navigator.file_tree_state.in_search_mode && is_active {
        let search_prefix = " File Navigator (Search: ";
        let cursor_x = area.x + search_prefix.len() as u16 + app.navigator.file_tree_state.search_query.len() as u16 + 1;
        let cursor_y = area.y;
        frame.set_cursor_position((cursor_x, cursor_y));
    }

    if app.navigator.file_tree_state.display_tree().root.is_empty() {
        let paragraph = Paragraph::new("No files found")
            .block(block)
            .style(Style::default().fg(theme.panel_title));
        frame.render_widget(paragraph, area);
        return;
    }

    let all_visible_nodes = app.navigator.file_tree_state.get_visible_nodes_with_depth();
    let viewport_height = (area.height as usize).saturating_sub(2);
    let scroll_offset = app.navigator.scroll_offset;
    
    let visible_nodes_with_depth: Vec<_> = all_visible_nodes
        .iter()
        .skip(scroll_offset)
        .take(viewport_height)
        .collect();

    let actual_rendered_height = visible_nodes_with_depth.len();
    let safe_cursor_position = app.navigator.cursor_position.min(actual_rendered_height.saturating_sub(1));

    let items: Vec<ListItem> = visible_nodes_with_depth
        .iter()
        .enumerate()
        .map(|(viewport_index, (node, display_depth))| {
            let status_char = match node.git_status {
                Some('M') => 'M',
                Some('A') => 'A',
                Some('D') => 'D',
                Some('?') => '?',
                _ => ' ',
            };

            let display_name = if node.is_dir {
                let expand_char = if node.is_expanded { "▼" } else { "▶" };
                if *display_depth == 0 {
                    format!("{} {}", expand_char, node.name)
                } else {
                    format!(
                        "{}{} {}",
                        " ".repeat(display_depth * 2),
                        expand_char,
                        node.name
                    )
                }
            } else {
                if *display_depth == 0 {
                    if status_char == ' ' {
                        format!("  {}", node.name)
                    } else {
                        format!("{} {}", status_char, node.name)
                    }
                } else {
                    if status_char == ' ' {
                        format!("{}  {}", " ".repeat(display_depth * 2), node.name)
                    } else {
                        format!(
                            "{}{} {}",
                            " ".repeat(display_depth * 2),
                            status_char,
                            node.name
                        )
                    }
                }
            };

            let is_selected = viewport_index == safe_cursor_position;

            let line = if is_selected {
                let content_width = (area.width as usize).saturating_sub(2);
                let display_len = display_name.chars().count();
                let padding_needed = content_width.saturating_sub(display_len);
                let padded_name = format!("{}{}", display_name, " ".repeat(padding_needed));
                Line::from(vec![Span::styled(
                    padded_name,
                    Style::default()
                        .fg(theme.file_selected_fg)
                        .bg(theme.file_selected_bg)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )])
            } else {
                let style = if node.is_dir {
                    Style::default()
                        .fg(theme.file_directory)
                        .add_modifier(ratatui::style::Modifier::BOLD)
                } else {
                    match node.git_status {
                        Some('M') => Style::default().fg(theme.file_git_modified),
                        Some('A') => Style::default().fg(theme.file_git_added),
                        Some('D') => Style::default().fg(theme.file_git_deleted),
                        Some('?') => Style::default().fg(theme.file_git_untracked),
                        _ => Style::default().fg(theme.file_default),
                    }
                };
                Line::from(vec![Span::styled(display_name, style)])
            };

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(block);
    let mut list_state = ListState::default();
    list_state.select(None);
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_commit_history(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme();
    let is_active = app.ui.active_panel == PanelFocus::History;
    let border_style = if is_active {
        Style::default().fg(theme.active_border)
    } else {
        Style::default().fg(theme.inactive_border)
    };

    let title = if let Some(ref path) = app.active_file_context {
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        if app.history.is_loading_more && !app.history.history_complete {
            format!(" Commit History ({}) - Loading... ", filename)
        } else if !app.history.history_complete {
            format!(" Commit History ({}) - {} commits (loading more...) ", filename, app.history.commit_list.len())
        } else {
            format!(" Commit History ({}) - {} commits ", filename, app.history.commit_list.len())
        }
    } else {
        " Commit History ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.history.commit_list.is_empty() {
        let paragraph = Paragraph::new("Select a file to view its history")
            .block(block)
            .style(Style::default().fg(theme.panel_title));
        frame.render_widget(paragraph, area);
        return;
    }

    let mut items: Vec<ListItem> = app
        .history
        .commit_list
        .iter()
        .map(|commit| {
            let line = Line::from(vec![
                Span::styled(&commit.short_hash, Style::default().fg(theme.commit_hash)),
                Span::raw(" "),
                Span::styled(&commit.date, Style::default().fg(theme.commit_date)),
                Span::raw(" "),
                Span::styled(&commit.author, Style::default().fg(theme.commit_author)),
                Span::raw(" "),
                Span::raw(&commit.subject),
            ]);
            ListItem::new(line)
        })
        .collect();

    // Add a loading indicator at the bottom if more commits are being loaded
    if !app.history.history_complete {
        let loading_line = if app.history.is_loading_more {
            Line::from(Span::styled("Loading more commits...", Style::default().fg(theme.panel_title)))
        } else {
            Line::from(Span::styled("More commits available (scroll to load)", Style::default().fg(theme.panel_title)))
        };
        items.push(ListItem::new(loading_line));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.commit_selected_bg)
                .fg(theme.commit_selected_fg),
        )
        .highlight_symbol(">> ");

    let mut list_state = app.history.list_state.clone();
    frame.render_stateful_widget(list, area, &mut list_state);
}

fn draw_code_inspector(frame: &mut Frame, app: &mut App, area: Rect) {
    let theme = get_theme();
    let is_active = app.ui.active_panel == PanelFocus::Inspector;
    let border_style = if is_active {
        Style::default().fg(theme.active_border)
    } else {
        Style::default().fg(theme.inactive_border)
    };

    // Update the visible height in the app state
    app.inspector.visible_height = area.height as usize;

    // Create a more informative title
    let title = if app.inspector.show_diff_view {
        " Code Inspector (Diff View) ".to_string()
    } else if let (Some(file_path), Some(commit_hash)) =
        (&app.active_file_context, &app.history.selected_commit_hash)
    {
        format!(
            " Code Inspector - {} @ {} ",
            file_path.file_name().unwrap_or_default().to_string_lossy(),
            &commit_hash[..8]
        )
    } else {
        " Code Inspector ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.inspector.current_content.is_empty() {
        let message = if app.active_file_context.is_none() {
            "Select a file to view its content"
        } else if app.history.selected_commit_hash.is_none() {
            "Select a commit to view file content at that point"
        } else if app.ui.is_loading {
            "Loading file content..."
        } else {
            "No content available for selected file/commit"
        };

        let paragraph = Paragraph::new(message)
            .block(block)
            .style(Style::default().fg(theme.panel_title));
        frame.render_widget(paragraph, area);
        return;
    }

    // Enhanced content display with syntax-aware styling
    let content_lines: Vec<Line> = app
        .inspector
        .current_content
        .iter()
        .enumerate()
        .skip(app.inspector.scroll_vertical as usize)
        .take((area.height - 2) as usize) // Account for borders
        .map(|(line_num, line)| {
            let line_number = format!("{:4} ", line_num + 1);

            // Basic syntax highlighting for common file types
            let line_style = get_line_style(line, &app.active_file_context);

            if line_num == app.inspector.cursor_line {
                // Calculate content width and add padding for full-width highlighting
                let content_width = (area.width as usize).saturating_sub(2); // Account for borders
                let line_number_width = line_number.len();
                let content_len = line.chars().count();
                let total_used = line_number_width + content_len;
                let padding_needed = content_width.saturating_sub(total_used);

                Line::from(vec![
                    Span::styled(
                        line_number,
                        Style::default()
                            .fg(theme.line_numbers_current)
                            .bg(theme.code_background_current)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}{}", line, " ".repeat(padding_needed)),
                        line_style
                            .bg(theme.code_background_current)
                            .fg(theme.code_foreground_current),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::styled(line_number, Style::default().fg(theme.line_numbers)),
                    Span::styled(line, line_style),
                ])
            }
        })
        .collect();

    let paragraph = Paragraph::new(content_lines)
        .block(block)
        .scroll((0, app.inspector.scroll_horizontal));

    frame.render_widget(paragraph, area);
}

/// Basic syntax highlighting based on file content and extension
fn get_line_style(line: &str, file_path: &Option<std::path::PathBuf>) -> Style {
    let theme = get_theme();
    let trimmed = line.trim();

    // Comments (works for most languages)
    if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("/*") {
        return Style::default().fg(theme.syntax_comment);
    }

    // Strings (basic detection)
    if trimmed.contains('"') || trimmed.contains('\'') {
        return Style::default().fg(theme.syntax_string);
    }

    // Keywords based on file extension
    if let Some(path) = file_path {
        if let Some(extension) = path.extension() {
            match extension.to_string_lossy().as_ref() {
                "rs" => {
                    if trimmed.starts_with("use ")
                        || trimmed.starts_with("pub ")
                        || trimmed.starts_with("fn ")
                        || trimmed.starts_with("struct ")
                        || trimmed.starts_with("enum ")
                        || trimmed.starts_with("impl ")
                    {
                        return Style::default()
                            .fg(theme.syntax_keyword)
                            .add_modifier(ratatui::style::Modifier::BOLD);
                    }
                }
                "js" | "ts" => {
                    if trimmed.starts_with("function ")
                        || trimmed.starts_with("const ")
                        || trimmed.starts_with("let ")
                        || trimmed.starts_with("var ")
                        || trimmed.starts_with("import ")
                        || trimmed.starts_with("export ")
                    {
                        return Style::default()
                            .fg(theme.syntax_keyword)
                            .add_modifier(ratatui::style::Modifier::BOLD);
                    }
                }
                "py" => {
                    if trimmed.starts_with("def ")
                        || trimmed.starts_with("class ")
                        || trimmed.starts_with("import ")
                        || trimmed.starts_with("from ")
                    {
                        return Style::default()
                            .fg(theme.syntax_keyword)
                            .add_modifier(ratatui::style::Modifier::BOLD);
                    }
                }
                _ => {}
            }
        }
    }

    // Default style
    Style::default().fg(theme.code_default)
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let theme = get_theme();
    let status_text = if app.ui.is_loading {
        format!("Loading... | {}", app.ui.status_message)
    } else {
        app.ui.status_message.clone()
    };

    let help_text = match app.ui.active_panel {
        PanelFocus::Navigator => "Tab: Switch panel | 1/2/3: Direct panel focus | []: Older/Younger commit | /: Search | ↑↓: Navigate | →←: Expand/Collapse",
        PanelFocus::History => "Tab: Switch panel | 1/2/3: Direct panel focus | []: Older/Younger commit | ↑↓: Navigate | Enter: Select commit",
        PanelFocus::Inspector => "Tab: Switch panel | 1/2/3: Direct panel focus | []: Older/Younger commit | ↑↓: Navigate | p: Previous change | n: Next change | d: Toggle diff",
    };

    let status_line = Line::from(vec![
        Span::styled(status_text, Style::default().fg(theme.status_bar_fg)),
        Span::raw(" | "),
        Span::styled(help_text, Style::default().fg(theme.status_help_text)),
    ]);

    let paragraph = Paragraph::new(status_line).style(Style::default().bg(theme.status_bar_bg));

    frame.render_widget(paragraph, area);
}
