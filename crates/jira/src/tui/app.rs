use std::{collections::HashSet, time::Duration};

use crate::datetime::build_worklog_started;
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use jira_core::{
    config::config_file_path,
    model::{Issue, UpdateIssueRequest},
    JiraClient,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
        Wrap,
    },
    Frame, Terminal,
};
use std::{io, path::PathBuf};

// ─── Mode ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    List,
    View,
    Search,
    Transition,
    Help,
    ColumnPicker,
}

const AVAILABLE_COLUMNS: [ColumnKind; 8] = [
    ColumnKind::Key,
    ColumnKind::Type,
    ColumnKind::Priority,
    ColumnKind::Status,
    ColumnKind::Assignee,
    ColumnKind::Created,
    ColumnKind::Updated,
    ColumnKind::Summary,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
enum ColumnKind {
    Key,
    Type,
    Priority,
    Status,
    Assignee,
    Created,
    Updated,
    Summary,
}

impl ColumnKind {
    fn label(self) -> &'static str {
        match self {
            ColumnKind::Key => "Key",
            ColumnKind::Type => "Type",
            ColumnKind::Priority => "Priority",
            ColumnKind::Status => "Status",
            ColumnKind::Assignee => "Assignee",
            ColumnKind::Created => "Created",
            ColumnKind::Updated => "Updated",
            ColumnKind::Summary => "Summary",
        }
    }

    fn width(self) -> Constraint {
        match self {
            ColumnKind::Key => Constraint::Length(12),
            ColumnKind::Type => Constraint::Length(8),
            ColumnKind::Priority => Constraint::Length(8),
            ColumnKind::Status => Constraint::Length(14),
            ColumnKind::Assignee => Constraint::Length(16),
            ColumnKind::Created => Constraint::Length(11),
            ColumnKind::Updated => Constraint::Length(11),
            ColumnKind::Summary => Constraint::Min(15),
        }
    }

    fn cell(self, issue: &Issue) -> Cell<'static> {
        match self {
            ColumnKind::Key => {
                Cell::from(issue.key.clone()).style(Style::default().fg(Color::Cyan))
            }
            ColumnKind::Type => Cell::from(issue.issue_type.clone()),
            ColumnKind::Priority => {
                Cell::from(issue.priority.clone().unwrap_or_else(|| "-".into()))
            }
            ColumnKind::Status => Cell::from(issue.status.clone())
                .style(Style::default().fg(status_color(&issue.status))),
            ColumnKind::Assignee => {
                Cell::from(issue.assignee.clone().unwrap_or_else(|| "-".into()))
            }
            ColumnKind::Created => Cell::from(
                issue
                    .created
                    .get(..10)
                    .unwrap_or(&issue.created)
                    .to_string(),
            )
            .style(Style::default().fg(Color::DarkGray)),
            ColumnKind::Updated => Cell::from(
                issue
                    .updated
                    .get(..10)
                    .unwrap_or(&issue.updated)
                    .to_string(),
            )
            .style(Style::default().fg(Color::DarkGray)),
            ColumnKind::Summary => {
                let summary = if issue.summary.len() > 40 {
                    format!("{}…", &issue.summary[..39])
                } else {
                    issue.summary.clone()
                };
                Cell::from(summary)
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TuiPreferences {
    visible_columns: Vec<ColumnKind>,
}

impl Default for TuiPreferences {
    fn default() -> Self {
        Self {
            visible_columns: AVAILABLE_COLUMNS.to_vec(),
        }
    }
}

impl TuiPreferences {
    fn load() -> Self {
        let path = tui_preferences_path();
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Self>(&raw).ok())
            .map(|mut prefs| {
                prefs.normalize();
                prefs
            })
            .unwrap_or_default()
    }

    fn save(&self) -> Result<()> {
        let path = tui_preferences_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("failed to create TUI preferences directory")?;
        }
        let payload =
            serde_json::to_string_pretty(self).context("failed to serialize TUI preferences")?;
        std::fs::write(&path, payload)
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    fn normalize(&mut self) {
        let mut seen = HashSet::new();
        self.visible_columns.retain(|c| seen.insert(*c));
        self.visible_columns
            .retain(|c| AVAILABLE_COLUMNS.contains(c));
        if self.visible_columns.is_empty() {
            self.visible_columns = AVAILABLE_COLUMNS.to_vec();
        }
        if !self.visible_columns.contains(&ColumnKind::Summary) {
            self.visible_columns.push(ColumnKind::Summary);
        }
    }
}

fn tui_preferences_path() -> PathBuf {
    let mut path = config_file_path();
    path.set_file_name("tui-preferences.json");
    path
}

// ─── App state ───────────────────────────────────────────────────────────────

pub struct App {
    issues: Vec<Issue>,
    table_state: TableState,
    mode: Mode,
    base_url: String,
    /// Current active JQL
    jql: String,
    /// Default project key (from --project flag or config)
    default_project: Option<String>,
    /// Buffer while the user is typing in Search mode
    search_input: String,
    /// Cursor position (char index) within search_input
    search_cursor: usize,
    /// Bottom status bar message (text, is_error)
    status: Option<(String, bool)>,
    /// Transitions for the selected issue (id, name)
    transitions: Vec<(String, String)>,
    transition_list_state: ListState,
    transition_issue_key: String,
    visible_columns: Vec<ColumnKind>,
    column_picker_state: ListState,
}

// ─── Actions returned from key handling ──────────────────────────────────────

enum AppAction {
    None,
    Quit,
    Refresh,
    ExecuteSearch(String),
    FetchTransitions,
    ExecuteTransition(String, String), // (issue_key, transition_id)
    OpenBrowser,
    CreateIssue,
    EditIssue(String),
    AssignIssue(String),
    AddComment(String),
    AddWorklog(String),
    EditLabels(String),
    EditComponents(String),
    UploadAttachment(String),
    SaveColumnPreferences,
    ResetColumnPreferences,
}

// ─── App impl ────────────────────────────────────────────────────────────────

impl App {
    fn new(jql: String, base_url: String, default_project: Option<String>) -> Self {
        let prefs = TuiPreferences::load();
        let mut column_picker_state = ListState::default();
        column_picker_state.select(Some(0));

        Self {
            issues: Vec::new(),
            table_state: TableState::default(),
            mode: Mode::List,
            base_url,
            jql,
            default_project,
            search_input: String::new(),
            search_cursor: 0,
            status: None,
            transitions: Vec::new(),
            transition_list_state: ListState::default(),
            transition_issue_key: String::new(),
            visible_columns: prefs.visible_columns,
            column_picker_state,
        }
    }

    fn set_issues(&mut self, issues: Vec<Issue>) {
        self.issues = issues;
        if self.issues.is_empty() {
            self.table_state.select(None);
        } else {
            self.table_state.select(Some(0));
        }
    }

    fn selected_issue(&self) -> Option<&Issue> {
        self.table_state.selected().and_then(|i| self.issues.get(i))
    }

    fn selected_issue_key(&self) -> Option<String> {
        self.selected_issue().map(|i| i.key.clone())
    }

    fn next_issue(&mut self) {
        if self.issues.is_empty() {
            return;
        }
        let i = self
            .table_state
            .selected()
            .map(|i| (i + 1).min(self.issues.len() - 1))
            .unwrap_or(0);
        self.table_state.select(Some(i));
    }

    fn prev_issue(&mut self) {
        if self.issues.is_empty() {
            return;
        }
        let i = self
            .table_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.table_state.select(Some(i));
    }

    fn next_transition(&mut self) {
        if self.transitions.is_empty() {
            return;
        }
        let i = self
            .transition_list_state
            .selected()
            .map(|i| (i + 1).min(self.transitions.len() - 1))
            .unwrap_or(0);
        self.transition_list_state.select(Some(i));
    }

    fn prev_transition(&mut self) {
        if self.transitions.is_empty() {
            return;
        }
        let i = self
            .transition_list_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        self.transition_list_state.select(Some(i));
    }

    fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = Some((msg.into(), is_error));
    }

    fn clear_status(&mut self) {
        self.status = None;
    }

    /// Process a key press and return the action to take.
    fn handle_key(&mut self, code: KeyCode) -> AppAction {
        match &self.mode {
            Mode::List => self.handle_list_key(code),
            Mode::View => self.handle_view_key(code),
            Mode::Search => self.handle_search_key(code),
            Mode::Transition => self.handle_transition_key(code),
            Mode::ColumnPicker => self.handle_column_picker_key(code),
            Mode::Help => {
                self.mode = Mode::List;
                AppAction::None
            }
        }
    }

    fn handle_list_key(&mut self, code: KeyCode) -> AppAction {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => AppAction::Quit,
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_issue();
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.prev_issue();
                AppAction::None
            }
            KeyCode::Enter => {
                if self.selected_issue().is_some() {
                    self.mode = Mode::View;
                    self.clear_status();
                }
                AppAction::None
            }
            KeyCode::Char('r') => AppAction::Refresh,
            KeyCode::Char('t') => AppAction::FetchTransitions,
            KeyCode::Char('o') => AppAction::OpenBrowser,
            KeyCode::Char('/') => {
                self.search_input = self.jql.clone();
                self.search_cursor = self.search_input.chars().count();
                self.mode = Mode::Search;
                AppAction::None
            }
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
                AppAction::None
            }
            KeyCode::Char('C') => {
                self.mode = Mode::ColumnPicker;
                AppAction::None
            }
            // Edit actions
            KeyCode::Char('c') => AppAction::CreateIssue,
            KeyCode::Char('e') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::EditIssue(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('a') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::AssignIssue(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char(';') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::AddComment(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('w') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::AddWorklog(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('l') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::EditLabels(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('m') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::EditComponents(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('u') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::UploadAttachment(key)
                } else {
                    AppAction::None
                }
            }
            _ => AppAction::None,
        }
    }

    fn handle_view_key(&mut self, code: KeyCode) -> AppAction {
        match code {
            KeyCode::Char('q') | KeyCode::Esc | KeyCode::Backspace => {
                self.mode = Mode::List;
                AppAction::None
            }
            KeyCode::Char('t') => AppAction::FetchTransitions,
            KeyCode::Char('o') => AppAction::OpenBrowser,
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
                AppAction::None
            }
            KeyCode::Char('e') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::EditIssue(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('a') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::AssignIssue(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char(';') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::AddComment(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('w') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::AddWorklog(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('l') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::EditLabels(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('m') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::EditComponents(key)
                } else {
                    AppAction::None
                }
            }
            KeyCode::Char('u') => {
                if let Some(key) = self.selected_issue_key() {
                    AppAction::UploadAttachment(key)
                } else {
                    AppAction::None
                }
            }
            _ => AppAction::None,
        }
    }

    fn handle_search_key(&mut self, code: KeyCode) -> AppAction {
        match code {
            KeyCode::Esc => {
                self.mode = Mode::List;
                AppAction::None
            }
            KeyCode::Enter => {
                let jql = self.search_input.trim().to_string();
                self.mode = Mode::List;
                if jql.is_empty() {
                    AppAction::None
                } else {
                    AppAction::ExecuteSearch(jql)
                }
            }
            KeyCode::Left => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                }
                AppAction::None
            }
            KeyCode::Right => {
                if self.search_cursor < self.search_input.chars().count() {
                    self.search_cursor += 1;
                }
                AppAction::None
            }
            KeyCode::Home => {
                self.search_cursor = 0;
                AppAction::None
            }
            KeyCode::End => {
                self.search_cursor = self.search_input.chars().count();
                AppAction::None
            }
            KeyCode::Backspace => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    let byte_pos = self
                        .search_input
                        .char_indices()
                        .nth(self.search_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(self.search_input.len());
                    let char_len = self.search_input[byte_pos..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    self.search_input.drain(byte_pos..byte_pos + char_len);
                }
                AppAction::None
            }
            KeyCode::Delete => {
                let len = self.search_input.chars().count();
                if self.search_cursor < len {
                    let byte_pos = self
                        .search_input
                        .char_indices()
                        .nth(self.search_cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(self.search_input.len());
                    let char_len = self.search_input[byte_pos..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    self.search_input.drain(byte_pos..byte_pos + char_len);
                }
                AppAction::None
            }
            KeyCode::Char(c) => {
                let byte_pos = self
                    .search_input
                    .char_indices()
                    .nth(self.search_cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(self.search_input.len());
                self.search_input.insert(byte_pos, c);
                self.search_cursor += 1;
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn handle_transition_key(&mut self, code: KeyCode) -> AppAction {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::List;
                self.transitions.clear();
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_transition();
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.prev_transition();
                AppAction::None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.transition_list_state.selected() {
                    if let Some((id, _)) = self.transitions.get(idx) {
                        let action = AppAction::ExecuteTransition(
                            self.transition_issue_key.clone(),
                            id.clone(),
                        );
                        self.mode = Mode::List;
                        return action;
                    }
                }
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn handle_column_picker_key(&mut self, code: KeyCode) -> AppAction {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = Mode::List;
                AppAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self
                    .column_picker_state
                    .selected()
                    .map(|i| (i + 1).min(AVAILABLE_COLUMNS.len() - 1))
                    .unwrap_or(0);
                self.column_picker_state.select(Some(i));
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self
                    .column_picker_state
                    .selected()
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0);
                self.column_picker_state.select(Some(i));
                AppAction::None
            }
            KeyCode::Char(' ') => {
                if let Some(idx) = self.column_picker_state.selected() {
                    let column = AVAILABLE_COLUMNS[idx];
                    if self.visible_columns.contains(&column) {
                        if self.visible_columns.len() > 1 {
                            self.visible_columns.retain(|c| *c != column);
                        } else {
                            self.set_status("Keep at least one visible column", true);
                        }
                    } else {
                        self.visible_columns.push(column);
                    }
                }
                AppAction::None
            }
            KeyCode::Enter | KeyCode::Char('s') => {
                self.mode = Mode::List;
                AppAction::SaveColumnPreferences
            }
            KeyCode::Char('a') => {
                self.visible_columns = AVAILABLE_COLUMNS.to_vec();
                self.set_status("Selected all available columns", false);
                AppAction::None
            }
            KeyCode::Char('r') => {
                self.visible_columns = TuiPreferences::default().visible_columns;
                AppAction::ResetColumnPreferences
            }
            _ => AppAction::None,
        }
    }
}

// ─── Suspend / resume helpers ─────────────────────────────────────────────────

/// Leave alternate screen so we can render normal terminal prompts.
fn suspend_tui<B: ratatui::backend::Backend + io::Write>(terminal: &mut Terminal<B>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Re-enter alternate screen after prompts are done.
fn resume_tui<B: ratatui::backend::Backend + io::Write>(terminal: &mut Terminal<B>) -> Result<()> {
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;
    Ok(())
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub async fn run_tui(client: JiraClient, project: Option<String>) -> Result<()> {
    let jql = if let Some(proj) = &project {
        format!("project = {proj} ORDER BY updated DESC")
    } else {
        "assignee = currentUser() ORDER BY updated DESC".to_string()
    };

    let base_url = client.base_url().to_string();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(jql.clone(), base_url, project.clone());

    // Initial load
    app.set_status("Loading issues...", false);
    terminal.draw(|f| ui(f, &mut app))?;
    match client.search_issues(&jql, None, Some(50)).await {
        Ok(result) => {
            app.set_issues(result.issues);
            app.clear_status();
        }
        Err(e) => app.set_status(format!("Error: {e}"), true),
    }

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if !event::poll(Duration::from_millis(100))? {
            continue;
        }

        let Ok(Event::Key(key)) = event::read() else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }

        match app.handle_key(key.code) {
            AppAction::Quit => break,

            AppAction::Refresh => {
                let jql = app.jql.clone();
                app.set_status("Refreshing...", false);
                terminal.draw(|f| ui(f, &mut app))?;
                match client.search_issues(&jql, None, Some(50)).await {
                    Ok(result) => {
                        app.set_issues(result.issues);
                        app.clear_status();
                    }
                    Err(e) => app.set_status(format!("Error: {e}"), true),
                }
            }

            AppAction::ExecuteSearch(jql) => {
                app.set_status("Searching...", false);
                terminal.draw(|f| ui(f, &mut app))?;
                match client.search_issues(&jql, None, Some(50)).await {
                    Ok(result) => {
                        app.jql = jql;
                        app.set_issues(result.issues);
                        app.clear_status();
                    }
                    Err(e) => {
                        app.set_status(format!("JQL error: {e}"), true);
                    }
                }
            }

            AppAction::FetchTransitions => {
                if let Some(key) = app.selected_issue_key() {
                    app.set_status("Fetching transitions...", false);
                    terminal.draw(|f| ui(f, &mut app))?;
                    match client.get_transitions(&key).await {
                        Ok(raw) => {
                            let transitions: Vec<(String, String)> = raw
                                .iter()
                                .filter_map(|t| {
                                    let id = t.get("id")?.as_str()?.to_string();
                                    let name = t.get("name")?.as_str()?.to_string();
                                    Some((id, name))
                                })
                                .collect();

                            if transitions.is_empty() {
                                app.set_status("No transitions available", true);
                            } else {
                                app.transitions = transitions;
                                app.transition_list_state = ListState::default();
                                app.transition_list_state.select(Some(0));
                                app.transition_issue_key = key;
                                app.mode = Mode::Transition;
                                app.clear_status();
                            }
                        }
                        Err(e) => app.set_status(format!("Error: {e}"), true),
                    }
                }
            }

            AppAction::ExecuteTransition(issue_key, transition_id) => {
                app.set_status(format!("Transitioning {issue_key}..."), false);
                terminal.draw(|f| ui(f, &mut app))?;
                match client.transition_issue(&issue_key, &transition_id).await {
                    Ok(_) => {
                        let jql = app.jql.clone();
                        app.set_status(format!("✓ Transitioned {issue_key}"), false);
                        terminal.draw(|f| ui(f, &mut app))?;
                        if let Ok(result) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(result.issues);
                        }
                    }
                    Err(e) => app.set_status(format!("Error: {e}"), true),
                }
            }

            AppAction::OpenBrowser => {
                if let Some(issue) = app.selected_issue() {
                    let url = format!("{}/browse/{}", app.base_url, issue.key);
                    let _ = open::that(&url);
                    app.set_status(format!("Opened {}", issue.key), false);
                }
            }

            // ── Edit actions (suspend TUI → prompt → resume TUI) ──────────
            AppAction::CreateIssue => {
                suspend_tui(&mut terminal)?;
                let result = tui_create_issue(&client, app.default_project.clone()).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(Some(key)) => {
                        let jql = app.jql.clone();
                        if let Ok(r) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(r.issues);
                        }
                        app.set_status(format!("✓ Created {key}"), false);
                    }
                    Ok(None) => app.set_status("Create cancelled", false),
                    Err(e) => app.set_status(format!("Create failed: {e}"), true),
                }
            }

            AppAction::EditIssue(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_edit_issue(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => {
                        let jql = app.jql.clone();
                        if let Ok(r) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(r.issues);
                        }
                        app.set_status(format!("✓ Updated {key}"), false);
                    }
                    Ok(false) => app.set_status("Edit cancelled", false),
                    Err(e) => app.set_status(format!("Edit failed: {e}"), true),
                }
            }

            AppAction::AssignIssue(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_assign_issue(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => {
                        let jql = app.jql.clone();
                        if let Ok(r) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(r.issues);
                        }
                        app.set_status(format!("✓ Assigned {key}"), false);
                    }
                    Ok(false) => app.set_status("Assign cancelled", false),
                    Err(e) => app.set_status(format!("Assign failed: {e}"), true),
                }
            }

            AppAction::AddComment(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_add_comment(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => app.set_status(format!("✓ Comment added to {key}"), false),
                    Ok(false) => app.set_status("Comment cancelled", false),
                    Err(e) => app.set_status(format!("Comment failed: {e}"), true),
                }
            }

            AppAction::AddWorklog(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_add_worklog(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => app.set_status(format!("✓ Worklog added to {key}"), false),
                    Ok(false) => app.set_status("Worklog cancelled", false),
                    Err(e) => app.set_status(format!("Worklog failed: {e}"), true),
                }
            }

            AppAction::EditLabels(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_edit_labels(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => {
                        let jql = app.jql.clone();
                        if let Ok(r) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(r.issues);
                        }
                        app.set_status(format!("✓ Labels updated on {key}"), false);
                    }
                    Ok(false) => app.set_status("Label edit cancelled", false),
                    Err(e) => app.set_status(format!("Label edit failed: {e}"), true),
                }
            }

            AppAction::EditComponents(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_edit_components(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => {
                        let jql = app.jql.clone();
                        if let Ok(r) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(r.issues);
                        }
                        app.set_status(format!("✓ Components updated on {key}"), false);
                    }
                    Ok(false) => app.set_status("Component edit cancelled", false),
                    Err(e) => app.set_status(format!("Component edit failed: {e}"), true),
                }
            }

            AppAction::UploadAttachment(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_upload_attachment(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => app.set_status(format!("✓ Attachment uploaded to {key}"), false),
                    Ok(false) => app.set_status("Upload cancelled", false),
                    Err(e) => app.set_status(format!("Upload failed: {e}"), true),
                }
            }

            AppAction::SaveColumnPreferences => {
                let mut prefs = TuiPreferences {
                    visible_columns: app.visible_columns.clone(),
                };
                prefs.normalize();
                app.visible_columns = prefs.visible_columns.clone();
                match prefs.save() {
                    Ok(()) => app.set_status(
                        format!(
                            "✓ Saved column preferences ({})",
                            format_column_summary(&app.visible_columns)
                        ),
                        false,
                    ),
                    Err(e) => {
                        app.set_status(format!("Failed to save column preferences: {e}"), true)
                    }
                }
            }

            AppAction::ResetColumnPreferences => {
                app.set_status(
                    format!(
                        "Reset to default columns ({})",
                        format_column_summary(&app.visible_columns)
                    ),
                    false,
                );
            }

            AppAction::None => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

// ─── Suspended interactive actions ───────────────────────────────────────────

#[derive(Clone)]
struct PickerOption {
    value: String,
    label: String,
}

impl std::fmt::Display for PickerOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn normalize_picker_query(input: &str) -> String {
    input.trim().to_lowercase()
}

fn prompt_search_term(prompt: &str) -> Result<Option<String>> {
    use inquire::Text;

    let input = Text::new(prompt).prompt_skippable()?;
    Ok(input.and_then(|s| {
        let trimmed = s.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }))
}

fn picker_option_matches(option: &PickerOption, query: &str) -> bool {
    let query = normalize_picker_query(query);
    if query.is_empty() {
        return true;
    }

    let haystack = normalize_picker_query(&format!("{} {}", option.label, option.value));
    haystack.contains(&query)
}

fn pick_single_option(prompt: &str, options: Vec<PickerOption>) -> Result<Option<PickerOption>> {
    use inquire::{Select, Text};

    let mut filtered = options.clone();
    if filtered.is_empty() {
        return Ok(None);
    }

    loop {
        let selected = Select::new(prompt, filtered.clone())
            .with_help_message("Enter to choose, Esc to cancel, or pick 'Search again' to refine")
            .prompt_skippable()?;

        let Some(selected) = selected else {
            return Ok(None);
        };

        if selected.value != "__search_again__" {
            return Ok(Some(selected));
        }

        let Some(query) = Text::new("Refine search:").prompt_skippable()? else {
            return Ok(None);
        };
        let query = normalize_picker_query(&query);
        if query.is_empty() {
            continue;
        }

        filtered = options
            .iter()
            .filter(|option| {
                option.value == "__search_again__"
                    || option.value == "me"
                    || picker_option_matches(option, &query)
            })
            .cloned()
            .collect();

        if filtered
            .iter()
            .all(|option| option.value == "__search_again__")
        {
            println!("  No matches for '{query}'. Try another search.");
            filtered = options.clone();
        }
    }
}

fn pick_multi_options(
    prompt: &str,
    options: Vec<PickerOption>,
) -> Result<Option<Vec<PickerOption>>> {
    use inquire::{MultiSelect, Text};

    let mut filtered = options.clone();
    if filtered.is_empty() {
        return Ok(None);
    }

    loop {
        let selected = MultiSelect::new(prompt, filtered.clone())
            .with_help_message(
                "Space toggles, Enter confirms, Esc cancels. Choose 'Search again' to refine.",
            )
            .prompt_skippable()?;

        let Some(selected) = selected else {
            return Ok(None);
        };

        if !selected
            .iter()
            .any(|option| option.value == "__search_again__")
        {
            return Ok(Some(selected));
        }

        let Some(query) = Text::new("Refine component search:").prompt_skippable()? else {
            return Ok(None);
        };
        let query = normalize_picker_query(&query);
        if query.is_empty() {
            continue;
        }

        filtered = options
            .iter()
            .filter(|option| {
                option.value == "__search_again__" || picker_option_matches(option, &query)
            })
            .cloned()
            .collect();

        if filtered
            .iter()
            .all(|option| option.value == "__search_again__")
        {
            println!("  No components matched '{query}'. Try another search.");
            filtered = options.clone();
        }
    }
}

async fn prompt_assignee_selection(client: &JiraClient, prompt: &str) -> Result<Option<String>> {
    let query = match prompt_search_term(prompt)? {
        Some(query) => query,
        None => return Ok(None),
    };

    let users = client.search_users(&query).await?;
    let mut options = vec![
        PickerOption {
            value: "me".to_string(),
            label: "Assign to me".to_string(),
        },
        PickerOption {
            value: "__search_again__".to_string(),
            label: "Search again...".to_string(),
        },
    ];

    for user in users {
        let display = user
            .get("displayName")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown user")
            .trim();
        let email = user
            .get("emailAddress")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let account_id = user
            .get("accountId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        if account_id.is_empty() {
            continue;
        }

        let mut parts = vec![display.to_string()];
        if !email.is_empty() {
            parts.push(format!("<{email}>"));
        }
        parts.push(format!("accountId: {account_id}"));
        let label = parts.join("  •  ");

        if !options.iter().any(|option| option.value == account_id) {
            options.push(PickerOption {
                value: account_id.to_string(),
                label,
            });
        }
    }

    if options.len() <= 2 {
        println!("  No matching users found.");
        return Ok(None);
    }

    let selected = pick_single_option("Pick assignee:", options)?;
    Ok(selected.map(|option| option.value))
}

async fn prompt_project_components(
    client: &JiraClient,
    project_key: &str,
    prompt: &str,
) -> Result<Option<Vec<String>>> {
    let query = match prompt_search_term(prompt)? {
        Some(query) => normalize_picker_query(&query),
        None => return Ok(None),
    };

    let raw_components = client.get_project_components(project_key).await?;
    let mut options: Vec<PickerOption> = raw_components
        .into_iter()
        .filter_map(|component| {
            let name = component.get("name").and_then(|v| v.as_str())?.trim();
            if name.is_empty() {
                return None;
            }
            Some(PickerOption {
                value: name.to_string(),
                label: name.to_string(),
            })
        })
        .filter(|option| picker_option_matches(option, &query))
        .collect();

    options.sort_by_key(|a| a.label.to_lowercase());
    options.dedup_by(|a, b| a.value == b.value);
    options.insert(
        0,
        PickerOption {
            value: "__search_again__".to_string(),
            label: "Search again...".to_string(),
        },
    );

    if options.len() == 1 {
        println!("  No matching components found for project {project_key}.");
        return Ok(None);
    }

    let selected = pick_multi_options(
        "Pick components (space to toggle, enter to confirm):",
        options,
    )?;

    let Some(selected) = selected else {
        return Ok(None);
    };

    let values = selected
        .into_iter()
        .filter(|option| option.value != "__search_again__")
        .map(|option| option.value)
        .collect::<Vec<_>>();

    Ok(Some(values))
}

/// Create a new issue interactively. Returns the created issue key, or None if cancelled.
async fn tui_create_issue(
    client: &JiraClient,
    default_project: Option<String>,
) -> Result<Option<String>> {
    use inquire::{Select, Text};
    use jira_core::model::CreateIssueRequestV2;

    println!("\n── Create Issue ──────────────────────────────────");

    let project = match default_project {
        Some(p) => {
            let input = Text::new("Project key:")
                .with_default(&p)
                .prompt_skippable()?;
            match input {
                Some(s) if !s.trim().is_empty() => s.trim().to_uppercase(),
                _ => return Ok(None),
            }
        }
        None => {
            let input = Text::new("Project key:").prompt_skippable()?;
            match input {
                Some(s) if !s.trim().is_empty() => s.trim().to_uppercase(),
                _ => return Ok(None),
            }
        }
    };

    let summary = match Text::new("Summary:").prompt_skippable()? {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return Ok(None),
    };

    let issue_type = if let Ok(types) = client.get_issue_types(&project).await {
        let names: Vec<String> = types.iter().map(|t| t.name.clone()).collect();
        if names.is_empty() {
            "Task".to_string()
        } else {
            Select::new("Issue type:", names)
                .prompt()
                .unwrap_or_else(|_| "Task".to_string())
        }
    } else {
        "Task".to_string()
    };

    let assignee =
        prompt_assignee_selection(client, "Search assignee (name/email, blank to skip):").await?;

    let priority = Text::new("Priority (blank to skip):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

    let req = CreateIssueRequestV2 {
        project_key: project,
        summary,
        description: None,
        description_adf: None,
        issue_type,
        assignee,
        priority,
        labels: Vec::new(),
        components: Vec::new(),
        parent: None,
        fix_versions: Vec::new(),
        custom_fields: std::collections::HashMap::new(),
    };

    let issue = client.create_issue_v2(req).await?;
    println!("✓ Created {}", issue.key);
    Ok(Some(issue.key))
}

/// Edit an existing issue — prompts for fields to change (blank = keep current).
/// Returns true if any update was made.
async fn tui_edit_issue(client: &JiraClient, key: &str) -> Result<bool> {
    use inquire::Text;

    println!("\n── Edit {key} ──────────────────────────────────────");
    println!("  Leave a field blank to keep its current value.\n");

    let summary = Text::new("New summary (blank to skip):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

    let assignee =
        prompt_assignee_selection(client, "Search new assignee (name/email, blank to skip):")
            .await?;

    let priority = Text::new("New priority (blank to skip):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

    if summary.is_none() && assignee.is_none() && priority.is_none() {
        println!("  Nothing to update.");
        return Ok(false);
    }

    let req = UpdateIssueRequest {
        summary,
        assignee,
        priority,
        ..Default::default()
    };

    client.update_issue(key, req).await?;
    println!("✓ Updated {key}");
    Ok(true)
}

/// Assign an issue to a specific user.
async fn tui_assign_issue(client: &JiraClient, key: &str) -> Result<bool> {
    println!("\n── Assign {key} ─────────────────────────────────────");

    let Some(assignee) =
        prompt_assignee_selection(client, "Search assignee (name/email, blank to cancel):").await?
    else {
        return Ok(false);
    };

    let req = UpdateIssueRequest {
        assignee: Some(assignee),
        ..Default::default()
    };

    client.update_issue(key, req).await?;
    println!("✓ Assigned {key}");
    Ok(true)
}

/// Add a comment to an issue.
async fn tui_add_comment(client: &JiraClient, key: &str) -> Result<bool> {
    use inquire::Text;

    println!("\n── Add Comment to {key} ──────────────────────────────");

    let body = match Text::new("Comment (blank to cancel):").prompt_skippable()? {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return Ok(false),
    };

    client.add_comment(key, &body).await?;
    println!("✓ Comment added to {key}");
    Ok(true)
}

/// Add a worklog entry to an issue.
async fn tui_add_worklog(client: &JiraClient, key: &str) -> Result<bool> {
    use chrono::Local;
    use inquire::Text;

    println!("\n── Add Worklog to {key} ──────────────────────────────");
    println!("  Time format examples: 2h, 30m, 1d, 1h 30m");
    println!(
        "  Date format: YYYY-MM-DD (blank = today {})",
        Local::now().format("%Y-%m-%d")
    );
    println!(
        "  Start time format: HH:MM or HH:MM:SS (blank = now {})\n",
        Local::now().format("%H:%M")
    );

    let time = match Text::new("Time spent (blank to cancel):").prompt_skippable()? {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return Ok(false),
    };

    let date = Text::new("Date (blank = today):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

    let start = Text::new("Start time (blank = now):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

    let comment = Text::new("Comment (blank to skip):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

    let started = build_worklog_started(date.as_deref(), start.as_deref())?;

    client
        .add_worklog(key, &time, comment.as_deref(), started.as_deref())
        .await?;
    println!("✓ Worklog added to {key}");
    Ok(true)
}

/// Set labels on an issue (replaces existing).
async fn tui_edit_labels(client: &JiraClient, key: &str) -> Result<bool> {
    use inquire::Text;

    println!("\n── Edit Labels on {key} ──────────────────────────────");
    println!("  Enter comma-separated labels. Blank to cancel.\n");

    let input = match Text::new("Labels (comma-separated):").prompt_skippable()? {
        Some(s) => s,
        None => return Ok(false),
    };

    if input.trim().is_empty() {
        return Ok(false);
    }

    let labels: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let req = UpdateIssueRequest {
        labels: Some(labels),
        ..Default::default()
    };

    client.update_issue(key, req).await?;
    println!("✓ Labels updated on {key}");
    Ok(true)
}

/// Set components on an issue (replaces existing).
async fn tui_edit_components(client: &JiraClient, key: &str) -> Result<bool> {
    println!("\n── Edit Components on {key} ──────────────────────────");

    let issue = client.get_issue(key).await?;
    let project_key = issue
        .key
        .split_once('-')
        .map(|(project, _)| project.to_string())
        .ok_or_else(|| anyhow::anyhow!("Could not determine project for {key}"))?;

    println!("  Components are limited to project {project_key}.");

    let Some(components) =
        prompt_project_components(client, &project_key, "Search components (blank to cancel):")
            .await?
    else {
        return Ok(false);
    };

    let req = UpdateIssueRequest {
        components: Some(components),
        ..Default::default()
    };

    client.update_issue(key, req).await?;
    println!("✓ Components updated on {key}");
    Ok(true)
}

/// Upload a file attachment to an issue.
async fn tui_upload_attachment(client: &JiraClient, key: &str) -> Result<bool> {
    use inquire::Text;

    println!("\n── Upload Attachment to {key} ────────────────────────");

    let path_str = match Text::new("File path (blank to cancel):").prompt_skippable()? {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return Ok(false),
    };

    let path = std::path::PathBuf::from(&path_str);
    if !path.exists() {
        anyhow::bail!("File not found: {path_str}");
    }

    client.upload_attachment(key, &path).await?;
    println!(
        "✓ Uploaded {} to {key}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    Ok(true)
}

// ─── UI rendering ────────────────────────────────────────────────────────────

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(0),    // content
            Constraint::Length(2), // footer (2 lines to accommodate keybinding hints)
        ])
        .split(size);

    // Header
    let title = match app.mode {
        Mode::List => format!(" Jira CLI  {}  ({} issues) ", app.jql, app.issues.len()),
        Mode::View => " Jira CLI — Issue Detail ".to_string(),
        Mode::Search => " Jira CLI — Search ".to_string(),
        Mode::Transition => " Jira CLI — Select Transition ".to_string(),
        Mode::Help => " Jira CLI — Help ".to_string(),
        Mode::ColumnPicker => " Jira CLI — Columns ".to_string(),
    };
    let header = Paragraph::new(title).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(header, chunks[0]);

    // Footer / status
    render_footer(f, app, chunks[2]);

    // Content
    match app.mode {
        Mode::List => render_list(f, app, chunks[1]),
        Mode::View => render_detail(f, app, chunks[1]),
        Mode::Search => {
            render_list(f, app, chunks[1]);
            render_search_bar(f, app, size);
        }
        Mode::Transition => {
            render_list(f, app, chunks[1]);
            render_transition_popup(f, app, size);
        }
        Mode::Help => {
            render_list(f, app, chunks[1]);
            render_help_popup(f, size);
        }
        Mode::ColumnPicker => {
            render_list(f, app, chunks[1]);
            render_column_picker_popup(f, app, size);
        }
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::List => {
            " j/k:move  Enter:view  t:transition  C:columns  c:create  e:edit  a:assign  ;:comment  w:worklog  l:labels  m:comps  u:upload  o:browser  r:refresh  /:search  ?:help  q:quit"
                .to_string()
        }
        Mode::View => {
            " Esc:back  t:transition  e:edit  a:assign  ;:comment  w:worklog  l:labels  m:comps  u:upload  o:browser  ?:help  q:quit"
                .to_string()
        }
        Mode::Search => " Type JQL  Enter:search  Esc:cancel".to_string(),
        Mode::Transition => " j/k:move  Enter:execute  Esc:cancel".to_string(),
        Mode::Help => " Any key: close".to_string(),
        Mode::ColumnPicker => " j/k:move  Space:toggle  a:all  Enter:save  Esc:cancel".to_string(),
    };

    let (fg, bg) = if let Some((_, true)) = &app.status {
        (Color::White, Color::Red)
    } else if let Some((msg, false)) = &app.status {
        // Show status message instead of keybindings
        let status_line = Paragraph::new(format!(" {msg}"))
            .style(Style::default().fg(Color::Black).bg(Color::Green));
        f.render_widget(status_line, area);
        return;
    } else {
        (Color::DarkGray, Color::Reset)
    };

    // Show error in footer
    if let Some((msg, true)) = &app.status {
        let err_line = Paragraph::new(format!(" ✗ {msg}")).style(Style::default().fg(fg).bg(bg));
        f.render_widget(err_line, area);
        return;
    }

    let footer = Paragraph::new(text)
        .style(Style::default().fg(Color::DarkGray))
        .wrap(Wrap { trim: false });
    f.render_widget(footer, area);
}

fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    let columns = if app.visible_columns.is_empty() {
        AVAILABLE_COLUMNS.to_vec()
    } else {
        app.visible_columns.clone()
    };

    let header_cells = columns.iter().map(|column| {
        Cell::from(column.label()).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.issues.iter().map(|issue| {
        Row::new(
            columns
                .iter()
                .map(|column| column.cell(issue))
                .collect::<Vec<_>>(),
        )
        .height(1)
    });

    let widths = columns
        .iter()
        .map(|column| column.width())
        .collect::<Vec<_>>();

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Issues "))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let Some(issue) = app.selected_issue() else {
        return;
    };

    let created = &issue.created[..10.min(issue.created.len())];
    let updated = &issue.updated[..10.min(issue.updated.len())];

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(
                issue.key.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" — "),
            Span::styled(
                issue.summary.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        field_line("Type", &issue.issue_type),
        field_line("Status", &issue.status),
        field_line("Project", &issue.project_key),
    ];

    if let Some(p) = &issue.priority {
        lines.push(field_line("Priority", p));
    }
    if let Some(a) = &issue.assignee {
        lines.push(field_line("Assignee", a));
    }
    if let Some(r) = &issue.reporter {
        lines.push(field_line("Reporter", r));
    }
    lines.push(field_line("Created", created));
    lines.push(field_line("Updated", updated));

    if !issue.attachments.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  Attachments ({}):", issue.attachments.len()),
            Style::default().fg(Color::DarkGray),
        )));
        for a in &issue.attachments {
            lines.push(Line::from(format!(
                "    • {} ({} bytes)",
                a.filename, a.size
            )));
        }
    }

    if let Some(desc) = &issue.description {
        let text = jira_core::adf::adf_to_text(desc);
        if !text.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Description:",
                Style::default().add_modifier(Modifier::UNDERLINED),
            )));
            lines.push(Line::from(""));
            for line in text.lines() {
                lines.push(Line::from(format!("  {line}")));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Issue Detail "),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_search_bar(f: &mut Frame, app: &App, area: Rect) {
    let popup = bottom_bar_rect(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" JQL Search ")
        .style(Style::default().bg(Color::Black));

    let input = Paragraph::new(app.search_input.as_str())
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(Clear, popup);
    f.render_widget(input, popup);

    // Place the terminal cursor at the current cursor position within the input.
    // popup inner area starts at x+1, y+1 (border).
    let before_cursor: String = app.search_input.chars().take(app.search_cursor).collect();
    let cursor_x = popup.x + 1 + before_cursor.len() as u16;
    let cursor_y = popup.y + 1;
    f.set_cursor_position((cursor_x, cursor_y));
}

fn render_transition_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(50, 60, area);

    let items: Vec<ListItem> = app
        .transitions
        .iter()
        .map(|(_, name)| ListItem::new(name.as_str()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Transition: {} ", app.transition_issue_key))
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_widget(Clear, popup_area);
    f.render_stateful_widget(list, popup_area, &mut app.transition_list_state);
}

fn render_column_picker_popup(f: &mut Frame, app: &mut App, area: Rect) {
    let popup_area = centered_rect(54, 72, area);
    let [summary_area, list_area, hint_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .areas(popup_area);

    let items: Vec<ListItem> = AVAILABLE_COLUMNS
        .iter()
        .map(|column| {
            let checked = if app.visible_columns.contains(column) {
                "[x]"
            } else {
                "[ ]"
            };
            ListItem::new(format!("{checked} {}", column.label()))
        })
        .collect();

    let selected_summary = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Selected: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_column_summary(&app.visible_columns),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(Span::styled(
            "Tip: press Space to toggle columns, then S or Enter to save.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
            .title(" Column Settings ")
            .style(Style::default().bg(Color::Black)),
    );

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .style(Style::default().bg(Color::Black)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let hints = Paragraph::new(vec![
        Line::from("↑/↓ move   Space toggle   a select all   r reset defaults"),
        Line::from("s or Enter save   Esc cancel"),
    ])
    .block(
        Block::default()
            .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
            .style(Style::default().bg(Color::Black)),
    )
    .style(Style::default().fg(Color::DarkGray));

    f.render_widget(Clear, popup_area);
    f.render_widget(selected_summary, summary_area);
    f.render_stateful_widget(list, list_area, &mut app.column_picker_state);
    f.render_widget(hints, hint_area);
}

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(70, 95, area);

    let lines = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Issue List:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  ↑/k       Move up"),
        Line::from("  ↓/j       Move down"),
        Line::from("  Enter     View issue detail"),
        Line::from("  t         Transition issue (in-TUI picker)"),
        Line::from("  C         Open column settings popup"),
        Line::from("  c         Create new issue"),
        Line::from("  e         Edit selected issue (summary/assignee/priority)"),
        Line::from("  a         Assign selected issue"),
        Line::from("  ;         Add comment to selected issue"),
        Line::from("  w         Add worklog to selected issue"),
        Line::from("  l         Set labels on selected issue"),
        Line::from("  m         Set components on selected issue"),
        Line::from("  u         Upload attachment to selected issue"),
        Line::from("  o         Open issue in browser"),
        Line::from("  r         Refresh list"),
        Line::from("  /         Search with JQL"),
        Line::from("  ?         Show this help"),
        Line::from("  q         Quit the TUI"),
        Line::from("  Esc       Back / cancel (quit from list)"),
        Line::from(""),
        Line::from(Span::styled(
            "Issue Detail:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  Esc / q   Back to list"),
        Line::from("  t         Transition this issue"),
        Line::from("  e / a / w / l / m / u   (same as list)"),
        Line::from("  o         Open in browser"),
        Line::from(""),
        Line::from(Span::styled(
            "Column Picker:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  ↑/k       Move up"),
        Line::from("  ↓/j       Move down"),
        Line::from("  Space     Toggle selected column"),
        Line::from("  a         Select all available columns"),
        Line::from("  r         Reset to default columns"),
        Line::from("  s / Enter Save preferences"),
        Line::from("  Esc       Cancel without saving"),
        Line::from(""),
        Line::from(Span::styled("Search:", Style::default().fg(Color::Yellow))),
        Line::from("  Type JQL, press Enter to search"),
        Line::from("  Left/Right  Move cursor   Home/End  Jump   Del  Delete forward"),
        Line::from("  Esc         Cancel"),
        Line::from(""),
        Line::from(Span::styled(
            "JQL Quick Reference:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from("  project = PROJ                       issues in a project"),
        Line::from("  assignee = currentUser()             assigned to me"),
        Line::from("  assignee = \"email@example.com\"       assigned to someone"),
        Line::from("  status = \"In Progress\"               by status"),
        Line::from("  status in (\"To Do\", \"In Progress\")   multiple statuses"),
        Line::from("  priority = High                      by priority"),
        Line::from("  sprint = openSprints()               current sprint"),
        Line::from("  updated >= -7d                       updated in last 7 days"),
        Line::from("  created >= -30d                      created in last 30 days"),
        Line::from("  text ~ \"login bug\"                   full-text search"),
        Line::from("  labels = backend                     by label"),
        Line::from("  ORDER BY updated DESC                sort order"),
        Line::from(""),
        Line::from(Span::styled(
            "Tip: combine with AND / OR — run `jirac issue jql --help` for more",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(Clear, popup_area);

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .style(Style::default().bg(Color::Black)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, popup_area);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn field_line<'a>(label: &'a str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {label:<12}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(value),
    ])
}

fn format_column_summary(columns: &[ColumnKind]) -> String {
    columns
        .iter()
        .map(|column| column.label())
        .collect::<Vec<_>>()
        .join(", ")
}

fn status_color(status: &str) -> Color {
    let s = status.to_lowercase();
    if s.contains("done") || s.contains("closed") || s.contains("resolved") {
        Color::Green
    } else if s.contains("progress") || s.contains("review") {
        Color::Yellow
    } else if s.contains("blocked") || s.contains("impediment") {
        Color::Red
    } else {
        Color::White
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn bottom_bar_rect(r: Rect) -> Rect {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(r);
    layout[1]
}
