use sqlx::AnyPool;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::db::DbType;
use std::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Tables,
    Query,
    Grid,
}

pub struct TabState {
    pub name: String,
    pub connection_uri: String,
    pub db_type: DbType,
    pub connection_pool: Option<AnyPool>,
    pub connection_status: String, // "Disconnected", "Connecting...", "Connected", "Failed: <error>"
    pub tables: Vec<String>,
    pub active_table: Option<String>,
    pub selected_view: ViewMode,
    pub query_editor_content: String,
    pub data_grid_headers: Vec<String>,
    pub raw_rows: Vec<Vec<String>>,
    pub data_grid_rows: Vec<Vec<String>>,
    pub grid_scroll_row: usize,
    pub search_input: String,
    pub in_search_mode: bool,
    pub editing_cell: Option<(usize, usize)>, // (row_idx, col_idx)
    pub edit_input: String,
    pub error: Option<String>,
    pub ssh_child: Option<std::process::Child>,
    pub ssh_local_port: Option<u16>,
}

impl TabState {
    pub fn new(_id: &str, name: &str, connection_uri: &str) -> Self {
        let db_type = crate::db::detect_db_type(connection_uri);
        Self {
            name: name.to_string(),
            connection_uri: connection_uri.to_string(),
            db_type,
            connection_pool: None,
            connection_status: "Disconnected".to_string(),
            tables: Vec::new(),
            active_table: None,
            selected_view: ViewMode::Tables,
            query_editor_content: String::new(),
            data_grid_headers: Vec::new(),
            raw_rows: Vec::new(),
            data_grid_rows: Vec::new(),
            grid_scroll_row: 0,
            search_input: String::new(),
            in_search_mode: false,
            editing_cell: None,
            edit_input: String::new(),
            error: None,
            ssh_child: None,
            ssh_local_port: None,
        }
    }

    pub fn apply_fuzzy_search(&mut self) {
        if !self.in_search_mode || self.search_input.is_empty() {
            self.data_grid_rows = self.raw_rows.clone();
            return;
        }

        let matcher = SkimMatcherV2::default();
        let query = self.search_input.to_lowercase();

        let mut scored_rows: Vec<(i64, Vec<String>)> = self.raw_rows
            .iter()
            .filter_map(|row| {
                let mut best_score = None;
                for cell in row {
                    if let Some(score) = matcher.fuzzy_match(&cell.to_lowercase(), &query) {
                        best_score = Some(best_score.unwrap_or(0).max(score));
                    }
                }
                best_score.map(|score| (score, row.clone()))
            })
            .collect();

        // Sort by matched score descending
        scored_rows.sort_by(|a, b| b.0.cmp(&a.0));
        self.data_grid_rows = scored_rows.into_iter().map(|item| item.1).collect();
        self.grid_scroll_row = 0;
    }
}

impl Drop for TabState {
    fn drop(&mut self) {
        if let Some(mut child) = self.ssh_child.take() {
            let _ = child.kill();
        }
    }
}

pub enum AppEvent {
    DbConnected {
        tab_idx: usize,
        pool: AnyPool,
        tables: Vec<String>,
        ssh_child: Option<std::process::Child>,
        ssh_port: Option<u16>,
    },
    DbConnectionFailed {
        tab_idx: usize,
        error: String,
    },
    QueryExecuted {
        tab_idx: usize,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    QueryFailed {
        tab_idx: usize,
        error: String,
    },
    UpdateExecuted {
        tab_idx: usize,
        _rows_affected: u64,
    },
    UpdateFailed {
        tab_idx: usize,
        error: String,
    },
    AiGeneratedSql {
        sql: String,
    },
    AiGenerationFailed {
        error: String,
    },
    AiExplainedSql {
        explanation: String,
    },
    AiExplainFailed {
        error: String,
    },
    ExportSuccess {
        path: String,
    },
    ExportFailed {
        error: String,
    },
}

pub struct App {
    pub tabs: Vec<TabState>,
    pub active_tab_idx: usize,
    pub sidebar_width: u16,
    pub show_ai_modal: bool,
    pub ai_query_input: String,
    pub show_connect_modal: bool,
    pub connect_input: String,
    pub pending_query: bool,
    pub should_quit: bool,
    
    // New Feature States
    pub show_history_modal: bool,
    pub query_history: Vec<String>,
    pub selected_history_idx: usize,
    
    pub show_export_modal: bool,
    pub export_format: String, // "CSV", "JSON", "Markdown"
    pub export_path_input: String,
    pub export_status: Option<Result<String, String>>, // Some(Ok(path)) or Some(Err(msg))
    
    pub show_ai_explain_modal: bool,
    pub ai_explain_output: String,
    
    pub bookmarks: Vec<crate::bookmarks::Bookmark>,
    pub selected_bookmark_idx: usize,
    pub show_add_bookmark_modal: bool,
    pub bookmark_name_input: String,
    pub bookmark_uri_input: String,
    pub bookmark_ssh_host: String,
    pub bookmark_ssh_port: String,
    pub bookmark_ssh_user: String,
    pub bookmark_ssh_key_path: String,
    pub bookmark_form_focus: usize, // 0: Name, 1: URI, 2: SSH Host, 3: SSH Port, 4: SSH User, 5: SSH Key Path, 6: Cancel, 7: Save
    
    pub g_pressed: bool,
}

impl App {
    pub fn new() -> Self {
        let bookmarks = crate::bookmarks::load_bookmarks().unwrap_or_default();
        let query_history = load_history();

        let mut tabs = vec![
            TabState::new(
                "local-sqlite",
                "Local SQLite",
                "sqlite://omnidb_test.db",
            ),
        ];

        // Load custom bookmarks into tabs initially if they exist
        for (idx, b) in bookmarks.iter().enumerate() {
            tabs.push(TabState::new(
                &format!("bookmark-{}", idx),
                &b.name,
                &b.connection_uri,
            ));
        }

        Self {
            tabs,
            active_tab_idx: 0,
            sidebar_width: 24,
            show_ai_modal: false,
            ai_query_input: String::new(),
            show_connect_modal: false,
            connect_input: String::new(),
            pending_query: false,
            should_quit: false,
            
            show_history_modal: false,
            query_history,
            selected_history_idx: 0,
            
            show_export_modal: false,
            export_format: "CSV".to_string(),
            export_path_input: "./export.csv".to_string(),
            export_status: None,
            
            show_ai_explain_modal: false,
            ai_explain_output: String::new(),
            
            bookmarks,
            selected_bookmark_idx: 0,
            show_add_bookmark_modal: false,
            bookmark_name_input: String::new(),
            bookmark_uri_input: String::new(),
            bookmark_ssh_host: String::new(),
            bookmark_ssh_port: "22".to_string(),
            bookmark_ssh_user: String::new(),
            bookmark_ssh_key_path: String::new(),
            bookmark_form_focus: 0,
            
            g_pressed: false,
        }
    }

    pub fn active_tab(&self) -> &TabState {
        &self.tabs[self.active_tab_idx]
    }

    pub fn active_tab_mut(&mut self) -> &mut TabState {
        &mut self.tabs[self.active_tab_idx]
    }

    pub fn next_tab(&mut self) {
        self.active_tab_idx = (self.active_tab_idx + 1) % self.tabs.len();
    }

    pub fn prev_tab(&mut self) {
        if self.active_tab_idx == 0 {
            self.active_tab_idx = self.tabs.len() - 1;
        } else {
            self.active_tab_idx -= 1;
        }
    }
}

pub fn load_history() -> Vec<String> {
    let mut path = crate::bookmarks::get_config_dir();
    path.push("history.txt");
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            return content.lines()
                .map(|s| s.to_string())
                .filter(|s| !s.trim().is_empty())
                .collect();
        }
    }
    vec![]
}

pub fn log_history(query: &str) {
    let mut path = crate::bookmarks::get_config_dir();
    let _ = fs::create_dir_all(&path);
    path.push("history.txt");
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        use std::io::Write;
        let _ = writeln!(file, "{}", query.replace('\n', " "));
    }
}
