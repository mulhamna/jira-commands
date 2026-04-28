use std::io;

use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use jira_core::{model::UpdateIssueRequest, JiraClient};
use ratatui::Terminal;

use super::prefs::SavedJql;

use super::picker::prompt_assignee_selection;

pub(super) fn suspend_tui<B>(terminal: &mut Terminal<B>) -> Result<()>
where
    B: ratatui::backend::Backend + io::Write,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

pub(super) fn resume_tui<B>(terminal: &mut Terminal<B>) -> Result<()>
where
    B: ratatui::backend::Backend + io::Write,
    B::Error: std::error::Error + Send + Sync + 'static,
{
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;
    Ok(())
}

pub(super) async fn tui_create_issue(
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
    Ok(Some(issue.key))
}

pub(super) fn tui_edit_saved_jql(
    existing: Option<&SavedJql>,
    suggested_jql: Option<&str>,
) -> Result<Option<SavedJql>> {
    use inquire::Text;

    println!("\n── Saved Query ─────────────────────────────────────");

    let name_default = existing.map(|saved| saved.name.as_str()).unwrap_or("");
    let jql_default = existing
        .map(|saved| saved.jql.as_str())
        .or(suggested_jql)
        .unwrap_or("");

    let name = match Text::new("Name:")
        .with_default(name_default)
        .prompt_skippable()?
    {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return Ok(None),
    };

    let jql = match Text::new("JQL:")
        .with_default(jql_default)
        .prompt_skippable()?
    {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return Ok(None),
    };

    Ok(Some(SavedJql { name, jql }))
}

pub(super) fn tui_confirm_delete_saved_jql(saved: &SavedJql) -> Result<bool> {
    use inquire::Confirm;

    println!("\n── Delete Saved Query ─────────────────────────────");
    println!("  {}", saved.name);
    println!("  {}\n", saved.jql);

    Confirm::new("Delete this saved query?")
        .with_default(false)
        .prompt()
        .map_err(Into::into)
}

pub(super) async fn tui_edit_labels(client: &JiraClient, key: &str) -> Result<bool> {
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
    Ok(true)
}
