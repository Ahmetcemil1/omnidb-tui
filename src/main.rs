use std::{io, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc::{self, Sender};

mod app;
mod db;
mod ai;
mod ui;
mod bookmarks;
mod solana;
mod redis;
mod mongo;
mod ethereum;

use app::{App, AppEvent, ViewMode, log_history};
use db::DbType;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Terminal configuration (Raw Mode & Alternate Screen)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 2. Initialize App State
    let mut app = App::new();

    // 3. Create async channel for background task events
    let (tx, mut rx) = mpsc::channel::<AppEvent>(100);

    // 4. Trigger initial connection for default SQLite database in background
    let tx_clone = tx.clone();
    let sqlite_uri = app.tabs[0].connection_uri.clone();
    app.tabs[0].connection_status = "Connecting...".to_string();
    tokio::spawn(async move {
        connect_database(0, sqlite_uri, DbType::Sqlite, None, tx_clone).await;
    });

    // 5. Main Event Loop
    let mut should_exit = false;
    while !should_exit {
        // Draw TUI frame
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Asynchronously poll for crossterm events and receive channel events
        tokio::select! {
            // Receive background events from channel
            Some(event) = rx.recv() => {
                handle_app_event(&mut app, event);
            }
            // Poll for terminal keyboard input events
            _ = tokio::time::sleep(Duration::from_millis(15)) => {
                if event::poll(Duration::ZERO)? {
                    if let Event::Key(key) = event::read()? {
                        if handle_key_input(&mut app, key, tx.clone()).await? {
                            should_exit = true;
                        }
                    }
                }
            }
        }
        
        if app.should_quit {
            should_exit = true;
        }
    }

    // 6. Restore Terminal State
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

/// Helper function to asynchronously connect to database and retrieve initial tables list
async fn connect_database(
    tab_idx: usize,
    mut uri: String,
    db_type: DbType,
    ssh_tunnel: Option<bookmarks::SshTunnelConfig>,
    tx: Sender<AppEvent>,
) {
    let mut ssh_child: Option<std::process::Child> = None;
    let mut local_port_opt: Option<u16> = None;

    if let Some(ssh_conf) = ssh_tunnel {
        let local_port = bookmarks::get_free_port();
        local_port_opt = Some(local_port);

        let default_port = match db_type {
            DbType::Postgres => 5432,
            DbType::MySql => 3306,
            DbType::Sqlite => 0,
            DbType::Solana => 8899,
            DbType::Redis => 6379,
            DbType::Mongo => 27017,
            DbType::Ethereum => 8545,
        };
        let (db_host, db_port) = bookmarks::parse_host_port_from_uri(&uri, default_port);

        let mut cmd = std::process::Command::new("ssh");
        cmd.arg("-o").arg("StrictHostKeyChecking=no")
           .arg("-N")
           .arg("-L")
           .arg(format!("127.0.0.1:{}:{}:{}", local_port, db_host, db_port))
           .arg(format!("{}@{}", ssh_conf.user, ssh_conf.host))
           .arg("-p").arg(ssh_conf.port.to_string());
        
        if !ssh_conf.key_path.is_empty() {
            cmd.arg("-i").arg(&ssh_conf.key_path);
        }

        match cmd.spawn() {
            Ok(child) => {
                ssh_child = Some(child);
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                uri = bookmarks::rewrite_uri_for_ssh(&uri, local_port);
            }
            Err(e) => {
                let _ = tx.send(AppEvent::DbConnectionFailed {
                    tab_idx,
                    error: format!("Failed to spawn SSH tunnel: {}", e),
                }).await;
                return;
            }
        }
    }

    if db_type == DbType::Solana {
        let url = if uri.starts_with("solana://") {
            let host = &uri[9..];
            if host == "localhost" || host.starts_with("localhost:") || host.starts_with("127.0.0.1") {
                format!("http://{}", host)
            } else {
                format!("https://{}", host)
            }
        } else {
            uri.clone()
        };
        
        let client = reqwest::Client::new();
        let health_req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getHealth"
        });
        
        let is_healthy = match client.post(&url).json(&health_req).send().await {
            Ok(res) => res.status().is_success(),
            Err(_) => false,
        };
        
        if is_healthy {
            sqlx::any::install_default_drivers();
            let pool = sqlx::any::AnyPoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            let tables = vec![
                "Account Info".to_string(),
                "Recent Transactions".to_string(),
                "Borsh IDL Parser".to_string()
            ];
            let _ = tx.send(AppEvent::DbConnected {
                tab_idx,
                pool,
                tables,
                ssh_child: None,
                ssh_port: None,
            }).await;
        } else {
            let _ = tx.send(AppEvent::DbConnectionFailed {
                tab_idx,
                error: "Failed to connect to Solana RPC endpoint: Connection refused or host unreachable".to_string()
            }).await;
        }
        return;
    }
    if db_type == DbType::Redis {
        sqlx::any::install_default_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let tables = vec!["keys".to_string(), "info".to_string()];
        let _ = tx.send(AppEvent::DbConnected {
            tab_idx,
            pool,
            tables,
            ssh_child: None,
            ssh_port: None,
        }).await;
        return;
    }

    if db_type == DbType::Mongo {
        sqlx::any::install_default_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let tables = vec!["collections".to_string(), "stats".to_string()];
        let _ = tx.send(AppEvent::DbConnected {
            tab_idx,
            pool,
            tables,
            ssh_child: None,
            ssh_port: None,
        }).await;
        return;
    }

    if db_type == DbType::Ethereum {
        sqlx::any::install_default_drivers();
        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let tables = vec!["Account Info".to_string(), "Latest Block".to_string(), "ERC-20 Tokens".to_string()];
        let _ = tx.send(AppEvent::DbConnected {
            tab_idx,
            pool,
            tables,
            ssh_child: None,
            ssh_port: None,
        }).await;
        return;
    }

    match db::connect(&uri).await {
        Ok(pool) => {
            match db::get_tables(&pool, db_type).await {
                Ok(tables) => {
                    let _ = tx.send(AppEvent::DbConnected {
                        tab_idx,
                        pool,
                        tables,
                        ssh_child,
                        ssh_port: local_port_opt,
                    }).await;
                }
                Err(e) => {
                    if let Some(mut child) = ssh_child {
                        let _ = child.kill();
                    }
                    let _ = tx.send(AppEvent::DbConnectionFailed { tab_idx, error: e.to_string() }).await;
                }
            }
        }
        Err(e) => {
            if let Some(mut child) = ssh_child {
                let _ = child.kill();
            }
            let _ = tx.send(AppEvent::DbConnectionFailed { tab_idx, error: e.to_string() }).await;
        }
    }
}

/// Helper function to run query in the background
async fn run_query(tab_idx: usize, pool: sqlx::AnyPool, sql: String, tx: Sender<AppEvent>) {
    match db::execute_query(&pool, &sql).await {
        Ok((headers, rows)) => {
            let _ = tx.send(AppEvent::QueryExecuted { tab_idx, headers, rows }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::QueryFailed { tab_idx, error: e.to_string() }).await;
        }
    }
}

async fn run_solana_query(tab_idx: usize, uri: String, command: String, tx: Sender<AppEvent>) {
    match solana::execute_solana_command(&uri, &command).await {
        Ok((headers, rows)) => {
            let _ = tx.send(AppEvent::QueryExecuted { tab_idx, headers, rows }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::QueryFailed { tab_idx, error: e.to_string() }).await;
        }
    }
}

async fn run_redis_query(tab_idx: usize, uri: String, command: String, tx: Sender<AppEvent>) {
    match redis::execute_redis_command(&uri, &command).await {
        Ok((headers, rows)) => {
            let _ = tx.send(AppEvent::QueryExecuted { tab_idx, headers, rows }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::QueryFailed { tab_idx, error: e.to_string() }).await;
        }
    }
}

async fn run_mongo_query(tab_idx: usize, uri: String, command: String, tx: Sender<AppEvent>) {
    match mongo::execute_mongo_command(&uri, &command).await {
        Ok((headers, rows)) => {
            let _ = tx.send(AppEvent::QueryExecuted { tab_idx, headers, rows }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::QueryFailed { tab_idx, error: e.to_string() }).await;
        }
    }
}

async fn run_ethereum_query(tab_idx: usize, uri: String, command: String, tx: Sender<AppEvent>) {
    match ethereum::execute_ethereum_command(&uri, &command).await {
        Ok((headers, rows)) => {
            let _ = tx.send(AppEvent::QueryExecuted { tab_idx, headers, rows }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::QueryFailed { tab_idx, error: e.to_string() }).await;
        }
    }
}

/// Helper function to run inline update in the background
async fn run_update(tab_idx: usize, pool: sqlx::AnyPool, sql: String, tx: Sender<AppEvent>) {
    match db::execute_update(&pool, &sql).await {
        Ok(rows_affected) => {
            let _ = tx.send(AppEvent::UpdateExecuted { tab_idx, _rows_affected: rows_affected }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::UpdateFailed { tab_idx, error: e.to_string() }).await;
        }
    }
}

/// Helper function to generate SQL using AI in the background
async fn run_ai(prompt: String, schema: String, db_type_str: String, tx: Sender<AppEvent>) {
    match ai::generate_sql(&prompt, &schema, &db_type_str).await {
        Ok(sql) => {
            let _ = tx.send(AppEvent::AiGeneratedSql { sql }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::AiGenerationFailed { error: e.to_string() }).await;
        }
    }
}

/// Helper function to explain query using AI in the background
async fn run_ai_explain(prompt: String, schema: String, db_type_str: String, tx: Sender<AppEvent>) {
    match ai::explain_sql(&prompt, &schema, &db_type_str).await {
        Ok(explanation) => {
            let _ = tx.send(AppEvent::AiExplainedSql { explanation }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::AiExplainFailed { error: e.to_string() }).await;
        }
    }
}

/// Helper function to diagnose Solana transaction errors using AI in the background
pub async fn _run_ai_diagnose(logs: String, err_msg: String, tx: Sender<AppEvent>) {
    match ai::diagnose_tx_error(&logs, &err_msg).await {
        Ok(explanation) => {
            let _ = tx.send(AppEvent::AiExplainedSql { explanation }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::AiExplainFailed { error: e.to_string() }).await;
        }
    }
}

/// Helper function to summarize IDL architecture using AI in the background
pub async fn _run_ai_idl_summarize(idl_json: String, tx: Sender<AppEvent>) {
    match ai::summarize_idl(&idl_json).await {
        Ok(explanation) => {
            let _ = tx.send(AppEvent::AiExplainedSql { explanation }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::AiExplainFailed { error: e.to_string() }).await;
        }
    }
}

/// Helper function to export data in the background
async fn run_export(
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    format: String,
    path: String,
    tx: Sender<AppEvent>,
) {
    let path_for_write = path.clone();
    let res = tokio::task::spawn_blocking(move || -> Result<()> {
        let mut content = String::new();
        match format.as_str() {
            "CSV" => {
                let escape_csv = |s: &str| {
                    if s.contains(',') || s.contains('"') || s.contains('\n') {
                        format!("\"{}\"", s.replace('"', "\"\""))
                    } else {
                        s.to_string()
                    }
                };
                content.push_str(&headers.iter().map(|h| escape_csv(h)).collect::<Vec<_>>().join(","));
                content.push('\n');
                for row in rows {
                    content.push_str(&row.iter().map(|c| escape_csv(c)).collect::<Vec<_>>().join(","));
                    content.push('\n');
                }
            }
            "JSON" => {
                let mut json_rows = Vec::new();
                for row in rows {
                    let mut map = serde_json::Map::new();
                    for (idx, header) in headers.iter().enumerate() {
                        if idx < row.len() {
                            map.insert(header.clone(), serde_json::Value::String(row[idx].clone()));
                        } else {
                            map.insert(header.clone(), serde_json::Value::Null);
                        }
                    }
                    json_rows.push(serde_json::Value::Object(map));
                }
                content = serde_json::to_string_pretty(&json_rows)?;
            }
            "Markdown" => {
                if headers.is_empty() {
                    return Ok(());
                }
                content.push_str("| ");
                content.push_str(&headers.join(" | "));
                content.push_str(" |\n");

                content.push_str("| ");
                content.push_str(&headers.iter().map(|_| "---").collect::<Vec<_>>().join(" | "));
                content.push_str(" |\n");

                for row in rows {
                    content.push_str("| ");
                    content.push_str(&row.join(" | "));
                    content.push_str(" |\n");
                }
            }
            _ => return Err(anyhow::anyhow!("Unknown format")),
        }
        std::fs::write(&path_for_write, content)?;
        Ok(())
    }).await;

    match res {
        Ok(Ok(())) => {
            let _ = tx.send(AppEvent::ExportSuccess { path }).await;
        }
        Ok(Err(e)) => {
            let _ = tx.send(AppEvent::ExportFailed { error: e.to_string() }).await;
        }
        Err(e) => {
            let _ = tx.send(AppEvent::ExportFailed { error: e.to_string() }).await;
        }
    }
}

/// Update App State based on background event results
fn handle_app_event(app: &mut App, event: AppEvent) {
    match event {
        AppEvent::DbConnected { tab_idx, pool, tables, ssh_child, ssh_port } => {
            if let Some(tab) = app.tabs.get_mut(tab_idx) {
                tab.connection_status = "Connected".to_string();
                tab.connection_pool = Some(pool);
                tab.tables = tables;
                tab.ssh_child = ssh_child;
                tab.ssh_local_port = ssh_port;
                if !tab.tables.is_empty() {
                    tab.active_table = Some(tab.tables[0].clone());
                }
            }
        }
        AppEvent::DbConnectionFailed { tab_idx, error } => {
            if let Some(tab) = app.tabs.get_mut(tab_idx) {
                tab.connection_status = format!("Failed: {}", error);
            }
        }
        AppEvent::QueryExecuted { tab_idx, headers, rows } => {
            if let Some(tab) = app.tabs.get_mut(tab_idx) {
                tab.data_grid_headers = headers;
                tab.raw_rows = rows;
                tab.apply_fuzzy_search(); // Populate data_grid_rows
                tab.error = None;
            }
            app.pending_query = false;
        }
        AppEvent::QueryFailed { tab_idx, error } => {
            if let Some(tab) = app.tabs.get_mut(tab_idx) {
                tab.error = Some(error);
                tab.data_grid_headers.clear();
                tab.raw_rows.clear();
                tab.data_grid_rows.clear();
            }
            app.pending_query = false;
        }
        AppEvent::UpdateExecuted { tab_idx, .. } => {
            if let Some(tab) = app.tabs.get_mut(tab_idx) {
                if let Some((r_idx, c_idx)) = tab.editing_cell {
                    if r_idx < tab.data_grid_rows.len() && c_idx < tab.data_grid_rows[r_idx].len() {
                        let new_val = tab.edit_input.clone();
                        tab.data_grid_rows[r_idx][c_idx] = new_val.clone();
                        let matched_row = &tab.data_grid_rows[r_idx];
                        if let Some(raw_idx) = tab.raw_rows.iter().position(|r| r[0] == matched_row[0]) {
                            tab.raw_rows[raw_idx][c_idx] = new_val;
                        }
                    }
                }
                tab.editing_cell = None;
                tab.edit_input.clear();
                tab.error = None;
            }
        }
        AppEvent::UpdateFailed { tab_idx, error } => {
            if let Some(tab) = app.tabs.get_mut(tab_idx) {
                tab.error = Some(error);
                tab.editing_cell = None;
                tab.edit_input.clear();
            }
        }
        AppEvent::AiGeneratedSql { sql } => {
            let active_tab = app.active_tab_mut();
            active_tab.query_editor_content = sql;
            app.pending_query = false;
        }
        AppEvent::AiGenerationFailed { error } => {
            let active_tab = app.active_tab_mut();
            active_tab.error = Some(error);
            app.pending_query = false;
        }
        AppEvent::AiExplainedSql { explanation } => {
            app.ai_explain_output = explanation;
            app.show_ai_explain_modal = true;
            app.pending_query = false;
        }
        AppEvent::AiExplainFailed { error } => {
            app.ai_explain_output = format!("Failed to analyze query:\n\n{}", error);
            app.show_ai_explain_modal = true;
            app.pending_query = false;
        }
        AppEvent::ExportSuccess { path } => {
            app.export_status = Some(Ok(path));
            app.pending_query = false;
        }
        AppEvent::ExportFailed { error } => {
            app.export_status = Some(Err(error));
            app.pending_query = false;
        }
    }
}

/// Handle Keyboard User Input Events
async fn handle_key_input(app: &mut App, key: KeyEvent, tx: Sender<AppEvent>) -> Result<bool> {
    let tab_idx = app.active_tab_idx;

    // --- 1. CELL EDIT MODE ---
    if let Some((r_idx, c_idx)) = app.tabs[tab_idx].editing_cell {
        match key.code {
            KeyCode::Esc => {
                app.tabs[tab_idx].editing_cell = None;
                app.tabs[tab_idx].edit_input.clear();
            }
            KeyCode::Enter => {
                let tab = &app.tabs[tab_idx];
                if let Some(pool) = &tab.connection_pool {
                    let table_name = tab.active_table.clone().unwrap_or_default();
                    if r_idx < tab.data_grid_rows.len() && !table_name.is_empty() {
                        let pk_col = tab.data_grid_headers[0].clone();
                        let pk_val = tab.data_grid_rows[r_idx][0].clone();
                        let col_name = tab.data_grid_headers[c_idx].clone();
                        let new_val = tab.edit_input.replace('\'', "''");
                        
                        let esc_table = db::escape_identifier(&table_name, tab.db_type);
                        let esc_col = db::escape_identifier(&col_name, tab.db_type);
                        let esc_pk = db::escape_identifier(&pk_col, tab.db_type);

                        let sql = format!(
                            "UPDATE {} SET {} = '{}' WHERE {} = '{}';",
                            esc_table, esc_col, new_val, esc_pk, pk_val
                        );
                        
                        let pool_clone = pool.clone();
                        tokio::spawn(run_update(tab_idx, pool_clone, sql, tx));
                    }
                }
            }
            KeyCode::Char(c) => {
                app.tabs[tab_idx].edit_input.push(c);
            }
            KeyCode::Backspace => {
                app.tabs[tab_idx].edit_input.pop();
            }
            _ => {}
        }
        return Ok(false);
    }

    // --- 2. FUZZY SEARCH FILTER MODE ---
    if app.tabs[tab_idx].in_search_mode {
        match key.code {
            KeyCode::Esc => {
                app.tabs[tab_idx].in_search_mode = false;
                app.tabs[tab_idx].search_input.clear();
                app.tabs[tab_idx].apply_fuzzy_search();
            }
            KeyCode::Enter => {
                app.tabs[tab_idx].in_search_mode = false;
            }
            KeyCode::Char(c) => {
                app.tabs[tab_idx].search_input.push(c);
                app.tabs[tab_idx].apply_fuzzy_search();
            }
            KeyCode::Backspace => {
                app.tabs[tab_idx].search_input.pop();
                app.tabs[tab_idx].apply_fuzzy_search();
            }
            _ => {}
        }
        return Ok(false);
    }

    // --- 3. AI PROMPT MODAL MODE ---
    if app.show_ai_modal {
        match key.code {
            KeyCode::Esc => {
                app.show_ai_modal = false;
                app.ai_query_input.clear();
            }
            KeyCode::Enter => {
                if !app.ai_query_input.is_empty() {
                    app.pending_query = true;
                    let prompt = std::mem::take(&mut app.ai_query_input);
                    app.show_ai_modal = false;

                    let pool_opt = app.tabs[tab_idx].connection_pool.clone();
                    let db_type = app.tabs[tab_idx].db_type;
                    let db_type_str = format!("{:?}", db_type);
                    let tx_clone = tx.clone();

                    tokio::spawn(async move {
                        let schema = if let Some(ref pool) = pool_opt {
                            db::get_schema(pool, db_type).await.unwrap_or_default()
                        } else {
                            String::new()
                        };
                        run_ai(prompt, schema, db_type_str, tx_clone).await;
                    });
                }
            }
            KeyCode::Char(c) => {
                app.ai_query_input.push(c);
            }
            KeyCode::Backspace => {
                app.ai_query_input.pop();
            }
            _ => {}
        }
        return Ok(false);
    }

    // --- 4. AI EXPLAIN MODAL MODE ---
    if app.show_ai_explain_modal {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.show_ai_explain_modal = false;
                app.ai_explain_output.clear();
            }
            _ => {}
        }
        return Ok(false);
    }

    // --- 5. EXPORT MODAL MODE ---
    if app.show_export_modal {
        match key.code {
            KeyCode::Esc => {
                app.show_export_modal = false;
                app.export_status = None;
            }
            KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                // Toggle between format (CSV, JSON, Markdown) and path fields
                app.export_format = if app.export_format == "CSV" {
                    "JSON".to_string()
                } else if app.export_format == "JSON" {
                    "Markdown".to_string()
                } else {
                    "CSV".to_string()
                };
            }
            KeyCode::Char(c) => {
                app.export_path_input.push(c);
            }
            KeyCode::Backspace => {
                app.export_path_input.pop();
            }
            KeyCode::Enter => {
                app.pending_query = true;
                let format = app.export_format.clone();
                let path = app.export_path_input.clone();
                let headers = app.tabs[tab_idx].data_grid_headers.clone();
                let rows = app.tabs[tab_idx].data_grid_rows.clone();
                
                tokio::spawn(run_export(headers, rows, format, path, tx.clone()));
                app.show_export_modal = false;
            }
            _ => {}
        }
        return Ok(false);
    }

    // --- 6. QUERY HISTORY MODAL MODE ---
    if app.show_history_modal {
        match key.code {
            KeyCode::Esc => {
                app.show_history_modal = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !app.query_history.is_empty() && app.selected_history_idx > 0 {
                    app.selected_history_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !app.query_history.is_empty() && app.selected_history_idx < app.query_history.len() - 1 {
                    app.selected_history_idx += 1;
                }
            }
            KeyCode::Enter => {
                if !app.query_history.is_empty() && app.selected_history_idx < app.query_history.len() {
                    let query = app.query_history[app.selected_history_idx].clone();
                    app.tabs[tab_idx].query_editor_content = query;
                }
                app.show_history_modal = false;
            }
            _ => {}
        }
        return Ok(false);
    }

    // --- 7. ADD CONNECTION BOOKMARK MODAL MODE ---
    if app.show_add_bookmark_modal {
        match key.code {
            KeyCode::Esc => {
                app.show_add_bookmark_modal = false;
            }
            KeyCode::Tab | KeyCode::Down => {
                app.bookmark_form_focus = (app.bookmark_form_focus + 1) % 8;
            }
            KeyCode::Up => {
                if app.bookmark_form_focus == 0 {
                    app.bookmark_form_focus = 7;
                } else {
                    app.bookmark_form_focus -= 1;
                }
            }
            KeyCode::Char(c) => {
                match app.bookmark_form_focus {
                    0 => app.bookmark_name_input.push(c),
                    1 => app.bookmark_uri_input.push(c),
                    2 => app.bookmark_ssh_host.push(c),
                    3 => app.bookmark_ssh_port.push(c),
                    4 => app.bookmark_ssh_user.push(c),
                    5 => app.bookmark_ssh_key_path.push(c),
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                match app.bookmark_form_focus {
                    0 => { app.bookmark_name_input.pop(); }
                    1 => { app.bookmark_uri_input.pop(); }
                    2 => { app.bookmark_ssh_host.pop(); }
                    3 => { app.bookmark_ssh_port.pop(); }
                    4 => { app.bookmark_ssh_user.pop(); }
                    5 => { app.bookmark_ssh_key_path.pop(); }
                    _ => {}
                }
            }
            KeyCode::Enter => {
                if app.bookmark_form_focus == 6 {
                    // Cancel
                    app.show_add_bookmark_modal = false;
                } else if app.bookmark_form_focus == 7 || app.bookmark_form_focus == 1 {
                    // Save bookmark
                    if !app.bookmark_name_input.is_empty() && !app.bookmark_uri_input.is_empty() {
                        let ssh = if !app.bookmark_ssh_host.is_empty() {
                            let port = app.bookmark_ssh_port.parse::<u16>().unwrap_or(22);
                            Some(bookmarks::SshTunnelConfig {
                                host: app.bookmark_ssh_host.clone(),
                                port,
                                user: app.bookmark_ssh_user.clone(),
                                key_path: app.bookmark_ssh_key_path.clone(),
                                remote_db_host: "127.0.0.1".to_string(),
                                remote_db_port: 0,
                            })
                        } else {
                            None
                        };

                        let b = bookmarks::Bookmark {
                            name: app.bookmark_name_input.clone(),
                            connection_uri: app.bookmark_uri_input.clone(),
                            ssh_tunnel: ssh,
                        };

                        app.bookmarks.push(b);
                        let _ = bookmarks::save_bookmarks(&app.bookmarks);
                        
                        // Clear form
                        app.bookmark_name_input.clear();
                        app.bookmark_uri_input.clear();
                        app.bookmark_ssh_host.clear();
                        app.bookmark_ssh_port = "22".to_string();
                        app.bookmark_ssh_user.clear();
                        app.bookmark_ssh_key_path.clear();
                        app.show_add_bookmark_modal = false;
                    }
                }
            }
            _ => {}
        }
        return Ok(false);
    }

    // --- 8. CONNECTION BOOKMARKS / RAW URI MODAL MODE ---
    if app.show_connect_modal {
        match key.code {
            KeyCode::Esc => {
                app.show_connect_modal = false;
                app.connect_input.clear();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !app.bookmarks.is_empty() && app.selected_bookmark_idx > 0 {
                    app.selected_bookmark_idx -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !app.bookmarks.is_empty() && app.selected_bookmark_idx < app.bookmarks.len() - 1 {
                    app.selected_bookmark_idx += 1;
                }
            }
            KeyCode::Char('a') => {
                // Open add Connection bookmark form
                app.show_add_bookmark_modal = true;
                app.bookmark_form_focus = 0;
            }
            KeyCode::Char('d') => {
                // Delete selected connection bookmark
                if !app.bookmarks.is_empty() && app.selected_bookmark_idx < app.bookmarks.len() {
                    app.bookmarks.remove(app.selected_bookmark_idx);
                    let _ = bookmarks::save_bookmarks(&app.bookmarks);
                    if app.selected_bookmark_idx > 0 && app.selected_bookmark_idx >= app.bookmarks.len() {
                        app.selected_bookmark_idx = app.bookmarks.len() - 1;
                    }
                }
            }
            KeyCode::Char(c) => {
                // User typing URI
                app.connect_input.push(c);
            }
            KeyCode::Backspace => {
                app.connect_input.pop();
            }
            KeyCode::Enter => {
                if !app.connect_input.is_empty() {
                    // Connect via raw URI
                    let uri = std::mem::take(&mut app.connect_input);
                    app.show_connect_modal = false;

                    let db_type = db::detect_db_type(&uri);
                    let name = match db_type {
                        DbType::Postgres => "Custom PostgreSQL",
                        DbType::MySql => "Custom MySQL",
                        DbType::Sqlite => "Custom SQLite",
                        DbType::Solana => "Custom Solana RPC",
                        DbType::Redis => "Custom Redis Key-Store",
                        DbType::Mongo => "Custom MongoDB Document-Store",
                        DbType::Ethereum => "Custom Ethereum EVM RPC",
                    };

                    let new_idx = app.tabs.len();
                    let mut new_tab = app::TabState::new(&format!("custom-{}", new_idx), name, &uri);
                    new_tab.connection_status = "Connecting...".to_string();
                    app.tabs.push(new_tab);
                    app.active_tab_idx = new_idx;

                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        connect_database(new_idx, uri, db_type, None, tx_clone).await;
                    });
                } else if !app.bookmarks.is_empty() && app.selected_bookmark_idx < app.bookmarks.len() {
                    // Connect via selected bookmark
                    let bookmark = app.bookmarks[app.selected_bookmark_idx].clone();
                    app.show_connect_modal = false;

                    let db_type = db::detect_db_type(&bookmark.connection_uri);
                    let new_idx = app.tabs.len();
                    let mut new_tab = app::TabState::new(&format!("bookmark-tab-{}", new_idx), &bookmark.name, &bookmark.connection_uri);
                    new_tab.connection_status = "Connecting...".to_string();
                    app.tabs.push(new_tab);
                    app.active_tab_idx = new_idx;

                    let tx_clone = tx.clone();
                    tokio::spawn(async move {
                        connect_database(new_idx, bookmark.connection_uri, db_type, bookmark.ssh_tunnel, tx_clone).await;
                    });
                }
            }
            _ => {}
        }
        return Ok(false);
    }

    // --- 9. TRIGGER MODAL KEYBOARD SHORTCUTS ---
    // Ctrl + Space -> Open AI modal
    if key.code == KeyCode::Char(' ') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.show_ai_modal = true;
        return Ok(false);
    }

    // Ctrl + N -> Open connection modal
    if key.code == KeyCode::Char('n') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.show_connect_modal = true;
        app.selected_bookmark_idx = 0;
        return Ok(false);
    }

    // Ctrl + H -> Open history modal
    if key.code == KeyCode::Char('h') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.show_history_modal = true;
        app.selected_history_idx = if app.query_history.is_empty() { 0 } else { app.query_history.len() - 1 };
        return Ok(false);
    }

    // Ctrl + X -> Open export modal (only if in Grid view)
    if key.code == KeyCode::Char('x') && key.modifiers.contains(KeyModifiers::CONTROL) {
        let tab = &app.tabs[tab_idx];
        if matches!(tab.selected_view, ViewMode::Grid) && !tab.data_grid_rows.is_empty() {
            app.show_export_modal = true;
            app.export_status = None;
        }
        return Ok(false);
    }

    // Ctrl + E -> Explain query using AI
    if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::CONTROL) {
        let tab = &mut app.tabs[tab_idx];
        if matches!(tab.selected_view, ViewMode::Query) {
            let sql = tab.query_editor_content.clone();
            if !sql.trim().is_empty() {
                app.pending_query = true;
                let pool_opt = tab.connection_pool.clone();
                let db_type = tab.db_type;
                let db_type_str = format!("{:?}", db_type);
                let tx_clone = tx.clone();

                tokio::spawn(async move {
                    let schema = if let Some(ref pool) = pool_opt {
                        db::get_schema(pool, db_type).await.unwrap_or_default()
                    } else {
                        String::new()
                    };
                    run_ai_explain(sql, schema, db_type_str, tx_clone).await;
                });
            }
        }
        return Ok(false);
    }

    // Ctrl + R or Ctrl + Enter -> Run custom SQL query
    if (key.code == KeyCode::Char('r') && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        let tab = &mut app.tabs[tab_idx];
        if matches!(tab.selected_view, ViewMode::Query) {
            let sql = tab.query_editor_content.clone();
            if !sql.trim().is_empty() {
                app.pending_query = true;
                log_history(&sql);
                app.query_history.push(sql.replace('\n', " "));
                
                if tab.db_type == DbType::Solana {
                    let uri_clone = tab.connection_uri.clone();
                    tokio::spawn(run_solana_query(tab_idx, uri_clone, sql, tx.clone()));
                } else if tab.db_type == DbType::Redis {
                    let uri_clone = tab.connection_uri.clone();
                    tokio::spawn(run_redis_query(tab_idx, uri_clone, sql, tx.clone()));
                } else if tab.db_type == DbType::Mongo {
                    let uri_clone = tab.connection_uri.clone();
                    tokio::spawn(run_mongo_query(tab_idx, uri_clone, sql, tx.clone()));
                } else if tab.db_type == DbType::Ethereum {
                    let uri_clone = tab.connection_uri.clone();
                    tokio::spawn(run_ethereum_query(tab_idx, uri_clone, sql, tx.clone()));
                } else if let Some(pool) = &tab.connection_pool {
                    let pool_clone = pool.clone();
                    tokio::spawn(run_query(tab_idx, pool_clone, sql, tx.clone()));
                } else {
                    app.pending_query = false;
                }
            }
        }
        return Ok(false);
    }

    // --- 10. MAIN VIEW NAVIGATION SHORTCUTS ---
    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.should_quit = true;
            return Ok(true);
        }
        // Change Tabs
        KeyCode::Tab => {
            app.next_tab();
            let next_idx = app.active_tab_idx;
            let next_tab = &mut app.tabs[next_idx];
            if next_tab.connection_status == "Disconnected" {
                next_tab.connection_status = "Connecting...".to_string();
                let uri = next_tab.connection_uri.clone();
                let db_type = next_tab.db_type;
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    connect_database(next_idx, uri, db_type, None, tx_clone).await;
                });
            }
        }
        KeyCode::BackTab => {
            app.prev_tab();
            let next_idx = app.active_tab_idx;
            let next_tab = &mut app.tabs[next_idx];
            if next_tab.connection_status == "Disconnected" {
                next_tab.connection_status = "Connecting...".to_string();
                let uri = next_tab.connection_uri.clone();
                let db_type = next_tab.db_type;
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    connect_database(next_idx, uri, db_type, None, tx_clone).await;
                });
            }
        }
        // Switch Views (Panels)
        KeyCode::Right => {
            let tab = &mut app.tabs[tab_idx];
            tab.selected_view = match tab.selected_view {
                ViewMode::Tables => ViewMode::Query,
                ViewMode::Query => ViewMode::Grid,
                ViewMode::Grid => ViewMode::Tables,
            };
            app.g_pressed = false;
        }
        KeyCode::Left => {
            let tab = &mut app.tabs[tab_idx];
            tab.selected_view = match tab.selected_view {
                ViewMode::Tables => ViewMode::Grid,
                ViewMode::Query => ViewMode::Tables,
                ViewMode::Grid => ViewMode::Query,
            };
            app.g_pressed = false;
        }
        
        // Vim / Arrow Keys for Panel Gezinme (Only when focused)
        KeyCode::Up | KeyCode::Char('k') => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            match tab.selected_view {
                ViewMode::Tables => {
                    if let Some(ref current) = tab.active_table {
                        if let Some(idx) = tab.tables.iter().position(|t| t == current) {
                            if idx > 0 {
                                tab.active_table = Some(tab.tables[idx - 1].clone());
                            } else {
                                tab.active_table = Some(tab.tables[tab.tables.len() - 1].clone());
                            }
                        }
                    }
                }
                ViewMode::Grid => {
                    if tab.grid_scroll_row > 0 {
                        tab.grid_scroll_row -= 1;
                    }
                }
                _ => {
                    // If in Query panel, allow typing 'k'
                    if key.code == KeyCode::Char('k') {
                        tab.query_editor_content.push('k');
                    }
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            match tab.selected_view {
                ViewMode::Tables => {
                    if let Some(ref current) = tab.active_table {
                        if let Some(idx) = tab.tables.iter().position(|t| t == current) {
                            if idx < tab.tables.len() - 1 {
                                tab.active_table = Some(tab.tables[idx + 1].clone());
                            } else {
                                tab.active_table = Some(tab.tables[0].clone());
                            }
                        }
                    }
                }
                ViewMode::Grid => {
                    if tab.grid_scroll_row < tab.data_grid_rows.len() - 1 {
                        tab.grid_scroll_row += 1;
                    }
                }
                _ => {
                    if key.code == KeyCode::Char('j') {
                        tab.query_editor_content.push('j');
                    }
                }
            }
        }
        
        // Vim h / l horizontal scroll row or page jumps
        KeyCode::Char('h') => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            match tab.selected_view {
                ViewMode::Grid => {
                    // Page Up jump for vim keys
                    if tab.grid_scroll_row >= 10 {
                        tab.grid_scroll_row -= 10;
                    } else {
                        tab.grid_scroll_row = 0;
                    }
                }
                ViewMode::Query => {
                    tab.query_editor_content.push('h');
                }
                _ => {}
            }
        }
        KeyCode::Char('l') => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            match tab.selected_view {
                ViewMode::Grid => {
                    // Page Down jump for vim keys
                    if tab.grid_scroll_row + 10 < tab.data_grid_rows.len() {
                        tab.grid_scroll_row += 10;
                    } else if !tab.data_grid_rows.is_empty() {
                        tab.grid_scroll_row = tab.data_grid_rows.len() - 1;
                    }
                }
                ViewMode::Query => {
                    tab.query_editor_content.push('l');
                }
                _ => {}
            }
        }
        
        // Vim 'gg' and 'G' navigation keys
        KeyCode::Char('g') => {
            let tab = &mut app.tabs[tab_idx];
            match tab.selected_view {
                ViewMode::Query => {
                    tab.query_editor_content.push('g');
                }
                ViewMode::Grid => {
                    if app.g_pressed {
                        tab.grid_scroll_row = 0;
                        app.g_pressed = false;
                    } else {
                        app.g_pressed = true;
                    }
                }
                ViewMode::Tables => {
                    if app.g_pressed {
                        if !tab.tables.is_empty() {
                            tab.active_table = Some(tab.tables[0].clone());
                        }
                        app.g_pressed = false;
                    } else {
                        app.g_pressed = true;
                    }
                }
            }
        }
        KeyCode::Char('G') => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            match tab.selected_view {
                ViewMode::Query => {
                    tab.query_editor_content.push('G');
                }
                ViewMode::Grid => {
                    if !tab.data_grid_rows.is_empty() {
                        tab.grid_scroll_row = tab.data_grid_rows.len() - 1;
                    }
                }
                ViewMode::Tables => {
                    if !tab.tables.is_empty() {
                        tab.active_table = Some(tab.tables[tab.tables.len() - 1].clone());
                    }
                }
            }
        }

        // Action key inputs (Fetch table)
        KeyCode::Enter => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            match tab.selected_view {
                ViewMode::Tables => {
                    if let Some(ref table) = tab.active_table {
                        if tab.db_type == DbType::Solana {
                            let template = match table.as_str() {
                                "Account Info" => "vines1Yue2Cx6GPJ8zb8T27221KszrrK46j35cSL2uR".to_string(),
                                "Recent Transactions" => "history vines1Yue2Cx6GPJ8zb8T27221KszrrK46j35cSL2uR".to_string(),
                                "Borsh IDL Parser" => "idl ./path_to_idl.json vines1Yue2Cx6GPJ8zb8T27221KszrrK46j35cSL2uR MyAccountStruct".to_string(),
                                _ => "".to_string(),
                            };
                            tab.query_editor_content = template;
                            tab.selected_view = ViewMode::Query;
                        } else if let Some(pool) = &tab.connection_pool {
                            app.pending_query = true;
                            let sql = format!("SELECT * FROM {} LIMIT 500;", table);
                            tab.query_editor_content = sql.clone();
                            
                            let pool_clone = pool.clone();
                            tokio::spawn(run_query(tab_idx, pool_clone, sql, tx.clone()));
                        }
                    }
                }
                ViewMode::Query => {
                    tab.query_editor_content.push('\n');
                }
                _ => {}
            }
        }
        // Enter Fuzzy Search Filter Mod
        KeyCode::Char('/') => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            if matches!(tab.selected_view, ViewMode::Grid) && !tab.data_grid_rows.is_empty() {
                tab.in_search_mode = true;
            } else if matches!(tab.selected_view, ViewMode::Query) {
                tab.query_editor_content.push('/');
            }
        }
        // Start Inline Cell Edit
        KeyCode::Char('i') => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            if matches!(tab.selected_view, ViewMode::Grid) && !tab.data_grid_rows.is_empty() {
                let r_idx = tab.grid_scroll_row;
                let c_idx = if tab.data_grid_headers.len() > 1 { 1 } else { 0 };
                tab.editing_cell = Some((r_idx, c_idx));
                tab.edit_input = tab.data_grid_rows[r_idx][c_idx].clone();
            } else if matches!(tab.selected_view, ViewMode::Query) {
                tab.query_editor_content.push('i');
            }
        }
        // Editor character inputs
        KeyCode::Char(c) => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            if matches!(tab.selected_view, ViewMode::Query) {
                tab.query_editor_content.push(c);
            }
        }
        KeyCode::Backspace => {
            let tab = &mut app.tabs[tab_idx];
            app.g_pressed = false;
            if matches!(tab.selected_view, ViewMode::Query) {
                tab.query_editor_content.pop();
            }
        }
        _ => {}
    }

    Ok(false)
}
