use anyhow::Result;
use jira_core::{
    model::{Issue, ProjectVersion},
    JiraClient,
};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct VersionBacklogPreview {
    pub version: ProjectVersion,
    pub total_open: u64,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone)]
pub struct IssueVersionInsight {
    pub issue_key: String,
    pub project_key: String,
    pub issue_fix_versions: Vec<String>,
    pub project_versions: Vec<ProjectVersion>,
    pub previews: Vec<VersionBacklogPreview>,
}

pub async fn load_issue_version_insight(
    client: &JiraClient,
    issue_key: &str,
    preview_limit: u32,
) -> Result<IssueVersionInsight> {
    let issue = client.get_issue(issue_key).await?;
    let project_key = issue
        .key
        .split_once('-')
        .map(|(project, _)| project.to_string())
        .unwrap_or_else(|| issue.project_key.clone());
    let issue_fix_versions = extract_fix_versions(&issue.fields);

    let mut project_versions = client.get_project_versions(&project_key).await?;
    sort_project_versions(&mut project_versions);

    let mut previews = Vec::new();
    for version_name in &issue_fix_versions {
        let Some(version) = project_versions
            .iter()
            .find(|version| version.name == *version_name)
            .cloned()
        else {
            continue;
        };

        let jql = format!(
            "project = \"{}\" AND fixVersion = \"{}\" AND statusCategory != Done ORDER BY updated DESC",
            escape_jql(&project_key),
            escape_jql(version_name),
        );
        let result = client
            .search_issues(&jql, None, Some(preview_limit))
            .await?;
        previews.push(VersionBacklogPreview {
            version,
            total_open: result.total.unwrap_or(result.issues.len() as u64),
            issues: result.issues,
        });
    }

    Ok(IssueVersionInsight {
        issue_key: issue.key,
        project_key,
        issue_fix_versions,
        project_versions,
        previews,
    })
}

pub async fn load_project_versions(
    client: &JiraClient,
    project_key: &str,
) -> Result<Vec<ProjectVersion>> {
    let mut versions = client.get_project_versions(project_key).await?;
    sort_project_versions(&mut versions);
    Ok(versions)
}

pub async fn load_version_backlog_preview(
    client: &JiraClient,
    project_key: &str,
    version: ProjectVersion,
    preview_limit: u32,
) -> Result<VersionBacklogPreview> {
    let jql = format!(
        "project = \"{}\" AND fixVersion = \"{}\" AND statusCategory != Done ORDER BY updated DESC",
        escape_jql(project_key),
        escape_jql(&version.name),
    );
    let result = client
        .search_issues(&jql, None, Some(preview_limit))
        .await?;
    Ok(VersionBacklogPreview {
        version,
        total_open: result.total.unwrap_or(result.issues.len() as u64),
        issues: result.issues,
    })
}

pub fn extract_fix_versions(fields: &Value) -> Vec<String> {
    fields
        .get("fixVersions")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("name").and_then(|v| v.as_str()))
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn escape_jql(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn sort_project_versions(versions: &mut [ProjectVersion]) {
    versions.sort_by_key(version_sort_key);
}

fn version_sort_key(version: &ProjectVersion) -> (u8, String, String) {
    let bucket = if version.archived {
        2
    } else if version.released {
        1
    } else {
        0
    };
    let date = version
        .release_date
        .clone()
        .or_else(|| version.start_date.clone())
        .unwrap_or_default();
    (bucket, date, version.name.to_lowercase())
}
