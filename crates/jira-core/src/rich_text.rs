use serde_json::Value;

mod wiki;
pub use wiki::wiki_to_text;

/// Render Jira rich text for display: wiki markup strings from API v2 or ADF from API v3.
/// Unrecognized response shapes are kept as JSON rather than disappearing.
pub fn rich_text_to_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(wiki) => wiki_to_text(wiki),
        Value::Object(doc)
            if doc.get("type").and_then(Value::as_str) == Some("doc")
                && doc.get("content").is_some_and(Value::is_array) =>
        {
            crate::adf::adf_to_text(value)
        }
        _ => value.to_string(),
    }
}

/// Collect mentioned user identifiers from Jira rich text: ADF mention nodes
/// from API v3, or `[~name]` / `[~accountid:id]` links in API v2 wiki markup.
/// Wiki identifiers are login names (or keys) on Data Center and account IDs
/// on Cloud, matching what `JiraClient::get_myself` returns for each.
pub fn mentioned_users(value: &Value) -> Vec<String> {
    match value {
        Value::String(wiki) => wiki_mentioned_users(wiki),
        _ => crate::adf::mentioned_account_ids(value),
    }
}

fn wiki_mentioned_users(wiki: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in strip_wiki_literal_blocks(wiki).lines() {
        let mut rest = line;
        while let Some(start) = rest.find("[~") {
            rest = &rest[start + 2..];
            let Some(end) = rest.find(']') else { break };
            let target = &rest[..end];
            let id = target.strip_prefix("accountid:").unwrap_or(target).trim();
            if !id.is_empty() && !id.contains(['[', '|']) && !out.iter().any(|seen| seen == id) {
                out.push(id.to_string());
            }
            rest = &rest[end + 1..];
        }
    }
    out
}

/// Drop `{code}` and `{noformat}` bodies, where `[~name]` is literal text.
// ponytail: line-based macro scan; inline `{{[~x]}}` monospace still counts as a mention.
fn strip_wiki_literal_blocks(wiki: &str) -> String {
    let mut out = String::new();
    let mut closing: Option<&str> = None;
    for line in wiki.lines() {
        let trimmed = line.trim_start();
        match closing {
            Some(tag) => {
                if let Some(pos) = line.find(tag) {
                    out.push_str(&line[pos + tag.len()..]);
                    out.push('\n');
                    closing = None;
                }
            }
            None => {
                let tag = ["{code", "{noformat"]
                    .into_iter()
                    .find(|tag| trimmed.starts_with(tag));
                match tag {
                    Some(open) => {
                        let close = if open == "{code" {
                            "{code}"
                        } else {
                            "{noformat}"
                        };
                        // The opening macro may carry parameters: `{code:java}`.
                        let body_start = trimmed.find('}').map_or(trimmed.len(), |i| i + 1);
                        let body = &trimmed[body_start..];
                        match body.find(close) {
                            Some(pos) => {
                                out.push_str(&body[pos + close.len()..]);
                                out.push('\n');
                            }
                            None => closing = Some(close),
                        }
                    }
                    None => {
                        out.push_str(line);
                        out.push('\n');
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_wiki_strings_without_changing_adf_output() {
        let adf = crate::adf::markdown_to_adf("## Details\n\n**Important** text");
        let wiki = json!("h2. Details\n\n*Important* text");

        assert_eq!(rich_text_to_text(&wiki), "## Details\n\nImportant text");
        assert_eq!(rich_text_to_text(&adf), crate::adf::adf_to_text(&adf));
        assert_eq!(rich_text_to_text(&adf), "## Details\nImportant text");
    }

    #[test]
    fn absent_or_empty_content_stays_empty() {
        for value in [
            Value::Null,
            json!(""),
            json!({"type": "doc", "content": []}),
        ] {
            assert_eq!(rich_text_to_text(&value), "");
        }
    }

    #[test]
    fn wiki_mentions_cover_data_center_and_cloud_links() {
        let wiki = json!(
            "Hi [~alice] and [~accountid:5b10ac8d82e05b22cc7d4ef5], cc [~alice].\n\
             {code:java}\n[~not_code]\n{code}\n\
             {noformat}[~not_noformat]{noformat} after [~bob]\n\
             [~] [~bad|x] [Docs|https://example.com]"
        );
        assert_eq!(
            mentioned_users(&wiki),
            ["alice", "5b10ac8d82e05b22cc7d4ef5", "bob"]
        );
    }

    #[test]
    fn adf_mentions_are_unchanged() {
        let adf = json!({"type": "doc", "version": 1, "content": [{
            "type": "paragraph",
            "content": [{"type": "mention", "attrs": {"id": "acc-1", "text": "@A"}}]
        }]});
        assert_eq!(mentioned_users(&adf), ["acc-1"]);
        assert!(mentioned_users(&Value::Null).is_empty());
    }

    #[test]
    fn unrecognized_response_shapes_remain_visible() {
        for value in [
            json!({"text": "Unexpected description"}),
            json!({"type": "doc", "content": "malformed"}),
            json!(["Unexpected", "description"]),
        ] {
            assert_eq!(rich_text_to_text(&value), value.to_string());
        }
    }
}
