use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use jira_core::{
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
use std::io;

// ─── Mode ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    List,
    View,
    Search,
    Transition,
    Help,
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
    /// Bottom status bar message (text, is_error)
    status: Option<(String, bool)>,
    /// Transitions for the selected issue (id, name)
    transitions: Vec<(String, String)>,
    transition_list_state: ListState,
    transition_issue_key: String,
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
    AddWorklog(String),
    EditLabels(String),
    EditComponents(String),
    UploadAttachment(String),
}

// ─── App impl ────────────────────────────────────────────────────────────────

impl App {
    fn new(jql: String, base_url: String, default_project: Option<String>) -> Self {
        Self {
            issues: Vec::new(),
            table_state: TableState::default(),
            mode: Mode::List,
            base_url,
            jql,
            default_project,
            search_input: String::new(),
            status: None,
            transitions: Vec::new(),
            transition_list_state: ListState::default(),
            transition_issue_key: String::new(),
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
                self.mode = Mode::Search;
                AppAction::None
            }
            KeyCode::Char('?') => {
                self.mode = Mode::Help;
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
            KeyCode::Backspace => {
                self.search_input.pop();
                AppAction::None
            }
            KeyCode::Char(c) => {
                self.search_input.push(c);
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

            AppAction::None => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

// ─── Suspended interactive actions ───────────────────────────────────────────

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

    // Fetch issue types for the project
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

    let assignee = Text::new("Assignee (email or 'me', blank to skip):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

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

    let assignee = Text::new("New assignee — email or 'me' (blank to skip):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

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
    use inquire::Text;

    println!("\n── Assign {key} ─────────────────────────────────────");

    let assignee =
        match Text::new("Assignee (email or 'me', blank to cancel):").prompt_skippable()? {
            Some(s) if !s.trim().is_empty() => s.trim().to_string(),
            _ => return Ok(false),
        };

    let req = UpdateIssueRequest {
        assignee: Some(assignee),
        ..Default::default()
    };

    client.update_issue(key, req).await?;
    println!("✓ Assigned {key}");
    Ok(true)
}

/// Add a worklog entry to an issue.
async fn tui_add_worklog(client: &JiraClient, key: &str) -> Result<bool> {
    use inquire::Text;

    println!("\n── Add Worklog to {key} ──────────────────────────────");
    println!("  Time format examples: 2h, 30m, 1d, 1h 30m\n");

    let time = match Text::new("Time spent (blank to cancel):").prompt_skippable()? {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return Ok(false),
    };

    let comment = Text::new("Comment (blank to skip):")
        .prompt_skippable()?
        .and_then(|s| {
            if s.trim().is_empty() {
                None
            } else {
                Some(s.trim().to_string())
            }
        });

    client
        .add_worklog(key, &time, comment.as_deref(), None)
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
    use inquire::Text;

    println!("\n── Edit Components on {key} ──────────────────────────");
    println!("  Enter comma-separated component names. Blank to cancel.\n");

    let input = match Text::new("Components (comma-separated):").prompt_skippable()? {
        Some(s) => s,
        None => return Ok(false),
    };

    if input.trim().is_empty() {
        return Ok(false);
    }

    let components: Vec<String> = input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

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
            Constraint::Length(1), // footer
        ])
        .split(size);

    // Header
    let title = match app.mode {
        Mode::List => format!(" Jira CLI  {}  ({} issues) ", app.jql, app.issues.len()),
        Mode::View => " Jira CLI — Issue Detail ".to_string(),
        Mode::Search => " Jira CLI — Search ".to_string(),
        Mode::Transition => " Jira CLI — Select Transition ".to_string(),
        Mode::Help => " Jira CLI — Help ".to_string(),
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
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::List => {
            " j/k:move  Enter:view  t:transition  c:create  e:edit  a:assign  w:worklog  l:labels  m:comps  u:upload  o:browser  r:refresh  /:search  ?:help  q:quit"
                .to_string()
        }
        Mode::View => {
            " Esc:back  t:transition  e:edit  a:assign  w:worklog  l:labels  m:comps  u:upload  o:browser  ?:help  q:quit"
                .to_string()
        }
        Mode::Search => " Type JQL  Enter:search  Esc:cancel".to_string(),
        Mode::Transition => " j/k:move  Enter:execute  Esc:cancel".to_string(),
        Mode::Help => " Any key: close".to_string(),
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

    let footer = Paragraph::new(text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, area);
}

fn render_list(f: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["Key", "Type", "Priority", "Status", "Assignee", "Summary"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows = app.issues.iter().map(|issue| {
        let summary = if issue.summary.len() > 40 {
            format!("{}…", &issue.summary[..39])
        } else {
            issue.summary.clone()
        };
        Row::new(vec![
            Cell::from(issue.key.clone()).style(Style::default().fg(Color::Cyan)),
            Cell::from(issue.issue_type.clone()),
            Cell::from(issue.priority.clone().unwrap_or_else(|| "-".into())),
            Cell::from(issue.status.clone())
                .style(Style::default().fg(status_color(&issue.status))),
            Cell::from(issue.assignee.clone().unwrap_or_else(|| "-".into())),
            Cell::from(summary),
        ])
        .height(1)
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(18),
            Constraint::Length(20),
            Constraint::Min(20),
        ],
    )
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

    // Place the terminal cursor at the end of the input text so the user can
    // see where they are typing. popup inner area starts at x+1, y+1 (border).
    let cursor_x = popup.x + 1 + app.search_input.len() as u16;
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

fn render_help_popup(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 85, area);

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
        Line::from("  c         Create new issue"),
        Line::from("  e         Edit selected issue (summary/assignee/priority)"),
        Line::from("  a         Assign selected issue"),
        Line::from("  w         Add worklog to selected issue"),
        Line::from("  l         Set labels on selected issue"),
        Line::from("  m         Set components on selected issue"),
        Line::from("  u         Upload attachment to selected issue"),
        Line::from("  o         Open issue in browser"),
        Line::from("  r         Refresh list"),
        Line::from("  /         Search with JQL"),
        Line::from("  ?         Show this help"),
        Line::from("  q / Esc   Quit"),
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
        Line::from(Span::styled("Search:", Style::default().fg(Color::Yellow))),
        Line::from("  Type JQL, press Enter to search"),
        Line::from("  Esc       Cancel"),
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
