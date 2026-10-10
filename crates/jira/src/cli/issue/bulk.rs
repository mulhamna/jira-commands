use super::manage::truncate;
use super::*;

pub(super) async fn handle_command(client: JiraClient, command: IssueCommand) -> Result<()> {
    match command {
        IssueCommand::Link { command } => handle_link_command(client, command).await,
        IssueCommand::BulkTransition {
            jql,
            to,
            force,
            json,
        } => bulk_transition(client, jql, to, force, json).await,
        IssueCommand::BulkUpdate {
            jql,
            assignee,
            priority,
            force,
            json,
        } => bulk_update(client, jql, assignee, priority, force, json).await,
        IssueCommand::Archive { jql, force } => archive(client, jql, force).await,
        IssueCommand::Jql { run, params } => jql_builder(client, run, params).await,
        IssueCommand::BulkCreate { manifest, json } => bulk_create(client, manifest, json).await,
        IssueCommand::Clone {
            key,
            project,
            summary,
            assignee,
            r#move,
            json,
        } => clone_issue(client, key, project, summary, assignee, r#move, json).await,
        IssueCommand::ChangeType {
            key,
            issue_type,
            json,
        } => change_issue_type(client, key, issue_type, json).await,
        IssueCommand::Move {
            key,
            project,
            issue_type,
            json,
        } => move_issue_native(client, key, project, issue_type, json).await,
        IssueCommand::Batch { manifest, json } => batch_manifest(client, manifest, json).await,
        _ => anyhow::bail!("Unsupported bulk issue command"),
    }
}
// ─── bulk transition ──────────────────────────────────────────────────────────

pub(super) async fn bulk_transition(
    client: JiraClient,
    jql: String,
    to: String,
    force: bool,
    json: bool,
) -> Result<()> {
    let spinner = spinner_new("Fetching issues...");
    let issues = client
        .get_all_issues(&jql)
        .await
        .context("Failed to fetch issues")?;
    spinner.finish_and_clear();

    if issues.is_empty() {
        println!("No issues found matching JQL.");
        return Ok(());
    }

    println!("Found {} issues.", issues.len());

    if !force {
        require_interactive("confirmation", "--force")?;
        let confirm = inquire::Confirm::new(&format!(
            "Transition all {} issues to '{to}'?",
            issues.len()
        ))
        .with_default(false)
        .prompt()
        .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Fetch available transitions from the first issue
    let transitions = client
        .get_transitions(&issues[0].key)
        .await
        .context("Failed to fetch transitions")?;

    let transition_id = transitions
        .iter()
        .find(|t| t.id == to || t.name.eq_ignore_ascii_case(&to))
        .map(|t| t.id.clone())
        .ok_or_else(|| anyhow::anyhow!("Transition '{}' not found", to))?;

    let pb = progress_bar(issues.len() as u64);

    let mut ok = 0u64;
    let mut failed: Vec<String> = Vec::new();

    for issue in &issues {
        pb.set_message(issue.key.clone());
        match client.transition_issue(&issue.key, &transition_id).await {
            Ok(_) => ok += 1,
            Err(e) => failed.push(format!("{}: {}", issue.key, e)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total": issues.len(),
                "succeeded": ok,
                "failed_count": failed.len(),
                "failed": failed,
            }))?
        );
    } else {
        println!("✓ Transitioned {ok}/{} issues to '{to}'", issues.len());
        if !failed.is_empty() {
            println!("✗ Failed ({}):", failed.len());
            for f in &failed {
                println!("  {f}");
            }
        }
    }

    Ok(())
}

// ─── bulk update ─────────────────────────────────────────────────────────────

pub(super) async fn bulk_update(
    client: JiraClient,
    jql: String,
    assignee: Option<String>,
    priority: Option<String>,
    force: bool,
    json: bool,
) -> Result<()> {
    if assignee.is_none() && priority.is_none() {
        anyhow::bail!("Nothing to update. Use --assignee or --priority.");
    }

    let spinner = spinner_new("Fetching issues...");
    let issues = client
        .get_all_issues(&jql)
        .await
        .context("Failed to fetch issues")?;
    spinner.finish_and_clear();

    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    println!("Found {} issues.", issues.len());

    if !force {
        require_interactive("confirmation", "--force")?;
        let confirm = inquire::Confirm::new(&format!("Update {} issues?", issues.len()))
            .with_default(false)
            .prompt()
            .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let req = UpdateIssueRequest {
        assignee: assignee.clone(),
        priority: priority.clone(),
        ..Default::default()
    };

    let pb = progress_bar(issues.len() as u64);

    let mut ok = 0u64;
    let mut failed: Vec<String> = Vec::new();

    for issue in &issues {
        pb.set_message(issue.key.clone());
        match client.update_issue(&issue.key, req.clone()).await {
            Ok(_) => ok += 1,
            Err(e) => failed.push(format!("{}: {}", issue.key, e)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "total": issues.len(),
                "succeeded": ok,
                "failed_count": failed.len(),
                "failed": failed,
            }))?
        );
    } else {
        println!("✓ Updated {ok}/{} issues", issues.len());
        if !failed.is_empty() {
            println!("✗ Failed ({}):", failed.len());
            for f in &failed {
                println!("  {f}");
            }
        }
    }

    Ok(())
}

// ─── archive ─────────────────────────────────────────────────────────────────

pub(super) async fn archive(client: JiraClient, jql: String, force: bool) -> Result<()> {
    let spinner = spinner_new("Fetching issues...");
    let issues = client
        .get_all_issues(&jql)
        .await
        .context("Failed to fetch issues")?;
    spinner.finish_and_clear();

    if issues.is_empty() {
        println!("No issues found.");
        return Ok(());
    }

    println!("Found {} issues.", issues.len());

    if !force {
        require_interactive("confirmation", "--force")?;
        let confirm = inquire::Confirm::new(&format!(
            "Archive {} issues? This cannot be undone.",
            issues.len()
        ))
        .with_default(false)
        .prompt()
        .context("Failed to read confirmation")?;
        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let keys: Vec<String> = issues.iter().map(|i| i.key.clone()).collect();

    let spinner = spinner_new(format!("Archiving {} issues...", keys.len()));
    client
        .archive_issues(&keys)
        .await
        .context("Failed to archive issues")?;
    spinner.finish_and_clear();
    println!("✓ Archived {} issues", keys.len());

    Ok(())
}

// ─── jql builder ─────────────────────────────────────────────────────────────

fn jql_params_filters_empty(p: &jira_core::jql::JqlParams) -> bool {
    p.project.is_none()
        && p.status.is_empty()
        && p.assignee.is_empty()
        && p.priority.is_empty()
        && p.labels.is_empty()
        && p.components.is_empty()
        && p.fix_versions.is_empty()
        && p.text.is_none()
        && p.created_after.is_none()
        && p.updated_after.is_none()
        && p.extra_clauses.is_empty()
}

fn load_jql_params(spec: &str) -> Result<jira_core::jql::JqlParams> {
    let raw = if let Some(path) = spec.strip_prefix('@') {
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {path}"))?
    } else {
        spec.to_string()
    };
    serde_json::from_str(&raw).context("Failed to parse JqlParams JSON")
}

fn prompt_jql_params() -> Result<jira_core::jql::JqlParams> {
    use jira_core::jql::{AssigneeFilter, JqlParams, OrderDir};

    require_interactive("JQL parameters", "--params <json|@file>")?;

    println!("JQL Builder — press Enter to skip any field\n");

    let project = Text::new("Project key (e.g. PROJ):")
        .prompt_skippable()
        .context("Failed to read project")?
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());

    let status_opts = vec![
        "To Do",
        "In Progress",
        "In Review",
        "Done",
        "Blocked",
        "(any)",
    ];
    let status_sel = Select::new("Status:", status_opts)
        .prompt()
        .context("Failed to read status")?;
    let status = if status_sel == "(any)" {
        Vec::new()
    } else {
        vec![status_sel.to_string()]
    };

    let assignee_opts = vec!["Me (currentUser)", "Unassigned", "Custom email", "(any)"];
    let assignee_sel = Select::new("Assignee:", assignee_opts)
        .prompt()
        .context("Failed to read assignee")?;
    let assignee = match assignee_sel {
        "Me (currentUser)" => vec![AssigneeFilter::CurrentUser],
        "Unassigned" => vec![AssigneeFilter::Empty],
        "Custom email" => {
            let email = Text::new("Email:")
                .prompt()
                .context("Failed to read email")?;
            vec![AssigneeFilter::Email { email }]
        }
        _ => Vec::new(),
    };

    let priority_opts = vec!["Highest", "High", "Medium", "Low", "Lowest", "(any)"];
    let priority_sel = Select::new("Priority:", priority_opts)
        .prompt()
        .context("Failed to read priority")?;
    let priority = if priority_sel == "(any)" {
        Vec::new()
    } else {
        vec![priority_sel.to_string()]
    };

    let order_opts = vec!["updated DESC", "created DESC", "priority DESC", "key ASC"];
    let order_sel = Select::new("Order by:", order_opts)
        .prompt()
        .context("Failed to read order")?;
    let order_by = vec![match order_sel {
        "updated DESC" => ("updated".to_string(), OrderDir::Desc),
        "created DESC" => ("created".to_string(), OrderDir::Desc),
        "priority DESC" => ("priority".to_string(), OrderDir::Desc),
        _ => ("key".to_string(), OrderDir::Asc),
    }];

    Ok(JqlParams {
        project,
        status,
        assignee,
        priority,
        order_by,
        ..Default::default()
    })
}

pub(super) async fn jql_builder(
    client: JiraClient,
    run: bool,
    params: Option<String>,
) -> Result<()> {
    let mut jql_params = if let Some(spec) = params {
        load_jql_params(&spec)?
    } else {
        prompt_jql_params()?
    };

    if jql_params_filters_empty(&jql_params) {
        jql_params
            .assignee
            .push(jira_core::jql::AssigneeFilter::CurrentUser);
    }
    if jql_params.order_by.is_empty() {
        jql_params
            .order_by
            .push(("updated".into(), jira_core::jql::OrderDir::Desc));
    }

    let jql = jira_core::jql::compose_jql(&jql_params).context("Failed to compose JQL")?;
    println!("\nGenerated JQL:\n  {jql}\n");

    if run {
        let spinner = spinner_new("Searching...");
        let result = client
            .search_issues(&jql, None, Some(25))
            .await
            .context("Search failed")?;
        spinner.finish_and_clear();

        if result.issues.is_empty() {
            println!("No issues found.");
            return Ok(());
        }

        println!("{:<12} {:<8} {:<20} SUMMARY", "KEY", "TYPE", "STATUS");
        println!("{}", "─".repeat(82));
        for issue in &result.issues {
            let summary = if issue.summary.len() > 38 {
                format!("{}…", &issue.summary[..37])
            } else {
                issue.summary.clone()
            };
            println!(
                "{:<12} {:<8} {:<20} {}",
                issue.key,
                truncate(&issue.issue_type, 7),
                truncate(&issue.status, 19),
                summary
            );
        }
        if let Some(total) = result.total {
            println!("\nShowing {} of {total}", result.issues.len());
        }
    }

    Ok(())
}

// ─── batch manifest runner ───────────────────────────────────────────────────

pub(super) async fn batch_manifest(
    client: JiraClient,
    manifest: std::path::PathBuf,
    json: bool,
) -> Result<()> {
    let content = std::fs::read_to_string(&manifest)
        .with_context(|| format!("Failed to read manifest: {}", manifest.display()))?;

    let entries: Vec<Value> =
        serde_json::from_str(&content).context("Manifest must be a JSON array of op objects")?;

    if entries.is_empty() {
        println!("Manifest is empty — nothing to run.");
        return Ok(());
    }

    println!("Running {} operations...", entries.len());
    let pb = progress_bar(entries.len() as u64);

    // Each result: {"op":..., "key":..., "status":..., "error": null|"..."}
    let mut results: Vec<Value> = Vec::new();

    for entry in &entries {
        let op = entry
            .get("op")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        pb.set_message(op.to_string());

        let result = match op {
            "create" => {
                let project = entry
                    .get("project")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let summary = entry
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let issue_type = entry
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Task")
                    .to_string();
                let assignee = entry
                    .get("assignee")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let priority = entry
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let labels: Vec<String> = entry
                    .get("labels")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let components: Vec<String> = entry
                    .get("components")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let fix_versions: Vec<String> = entry
                    .get("fix_versions")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                let parent = entry
                    .get("parent")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let description = entry
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let custom_fields: HashMap<String, FieldValue> = entry
                    .get("fields")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| (k.clone(), FieldValue::Raw(v.clone())))
                            .collect()
                    })
                    .unwrap_or_default();

                let req = CreateIssueRequestV2 {
                    project_key: project,
                    summary,
                    description,
                    description_adf: None,
                    issue_type,
                    assignee,
                    priority,
                    labels,
                    components,
                    fix_versions,
                    parent,
                    custom_fields,
                };
                match client.create_issue_v2(req).await {
                    Ok(issue) => {
                        serde_json::json!({ "op": op, "key": issue.key, "status": "created" })
                    }
                    Err(e) => {
                        serde_json::json!({ "op": op, "key": "", "status": "failed", "error": e.to_string() })
                    }
                }
            }
            "update" => {
                let key = entry
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let req = UpdateIssueRequest {
                    summary: entry
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    assignee: entry
                        .get("assignee")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    priority: entry
                        .get("priority")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    labels: entry.get("labels").and_then(|v| v.as_array()).map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    }),
                    components: entry.get("components").and_then(|v| v.as_array()).map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str())
                            .map(String::from)
                            .collect()
                    }),
                    ..Default::default()
                };
                match client.update_issue(&key, req).await {
                    Ok(_) => serde_json::json!({ "op": op, "key": key, "status": "updated" }),
                    Err(e) => {
                        serde_json::json!({ "op": op, "key": key, "status": "failed", "error": e.to_string() })
                    }
                }
            }
            "transition" => {
                let key = entry
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let to = entry
                    .get("to")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let trans_result: anyhow::Result<()> = async {
                    let transitions = client
                        .get_transitions(&key)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                    let tid = transitions
                        .iter()
                        .find(|t| t.id == to || t.name.eq_ignore_ascii_case(&to))
                        .map(|t| t.id.clone())
                        .ok_or_else(|| anyhow::anyhow!("Transition '{}' not found", to))?;
                    client
                        .transition_issue(&key, &tid)
                        .await
                        .map_err(|e| anyhow::anyhow!(e))
                }
                .await;

                match trans_result {
                    Ok(_) => {
                        serde_json::json!({ "op": op, "key": key, "status": format!("transitioned to '{to}'") })
                    }
                    Err(e) => {
                        serde_json::json!({ "op": op, "key": key, "status": "failed", "error": e.to_string() })
                    }
                }
            }
            "archive" => {
                let key = entry
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                match client.archive_issues(std::slice::from_ref(&key)).await {
                    Ok(_) => serde_json::json!({ "op": op, "key": key, "status": "archived" }),
                    Err(e) => {
                        serde_json::json!({ "op": op, "key": key, "status": "failed", "error": e.to_string() })
                    }
                }
            }
            _ => {
                serde_json::json!({ "op": op, "key": "", "status": "skipped", "error": format!("Unknown op: '{op}'") })
            }
        };

        results.push(result);
        pb.inc(1);
    }

    pb.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        let succeeded = results
            .iter()
            .filter(|r| r.get("error").map(|e| e.is_null()).unwrap_or(true))
            .count();
        println!("✓ {succeeded}/{} operations completed", results.len());
        for r in &results {
            let op_str = r.get("op").and_then(|v| v.as_str()).unwrap_or("?");
            let key_str = r.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let status_str = r.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            let key_display = if key_str.is_empty() {
                String::new()
            } else {
                format!(" {key_str}")
            };
            if let Some(err) = r.get("error").and_then(|v| v.as_str()) {
                println!("  ✗ {op_str}{key_display}: {err}");
            } else {
                println!("  ✓ {op_str}{key_display}: {status_str}");
            }
        }
    }

    Ok(())
}

// ─── native move / type change ───────────────────────────────────────────────

pub(super) async fn change_issue_type(
    client: JiraClient,
    key: String,
    issue_type: String,
    json: bool,
) -> Result<()> {
    let spinner = spinner_new(format!("Fetching {key}..."));
    let source = client
        .get_issue(&key)
        .await
        .context("Failed to fetch source issue")?;
    spinner.finish_and_clear();

    let target_issue_type = client
        .get_issue_type_by_name(&source.project_key, &issue_type)
        .await
        .with_context(|| {
            format!(
                "Failed to resolve issue type '{}' in project {}",
                issue_type, source.project_key
            )
        })?;

    let spinner = spinner_new(format!("Changing issue type for {key}..."));
    let moved = client
        .move_issue(&key, &source.project_key, &target_issue_type.id, None)
        .await
        .context("Failed to change issue type")?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&moved)?);
    } else {
        println!(
            "✓ Changed issue type: {} → {} ({})",
            key, moved.key, moved.issue_type
        );
    }

    Ok(())
}

pub(super) async fn move_issue_native(
    client: JiraClient,
    key: String,
    project: String,
    issue_type: Option<String>,
    json: bool,
) -> Result<()> {
    let spinner = spinner_new(format!("Fetching {key}..."));
    let source = client
        .get_issue(&key)
        .await
        .context("Failed to fetch source issue")?;
    spinner.finish_and_clear();

    let target_issue_type_name = issue_type.unwrap_or_else(|| source.issue_type.clone());
    let target_issue_type = client
        .get_issue_type_by_name(&project, &target_issue_type_name)
        .await
        .with_context(|| {
            format!(
                "Failed to resolve issue type '{}' in project {}",
                target_issue_type_name, project
            )
        })?;

    let spinner = spinner_new(format!("Moving {key} to {project}..."));
    let moved = client
        .move_issue(&key, &project, &target_issue_type.id, None)
        .await
        .context("Failed to move issue")?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&moved)?);
    } else {
        println!(
            "✓ Moved natively: {} → {} ({})",
            key, moved.key, moved.project_key
        );
    }

    Ok(())
}

// ─── clone / move ────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) async fn clone_issue(
    client: JiraClient,
    key: String,
    project: Option<String>,
    summary_override: Option<String>,
    assignee: Option<String>,
    move_issue: bool,
    json: bool,
) -> Result<()> {
    // Fetch source issue
    let spinner = spinner_new(format!("Fetching {key}..."));
    let source = client
        .get_issue(&key)
        .await
        .context("Failed to fetch source issue")?;
    spinner.finish_and_clear();

    let target_project = project.unwrap_or_else(|| source.project_key.clone());
    let summary = summary_override.unwrap_or_else(|| source.summary.clone());

    // Resolve labels and components from raw fields
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

    let req = CreateIssueRequestV2 {
        project_key: target_project,
        summary,
        description: None,
        description_adf: source.description.clone(),
        issue_type: source.issue_type.clone(),
        assignee,
        priority: source.priority.clone(),
        labels,
        components,
        fix_versions,
        parent: None,
        custom_fields: HashMap::new(),
    };

    let spinner = spinner_new("Cloning issue...");
    let clone = client
        .create_issue_v2(req)
        .await
        .context("Failed to clone issue")?;
    spinner.finish_and_clear();

    if move_issue {
        // Confirm before deleting original
        require_interactive(
            "deletion of the original after clone",
            "`jirac issue move` (native move) or run interactively",
        )?;
        let confirm = inquire::Confirm::new(&format!(
            "Delete original {key} after cloning to {}?",
            clone.key
        ))
        .with_default(false)
        .prompt()
        .context("Failed to read confirmation")?;

        if confirm {
            let spinner = spinner_new(format!("Deleting {key}..."));
            client
                .delete_issue(&key)
                .await
                .context("Failed to delete original issue")?;
            spinner.finish_and_clear();
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&clone)?);
    } else if move_issue {
        println!("✓ Moved: {} → {}", key, clone.key);
    } else {
        println!("✓ Cloned: {} → {} — {}", key, clone.key, clone.summary);
    }

    Ok(())
}

// ─── bulk create ─────────────────────────────────────────────────────────────

pub(super) async fn bulk_create(
    client: JiraClient,
    manifest: std::path::PathBuf,
    json: bool,
) -> Result<()> {
    let content = std::fs::read_to_string(&manifest)
        .with_context(|| format!("Failed to read manifest: {}", manifest.display()))?;

    let entries: Vec<Value> =
        serde_json::from_str(&content).context("Manifest must be a JSON array of issue objects")?;

    if entries.is_empty() {
        println!("Manifest is empty — nothing to create.");
        return Ok(());
    }

    println!("Creating {} issues from manifest...", entries.len());
    let pb = progress_bar(entries.len() as u64);

    let mut created_issues: Vec<jira_core::model::Issue> = Vec::new();
    let mut created: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();

    for entry in &entries {
        let project_key = entry
            .get("project")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Each manifest entry must have a \"project\" field"))?
            .to_string();

        let summary = entry
            .get("summary")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Each manifest entry must have a \"summary\" field"))?
            .to_string();

        let issue_type = entry
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Task")
            .to_string();

        let assignee = entry
            .get("assignee")
            .and_then(|v| v.as_str())
            .map(String::from);
        let priority = entry
            .get("priority")
            .and_then(|v| v.as_str())
            .map(String::from);
        let parent = entry
            .get("parent")
            .and_then(|v| v.as_str())
            .map(String::from);

        let description = entry
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let labels: Vec<String> = entry
            .get("labels")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let components: Vec<String> = entry
            .get("components")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let fix_versions: Vec<String> = entry
            .get("fix_versions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| s.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        // Custom fields from "fields" object
        let custom_fields: HashMap<String, FieldValue> = entry
            .get("fields")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), FieldValue::Raw(v.clone())))
                    .collect()
            })
            .unwrap_or_default();

        pb.set_message(summary.clone());

        let req = CreateIssueRequestV2 {
            project_key,
            summary: summary.clone(),
            description,
            description_adf: None,
            issue_type,
            assignee,
            priority,
            labels,
            components,
            parent,
            fix_versions,
            custom_fields,
        };

        match client.create_issue_v2(req).await {
            Ok(issue) => {
                created.push(format!("{} — {}", issue.key, issue.summary));
                created_issues.push(issue);
            }
            Err(e) => failed.push(format!("\"{}\" failed: {}", summary, e)),
        }
        pb.inc(1);
    }

    pb.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&created_issues)?);
    } else {
        println!("✓ Created {}/{} issues:", created.len(), entries.len());
        for c in &created {
            println!("  {c}");
        }
        if !failed.is_empty() {
            println!("✗ Failed ({}):", failed.len());
            for f in &failed {
                println!("  {f}");
            }
        }
    }
    Ok(())
}

pub(super) async fn handle_link_command(client: JiraClient, cmd: LinkCommand) -> Result<()> {
    match cmd {
        LinkCommand::ListTypes { json } => {
            let types = client.list_issue_link_types().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&types)?);
            } else {
                println!(
                    "{:<10} {:<15} {:<20} {:<20}",
                    "ID", "Name", "Inward", "Outward"
                );
                println!("{}", "-".repeat(65));
                for t in types {
                    println!(
                        "{:<10} {:<15} {:<20} {:<20}",
                        t.id, t.name, t.inward, t.outward
                    );
                }
            }
        }
        LinkCommand::Add {
            outward,
            inward,
            link_type,
            comment,
        } => {
            client
                .link_issues(&outward, &inward, &link_type, comment.as_deref())
                .await?;
            println!("✓ Linked {outward} to {inward} as '{link_type}'");
        }
        LinkCommand::Delete { id, force } => {
            if !force {
                require_interactive("confirmation", "--force")?;
                let confirmed = inquire::Confirm::new(&format!("Delete issue link {id}?"))
                    .with_default(false)
                    .prompt()?;
                if !confirmed {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            client.delete_issue_link(&id).await?;
            println!("✓ Deleted issue link {id}");
        }
    }
    Ok(())
}
