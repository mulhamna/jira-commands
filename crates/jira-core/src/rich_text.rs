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
