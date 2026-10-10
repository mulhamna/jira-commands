use super::*;

pub(super) async fn handle_command(
    client: JiraClient,
    command: IssueCommand,
    default_project: Option<String>,
) -> Result<()> {
    match command {
        IssueCommand::Create {
            project,
            summary,
            issue_type,
            assignee,
            priority,
            description_file,
            description_format,
            labels,
            components,
            parent,
            fix_version,
            sprint,
            attachments,
            field,
            no_custom_fields,
            json,
        } => {
            create_issue(
                client,
                project.or(default_project),
                summary,
                issue_type,
                assignee,
                priority,
                description_file,
                description_format,
                labels,
                components,
                parent,
                fix_version,
                sprint,
                attachments,
                field,
                no_custom_fields,
                json,
            )
            .await
        }
        IssueCommand::Update {
            key,
            summary,
            assignee,
            priority,
            description_file,
            description_format,
            labels,
            components,
            fix_version,
            parent,
            field,
            json,
        } => {
            update_issue(
                client,
                key,
                summary,
                assignee,
                priority,
                description_file,
                description_format,
                labels,
                components,
                fix_version,
                parent,
                field,
                json,
            )
            .await
        }
        IssueCommand::Delete { key, force } => delete_issue(client, key, force).await,
        IssueCommand::Transition {
            key,
            transition,
            json,
        } => transition_issue(client, key, transition, json).await,
        IssueCommand::Attach { key, files } => attach_files(client, key, files).await,
        IssueCommand::Attachment { command } => attachment(client, command).await,
        IssueCommand::Fields {
            project,
            issue_type,
            required_only,
            json,
        } => {
            list_fields(
                client,
                project.or(default_project),
                issue_type,
                required_only,
                json,
            )
            .await
        }
        IssueCommand::Render {
            input,
            format,
            output,
        } => render_issue_content(input, format, output),
        _ => anyhow::bail!("Unsupported issue management command"),
    }
}
// ─── create ──────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) async fn create_issue(
    client: JiraClient,
    project: Option<String>,
    summary: Option<String>,
    issue_type: Option<String>,
    assignee: Option<String>,
    priority: Option<String>,
    description_file: Option<std::path::PathBuf>,
    description_format: String,
    labels: Option<String>,
    components: Option<String>,
    parent: Option<String>,
    fix_version: Option<String>,
    sprint: Option<String>,
    attachments: Vec<std::path::PathBuf>,
    field: Vec<String>,
    no_custom_fields: bool,
    json: bool,
) -> Result<()> {
    // 1. Project key
    let project_key = match project {
        Some(p) => p,
        None => {
            require_interactive("project key", "--project")?;
            Text::new("Project key:")
                .prompt()
                .context("Failed to read project key")?
        }
    };

    // 2. Issue type — interactive picker if not supplied
    let (issue_type_name, issue_type_id) =
        resolve_issue_type(&client, &project_key, issue_type).await?;

    // 3. Summary
    let summary = match summary {
        Some(s) => s,
        None => {
            require_interactive("summary", "--summary")?;
            Text::new("Summary:")
                .prompt()
                .context("Failed to read summary")?
        }
    };

    // 4. Description from file
    let (description, description_adf) =
        read_description_file(description_file.as_deref(), &description_format)?;

    // 5. Custom fields — combine --field flags + interactive prompts
    let mut custom_fields = parse_field_flags(&field)?;
    if !no_custom_fields {
        let interactive = collect_custom_fields(&client, &project_key, &issue_type_id).await?;
        for (k, v) in interactive {
            custom_fields.entry(k).or_insert(v);
        }
    }

    if let Some(sprint) = sprint {
        let (field_id, field_value) =
            resolve_sprint_assignment(&client, &project_key, &issue_type_id, &sprint).await?;
        custom_fields.insert(field_id, field_value);
    }

    let req = CreateIssueRequestV2 {
        project_key: project_key.clone(),
        summary,
        description,
        description_adf,
        issue_type: issue_type_name,
        assignee,
        priority,
        labels: parse_csv(labels.as_deref()),
        components: parse_csv(components.as_deref()),
        parent,
        fix_versions: parse_csv(fix_version.as_deref()),
        custom_fields,
    };

    let spinner = spinner_new("Creating issue...");
    let issue = client
        .create_issue_v2(req)
        .await
        .context("Failed to create issue")?;
    spinner.finish_and_clear();

    // Attach files if provided
    let had_attachments = !attachments.is_empty();
    if had_attachments {
        attach_files(client.clone(), issue.key.clone(), attachments).await?;
    }

    if json {
        // Re-fetch to include any attachment metadata
        let full = if had_attachments {
            match client.get_issue(&issue.key).await {
                Ok(refreshed) => refreshed,
                Err(e) => {
                    eprintln!(
                        "warning: re-fetch after attach failed ({e}); attachment metadata may be missing"
                    );
                    issue
                }
            }
        } else {
            issue
        };
        println!("{}", serde_json::to_string_pretty(&full)?);
    } else {
        println!("✓ Created: {} — {}", issue.key, issue.summary);
    }

    Ok(())
}

/// Resolve issue type: use the provided name directly (skip API call) or show a picker.
async fn resolve_issue_type(
    client: &JiraClient,
    project_key: &str,
    issue_type: Option<String>,
) -> Result<(String, String)> {
    // If user gave a name, we still need the ID for field fetching — try to look it up
    let spinner = spinner_new(format!("Fetching issue types for {project_key}..."));
    let types_result = client.get_issue_types(project_key).await;
    spinner.finish_and_clear();

    match types_result {
        Ok(types) if !types.is_empty() => {
            if let Some(name) = issue_type {
                // Find matching type by name (case-insensitive)
                if let Some(t) = types
                    .iter()
                    .find(|t| t.name.to_lowercase() == name.to_lowercase())
                {
                    return Ok((t.name.clone(), t.id.clone()));
                }
                // Not found — use name as-is with empty ID (will skip custom field prompts)
                return Ok((name, String::new()));
            }

            // Interactive picker
            require_interactive("issue type", "--type")?;
            let options: Vec<String> = types.iter().map(|t| t.name.clone()).collect();
            let selected = Select::new("Issue type:", options)
                .prompt()
                .context("Failed to select issue type")?;

            let id = types
                .iter()
                .find(|t| t.name == selected)
                .map(|t| t.id.clone())
                .unwrap_or_default();

            Ok((selected, id))
        }
        _ => {
            // API call failed or returned empty — fall back gracefully
            let name = match issue_type {
                Some(n) => n,
                None => {
                    require_interactive("issue type", "--type")?;
                    Text::new("Issue type (e.g. Task, Bug, Story):")
                        .with_default("Task")
                        .prompt()
                        .context("Failed to read issue type")?
                }
            };
            Ok((name, String::new()))
        }
    }
}

/// Prompt for required custom fields that are not standard (summary/assignee/priority/type).
async fn collect_custom_fields(
    client: &JiraClient,
    project_key: &str,
    issue_type_id: &str,
) -> Result<HashMap<String, FieldValue>> {
    if issue_type_id.is_empty() {
        return Ok(HashMap::new());
    }

    let cache = FieldCache::new();
    let fields = cache.get_or_fetch(client, project_key, issue_type_id).await;

    let fields = match fields {
        Ok(f) => f,
        Err(_) => return Ok(HashMap::new()), // soft fail — don't block issue creation
    };

    // Standard fields handled by CLI flags — skip them
    const SKIP_IDS: &[&str] = &[
        "summary",
        "description",
        "issuetype",
        "project",
        "assignee",
        "reporter",
        "priority",
        "status",
        "created",
        "updated",
        "comment",
        "attachment",
        "labels",
        "fixVersions",
        "versions",
        "components",
    ];

    let custom: Vec<_> = fields
        .iter()
        .filter(|f| f.required && !SKIP_IDS.contains(&f.id.as_str()))
        .collect();

    if custom.is_empty() {
        return Ok(HashMap::new());
    }

    require_interactive("required custom fields", "--field / --no-custom-fields")?;

    println!("\nRequired custom fields:");
    println!("{}", "─".repeat(40));

    let mut result = HashMap::new();

    for field in custom {
        let kind = field.kind();
        let value = match kind {
            FieldKind::Text | FieldKind::Url => {
                let v = Text::new(&format!("{}:", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                FieldValue::Text(v)
            }
            FieldKind::Number => {
                let raw = Text::new(&format!("{} (number):", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                let n: f64 = raw
                    .trim()
                    .parse()
                    .context(format!("'{}' must be a number", field.name))?;
                FieldValue::Number(n)
            }
            FieldKind::DateTime => {
                let v = Text::new(&format!("{} (YYYY-MM-DD):", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                FieldValue::Date(v)
            }
            FieldKind::Select => {
                let options = select_options(field.allowed_values.as_deref());
                if options.is_empty() {
                    let v = Text::new(&format!("{}:", field.name))
                        .prompt()
                        .context("Failed to read field")?;
                    FieldValue::SelectName(v)
                } else {
                    let selected = Select::new(&format!("{}:", field.name), options)
                        .prompt()
                        .context("Failed to select")?;
                    FieldValue::SelectName(selected)
                }
            }
            FieldKind::MultiSelect => {
                let options = select_options(field.allowed_values.as_deref());
                if options.is_empty() {
                    let raw = Text::new(&format!("{} (comma-separated):", field.name))
                        .prompt()
                        .context("Failed to read field")?;
                    let vs: Vec<String> = raw.split(',').map(|s| s.trim().to_string()).collect();
                    FieldValue::MultiSelect(vs)
                } else {
                    let selected = MultiSelect::new(&format!("{}:", field.name), options)
                        .prompt()
                        .context("Failed to select")?;
                    FieldValue::MultiSelect(selected)
                }
            }
            FieldKind::User | FieldKind::UserArray => {
                let v = Text::new(&format!("{} (email):", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                FieldValue::UserEmail(v)
            }
            FieldKind::Labels => {
                let raw = Text::new(&format!("{} (space-separated labels):", field.name))
                    .prompt()
                    .context("Failed to read field")?;
                let ls: Vec<String> = raw.split_whitespace().map(|s| s.to_string()).collect();
                FieldValue::Labels(ls)
            }
            // Skip checkbox, cascading, and unknown in required prompts
            _ => continue,
        };

        result.insert(field.id.clone(), value);
    }

    Ok(result)
}

/// Extract display strings from `allowedValues`.
fn select_options(allowed: Option<&[serde_json::Value]>) -> Vec<String> {
    allowed
        .map(|vals: &[serde_json::Value]| {
            vals.iter()
                .filter_map(|v: &serde_json::Value| {
                    v.get("value")
                        .or_else(|| v.get("name"))
                        .and_then(|s: &serde_json::Value| s.as_str())
                        .map(|s: &str| s.to_string())
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

// ─── update ──────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(super) async fn update_issue(
    client: JiraClient,
    key: String,
    summary: Option<String>,
    assignee: Option<String>,
    priority: Option<String>,
    description_file: Option<std::path::PathBuf>,
    description_format: String,
    labels: Option<String>,
    components: Option<String>,
    fix_version: Option<String>,
    parent: Option<String>,
    field: Vec<String>,
    json: bool,
) -> Result<()> {
    let (description, description_adf) =
        read_description_file(description_file.as_deref(), &description_format)?;

    let custom_fields = parse_field_flags(&field)?;
    let labels_vec = labels.as_deref().map(|s| parse_csv(Some(s)));
    let components_vec = components.as_deref().map(|s| parse_csv(Some(s)));
    let fix_versions_vec = fix_version.as_deref().map(|s| parse_csv(Some(s)));

    let has_changes = summary.is_some()
        || assignee.is_some()
        || priority.is_some()
        || description.is_some()
        || description_adf.is_some()
        || labels_vec.is_some()
        || components_vec.is_some()
        || fix_versions_vec.is_some()
        || parent.is_some()
        || !custom_fields.is_empty();

    if !has_changes {
        println!(
            "No fields to update. Use --summary, --assignee, --priority, --description-file, --labels, --components, --fix-version, --parent, or --field."
        );
        return Ok(());
    }

    let req = UpdateIssueRequest {
        summary,
        description,
        description_adf,
        assignee,
        priority,
        labels: labels_vec,
        components: components_vec,
        fix_versions: fix_versions_vec,
        parent,
        custom_fields,
        ..Default::default()
    };

    let spinner = spinner_new(format!("Updating {key}..."));
    client
        .update_issue(&key, req)
        .await
        .context("Failed to update issue")?;
    spinner.finish_and_clear();

    if json {
        let issue = client
            .get_issue(&key)
            .await
            .context("Failed to fetch updated issue")?;
        println!("{}", serde_json::to_string_pretty(&issue)?);
    } else {
        println!("✓ Updated: {key}");
    }
    Ok(())
}

// ─── delete ──────────────────────────────────────────────────────────────────

pub(super) async fn delete_issue(client: JiraClient, key: String, force: bool) -> Result<()> {
    if !force {
        require_interactive("confirmation", "--force")?;
        let confirm = inquire::Confirm::new(&format!("Delete {key}? This cannot be undone."))
            .with_default(false)
            .prompt()
            .context("Failed to read confirmation")?;

        if !confirm {
            println!("Aborted.");
            return Ok(());
        }
    }

    let spinner = spinner_new(format!("Deleting {key}..."));
    client
        .delete_issue(&key)
        .await
        .context("Failed to delete issue")?;
    spinner.finish_and_clear();
    println!("✓ Deleted: {key}");
    Ok(())
}

// ─── transition ──────────────────────────────────────────────────────────────

pub(super) async fn transition_issue(
    client: JiraClient,
    key: String,
    transition: Option<String>,
    json: bool,
) -> Result<()> {
    let spinner = spinner_new(format!("Fetching transitions for {key}..."));
    let transitions = client
        .get_transitions(&key)
        .await
        .context("Failed to fetch transitions")?;
    spinner.finish_and_clear();

    if transitions.is_empty() {
        println!("No transitions available for {key}.");
        return Ok(());
    }

    let transition_id = if let Some(name_or_id) = transition {
        transitions
            .iter()
            .find(|t| t.id == name_or_id || t.name == name_or_id)
            .map(|t| t.id.clone())
            .ok_or_else(|| anyhow::anyhow!("Transition '{}' not found", name_or_id))?
    } else {
        require_interactive("target transition", "the transition name/id argument")?;
        let options: Vec<String> = transitions
            .iter()
            .map(|t| format!("{} [{}]", t.name, t.id))
            .collect();

        let selected = Select::new("Select transition:", options.clone())
            .prompt()
            .context("Failed to select transition")?;

        selected
            .trim_end_matches(']')
            .rsplit('[')
            .next()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to parse transition ID"))?
    };

    let spinner = spinner_new(format!("Transitioning {key}..."));
    client
        .transition_issue(&key, &transition_id)
        .await
        .context("Failed to transition issue")?;
    spinner.finish_and_clear();

    if json {
        let issue = client
            .get_issue(&key)
            .await
            .context("Failed to fetch transitioned issue")?;
        println!("{}", serde_json::to_string_pretty(&issue)?);
    } else {
        println!("✓ Transitioned: {key}");
    }
    Ok(())
}

// ─── attach ──────────────────────────────────────────────────────────────────

pub(super) async fn attach_files(
    client: JiraClient,
    key: String,
    files: Vec<std::path::PathBuf>,
) -> Result<()> {
    for path in &files {
        if !path.exists() {
            anyhow::bail!("File not found: {}", path.display());
        }
    }

    for path in &files {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let spinner = spinner_new(format!("Uploading {name}..."));
        let attachments = client
            .upload_attachment(&key, path)
            .await
            .with_context(|| format!("Failed to upload {}", path.display()))?;
        spinner.finish_and_clear();

        for a in &attachments {
            println!("✓ Attached: {} ({} bytes)", a.filename, a.size);
        }
    }

    Ok(())
}

// ─── attachment ──────────────────────────────────────────────────────────────

pub(super) async fn attachment(client: JiraClient, cmd: AttachmentCommand) -> Result<()> {
    match cmd {
        AttachmentCommand::List { key, json } => attachment_list(client, key, json).await,
        AttachmentCommand::Download {
            id,
            out,
            filename,
            force,
        } => attachment_download(client, id, out, filename, force).await,
        AttachmentCommand::Delete { id, force } => attachment_delete(client, id, force).await,
    }
}

async fn attachment_list(client: JiraClient, key: String, json: bool) -> Result<()> {
    let spinner = spinner_new(format!("Fetching attachments for {key}..."));
    let attachments = client
        .list_attachments(&key)
        .await
        .with_context(|| format!("Failed to list attachments for {key}"))?;
    spinner.finish_and_clear();

    if json {
        println!("{}", serde_json::to_string_pretty(&attachments)?);
        return Ok(());
    }

    if attachments.is_empty() {
        println!("No attachments on {key}.");
        return Ok(());
    }

    for a in &attachments {
        println!(
            "{:<10} {:<10} {:>10}  {}",
            a.id, a.mime_type, a.size, a.filename
        );
    }
    Ok(())
}

async fn attachment_download(
    client: JiraClient,
    id: String,
    out: Option<std::path::PathBuf>,
    filename: Option<String>,
    force: bool,
) -> Result<()> {
    let spinner = spinner_new(format!("Downloading attachment {id}..."));
    let (server_name, bytes, mime) = client
        .download_attachment(&id)
        .await
        .with_context(|| format!("Failed to download attachment {id}"))?;
    spinner.finish_and_clear();

    let out_dir = out.unwrap_or_else(|| std::path::PathBuf::from("."));
    if !out_dir.exists() {
        std::fs::create_dir_all(&out_dir)
            .with_context(|| format!("Failed to create {}", out_dir.display()))?;
    }
    let name = filename.unwrap_or(server_name);
    let dest = out_dir.join(&name);
    if dest.exists() && !force {
        anyhow::bail!(
            "{} already exists. Use --force to overwrite.",
            dest.display()
        );
    }
    std::fs::write(&dest, &bytes).with_context(|| format!("Failed to write {}", dest.display()))?;
    println!(
        "✓ Saved {} ({} bytes, {})",
        dest.display(),
        bytes.len(),
        mime
    );
    Ok(())
}

async fn attachment_delete(client: JiraClient, id: String, force: bool) -> Result<()> {
    if !force {
        require_interactive("confirmation", "--force")?;
        let ok = Confirm::new(&format!("Delete attachment {id}?"))
            .with_default(false)
            .prompt()
            .context("Failed to read confirmation")?;
        if !ok {
            println!("Aborted.");
            return Ok(());
        }
    }
    let spinner = spinner_new(format!("Deleting attachment {id}..."));
    client
        .delete_attachment(&id)
        .await
        .with_context(|| format!("Failed to delete attachment {id}"))?;
    spinner.finish_and_clear();
    println!("✓ Deleted attachment {id}");
    Ok(())
}

// ─── fields ──────────────────────────────────────────────────────────────────

pub(super) async fn list_fields(
    client: JiraClient,
    project: Option<String>,
    issue_type_filter: Option<String>,
    required_only: bool,
    json: bool,
) -> Result<()> {
    let project_key = match project {
        Some(p) => p,
        None => {
            require_interactive("project key", "--project")?;
            Text::new("Project key:")
                .prompt()
                .context("Failed to read project key")?
        }
    };

    // Get issue types to resolve the ID
    let spinner = spinner_new(format!("Fetching issue types for {project_key}..."));
    let types = client
        .get_issue_types(&project_key)
        .await
        .context("Failed to fetch issue types")?;
    spinner.finish_and_clear();

    let issue_type: IssueType = if let Some(filter) = issue_type_filter {
        types
            .into_iter()
            .find(|t| t.name.to_lowercase() == filter.to_lowercase())
            .ok_or_else(|| {
                anyhow::anyhow!("Issue type '{}' not found in {}", filter, project_key)
            })?
    } else {
        require_interactive("issue type", "--type")?;
        let options: Vec<String> = types.iter().map(|t| t.name.clone()).collect();
        let selected = Select::new("Issue type:", options)
            .prompt()
            .context("Failed to select issue type")?;
        types
            .into_iter()
            .find(|t| t.name == selected)
            .expect("selected issue type must exist")
    };

    let spinner = spinner_new(format!(
        "Fetching fields for {} / {}...",
        project_key, issue_type.name
    ));
    let mut fields = client
        .get_fields_for_issue_type(&project_key, &issue_type.id)
        .await
        .context("Failed to fetch fields")?;
    spinner.finish_and_clear();

    if required_only {
        fields.retain(|f| f.required);
    }

    // Sort: required first, then by name
    fields.sort_by(|a, b| b.required.cmp(&a.required).then(a.name.cmp(&b.name)));

    if json {
        println!("{}", serde_json::to_string_pretty(&fields)?);
        return Ok(());
    }

    println!(
        "\nFields for {} / {} ({} total):\n",
        project_key,
        issue_type.name,
        fields.len()
    );
    println!("{:<30} {:<20} {:<12} REQUIRED", "NAME", "ID", "TYPE");
    println!("{}", "─".repeat(72));

    for f in &fields {
        println!(
            "{:<30} {:<20} {:<12} {}",
            truncate(&f.name, 29),
            truncate(&f.id, 19),
            truncate(&f.field_type, 11),
            if f.required { "✓" } else { "" }
        );
    }

    Ok(())
}

pub(super) fn render_issue_content(
    input: Option<std::path::PathBuf>,
    format: String,
    output: String,
) -> Result<()> {
    let content = read_render_input(input.as_deref())?;
    let format = normalize_render_format(&format)?;
    let output = normalize_render_output(&output)?;

    let adf = match format {
        "markdown" => jira_core::adf::markdown_to_adf(&content),
        "text" => jira_core::adf::plain_text_to_adf(&content),
        "adf" => serde_json::from_str::<Value>(&content)
            .context("--format adf requires valid JSON ADF content")?,
        _ => unreachable!(),
    };

    match output {
        "adf" => println!("{}", serde_json::to_string_pretty(&adf)?),
        "text" => println!("{}", jira_core::adf::adf_to_text(&adf)),
        _ => unreachable!(),
    }

    Ok(())
}

// ─── helpers ─────────────────────────────────────────────────────────────────

fn read_render_input(path: Option<&std::path::Path>) -> Result<String> {
    match path {
        Some(path) => std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read input file: {}", path.display())),
        None => {
            use std::io::{IsTerminal, Read};

            // A TTY with nothing piped would block on stdin forever — refuse instead.
            // Piped input is NOT a terminal, so it still reads normally below.
            if std::io::stdin().is_terminal() {
                anyhow::bail!(
                    "no input file and nothing piped on stdin. Pass a file path or pipe content."
                );
            }

            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .context("Failed to read stdin")?;
            Ok(input)
        }
    }
}

fn normalize_render_format(value: &str) -> Result<&str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "markdown" | "md" => Ok("markdown"),
        "text" | "txt" => Ok("text"),
        "adf" | "json" => Ok("adf"),
        other => {
            anyhow::bail!("Unsupported input format '{other}'. Use one of: markdown, text, adf")
        }
    }
}

fn normalize_render_output(value: &str) -> Result<&str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "adf" | "json" => Ok("adf"),
        "text" | "txt" => Ok("text"),
        other => anyhow::bail!("Unsupported output format '{other}'. Use one of: adf, text"),
    }
}

pub(super) fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s[..max_len.saturating_sub(1)])
    }
}

async fn resolve_sprint_assignment(
    client: &JiraClient,
    project_key: &str,
    issue_type_id: &str,
    sprint: &str,
) -> Result<(String, FieldValue)> {
    if issue_type_id.is_empty() {
        anyhow::bail!(
            "Sprint assignment requires a resolved issue type so Jira fields can be inspected"
        );
    }

    let fields = client
        .get_fields_for_issue_type(project_key, issue_type_id)
        .await
        .context("Failed to inspect fields for sprint assignment")?;

    let sprint_field = fields
        .into_iter()
        .find(|field| {
            field.name.eq_ignore_ascii_case("Sprint")
                || field
                    .schema
                    .as_ref()
                    .and_then(|schema| schema.get("custom"))
                    .and_then(|value| value.as_str())
                    .map(|custom| custom.contains("gh-sprint"))
                    .unwrap_or(false)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Sprint is not available for project {} / this issue type on create",
                project_key
            )
        })?;

    let sprint_id = if let Ok(id) = sprint.trim().parse::<u64>() {
        id
    } else {
        resolve_sprint_id_by_name(client, project_key, sprint).await?
    };

    Ok((
        sprint_field.id,
        FieldValue::Raw(serde_json::json!([{ "id": sprint_id }])),
    ))
}

async fn resolve_sprint_id_by_name(
    client: &JiraClient,
    project_key: &str,
    sprint_name: &str,
) -> Result<u64> {
    let boards = client
        .raw_request(
            "GET",
            &format!("/rest/agile/1.0/board?projectKeyOrId={project_key}&maxResults=100"),
            None,
        )
        .await
        .context("Failed to list boards for sprint resolution")?
        .unwrap_or(Value::Null);

    let board_values = boards
        .get("values")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("Unexpected board response while resolving sprint"))?;

    let mut matches = Vec::new();

    for board in board_values {
        let board_id = match board.get("id").and_then(Value::as_u64) {
            Some(id) => id,
            None => continue,
        };

        let response = client
            .raw_request(
                "GET",
                &format!(
                    "/rest/agile/1.0/board/{board_id}/sprint?state=active,future,closed&maxResults=100"
                ),
                None,
            )
            .await;

        let Ok(Some(payload)) = response else {
            continue;
        };

        if let Some(values) = payload.get("values").and_then(Value::as_array) {
            for sprint in values {
                let Some(name) = sprint.get("name").and_then(Value::as_str) else {
                    continue;
                };
                if name.eq_ignore_ascii_case(sprint_name) {
                    if let Some(id) = sprint.get("id").and_then(Value::as_u64) {
                        matches.push((id, board_id, name.to_string()));
                    }
                }
            }
        }
    }

    match matches.len() {
        0 => anyhow::bail!(
            "Sprint '{}' was not found on any sprint-enabled board for project {}",
            sprint_name,
            project_key
        ),
        1 => Ok(matches[0].0),
        _ => {
            let options = matches
                .into_iter()
                .map(|(id, board_id, name)| format!("{name} (id:{id}, board:{board_id})"))
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::bail!(
                "Sprint '{}' matched multiple sprints. Use a numeric sprint ID instead: {}",
                sprint_name,
                options
            )
        }
    }
}
