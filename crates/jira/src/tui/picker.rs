use anyhow::Result;
use jira_core::JiraClient;

#[derive(Clone)]
pub(super) struct PickerOption {
    pub(super) value: String,
    pub(super) label: String,
}

impl std::fmt::Display for PickerOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

pub(super) fn normalize_picker_query(input: &str) -> String {
    input.trim().to_lowercase()
}

pub(super) fn prompt_search_term(prompt: &str) -> Result<Option<String>> {
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

pub(super) fn picker_option_matches(option: &PickerOption, query: &str) -> bool {
    let query = normalize_picker_query(query);
    if query.is_empty() {
        return true;
    }

    let haystack = normalize_picker_query(&format!("{} {}", option.label, option.value));
    haystack.contains(&query)
}

pub(super) fn pick_single_option(
    prompt: &str,
    options: Vec<PickerOption>,
) -> Result<Option<PickerOption>> {
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

pub(super) async fn prompt_assignee_selection(
    client: &JiraClient,
    prompt: &str,
) -> Result<Option<String>> {
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
