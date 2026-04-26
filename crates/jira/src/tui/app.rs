use std::{collections::HashSet, io, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use jira_core::config::{config_file_path, JiraConfig, JiraProfilesFile};
use jira_core::{
    model::{Issue, UpdateIssueRequest},
    JiraClient,
};

use super::panel::{DetailData, DetailTab, Focus};
use ratatui::{
    backend::CrosstermBackend,
    widgets::{ListState, TableState},
    Terminal,
};

use super::column::{format_column_summary, ColumnKind};
use super::keys;
use super::mode::Mode;
use super::picker::PickerOption;
use super::prefs::{SavedJql, TuiPreferences};
use super::prompts::{
    resume_tui, suspend_tui, tui_add_comment, tui_add_worklog, tui_confirm_delete_saved_jql,
    tui_create_issue, tui_edit_issue, tui_edit_labels, tui_edit_saved_jql, tui_upload_attachment,
};
use super::render::ui;
use super::theme::ThemeName;

pub(super) struct App {
    pub(super) issues: Vec<Issue>,
    pub(super) table_state: TableState,
    pub(super) mode: Mode,
    pub(super) focus: Focus,
    pub(super) active_tab: DetailTab,
    pub(super) detail: DetailData,
    pub(super) base_url: String,
    pub(super) jql: String,
    pub(super) default_project: Option<String>,
    pub(super) search_input: String,
    pub(super) search_cursor: usize,
    pub(super) status: Option<(String, bool)>,
    pub(super) transitions: Vec<(String, String)>,
    pub(super) transition_list_state: ListState,
    pub(super) transition_issue_key: String,
    pub(super) visible_columns: Vec<ColumnKind>,
    pub(super) column_picker_state: ListState,
    pub(super) assignee_query: String,
    pub(super) assignee_cursor: usize,
    pub(super) assignee_options: Vec<PickerOption>,
    pub(super) assignee_state: ListState,
    pub(super) assignee_issue_key: String,
    pub(super) component_query: String,
    pub(super) component_cursor: usize,
    pub(super) component_options: Vec<PickerOption>,
    pub(super) component_selected: HashSet<String>,
    pub(super) component_state: ListState,
    pub(super) component_issue_key: String,
    pub(super) component_project_key: String,
    pub(super) prefs: TuiPreferences,
    pub(super) saved_jql_state: ListState,
    pub(super) theme_state: ListState,
    pub(super) server_info_lines: Vec<String>,
    pub(super) config_lines: Vec<String>,
    pub(super) detail_scroll: u16,
}

pub(super) enum AppAction {
    None,
    Quit,
    Refresh,
    ExecuteSearch(String),
    FetchTransitions,
    ExecuteTransition(String, String),
    OpenBrowser,
    CreateIssue,
    EditIssue(String),
    AssignIssue(String),
    OpenAssigneePicker(String),
    RefreshAssigneeOptions,
    AddComment(String),
    AddWorklog(String),
    EditLabels(String),
    EditComponents(String),
    OpenComponentPicker(String),
    RefreshComponentOptions,
    UploadAttachment(String),
    SaveColumnPreferences,
    ResetColumnPreferences,
    ApplySavedJql(String),
    CreateSavedJql,
    EditSavedJql(usize),
    DeleteSavedJql(usize),
    SaveTheme,
    LoadServerInfo,
    LoadConfigView,
    WarmActiveTab,
}

impl App {
    pub(super) async fn warm_active_tab(&mut self, client: &JiraClient) {
        let Some(key) = self.selected_issue_key() else {
            return;
        };
        self.detail.reset_for(&key);

        match self.active_tab {
            DetailTab::Comments => {
                if self.detail.comments.is_none() {
                    match client.get_comments(&key).await {
                        Ok(comments) => self.detail.comments = Some(comments),
                        Err(e) => self.set_status(format!("Comments load failed: {e}"), true),
                    }
                }
            }
            DetailTab::Worklog => {
                if self.detail.worklogs.is_none() {
                    match client.get_worklogs(&key).await {
                        Ok(worklogs) => self.detail.worklogs = Some(worklogs),
                        Err(e) => self.set_status(format!("Worklog load failed: {e}"), true),
                    }
                }
            }
            DetailTab::Links => {
                if self.detail.remote_links.is_none() {
                    match client.get_remote_links(&key).await {
                        Ok(links) => self.detail.remote_links = Some(links),
                        Err(e) => self.set_status(format!("Links load failed: {e}"), true),
                    }
                }
            }
            DetailTab::Attachments | DetailTab::Subtasks | DetailTab::Summary => {}
        }
    }

    fn new(jql: String, base_url: String, default_project: Option<String>) -> Self {
        let prefs = TuiPreferences::load();
        let mut column_picker_state = ListState::default();
        column_picker_state.select(Some(0));
        let mut saved_jql_state = ListState::default();
        saved_jql_state.select(Some(0));
        let mut theme_state = ListState::default();
        let theme_idx = ThemeName::ALL
            .iter()
            .position(|theme| *theme == prefs.theme)
            .unwrap_or(0);
        theme_state.select(Some(theme_idx));

        Self {
            issues: Vec::new(),
            table_state: TableState::default(),
            mode: Mode::Browse,
            focus: Focus::List,
            active_tab: DetailTab::Summary,
            detail: DetailData::default(),
            base_url,
            jql,
            default_project,
            search_input: String::new(),
            search_cursor: 0,
            status: None,
            transitions: Vec::new(),
            transition_list_state: ListState::default(),
            transition_issue_key: String::new(),
            visible_columns: prefs.visible_columns.clone(),
            column_picker_state,
            assignee_query: String::new(),
            assignee_cursor: 0,
            assignee_options: Vec::new(),
            assignee_state: ListState::default(),
            assignee_issue_key: String::new(),
            component_query: String::new(),
            component_cursor: 0,
            component_options: Vec::new(),
            component_selected: HashSet::new(),
            component_state: ListState::default(),
            component_issue_key: String::new(),
            component_project_key: String::new(),
            prefs,
            saved_jql_state,
            theme_state,
            server_info_lines: Vec::new(),
            config_lines: Vec::new(),
            detail_scroll: 0,
        }
    }

    pub(super) fn set_issues(&mut self, issues: Vec<Issue>) {
        let prev_key = self.selected_issue_key();
        self.issues = issues;
        if self.issues.is_empty() {
            self.table_state.select(None);
            self.focus = Focus::List;
            return;
        }

        let selected = prev_key
            .as_ref()
            .and_then(|key| self.issues.iter().position(|issue| &issue.key == key))
            .unwrap_or(0);
        self.table_state.select(Some(selected));
        self.ensure_detail_context();
    }

    pub(super) fn selected_issue(&self) -> Option<&Issue> {
        self.table_state.selected().and_then(|i| self.issues.get(i))
    }

    pub(super) fn selected_issue_key(&self) -> Option<String> {
        self.selected_issue().map(|i| i.key.clone())
    }

    pub(super) fn next_issue(&mut self) {
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

    pub(super) fn prev_issue(&mut self) {
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

    pub(super) fn next_transition(&mut self) {
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

    pub(super) fn prev_transition(&mut self) {
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

    pub(super) fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = Some((msg.into(), is_error));
    }

    pub(super) fn clear_status(&mut self) {
        self.status = None;
    }

    pub(super) fn ensure_detail_context(&mut self) {
        if let Some(key) = self.selected_issue_key() {
            let before = self.detail.issue_key.clone();
            self.detail.reset_for(&key);
            if before != self.detail.issue_key {
                self.reset_detail_scroll();
            }
        }
    }

    pub(super) fn open_detail(&mut self) {
        self.focus = Focus::Detail;
        self.ensure_detail_context();
        self.reset_detail_scroll();
    }

    pub(super) fn close_detail(&mut self) {
        self.focus = Focus::List;
        self.reset_detail_scroll();
    }

    pub(super) fn reset_detail_scroll(&mut self) {
        self.detail_scroll = 0;
    }

    pub(super) fn scroll_detail_down(&mut self, amount: u16) {
        self.detail_scroll = self.detail_scroll.saturating_add(amount.max(1));
    }

    pub(super) fn scroll_detail_up(&mut self, amount: u16) {
        self.detail_scroll = self.detail_scroll.saturating_sub(amount.max(1));
    }

    pub(super) fn set_active_tab(&mut self, tab: DetailTab) {
        if self.active_tab != tab {
            self.active_tab = tab;
            self.reset_detail_scroll();
        }
    }

    pub(super) fn selected_saved_jql(&self) -> Option<&SavedJql> {
        self.saved_jql_state
            .selected()
            .and_then(|i| self.prefs.saved_jqls.get(i))
    }

    pub(super) fn selected_saved_jql_index(&self) -> Option<usize> {
        self.saved_jql_state
            .selected()
            .filter(|index| *index < self.prefs.saved_jqls.len())
    }

    pub(super) fn clamp_saved_jql_selection(&mut self) {
        if self.prefs.saved_jqls.is_empty() {
            self.saved_jql_state.select(None);
            return;
        }

        let idx = self
            .saved_jql_state
            .selected()
            .map(|i| i.min(self.prefs.saved_jqls.len() - 1))
            .unwrap_or(0);
        self.saved_jql_state.select(Some(idx));
    }

    pub(super) fn selected_theme(&self) -> ThemeName {
        self.theme_state
            .selected()
            .and_then(|i| ThemeName::ALL.get(i).copied())
            .unwrap_or(self.prefs.theme)
    }

    pub(super) fn load_config_lines(&mut self) {
        let path = config_file_path();
        let mut lines = vec![format!("Config file: {}", path.display()), String::new()];

        match JiraProfilesFile::load() {
            Ok(store) => {
                let current = store
                    .current_profile_name()
                    .unwrap_or_else(|| "(none)".to_string());
                lines.push(format!("Current profile: {current}"));
                lines.push(format!("Profiles: {}", store.profiles.len()));
                lines.push(String::new());

                for (name, profile) in &store.profiles {
                    let marker = if Some(name.as_str()) == store.current_profile.as_deref() {
                        "*"
                    } else {
                        " "
                    };
                    lines.push(format!("{marker} {name}"));
                    lines.push(format!("  URL: {}", profile.base_url));
                    lines.push(format!(
                        "  User: {}",
                        if profile.email.trim().is_empty() {
                            "(empty)"
                        } else {
                            profile.email.as_str()
                        }
                    ));
                    lines.push(format!(
                        "  Project: {}",
                        profile.project.as_deref().unwrap_or("(none)")
                    ));
                    lines.push(format!("  Timeout: {}s", profile.timeout_secs));
                    lines.push(format!("  Deployment: {:?}", profile.deployment));
                    lines.push(format!("  Auth: {:?}", profile.auth_type));
                    lines.push(format!("  API: v{}", profile.api_version));
                    lines.push(format!(
                        "  Token: {}",
                        if profile
                            .token
                            .as_deref()
                            .map(|t| !t.trim().is_empty())
                            .unwrap_or(false)
                        {
                            "present"
                        } else {
                            "missing"
                        }
                    ));
                    lines.push(String::new());
                }
            }
            Err(e) => {
                lines.push(format!("Config parse failed: {e}"));
                lines.push(String::new());
                match std::fs::read_to_string(&path) {
                    Ok(_raw) => {
                        lines.push(
                            "Raw config preview suppressed to avoid exposing secrets.".to_string(),
                        );
                    }
                    Err(read_err) => {
                        lines.push(format!("Failed to read raw file: {read_err}"));
                    }
                }
            }
        }

        lines.push("Environment overrides (detected now):".to_string());
        let active = JiraConfig::load();
        match active {
            Ok(cfg) => {
                lines.push(format!(
                    "  JIRA_PROFILE => {}",
                    std::env::var("JIRA_PROFILE").unwrap_or_else(|_| "(unset)".to_string())
                ));
                lines.push(format!(
                    "  JIRA_URL => {}",
                    if std::env::var("JIRA_URL").is_ok() {
                        "set"
                    } else {
                        "unset"
                    }
                ));
                lines.push(format!(
                    "  JIRA_EMAIL => {}",
                    if std::env::var("JIRA_EMAIL").is_ok() {
                        "set"
                    } else {
                        "unset"
                    }
                ));
                lines.push(format!(
                    "  JIRA_TOKEN => {}",
                    if std::env::var("JIRA_TOKEN").is_ok() {
                        "set"
                    } else {
                        "unset"
                    }
                ));
                lines.push(format!(
                    "  Effective profile: {}",
                    cfg.profile_name.unwrap_or_else(|| "(unknown)".to_string())
                ));
                lines.push(format!("  Effective URL: {}", cfg.base_url));
                lines.push(format!(
                    "  Effective project: {}",
                    cfg.project.unwrap_or_else(|| "(none)".to_string())
                ));
                lines.push(format!("  Effective timeout: {}s", cfg.timeout_secs));
            }
            Err(e) => lines.push(format!("  Failed to load effective config: {e}")),
        }

        self.config_lines = lines;
    }
}

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

        match keys::handle_key(&mut app, key.code) {
            AppAction::Quit => break,

            AppAction::Refresh => {
                let jql = app.jql.clone();
                app.set_status("Refreshing...", false);
                terminal.draw(|f| ui(f, &mut app))?;
                match client.search_issues(&jql, None, Some(50)).await {
                    Ok(result) => {
                        app.set_issues(result.issues);
                        if app.focus == Focus::Detail {
                            app.warm_active_tab(&client).await;
                        }
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
                                app.focus = Focus::List;
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
                            app.warm_active_tab(&client).await;
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

            AppAction::OpenAssigneePicker(key) => {
                app.assignee_issue_key = key;
                app.focus = Focus::List;
                app.assignee_query.clear();
                app.assignee_cursor = 0;
                app.assignee_options = vec![PickerOption {
                    value: "me".to_string(),
                    label: "Assign to me".to_string(),
                }];
                app.assignee_state = ListState::default();
                app.assignee_state.select(Some(0));
                app.mode = Mode::AssigneePicker;
                app.set_status("Loading assignees...", false);
                match client.search_users("").await {
                    Ok(users) => {
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
                            if !app
                                .assignee_options
                                .iter()
                                .any(|option| option.value == account_id)
                            {
                                app.assignee_options.push(PickerOption {
                                    value: account_id.to_string(),
                                    label,
                                });
                            }
                        }
                        app.clear_status();
                    }
                    Err(e) => app.set_status(format!("Assignee lookup failed: {e}"), true),
                }
            }

            AppAction::RefreshAssigneeOptions => {
                let query = app.assignee_query.clone();
                app.set_status("Searching assignees...", false);
                match client.search_users(&query).await {
                    Ok(users) => {
                        app.assignee_options = vec![PickerOption {
                            value: "me".to_string(),
                            label: "Assign to me".to_string(),
                        }];
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
                            app.assignee_options.push(PickerOption {
                                value: account_id.to_string(),
                                label: parts.join("  •  "),
                            });
                        }
                        app.assignee_state.select(Some(0));
                        app.clear_status();
                    }
                    Err(e) => app.set_status(format!("Assignee lookup failed: {e}"), true),
                }
            }

            AppAction::OpenComponentPicker(key) => {
                app.component_issue_key = key.clone();
                app.component_query.clear();
                app.component_cursor = 0;
                app.component_selected.clear();
                app.component_options.clear();
                app.component_state = ListState::default();
                match client.get_issue(&key).await {
                    Ok(issue) => {
                        let project_key = issue
                            .key
                            .split_once('-')
                            .map(|(project, _)| project.to_string())
                            .unwrap_or(issue.project_key.clone());
                        app.component_project_key = project_key.clone();
                        app.mode = Mode::ComponentPicker;
                        app.focus = Focus::List;
                        app.set_status(format!("Loading components for {project_key}..."), false);
                        match client.get_project_components(&project_key).await {
                            Ok(components) => {
                                app.component_options = components
                                    .into_iter()
                                    .filter_map(|component| {
                                        let name =
                                            component.get("name").and_then(|v| v.as_str())?.trim();
                                        if name.is_empty() {
                                            return None;
                                        }
                                        Some(PickerOption {
                                            value: name.to_string(),
                                            label: name.to_string(),
                                        })
                                    })
                                    .collect();
                                app.component_options
                                    .sort_by_key(|option| option.label.to_lowercase());
                                app.component_state.select(Some(0));
                                app.clear_status();
                            }
                            Err(e) => app.set_status(format!("Component lookup failed: {e}"), true),
                        }
                    }
                    Err(e) => app.set_status(format!("Issue lookup failed: {e}"), true),
                }
            }

            AppAction::RefreshComponentOptions => {
                let query = app.component_query.to_lowercase();
                if let Ok(components) = client
                    .get_project_components(&app.component_project_key)
                    .await
                {
                    app.component_options = components
                        .into_iter()
                        .filter_map(|component| {
                            let name = component.get("name").and_then(|v| v.as_str())?.trim();
                            if name.is_empty() {
                                return None;
                            }
                            if !query.is_empty() && !name.to_lowercase().contains(&query) {
                                return None;
                            }
                            Some(PickerOption {
                                value: name.to_string(),
                                label: name.to_string(),
                            })
                        })
                        .collect();
                    app.component_options
                        .sort_by_key(|option| option.label.to_lowercase());
                    app.component_state.select(Some(0));
                }
            }

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

            AppAction::AssignIssue(assignee) => {
                let key = app.assignee_issue_key.clone();
                let req = UpdateIssueRequest {
                    assignee: Some(assignee),
                    ..Default::default()
                };
                match client.update_issue(&key, req).await {
                    Ok(()) => {
                        let jql = app.jql.clone();
                        if let Ok(r) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(r.issues);
                        }
                        app.set_status(format!("✓ Assigned {key}"), false);
                    }
                    Err(e) => app.set_status(format!("Assign failed: {e}"), true),
                }
            }

            AppAction::AddComment(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_add_comment(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => {
                        app.detail.comments = None;
                        app.warm_active_tab(&client).await;
                        app.set_status(format!("✓ Comment added to {key}"), false)
                    }
                    Ok(false) => app.set_status("Comment cancelled", false),
                    Err(e) => app.set_status(format!("Comment failed: {e}"), true),
                }
            }

            AppAction::AddWorklog(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_add_worklog(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => {
                        app.detail.worklogs = None;
                        app.warm_active_tab(&client).await;
                        app.set_status(format!("✓ Worklog added to {key}"), false)
                    }
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
                let components = app.component_selected.iter().cloned().collect::<Vec<_>>();
                let req = UpdateIssueRequest {
                    components: Some(components),
                    ..Default::default()
                };
                match client.update_issue(&key, req).await {
                    Ok(()) => {
                        let jql = app.jql.clone();
                        if let Ok(r) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(r.issues);
                        }
                        app.set_status(format!("✓ Components updated on {key}"), false);
                    }
                    Err(e) => app.set_status(format!("Component edit failed: {e}"), true),
                }
            }

            AppAction::UploadAttachment(key) => {
                suspend_tui(&mut terminal)?;
                let result = tui_upload_attachment(&client, &key).await;
                resume_tui(&mut terminal)?;
                match result {
                    Ok(true) => {
                        let jql = app.jql.clone();
                        if let Ok(r) = client.search_issues(&jql, None, Some(50)).await {
                            app.set_issues(r.issues);
                            app.warm_active_tab(&client).await;
                        }
                        app.set_status(format!("✓ Attachment uploaded to {key}"), false)
                    }
                    Ok(false) => app.set_status("Upload cancelled", false),
                    Err(e) => app.set_status(format!("Upload failed: {e}"), true),
                }
            }

            AppAction::SaveColumnPreferences => {
                app.prefs.visible_columns = app.visible_columns.clone();
                app.prefs.normalize();
                app.visible_columns = app.prefs.visible_columns.clone();
                match app.prefs.save() {
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

            AppAction::ApplySavedJql(jql) => {
                app.set_status("Loading saved query...", false);
                terminal.draw(|f| ui(f, &mut app))?;
                match client.search_issues(&jql, None, Some(50)).await {
                    Ok(result) => {
                        app.jql = jql;
                        app.set_issues(result.issues);
                        app.clear_status();
                    }
                    Err(e) => app.set_status(format!("Saved query failed: {e}"), true),
                }
            }

            AppAction::CreateSavedJql => {
                let current_jql = (!app.jql.trim().is_empty()).then_some(app.jql.as_str());
                suspend_tui(&mut terminal)?;
                let result = tui_edit_saved_jql(None, current_jql);
                resume_tui(&mut terminal)?;
                match result {
                    Ok(Some(saved)) => {
                        app.prefs.saved_jqls.push(saved);
                        let new_index = app.prefs.saved_jqls.len().saturating_sub(1);
                        app.saved_jql_state.select(Some(new_index));
                        match app.prefs.save() {
                            Ok(()) => app.set_status("✓ Saved query added", false),
                            Err(e) => app.set_status(
                                format!("Failed to save saved query preferences: {e}"),
                                true,
                            ),
                        }
                    }
                    Ok(None) => app.set_status("Saved query create cancelled", false),
                    Err(e) => app.set_status(format!("Saved query create failed: {e}"), true),
                }
            }

            AppAction::EditSavedJql(index) => {
                let existing = app.prefs.saved_jqls.get(index).cloned();
                suspend_tui(&mut terminal)?;
                let result = tui_edit_saved_jql(existing.as_ref(), None);
                resume_tui(&mut terminal)?;
                match result {
                    Ok(Some(saved)) => {
                        if let Some(slot) = app.prefs.saved_jqls.get_mut(index) {
                            *slot = saved;
                        }
                        app.saved_jql_state.select(Some(index));
                        match app.prefs.save() {
                            Ok(()) => app.set_status("✓ Saved query updated", false),
                            Err(e) => app.set_status(
                                format!("Failed to save saved query preferences: {e}"),
                                true,
                            ),
                        }
                    }
                    Ok(None) => app.set_status("Saved query edit cancelled", false),
                    Err(e) => app.set_status(format!("Saved query edit failed: {e}"), true),
                }
            }

            AppAction::DeleteSavedJql(index) => {
                let existing = app.prefs.saved_jqls.get(index).cloned();
                if let Some(saved) = existing {
                    suspend_tui(&mut terminal)?;
                    let result = tui_confirm_delete_saved_jql(&saved);
                    resume_tui(&mut terminal)?;
                    match result {
                        Ok(true) => {
                            app.prefs.saved_jqls.remove(index);
                            app.clamp_saved_jql_selection();
                            match app.prefs.save() {
                                Ok(()) => app.set_status("✓ Saved query deleted", false),
                                Err(e) => app.set_status(
                                    format!("Failed to save saved query preferences: {e}"),
                                    true,
                                ),
                            }
                        }
                        Ok(false) => app.set_status("Saved query delete cancelled", false),
                        Err(e) => app.set_status(format!("Saved query delete failed: {e}"), true),
                    }
                }
            }

            AppAction::SaveTheme => {
                app.prefs.theme = app.selected_theme();
                match app.prefs.save() {
                    Ok(()) => {
                        app.set_status(format!("✓ Theme set to {}", app.prefs.theme.label()), false)
                    }
                    Err(e) => app.set_status(format!("Theme save failed: {e}"), true),
                }
            }

            AppAction::LoadServerInfo => {
                app.set_status("Loading server info...", false);
                terminal.draw(|f| ui(f, &mut app))?;
                match client.get_server_info().await {
                    Ok(info) => {
                        let mut lines = Vec::new();
                        let field = |key: &str| info.get(key).and_then(|v| v.as_str());

                        lines.push("Server Summary".to_string());
                        lines.push(String::new());
                        lines.push(format!(
                            "Base URL: {}",
                            field("baseUrl").unwrap_or(&app.base_url)
                        ));
                        lines.push(format!(
                            "Version: {}",
                            field("version").unwrap_or("unknown")
                        ));
                        lines.push(format!(
                            "Build number: {}",
                            info.get("buildNumber")
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        ));
                        lines.push(format!(
                            "Deployment type: {}",
                            field("deploymentType").unwrap_or("unknown")
                        ));
                        lines.push(format!(
                            "Version numbers: {}",
                            info.get("versionNumbers")
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        ));
                        lines.push(format!(
                            "Default locale: {}",
                            field("defaultLocale").unwrap_or("unknown")
                        ));
                        lines.push(String::new());
                        lines.push("Raw preview:".to_string());
                        lines.extend(
                            serde_json::to_string_pretty(&info)
                                .unwrap_or_else(|_| format!("{info:#?}"))
                                .lines()
                                .take(40)
                                .map(|line| line.to_string()),
                        );
                        app.server_info_lines = lines;
                        app.clear_status();
                    }
                    Err(e) => app.set_status(format!("Server info failed: {e}"), true),
                }
            }

            AppAction::LoadConfigView => {
                app.load_config_lines();
                app.clear_status();
            }

            AppAction::WarmActiveTab => {
                app.warm_active_tab(&client).await;
            }

            AppAction::None => {}
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
