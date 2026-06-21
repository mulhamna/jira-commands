use std::{collections::HashMap, path::PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use serde_json::{json, Value};

use jira_core::model::{field::Field, CreateIssueRequestV2, Issue, UpdateIssueRequest};

use crate::{
    error::{AppError, AppResult},
    models::{
        ArchiveArgs, AttachmentInput, BatchArgs, BatchOperation, BulkCommentArgs,
        BulkTransitionArgs, BulkUpdateArgs, IssueAttachArgs, IssueCloneArgs, IssueCreateArgs,
        IssueDeleteArgs, IssueFieldsArgs, IssueKeyArgs, IssueListArgs, IssueMoveArgs,
        IssueNotificationsArgs, IssueStandupArgs, IssueTransitionArgs, IssueTypesListArgs,
        IssueUpdateArgs, SprintSummaryArgs,
    },
};

use super::{
    jql::{escape_jql_literal, parse_relative_window},
    notify::{resolve_transition, scan_issue_notifications},
    request::{
        build_update_issue_request_from_batch, build_update_issue_request_from_issue,
        create_issue_request_from_batch_create, create_issue_request_from_issue, issue_is_blocked,
        issue_status_category, issue_updated_at,
    },
    shared::{normalize_issue_keys, require_confirm, slim_issue, to_value},
    JiraApp,
};

impl JiraApp {
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
        let issues: Vec<Value> = result.issues.iter().map(slim_issue).collect();
        Ok(json!({
            "jql": jql,
            "issues": issues,
            "next_page_token": result.next_page_token,
            "total": result.total
        }))
    }

    pub async fn issue_standup(&self, args: IssueStandupArgs) -> AppResult<Value> {
        let limit = args.limit.unwrap_or(50);
        if !(1..=100).contains(&limit) {
            return Err(AppError::validation("limit must be between 1 and 100"));
        }

        let since = args.since.unwrap_or_else(|| "2d".to_string());
        let cutoff = Utc::now() - parse_relative_window(&since)?;
        let client = self.build_client()?;
        let query = if let Some(jql) = args.jql {
            jql
        } else if let Some(project_key) = args.project_key {
            format!("project = {project_key} AND assignee = currentUser() ORDER BY updated DESC")
        } else {
            "assignee = currentUser() ORDER BY updated DESC".to_string()
        };

        let issues = client
            .search_issues(&query, None, Some(limit))
            .await?
            .issues;

        let mut done = vec![];
        let mut in_progress = vec![];
        let mut next_up = vec![];
        let mut blocked = vec![];
        let mut other = vec![];

        for issue in issues {
            let category = issue_status_category(&issue);
            let is_done_recent = category == "done"
                && issue_updated_at(&issue)
                    .map(|updated| updated >= cutoff)
                    .unwrap_or(false);

            if issue_is_blocked(&issue) {
                blocked.push(issue);
            } else if is_done_recent {
                done.push(issue);
            } else if category == "indeterminate" {
                in_progress.push(issue);
            } else if category == "new" {
                next_up.push(issue);
            } else {
                other.push(issue);
            }
        }

        let slim = |issues: &[Issue]| issues.iter().map(slim_issue).collect::<Vec<_>>();
        Ok(json!({
            "query": query,
            "since": since,
            "recently_done": slim(&done),
            "in_progress": slim(&in_progress),
            "next_up": slim(&next_up),
            "blocked": slim(&blocked),
            "other": slim(&other),
        }))
    }

    pub async fn issue_sprint_summary(&self, args: SprintSummaryArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let limit = args.limit.unwrap_or(100);
        if !(1..=100).contains(&limit) {
            return Err(AppError::validation("limit must be between 1 and 100"));
        }

        let sprint_label = args
            .sprint
            .clone()
            .unwrap_or_else(|| "openSprints()".to_string());
        let sprint_clause = match args.sprint.as_deref() {
            Some(value) if value.trim().parse::<u64>().is_ok() => {
                format!("sprint = {}", value.trim())
            }
            Some(value) => format!("sprint = \"{}\"", escape_jql_literal(value.trim())),
            None => "sprint in openSprints()".to_string(),
        };
        let query = format!(
            "project = {} AND {} ORDER BY status ASC, updated DESC",
            args.project_key, sprint_clause
        );

        let issues = client
            .search_issues(&query, None, Some(limit))
            .await?
            .issues;
        let mut by_status: HashMap<String, Vec<Issue>> = HashMap::new();
        let mut by_assignee: HashMap<String, usize> = HashMap::new();
        let mut done_count = 0usize;
        let mut in_progress_count = 0usize;
        let mut todo_count = 0usize;
        let mut blocked_count = 0usize;

        for issue in issues {
            let category = issue_status_category(&issue);
            if issue_is_blocked(&issue) {
                blocked_count += 1;
            }
            match category.as_str() {
                "done" => done_count += 1,
                "indeterminate" => in_progress_count += 1,
                "new" => todo_count += 1,
                _ => {}
            }
            let assignee = issue
                .assignee
                .clone()
                .unwrap_or_else(|| "Unassigned".to_string());
            *by_assignee.entry(assignee).or_insert(0) += 1;
            by_status
                .entry(issue.status.clone())
                .or_default()
                .push(issue);
        }

        let total: usize = by_status.values().map(Vec::len).sum();
        let by_status: HashMap<String, Vec<Value>> = by_status
            .into_iter()
            .map(|(status, issues)| (status, issues.iter().map(slim_issue).collect()))
            .collect();

        Ok(json!({
            "project": args.project_key,
            "sprint": sprint_label,
            "query": query,
            "total": total,
            "done": done_count,
            "in_progress": in_progress_count,
            "todo": todo_count,
            "blocked": blocked_count,
            "by_assignee": by_assignee,
            "by_status": by_status,
        }))
    }

    pub async fn issue_notifications(&self, args: IssueNotificationsArgs) -> AppResult<Value> {
        let limit = args.limit.unwrap_or(50);
        if !(1..=100).contains(&limit) {
            return Err(AppError::validation("limit must be between 1 and 100"));
        }

        let since = args.since.unwrap_or_else(|| "7d".to_string());
        let client = self.build_client()?;
        let scan =
            scan_issue_notifications(&client, args.project_key.as_deref(), &since, limit).await?;
        let unread_count = scan.entries.iter().filter(|entry| !entry.read).count();

        Ok(json!({
            "project_key": args.project_key,
            "since": since,
            "jql": scan.jql,
            "scanned_issues": scan.scanned_issues,
            "comment_errors": scan.comment_errors,
            "total": scan.entries.len(),
            "unread": unread_count,
            "notifications": scan.entries,
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
            .create_issue_v2(create_issue_request_from_issue(args))
            .await?;

        Ok(slim_issue(&issue))
    }

    pub async fn issue_update(&self, args: IssueUpdateArgs) -> AppResult<Value> {
        let client = self.build_client()?;
        let key = args.key.clone();
        let request = build_update_issue_request_from_issue(args)?;

        client.update_issue(&key, request).await?;
        let issue = client.get_issue(&key).await?;
        Ok(slim_issue(&issue))
    }

    pub async fn issue_bulk_comment(&self, args: BulkCommentArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        if args.body.trim().is_empty() {
            return Err(AppError::validation("Comment body cannot be empty"));
        }
        if args.jql.is_none() && args.keys.as_ref().is_none_or(|keys| keys.is_empty()) {
            return Err(AppError::validation(
                "Provide jql or at least one issue key to target",
            ));
        }

        let client = self.build_client()?;
        let target_keys = if let Some(jql) = args.jql.as_deref() {
            client
                .get_all_issues(jql)
                .await?
                .into_iter()
                .map(|issue| issue.key)
                .collect::<Vec<_>>()
        } else {
            normalize_issue_keys(args.keys.unwrap_or_default())
        };

        if target_keys.is_empty() {
            return Err(AppError::validation(
                "Provide jql or at least one issue key to target",
            ));
        }

        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
        for key in &target_keys {
            match client.add_comment(key, &args.body).await {
                Ok(comment) => succeeded.push(json!({"key": key, "comment_id": comment.id})),
                Err(err) => failed.push(json!({"key": key, "error": err.to_string()})),
            }
        }

        Ok(json!({
            "targets": target_keys,
            "total": succeeded.len() + failed.len(),
            "succeeded": succeeded,
            "failed": failed,
        }))
    }

    pub async fn issue_batch(&self, args: BatchArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;

        if args.operations.is_empty() {
            return Ok(json!({
                "total": 0,
                "succeeded": 0,
                "failed_count": 0,
                "results": []
            }));
        }

        let client = self.build_client()?;

        let total = args.operations.len();
        let mut results = Vec::new();
        let mut succeeded = 0usize;

        for operation in args.operations {
            let result = match operation {
                BatchOperation::Create(entry) => {
                    let summary = entry.summary.clone();
                    match client
                        .create_issue_v2(create_issue_request_from_batch_create(entry))
                        .await
                    {
                        Ok(issue) => {
                            succeeded += 1;
                            json!({
                                "op": "create",
                                "key": issue.key,
                                "status": "created",
                                "issue": slim_issue(&issue)
                            })
                        }
                        Err(err) => json!({
                            "op": "create",
                            "summary": summary,
                            "status": "failed",
                            "error": err.to_string()
                        }),
                    }
                }
                BatchOperation::Update(entry) => {
                    let key = entry.key.clone();
                    match build_update_issue_request_from_batch(entry) {
                        Ok(request) => match client.update_issue(&key, request).await {
                            Ok(_) => {
                                succeeded += 1;
                                json!({
                                    "op": "update",
                                    "key": key,
                                    "status": "updated"
                                })
                            }
                            Err(err) => json!({
                                "op": "update",
                                "key": key,
                                "status": "failed",
                                "error": err.to_string()
                            }),
                        },
                        Err(err) => json!({
                            "op": "update",
                            "key": key,
                            "status": "failed",
                            "error": err.to_string()
                        }),
                    }
                }
                BatchOperation::Transition(entry) => {
                    let key = entry.key.clone();
                    let transition = entry.transition.clone();
                    match resolve_transition(&client, &key, &transition).await {
                        Ok(resolved) => match client.transition_issue(&key, &resolved.id).await {
                            Ok(_) => {
                                succeeded += 1;
                                json!({
                                    "op": "transition",
                                    "key": key,
                                    "status": "transitioned",
                                    "transition": {
                                        "id": resolved.id,
                                        "name": resolved.name
                                    }
                                })
                            }
                            Err(err) => json!({
                                "op": "transition",
                                "key": key,
                                "status": "failed",
                                "error": err.to_string()
                            }),
                        },
                        Err(err) => json!({
                            "op": "transition",
                            "key": key,
                            "status": "failed",
                            "error": err.to_string()
                        }),
                    }
                }
                BatchOperation::Archive(entry) => {
                    let key = entry.key;
                    match client.archive_issues(std::slice::from_ref(&key)).await {
                        Ok(_) => {
                            succeeded += 1;
                            json!({
                                "op": "archive",
                                "key": key,
                                "status": "archived"
                            })
                        }
                        Err(err) => json!({
                            "op": "archive",
                            "key": key,
                            "status": "failed",
                            "error": err.to_string()
                        }),
                    }
                }
            };
            results.push(result);
        }

        Ok(json!({
            "total": total,
            "succeeded": succeeded,
            "failed_count": total - succeeded,
            "results": results
        }))
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

    pub async fn issue_move(&self, args: IssueMoveArgs) -> AppResult<Value> {
        require_confirm(args.confirm)?;
        let client = self.build_client()?;
        let issue = client
            .move_issue(
                &args.key,
                &args.project_key,
                &args.issue_type_id,
                args.parent.as_deref(),
            )
            .await?;
        Ok(slim_issue(&issue))
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
}
