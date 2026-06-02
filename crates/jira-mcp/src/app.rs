use std::{collections::HashMap, path::PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use jira_core::{
    config::{
        config_file_path, default_profile_name, parse_auth_type, parse_deployment, JiraConfig,
        JiraProfilesFile,
    },
    model::{
        field::{Field, FieldValue},
        CreateIssueRequestV2, CreateProjectVersionRequest, UpdateIssueRequest,
        UpdateProjectVersionRequest,
    },
    JiraClient,
};
use serde::Serialize;
use serde_json::{json, Value};
use url::form_urlencoded;

use crate::{
    error::{AppError, AppResult},
    models::{
        ApiRequestArgs, ArchiveArgs, AttachmentInput, AuthSetCredentialsArgs, BulkTransitionArgs,
        BulkUpdateArgs, CommentAddArgs, IssueAttachArgs, IssueCloneArgs, IssueCreateArgs,
        IssueDeleteArgs, IssueFieldsArgs, IssueKeyArgs, IssueLinkCreateArgs, IssueLinkDeleteArgs,
        IssueListArgs, IssueTransitionArgs, IssueTypesListArgs, IssueUpdateArgs, ProjectKeyArgs,
        ProjectVersionCreateArgs, ProjectVersionUpdateArgs, RemoteLinkAddArgs,
        RemoteLinkDeleteArgs, SprintAddIssueArgs, SprintCreateArgs, SprintDeleteArgs,
        SprintListArgs, SprintUpdateArgs, WorklogAddArgs, WorklogDeleteArgs,
    },
};

#[derive(Debug, Clone, Default)]
pub struct JiraApp;

impl JiraApp {
    pub fn auth_status(&self) -> AppResult<Value> {
        let config = self.load_config()?;
        let store = JiraProfilesFile::load()?;
        Ok(json!({
            "configured": !config.base_url.is_empty() && (!config.requires_user_identity() || !config.email.is_empty()),
            "profile": config.profile_name.clone(),
            "url": value_or_null(config.base_url.clone()),
            "email": value_or_null(config.email.clone()),
            "token_present": config.token_present(),
            "project": config.project,
            "timeout_secs": config.timeout_secs,
            "deployment": format!("{:?}", config.deployment).to_lowercase(),
            "auth_type": format!("{:?}", config.auth_type).to_lowercase(),
            "api_version": config.api_version,
            "profiles": store.profiles.keys().cloned().collect::<Vec<_>>(),
            "config_path": config_file_path().display().to_string()
        }))
    }

    pub fn auth_set_credentials(&self, args: AuthSetCredentialsArgs) -> AppResult<Value> {
        if args.url.is_none()
            && args.email.is_none()
            && args.token.is_none()
            && args.project.is_none()
            && args.timeout_secs.is_none()
            && args.deployment.is_none()
            && args.auth_type.is_none()
        {
            return Err(AppError::validation(
                "Provide at least one of url, email, token, project, timeout_secs, deployment, or auth_type",
            ));
        }

        let store = JiraProfilesFile::load().unwrap_or_default();
        let profile_name = args
            .profile
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| store.current_profile_name())
            .unwrap_or_else(default_profile_name);

        let mut config: JiraConfig = store
            .profiles
            .get(&profile_name)
            .cloned()
            .map(Into::into)
            .unwrap_or_else(JiraConfig::default);
        config.profile_name = Some(profile_name.clone());

        if let Some(url) = args.url {
            config.base_url = url.trim().to_string();
        }
        if let Some(email) = args.email {
            config.email = email.trim().to_string();
        }
        if let Some(token) = args.token {
            config.token = Some(token);
        }
        if let Some(project) = args.project {
            config.project = if project.trim().is_empty() {
                None
            } else {
                Some(project.trim().to_string())
            };
        }
        if let Some(timeout_secs) = args.timeout_secs {
            config.timeout_secs = timeout_secs;
        }
        if let Some(deployment) = args.deployment {
            config.deployment = parse_deployment(&deployment)
                .ok_or_else(|| AppError::validation("deployment must be cloud or datacenter"))?;
            config.api_version = 0;
        }
        if let Some(auth_type) = args.auth_type {
            config.auth_type = parse_auth_type(&auth_type).ok_or_else(|| {
                AppError::validation(
                    "auth_type must be cloud_api_token, datacenter_pat, or datacenter_basic",
                )
            })?;
        }
        if !config.requires_user_identity() {
            config.email.clear();
        }

        config.save()?;
        self.auth_status()
    }

    pub fn auth_logout(&self) -> AppResult<Value> {
        let mut config = self.load_config().unwrap_or_default();
        config.token = None;
        config.save()?;
        self.auth_status()
    }

    pub async fn issue_list(&self, args: IssueListArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let limit = args.limit.unwrap_or(25);
        if !(1..=100).contains(&limit) {
            return Err(AppError::validation("limit must be between 1 and 100"));
        }

        let jql = if let Some(jql) = args.jql {
            jql
        } else if let Some(project_key) = args.project_key {
            format!("project = {project_key} ORDER BY updated DESC")
        } else {
            "assignee = currentUser() ORDER BY updated DESC".to_string()
        };

        let result = client.search_issues(&jql, None, Some(limit)).await?;
        Ok(json!({
            "jql": jql,
            "issues": result.issues,
            "next_page_token": result.next_page_token,
            "total": result.total
        }))
    }

    pub async fn issue_view(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let issue = client.get_issue(&args.key).await?;
        to_value(issue)
    }

    pub async fn issue_types_list(&self, args: IssueTypesListArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let issue_types = client.get_issue_types(&args.project_key).await?;
        Ok(json!({
            "project_key": args.project_key,
            "issue_types": issue_types
        }))
    }

    pub async fn issue_fields(&self, args: IssueFieldsArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let mut fields: Vec<Field> = if let Some(issue_type_id) = args.issue_type_id {
            client
                .get_fields_for_issue_type(&args.project_key, &issue_type_id)
                .await?
        } else {
            client.get_project_fields(&args.project_key).await?
        };

        if args.required_only.unwrap_or(false) {
            fields.retain(|field| field.required);
        }

        Ok(json!({
            "project_key": args.project_key,
            "fields": fields
        }))
    }

    pub async fn sprint_list(&self, args: SprintListArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let sprints = if let Some(states) = args.states.as_ref().filter(|states| !states.is_empty())
        {
            let normalized: Vec<String> = states
                .iter()
                .map(|state| normalize_sprint_state(state))
                .collect::<AppResult<_>>()?;
            let state_refs: Vec<&str> = normalized.iter().map(String::as_str).collect();
            client
                .list_sprints_for_project_with_states(&args.project_key, &state_refs)
                .await?
        } else {
            client.list_sprints_for_project(&args.project_key).await?
        };

        Ok(json!({
            "project_key": args.project_key,
            "sprints": sprints
        }))
    }

    pub async fn sprint_create(&self, args: SprintCreateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let sprint = client
            .create_sprint(
                args.board_id,
                &args.name,
                args.start_date.as_deref(),
                args.end_date.as_deref(),
                args.goal.as_deref(),
            )
            .await?;
        to_value(sprint)
    }

    pub async fn sprint_update(&self, args: SprintUpdateArgs) -> AppResult<Value> {
        let has_changes = args.name.is_some()
            || args.state.is_some()
            || args.start_date.is_some()
            || args.end_date.is_some()
            || args.goal.is_some();
        if !has_changes {
            return Err(AppError::validation(
                "Provide at least one sprint field to update",
            ));
        }

        let mut body = serde_json::Map::new();
        if let Some(name) = args.name {
            body.insert("name".into(), Value::String(name));
        }
        if let Some(state) = args.state {
            body.insert(
                "state".into(),
                Value::String(normalize_sprint_state(&state)?),
            );
        }
        if let Some(start_date) = args.start_date {
            body.insert("startDate".into(), Value::String(start_date));
        }
        if let Some(end_date) = args.end_date {
            body.insert("endDate".into(), Value::String(end_date));
        }
        if let Some(goal) = args.goal {
            body.insert("goal".into(), Value::String(goal));
        }

        let client = self.build_client()?;
        let sprint = client
            .update_sprint(args.sprint_id, Value::Object(body))
            .await?;
        to_value(sprint)
    }

    pub async fn sprint_delete(&self, args: SprintDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_sprint(args.sprint_id).await?;
        Ok(json!({
            "sprint_id": args.sprint_id,
            "deleted": true
        }))
    }

    pub async fn sprint_add_issue(&self, args: SprintAddIssueArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        client
            .add_issue_to_sprint(args.sprint_id, &args.issue_key)
            .await?;
        Ok(json!({
            "sprint_id": args.sprint_id,
            "issue_key": args.issue_key,
            "added": true
        }))
    }

    pub async fn project_component_list(&self, args: ProjectKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let components = client.get_project_components(&args.project_key).await?;
        Ok(json!({
            "project_key": args.project_key,
            "components": components
        }))
    }

    pub async fn project_version_list(&self, args: ProjectKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let versions = client.get_project_versions(&args.project_key).await?;
        Ok(json!({
            "project_key": args.project_key,
            "versions": versions
        }))
    }

    pub async fn project_version_create(&self, args: ProjectVersionCreateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let version = client
            .create_project_version(&CreateProjectVersionRequest {
                name: args.name,
                project: args.project_key,
                description: args.description,
                archived: args.archived.unwrap_or(false),
                released: args.released.unwrap_or(false),
                release_date: args.release_date,
                start_date: args.start_date,
            })
            .await?;
        to_value(version)
    }

    pub async fn project_version_update(&self, args: ProjectVersionUpdateArgs) -> AppResult<Value> {
        let has_changes = args.name.is_some()
            || args.description.is_some()
            || args.archived.is_some()
            || args.released.is_some()
            || args.release_date.is_some()
            || args.start_date.is_some();
        if !has_changes {
            return Err(AppError::validation(
                "Provide at least one project version field to update",
            ));
        }

        let client = self.build_client()?;
        let version = client
            .update_project_version(
                &args.version_id,
                &UpdateProjectVersionRequest {
                    name: args.name,
                    description: args.description,
                    archived: args.archived,
                    released: args.released,
                    release_date: args.release_date,
                    start_date: args.start_date,
                },
            )
            .await?;
        to_value(version)
    }

    pub async fn issue_transitions_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let transitions = client.get_transitions(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "transitions": transitions
        }))
    }

    pub async fn issue_create(&self, args: IssueCreateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let issue = client
            .create_issue_v2(CreateIssueRequestV2 {
                project_key: args.project_key,
                summary: args.summary,
                description: args.description,
                description_adf: args.description_adf,
                issue_type: args.issue_type,
                assignee: args.assignee,
                priority: args.priority,
                labels: args.labels.unwrap_or_default(),
                components: args.components.unwrap_or_default(),
                parent: args.parent,
                fix_versions: args.fix_versions.unwrap_or_default(),
                custom_fields: map_custom_fields(args.custom_fields),
            })
            .await?;

        to_value(issue)
    }

    pub async fn issue_update(&self, args: IssueUpdateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let custom_fields = map_custom_fields(args.custom_fields);
        let has_changes = args.summary.is_some()
            || args.description.is_some()
            || args.description_adf.is_some()
            || args.assignee.is_some()
            || args.priority.is_some()
            || args.labels.is_some()
            || args.components.is_some()
            || args.parent.is_some()
            || args.fix_versions.is_some()
            || !custom_fields.is_empty();

        if !has_changes {
            return Err(AppError::validation(
                "Provide at least one field to update on the issue",
            ));
        }

        let key = args.key;
        client
            .update_issue(
                &key,
                UpdateIssueRequest {
                    summary: args.summary,
                    description: args.description,
                    description_adf: args.description_adf,
                    assignee: args.assignee,
                    priority: args.priority,
                    labels: args.labels,
                    components: args.components,
                    fix_versions: args.fix_versions,
                    parent: args.parent,
                    custom_fields,
                    ..Default::default()
                },
            )
            .await?;
        let issue = client.get_issue(&key).await?;
        to_value(issue)
    }

    pub async fn issue_delete(&self, args: IssueDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_issue(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "deleted": true
        }))
    }

    pub async fn issue_clone(&self, args: IssueCloneArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let source = client.get_issue(&args.key).await?;
        let target_project = args
            .project_key
            .unwrap_or_else(|| source.project_key.clone());
        let summary = args.summary.unwrap_or_else(|| source.summary.clone());

        let labels: Vec<String> = source
            .fields
            .get("labels")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let components: Vec<String> = source
            .fields
            .get("components")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| c.get("name").and_then(|n| n.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let fix_versions: Vec<String> = source
            .fields
            .get("fixVersions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let cloned = client
            .create_issue_v2(CreateIssueRequestV2 {
                project_key: target_project,
                summary,
                description: None,
                description_adf: source.description.clone(),
                issue_type: source.issue_type.clone(),
                assignee: args.assignee,
                priority: source.priority.clone(),
                labels,
                components,
                parent: None,
                fix_versions,
                custom_fields: HashMap::new(),
            })
            .await?;

        let move_original = args.move_original.unwrap_or(false);
        let mut deleted_original = false;
        if move_original {
            require_confirm(args.confirm)?;
            client.delete_issue(&args.key).await?;
            deleted_original = true;
        }

        Ok(json!({
            "source_key": args.key,
            "move_original": move_original,
            "deleted_original": deleted_original,
            "issue": cloned
        }))
    }

    pub async fn issue_transition(&self, args: IssueTransitionArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let resolved = resolve_transition(&client, &args.key, &args.transition).await?;
        client.transition_issue(&args.key, &resolved.id).await?;
        let issue = client.get_issue(&args.key).await?;
        Ok(json!({
            "transition": {
                "id": resolved.id,
                "name": resolved.name
            },
            "issue": issue
        }))
    }

    pub async fn issue_attach(&self, args: IssueAttachArgs) -> AppResult<Value> {
        if args.attachments.is_empty() {
            return Err(AppError::validation("attachments must not be empty"));
        }

        let client = self.build_client()?;
        let mut uploaded = Vec::new();

        for attachment in args.attachments {
            let mut result = match attachment {
                AttachmentInput::Path { path } => {
                    let path = PathBuf::from(path);
                    client.upload_attachment(&args.key, path.as_path()).await?
                }
                AttachmentInput::Inline {
                    filename,
                    media_type,
                    base64,
                } => {
                    let bytes = STANDARD.decode(base64)?;
                    client
                        .upload_attachment_bytes(&args.key, &filename, bytes, media_type.as_deref())
                        .await?
                }
            };
            uploaded.append(&mut result);
        }

        Ok(json!({
            "key": args.key,
            "attachments": uploaded
        }))
    }

    pub async fn comment_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let comments = client.get_comments(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "comments": comments
        }))
    }

    pub async fn comment_add(&self, args: CommentAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let comment = client.add_comment(&args.key, &args.body).await?;
        to_value(comment)
    }

    pub async fn issue_link_types_list(&self) -> AppResult<Value> {
        let client = self.build_client()?;
        let link_types = client.list_issue_link_types().await?;
        Ok(json!({
            "link_types": link_types
        }))
    }

    pub async fn issue_link_create(&self, args: IssueLinkCreateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        client
            .link_issues(
                &args.outward_key,
                &args.inward_key,
                &args.link_type,
                args.comment.as_deref(),
            )
            .await?;
        Ok(json!({
            "outward_key": args.outward_key,
            "inward_key": args.inward_key,
            "link_type": args.link_type,
            "created": true
        }))
    }

    pub async fn issue_link_delete(&self, args: IssueLinkDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_issue_link(&args.link_id).await?;
        Ok(json!({
            "link_id": args.link_id,
            "deleted": true
        }))
    }

    pub async fn remote_link_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let links = client.get_remote_links(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "remote_links": links
        }))
    }

    pub async fn remote_link_add(&self, args: RemoteLinkAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let link = client
            .add_remote_link(&args.key, &args.url, &args.title)
            .await?;
        Ok(json!({
            "key": args.key,
            "remote_link": link
        }))
    }

    pub async fn remote_link_delete(&self, args: RemoteLinkDeleteArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        client.delete_remote_link(&args.key, &args.link_id).await?;
        Ok(json!({
            "key": args.key,
            "link_id": args.link_id,
            "deleted": true
        }))
    }

    pub async fn worklog_list(&self, args: IssueKeyArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let worklogs = client.get_worklogs(&args.key).await?;
        Ok(json!({
            "key": args.key,
            "worklogs": worklogs
        }))
    }

    pub async fn worklog_add(&self, args: WorklogAddArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let worklog = client
            .add_worklog(
                &args.key,
                &args.time_spent,
                args.comment.as_deref(),
                args.started.as_deref(),
            )
            .await?;
        to_value(worklog)
    }

    pub async fn worklog_delete(&self, args: WorklogDeleteArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        client.delete_worklog(&args.key, &args.id).await?;
        Ok(json!({
            "key": args.key,
            "id": args.id,
            "deleted": true
        }))
    }

    pub async fn issue_bulk_transition(&self, args: BulkTransitionArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        let issues = client.get_all_issues(&args.jql).await?;
        if issues.is_empty() {
            return Ok(json!({
                "jql": args.jql,
                "total": 0,
                "succeeded": 0,
                "failed_count": 0,
                "failed": []
            }));
        }

        let transition = resolve_transition(&client, &issues[0].key, &args.to).await?;
        let total = issues.len();
        let mut succeeded = 0usize;
        let mut failed = Vec::new();

        for issue in issues {
            match client.transition_issue(&issue.key, &transition.id).await {
                Ok(_) => succeeded += 1,
                Err(err) => failed.push(json!({
                    "key": issue.key,
                    "error": err.to_string()
                })),
            }
        }

        Ok(json!({
            "jql": args.jql,
            "transition": {
                "id": transition.id,
                "name": transition.name
            },
            "total": total,
            "succeeded": succeeded,
            "failed_count": failed.len(),
            "failed": failed
        }))
    }

    pub async fn issue_bulk_update(&self, args: BulkUpdateArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        if args.assignee.is_none() && args.priority.is_none() {
            return Err(AppError::validation(
                "Provide assignee and/or priority for a bulk update",
            ));
        }

        let client = self.build_client()?;
        let issues = client.get_all_issues(&args.jql).await?;
        if issues.is_empty() {
            return Ok(json!({
                "jql": args.jql,
                "total": 0,
                "succeeded": 0,
                "failed_count": 0,
                "failed": []
            }));
        }

        let total = issues.len();
        let request = UpdateIssueRequest {
            assignee: args.assignee,
            priority: args.priority,
            ..Default::default()
        };
        let mut succeeded = 0usize;
        let mut failed = Vec::new();

        for issue in issues {
            match client.update_issue(&issue.key, request.clone()).await {
                Ok(_) => succeeded += 1,
                Err(err) => failed.push(json!({
                    "key": issue.key,
                    "error": err.to_string()
                })),
            }
        }

        Ok(json!({
            "jql": args.jql,
            "total": total,
            "succeeded": succeeded,
            "failed_count": failed.len(),
            "failed": failed
        }))
    }

    pub async fn issue_archive(&self, args: ArchiveArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        let issues = client.get_all_issues(&args.jql).await?;
        let keys: Vec<String> = issues.into_iter().map(|issue| issue.key).collect();

        if keys.is_empty() {
            return Ok(json!({
                "jql": args.jql,
                "total": 0,
                "archived": 0,
                "keys": []
            }));
        }

        client.archive_issues(&keys).await?;
        Ok(json!({
            "jql": args.jql,
            "total": keys.len(),
            "archived": keys.len(),
            "keys": keys
        }))
    }

    pub async fn plan_list(&self) -> AppResult<Value> {
        let client = self.build_client()?;
        let plans = client.get_plans().await?;
        Ok(json!({ "plans": plans }))
    }

    pub async fn api_request(&self, args: ApiRequestArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let method = normalize_method(&args.method)?;
        let path = build_api_path(args.path, args.query)?;
        let body = client.raw_request(&method, &path, args.body).await?;
        Ok(json!({
            "method": method,
            "path": path,
            "body": body
        }))
    }

    fn load_config(&self) -> AppResult<JiraConfig> {
        JiraConfig::load().map_err(Into::into)
    }

    fn build_client(&self) -> AppResult<JiraClient> {
        let config = self.load_config()?;

        if config.base_url.trim().is_empty() {
            return Err(AppError::auth_missing(
                "Jira URL not configured. Set JIRA_URL or save credentials first.",
            ));
        }
        if config.requires_user_identity() && config.email.trim().is_empty() {
            return Err(AppError::auth_missing(
                "Jira user identity not configured. Set JIRA_EMAIL or save credentials first.",
            ));
        }
        if !config.token_present() {
            return Err(AppError::auth_missing(
                "Jira API token not configured. Set JIRA_TOKEN or save credentials first.",
            ));
        }

        Ok(JiraClient::new(config))
    }
}

#[derive(Debug, Clone)]
struct ResolvedTransition {
    id: String,
    name: String,
}

fn normalize_sprint_state(state: &str) -> AppResult<String> {
    let normalized = state.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "active" | "future" | "closed" => Ok(normalized),
        _ => Err(AppError::validation(
            "sprint state must be one of active, future, or closed",
        )),
    }
}

async fn resolve_transition(
    client: &JiraClient,
    key: &str,
    name_or_id: &str,
) -> AppResult<ResolvedTransition> {
    let transitions = client.get_transitions(key).await?;
    let found = transitions
        .into_iter()
        .find(|t| t.id == name_or_id || t.name.eq_ignore_ascii_case(name_or_id))
        .ok_or_else(|| {
            AppError::not_found(
                format!("Transition '{name_or_id}' not found for {key}"),
                Some(json!({ "key": key })),
            )
        })?;

    Ok(ResolvedTransition {
        id: found.id,
        name: found.name,
    })
}

fn map_custom_fields(
    custom_fields: Option<std::collections::BTreeMap<String, Value>>,
) -> HashMap<String, FieldValue> {
    custom_fields
        .unwrap_or_default()
        .into_iter()
        .map(|(key, value)| (key, FieldValue::Raw(value)))
        .collect()
}

fn require_confirm(confirm: Option<bool>) -> AppResult<()> {
    if confirm == Some(true) {
        Ok(())
    } else {
        Err(AppError::unsafe_operation(
            "This operation requires confirm=true",
        ))
    }
}

fn normalize_method(method: &str) -> AppResult<String> {
    let method = method.trim().to_ascii_uppercase();
    match method.as_str() {
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" => Ok(method),
        _ => Err(AppError::validation(
            "method must be one of GET, POST, PUT, PATCH, or DELETE",
        )),
    }
}

fn build_api_path(
    path: String,
    query: Option<std::collections::BTreeMap<String, Value>>,
) -> AppResult<String> {
    if !path.starts_with('/') {
        return Err(AppError::validation("path must start with '/'"));
    }
    if path.contains('?') && query.is_some() {
        return Err(AppError::validation(
            "path already contains a query string; omit the query argument",
        ));
    }

    let Some(query) = query else {
        return Ok(path);
    };
    if query.is_empty() {
        return Ok(path);
    }

    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in query {
        serializer.append_pair(&key, &query_value_to_string(key.as_str(), value)?);
    }
    let encoded = serializer.finish();
    Ok(format!("{path}?{encoded}"))
}

fn query_value_to_string(key: &str, value: Value) -> AppResult<String> {
    match value {
        Value::Null => Err(AppError::validation(format!(
            "query parameter '{key}' cannot be null"
        ))),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Number(value) => Ok(value.to_string()),
        Value::String(value) => Ok(value),
        Value::Array(_) | Value::Object(_) => Err(AppError::validation(format!(
            "query parameter '{key}' must be a string, number, or boolean"
        ))),
    }
}

fn to_value<T>(value: T) -> AppResult<Value>
where
    T: Serialize,
{
    serde_json::to_value(value).map_err(Into::into)
}

fn value_or_null(value: String) -> Value {
    if value.trim().is_empty() {
        Value::Null
    } else {
        Value::String(value)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;
    use tempfile::TempDir;
    use wiremock::{
        matchers::{method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    use super::*;

    fn set_config_home_vars(temp_dir: &TempDir) {
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());
        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("USERPROFILE", temp_dir.path());
        std::env::set_var("APPDATA", temp_dir.path());
        std::env::set_var("LOCALAPPDATA", temp_dir.path());
    }

    fn clear_config_home_vars() {
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("HOME");
        std::env::remove_var("USERPROFILE");
        std::env::remove_var("APPDATA");
        std::env::remove_var("LOCALAPPDATA");
    }

    fn set_test_env(temp_dir: &TempDir, base_url: Option<&str>) {
        set_config_home_vars(temp_dir);
        match base_url {
            Some(base_url) => {
                std::env::set_var("JIRA_URL", base_url);
                std::env::set_var("JIRA_EMAIL", "dev@example.com");
                std::env::set_var("JIRA_TOKEN", "token-123");
            }
            None => {
                std::env::remove_var("JIRA_URL");
                std::env::remove_var("JIRA_EMAIL");
                std::env::remove_var("JIRA_TOKEN");
            }
        }
    }

    fn clear_test_env() {
        clear_config_home_vars();
        std::env::remove_var("JIRA_URL");
        std::env::remove_var("JIRA_EMAIL");
        std::env::remove_var("JIRA_TOKEN");
    }

    fn sample_issue() -> Value {
        json!({
            "id": "10001",
            "key": "PROJ-1",
            "fields": {
                "summary": "Sample issue",
                "description": null,
                "status": { "name": "To Do" },
                "assignee": { "displayName": "Dev User" },
                "reporter": { "displayName": "Reporter User" },
                "priority": { "name": "High" },
                "issuetype": { "name": "Task" },
                "project": { "key": "PROJ" },
                "created": "2026-04-19T00:00:00.000+0000",
                "updated": "2026-04-19T00:00:00.000+0000",
                "attachment": []
            }
        })
    }

    #[tokio::test]
    #[serial]
    async fn destructive_actions_require_confirm() {
        let err = JiraApp
            .issue_delete(IssueDeleteArgs {
                key: "PROJ-1".into(),
                confirm: None,
            })
            .await
            .expect_err("missing confirm should fail");

        assert_eq!(err.to_mcp().message, "unsafe_operation");
    }

    #[tokio::test]
    #[serial]
    async fn auth_round_trip_uses_shared_config_file() {
        let temp_dir = TempDir::new().expect("tempdir");
        set_test_env(&temp_dir, None);

        let status = JiraApp
            .auth_set_credentials(AuthSetCredentialsArgs {
                profile: None,
                url: Some("https://example.atlassian.net".into()),
                email: Some("dev@example.com".into()),
                token: Some("secret".into()),
                project: Some("PROJ".into()),
                timeout_secs: Some(45),
                deployment: None,
                auth_type: None,
            })
            .expect("set credentials");
        assert_eq!(status["token_present"], Value::Bool(true));
        assert_eq!(status["project"], Value::String("PROJ".into()));

        let status = JiraApp.auth_logout().expect("logout");
        assert_eq!(status["token_present"], Value::Bool(false));

        clear_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn issue_list_defaults_to_current_user_jql() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mock_server = MockServer::start().await;
        set_test_env(&temp_dir, Some(&mock_server.uri()));

        Mock::given(method("POST"))
            .and(path("/rest/api/3/search/jql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issues": [sample_issue()],
                "nextPageToken": null,
                "total": 1
            })))
            .mount(&mock_server)
            .await;

        let result = JiraApp
            .issue_list(IssueListArgs {
                project_key: None,
                jql: None,
                limit: Some(10),
            })
            .await
            .expect("issue list");

        assert_eq!(
            result["jql"],
            Value::String("assignee = currentUser() ORDER BY updated DESC".into())
        );
        assert_eq!(result["issues"].as_array().map(Vec::len), Some(1));

        clear_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn api_request_serializes_query_parameters() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mock_server = MockServer::start().await;
        set_test_env(&temp_dir, Some(&mock_server.uri()));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/project"))
            .and(query_param("expand", "lead"))
            .and(query_param("startAt", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "ok": true
            })))
            .mount(&mock_server)
            .await;

        let response = JiraApp
            .api_request(ApiRequestArgs {
                method: "get".into(),
                path: "/rest/api/3/project".into(),
                query: Some(
                    [
                        ("expand".to_string(), Value::String("lead".into())),
                        ("startAt".to_string(), Value::Number(1.into())),
                    ]
                    .into_iter()
                    .collect(),
                ),
                body: None,
            })
            .await
            .expect("api request");

        assert_eq!(response["method"], Value::String("GET".into()));
        assert_eq!(response["body"]["ok"], Value::Bool(true));

        clear_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn project_component_list_returns_components() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mock_server = MockServer::start().await;
        set_test_env(&temp_dir, Some(&mock_server.uri()));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/project/PROJ/components"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": "1001",
                    "name": "Platform",
                    "description": "Core services"
                }
            ])))
            .mount(&mock_server)
            .await;

        let result = JiraApp
            .project_component_list(ProjectKeyArgs {
                project_key: "PROJ".into(),
            })
            .await
            .expect("component list");

        assert_eq!(result["project_key"], Value::String("PROJ".into()));
        assert_eq!(result["components"].as_array().map(Vec::len), Some(1));

        clear_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn sprint_list_uses_requested_states() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mock_server = MockServer::start().await;
        set_test_env(&temp_dir, Some(&mock_server.uri()));

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board"))
            .and(query_param("projectKeyOrId", "PROJ"))
            .and(query_param("maxResults", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{ "id": 77 }]
            })))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/rest/agile/1.0/board/77/sprint"))
            .and(query_param("state", "active,closed"))
            .and(query_param("maxResults", "200"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "values": [{
                    "id": 55,
                    "name": "Sprint 55",
                    "state": "active",
                    "goal": "Ship metadata tools"
                }]
            })))
            .mount(&mock_server)
            .await;

        let result = JiraApp
            .sprint_list(SprintListArgs {
                project_key: "PROJ".into(),
                states: Some(vec!["active".into(), "closed".into()]),
            })
            .await
            .expect("sprint list");

        assert_eq!(result["project_key"], Value::String("PROJ".into()));
        assert_eq!(result["sprints"].as_array().map(Vec::len), Some(1));
        assert_eq!(result["sprints"][0]["id"], Value::Number(55u64.into()));

        clear_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn sprint_update_requires_at_least_one_field() {
        let err = JiraApp
            .sprint_update(SprintUpdateArgs {
                sprint_id: 42,
                name: None,
                state: None,
                start_date: None,
                end_date: None,
                goal: None,
            })
            .await
            .expect_err("missing sprint changes should fail");

        assert_eq!(err.to_mcp().message, "validation_error");
    }

    #[tokio::test]
    #[serial]
    async fn project_version_update_requires_changes() {
        let err = JiraApp
            .project_version_update(ProjectVersionUpdateArgs {
                version_id: "1000".into(),
                name: None,
                description: None,
                archived: None,
                released: None,
                release_date: None,
                start_date: None,
            })
            .await
            .expect_err("missing version changes should fail");

        assert_eq!(err.to_mcp().message, "validation_error");
    }

    #[test]
    fn build_api_path_rejects_duplicate_query_sources() {
        let err = build_api_path(
            "/rest/api/3/project?expand=lead".into(),
            Some(
                [("startAt".to_string(), Value::Number(1.into()))]
                    .into_iter()
                    .collect(),
            ),
        )
        .expect_err("duplicate query should fail");

        assert_eq!(err.to_mcp().message, "validation_error");
    }

    #[tokio::test]
    #[serial]
    async fn issue_link_types_list_returns_values() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mock_server = MockServer::start().await;
        set_test_env(&temp_dir, Some(&mock_server.uri()));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issueLinkType"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issueLinkTypes": [{
                    "id": "10000",
                    "name": "Blocks",
                    "inward": "is blocked by",
                    "outward": "blocks"
                }]
            })))
            .mount(&mock_server)
            .await;

        let result = JiraApp
            .issue_link_types_list()
            .await
            .expect("link type list");

        assert_eq!(result["link_types"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            result["link_types"][0]["name"],
            Value::String("Blocks".into())
        );

        clear_test_env();
    }

    #[tokio::test]
    #[serial]
    async fn remote_link_list_returns_links() {
        let temp_dir = TempDir::new().expect("tempdir");
        let mock_server = MockServer::start().await;
        set_test_env(&temp_dir, Some(&mock_server.uri()));

        Mock::given(method("GET"))
            .and(path("/rest/api/3/issue/PROJ-1/remotelink"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
                "id": 321,
                "relationship": "references",
                "object": {
                    "title": "Spec",
                    "url": "https://example.com/spec"
                }
            }])))
            .mount(&mock_server)
            .await;

        let result = JiraApp
            .remote_link_list(IssueKeyArgs {
                key: "PROJ-1".into(),
            })
            .await
            .expect("remote links");

        assert_eq!(result["key"], Value::String("PROJ-1".into()));
        assert_eq!(result["remote_links"].as_array().map(Vec::len), Some(1));
        assert_eq!(
            result["remote_links"][0]["object"]["title"],
            Value::String("Spec".into())
        );

        clear_test_env();
    }
}
