use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
    Frame,
};

use crate::app::{App, ViewMode};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // 1. Vertical Layout: Tab Bar, Main Panel, Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tab Bar
            Constraint::Min(0),    // Main Panel
            Constraint::Length(1), // Footer
        ])
        .split(size);

    // --- 1. TAB BAR (TABS) RENDERING ---
    let tab_titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(idx, tab)| {
            let style = if idx == app.active_tab_idx {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            
            // Format status icon/text
            let status_indicator = match tab.connection_status.as_str() {
                "Connected" => "🟢",
                "Connecting..." => "🟡",
                s if s.starts_with("Failed") => "🔴",
                _ => "⚪",
            };

            Line::from(vec![
                Span::styled(format!(" [{}] ", idx + 1), style),
                Span::styled(&tab.name, style),
                Span::styled(format!(" {} ", status_indicator), Style::default()),
            ])
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" OmniDB TUI (v0.1.0) "),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED),
        )
        .select(app.active_tab_idx);
    f.render_widget(tabs, chunks[0]);

    // --- 2. MAIN PANEL (SIDEBAR + CONTENT) RENDERING ---
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(app.sidebar_width), // Sidebar
            Constraint::Min(0),                    // Right Content
        ])
        .split(chunks[1]);

    let active_tab = app.active_tab();

    // --- SIDEBAR (TABLES) ---
    let sidebar_is_active = matches!(active_tab.selected_view, ViewMode::Tables);
    let sidebar_border_color = if sidebar_is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let tables_items: Vec<Line> = if active_tab.tables.is_empty() {
        vec![Line::from(Span::styled(
            "   (No tables)",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        active_tab
            .tables
            .iter()
            .map(|t| {
                let is_selected = active_tab.active_table.as_ref() == Some(t);
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let prefix = if is_selected { "  ➔ " } else { "    " };
                Line::from(vec![
                    Span::styled(prefix, Style::default().fg(Color::Magenta)),
                    Span::styled(t, style),
                ])
            })
            .collect()
    };

    let sidebar_paragraph = Paragraph::new(tables_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(sidebar_border_color))
            .title(" 📁 Tables "),
    );
    f.render_widget(sidebar_paragraph, main_layout[0]);

    // --- RIGHT CONTENT (Query Editor + Data Grid) ---
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Query Editor
            Constraint::Min(0),    // Data Grid
        ])
        .split(main_layout[1]);

    // --- SQL QUERY EDITOR ---
    let query_is_active = matches!(active_tab.selected_view, ViewMode::Query);
    let query_border_color = if query_is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let query_paragraph = Paragraph::new(active_tab.query_editor_content.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(query_border_color))
            .title(" 📝 SQL Query Editor "),
    );
    f.render_widget(query_paragraph, right_chunks[0]);

    // --- DATA GRID / GRID STATUS PANEL ---
    let grid_is_active = matches!(active_tab.selected_view, ViewMode::Grid);
    let grid_border_color = if grid_is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    // Render connection error or loading status instead of data if appropriate
    if active_tab.connection_status.starts_with("Failed") {
        let err_block = Paragraph::new(format!(
            "\n  Connection Failed!\n\n  Error details: {}\n\n  Please check your connection string or settings.",
            active_tab.connection_status
        ))
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" ⚠️ Error "),
        );
        f.render_widget(err_block, right_chunks[1]);
    } else if active_tab.connection_status == "Connecting..." {
        let loading_block = Paragraph::new("\n  Connecting to database... Please wait.")
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" ⏳ Connecting "),
            );
        f.render_widget(loading_block, right_chunks[1]);
    } else if app.pending_query {
        let running_block = Paragraph::new("\n  Executing asynchronous database task...")
            .style(Style::default().fg(Color::Yellow))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" ⏳ Working "),
            );
        f.render_widget(running_block, right_chunks[1]);
    } else if let Some(ref err) = active_tab.error {
        let sql_err_block = Paragraph::new(format!(
            "\n  SQL Query Execution Failed!\n\n  Error details: {}",
            err
        ))
        .style(Style::default().fg(Color::Red))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red))
                .title(" ⚠️ SQL Error "),
        );
        f.render_widget(sql_err_block, right_chunks[1]);
    } else if active_tab.data_grid_headers.is_empty() {
        let empty_block = Paragraph::new("\n  No data to show.\n  Run a query or select a table from the sidebar to fetch data.")
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(grid_border_color))
                    .title(" 📊 Data Grid "),
            );
        f.render_widget(empty_block, right_chunks[1]);
    } else {
        // Render the actual data table
        let header_cells = active_tab.data_grid_headers.iter().map(|h| {
            Cell::from(h.as_str()).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells)
            .style(Style::default().bg(Color::Rgb(30, 30, 40)))
            .height(1);

        // Convert rows to cells, taking editing mode into account
        let rows = active_tab
            .data_grid_rows
            .iter()
            .enumerate()
            .map(|(r_idx, row_data)| {
                let cells = row_data.iter().enumerate().map(|(c_idx, cell_value)| {
                    if Some((r_idx, c_idx)) == active_tab.editing_cell {
                        // Styled as active edit box
                        Cell::from(format!("> {} <", active_tab.edit_input))
                            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    } else {
                        Cell::from(cell_value.as_str())
                    }
                });
                
                let row_style = if r_idx == active_tab.grid_scroll_row {
                    Style::default().bg(Color::Rgb(50, 50, 70))
                } else {
                    Style::default()
                };
                Row::new(cells).style(row_style).height(1)
            });

        // Compute column widths dynamically based on content length
        let col_count = active_tab.data_grid_headers.len();
        let mut widths = Vec::new();
        for col_idx in 0..col_count {
            let header_len = active_tab.data_grid_headers[col_idx].len();
            let mut max_len = header_len;
            for row in &active_tab.data_grid_rows {
                if col_idx < row.len() {
                    max_len = max_len.max(row[col_idx].len());
                }
            }
            // Give 3 characters of padding, minimum width of 10
            widths.push(Constraint::Min((max_len + 3).max(10) as u16));
        }

        let grid_title = if active_tab.in_search_mode {
            format!(" 📊 Data Grid [Fuzzy Filter: {}_] ", active_tab.search_input)
        } else {
            " 📊 Data Grid ".to_string()
        };

        let grid_table = Table::new(rows, &widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(grid_border_color))
                    .title(grid_title),
            );
        f.render_widget(grid_table, right_chunks[1]);
    }

    // --- 3. FOOTER (SHORTCUTS) RENDERING ---
    let shortcut_text = if active_tab.editing_cell.is_some() {
        " [Enter] Save Edit | [Esc] Cancel Edit | (Type value to modify cell)"
    } else {
        match active_tab.selected_view {
            ViewMode::Tables => " [Tab] Next Tab | [←/→] Panels | [↑/↓/j/k] Select Table | [Enter] Fetch | [Ctrl+N] Bookmarks | [Ctrl+H] History | [q] Quit",
            ViewMode::Query => " [Tab] Next Tab | [←/→] Panels | [Ctrl+Space] AI SQL | [Ctrl+E] AI Explain | [Ctrl+R] Run SQL | [Ctrl+N] Bookmarks | [Ctrl+H] History | [q] Quit",
            ViewMode::Grid => " [Tab] Next Tab | [←/→] Panels | [↑/↓/j/k] Select Row | [i] Edit Cell | [/] Fuzzy Filter | [Ctrl+X] Export | [Ctrl+N] Bookmarks | [Ctrl+H] History | [q] Quit",
        }
    };

    let footer_text = if active_tab.in_search_mode && active_tab.editing_cell.is_none() {
        " [Esc] Exit Filter | (Type to filter results instantly)"
    } else if app.show_connect_modal {
        " [Enter] Connect DB | [Esc] Cancel Modal | [a] Add Bookmark | [d] Delete | (Type URI or select Saved Bookmark)"
    } else if app.show_add_bookmark_modal {
        " [Tab] Cycle Fields | [Enter] Save (on Save button) | [Esc] Cancel Bookmark Form"
    } else {
        shortcut_text
    };

    let footer = Paragraph::new(footer_text).style(
        Style::default()
            .bg(Color::Rgb(15, 15, 20))
            .fg(Color::Gray),
    );
    f.render_widget(footer, chunks[2]);

    // --- OVERLAY MODALS ---
    if app.show_ai_modal {
        draw_ai_modal(f, app, size);
    }

    if app.show_connect_modal {
        draw_connect_modal(f, app, size);
    }

    if app.show_add_bookmark_modal {
        draw_add_bookmark_modal(f, app, size);
    }

    if app.show_export_modal {
        draw_export_modal(f, app, size);
    }

    if app.show_ai_explain_modal {
        draw_ai_explain_modal(f, app, size);
    }

    if app.show_history_modal {
        draw_history_modal(f, app, size);
    }
}

fn draw_ai_modal(f: &mut Frame, app: &App, size: Rect) {
    let modal_rect = Rect {
        x: size.width / 4,
        y: size.height / 3,
        width: size.width / 2,
        height: 6,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .title(" 🤖 AI Text-to-SQL (Ctrl+Space) ");

    let prompt_text = format!(
        "\n Prompt: {}\n\n (Type your query in natural language and press Enter to generate SQL)",
        app.ai_query_input
    );

    let paragraph = Paragraph::new(prompt_text).block(block);
    
    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear_block, modal_rect);
    f.render_widget(paragraph, modal_rect);
}

fn draw_connect_modal(f: &mut Frame, app: &App, size: Rect) {
    let modal_rect = Rect {
        x: size.width / 8,
        y: size.height / 5,
        width: size.width * 3 / 4,
        height: 14,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .title(" 🔌 Connect to Database (Ctrl+N) ");

    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear_block, modal_rect);
    f.render_widget(block, modal_rect);

    // Split connection modal into 2 columns: Saved Bookmarks on Left, Quick URI Input on Right
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Bookmarks List
            Constraint::Percentage(50), // Quick URI connect
        ])
        .split(modal_rect);

    // Left Column: Saved Bookmarks
    let mut bookmarks_items = Vec::new();
    if app.bookmarks.is_empty() {
        bookmarks_items.push(Line::from(Span::styled("  (No saved connection bookmarks)", Style::default().fg(Color::DarkGray))));
        bookmarks_items.push(Line::from(Span::styled("  Press [a] to add a new bookmark", Style::default().fg(Color::Cyan))));
    } else {
        for (idx, b) in app.bookmarks.iter().enumerate() {
            let is_selected = idx == app.selected_bookmark_idx;
            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_selected { " ➔ " } else { "   " };
            
            let ssh_suffix = if b.ssh_tunnel.is_some() { " 🔒 [SSH]" } else { "" };
            
            bookmarks_items.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(&b.name, style),
                Span::styled(ssh_suffix, Style::default().fg(Color::Cyan)),
            ]));
        }
    }

    let bookmarks_block = Paragraph::new(bookmarks_items)
        .block(Block::default().borders(Borders::RIGHT).border_style(Style::default().fg(Color::DarkGray)).title(" Saved Bookmarks (Use j/k, Enter to connect, d to delete) "));
    f.render_widget(bookmarks_block, columns[0]);

    // Right Column: Raw URI Input
    let body_text = format!(
        "\n Quick Connection URI:\n > {}\n\n Examples:\n - sqlite://omnidb_test.db\n - postgres://postgres@localhost:5432/postgres\n - mysql://root@localhost:3306/mysql\n\n Press [a] to Add Connection Bookmark",
        app.connect_input
    );
    let raw_connect_block = Paragraph::new(body_text)
        .block(Block::default().title(" Quick Connect (Type URI + Enter) "));
    f.render_widget(raw_connect_block, columns[1]);
}

fn draw_add_bookmark_modal(f: &mut Frame, app: &App, size: Rect) {
    let modal_rect = Rect {
        x: size.width / 6,
        y: size.height / 6,
        width: size.width * 2 / 3,
        height: 18,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .title(" 📁 Add Connection Bookmark ");

    let form_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title/Padding
            Constraint::Length(2), // Bookmark Name
            Constraint::Length(2), // Connection URI
            Constraint::Length(2), // SSH Host
            Constraint::Length(2), // SSH Port
            Constraint::Length(2), // SSH User
            Constraint::Length(2), // SSH Key Path
            Constraint::Length(3), // Buttons: Cancel / Save
        ])
        .split(modal_rect);

    let get_style = |idx: usize| {
        if app.bookmark_form_focus == idx {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear_block, modal_rect);
    f.render_widget(block, modal_rect);

    let name_p = Paragraph::new(format!("  Bookmark Name: {}", app.bookmark_name_input))
        .block(Block::default().borders(Borders::BOTTOM).border_style(get_style(0)));
    f.render_widget(name_p, form_layout[1]);

    let uri_p = Paragraph::new(format!("  DB Connection URI: {}", app.bookmark_uri_input))
        .block(Block::default().borders(Borders::BOTTOM).border_style(get_style(1)));
    f.render_widget(uri_p, form_layout[2]);

    let ssh_host_p = Paragraph::new(format!("  SSH Host (Optional): {}", app.bookmark_ssh_host))
        .block(Block::default().borders(Borders::BOTTOM).border_style(get_style(2)));
    f.render_widget(ssh_host_p, form_layout[3]);

    let ssh_port_p = Paragraph::new(format!("  SSH Port (Optional): {}", app.bookmark_ssh_port))
        .block(Block::default().borders(Borders::BOTTOM).border_style(get_style(3)));
    f.render_widget(ssh_port_p, form_layout[4]);

    let ssh_user_p = Paragraph::new(format!("  SSH User (Optional): {}", app.bookmark_ssh_user))
        .block(Block::default().borders(Borders::BOTTOM).border_style(get_style(4)));
    f.render_widget(ssh_user_p, form_layout[5]);

    let ssh_key_p = Paragraph::new(format!("  SSH Key Path (Optional): {}", app.bookmark_ssh_key_path))
        .block(Block::default().borders(Borders::BOTTOM).border_style(get_style(5)));
    f.render_widget(ssh_key_p, form_layout[6]);

    let btn_style_cancel = if app.bookmark_form_focus == 6 {
        Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };
    let btn_style_save = if app.bookmark_form_focus == 7 {
        Style::default().bg(Color::Green).fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let btn_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(form_layout[7]);

    let cancel_btn = Paragraph::new("\n    [ CANCEL ]    ").style(btn_style_cancel);
    let save_btn = Paragraph::new("\n    [ SAVE BOOKMARK ]    ").style(btn_style_save);

    f.render_widget(cancel_btn, btn_layout[0]);
    f.render_widget(save_btn, btn_layout[1]);
}

fn draw_export_modal(f: &mut Frame, app: &App, size: Rect) {
    let modal_rect = Rect {
        x: size.width / 4,
        y: size.height / 3,
        width: size.width / 2,
        height: 8,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        .title(" 📥 Export Data Grid ");

    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear_block, modal_rect);

    let format_status = format!(
        "\n  Export Format: [ {} ] (Press Tab or Down to cycle format)\n\n  Target Path: {}\n\n  Press Enter to Execute Export | Esc to Cancel",
        app.export_format, app.export_path_input
    );

    let status_text = match &app.export_status {
        Some(Ok(path)) => format!("\n  Export Succeeded!\n  File saved to: {}", path),
        Some(Err(err)) => format!("\n  Export Failed!\n  Error details: {}", err),
        None => format_status,
    };

    let paragraph = Paragraph::new(status_text).block(block);
    f.render_widget(paragraph, modal_rect);
}

fn draw_ai_explain_modal(f: &mut Frame, app: &App, size: Rect) {
    let modal_rect = Rect {
        x: size.width / 10,
        y: size.height / 10,
        width: size.width * 4 / 5,
        height: size.height * 4 / 5,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD))
        .title(" 🤖 AI Explain & Optimize (Ctrl+E) ");

    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear_block, modal_rect);
    f.render_widget(block, modal_rect);

    let content_rect = Rect {
        x: modal_rect.x + 2,
        y: modal_rect.y + 2,
        width: modal_rect.width - 4,
        height: modal_rect.height - 4,
    };

    let explanation_text = format!(
        "{}\n\n[ Press Enter or Esc to Close ]",
        app.ai_explain_output
    );
    let paragraph = Paragraph::new(explanation_text).wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(paragraph, content_rect);
}

fn draw_history_modal(f: &mut Frame, app: &App, size: Rect) {
    let modal_rect = Rect {
        x: size.width / 6,
        y: size.height / 5,
        width: size.width * 2 / 3,
        height: 12,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .title(" 📝 SQL Query History (Ctrl+H) ");

    let clear_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(clear_block, modal_rect);
    f.render_widget(block, modal_rect);

    let mut history_items = Vec::new();
    if app.query_history.is_empty() {
        history_items.push(Line::from(Span::styled("  (No queries run in history yet)", Style::default().fg(Color::DarkGray))));
    } else {
        for (idx, q) in app.query_history.iter().enumerate() {
            let is_selected = idx == app.selected_history_idx;
            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if is_selected { " ➔ " } else { "   " };
            
            let display_q = if q.len() > 60 {
                format!("{}...", &q[..57])
            } else {
                q.clone()
            };

            history_items.push(Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(display_q, style),
            ]));
        }
    }

    let paragraph = Paragraph::new(history_items)
        .block(Block::default().title(" Select Query (Use j/k or arrows, Enter to load) "));
    
    let inner_rect = Rect {
        x: modal_rect.x + 1,
        y: modal_rect.y + 1,
        width: modal_rect.width - 2,
        height: modal_rect.height - 2,
    };
    f.render_widget(paragraph, inner_rect);
}
