use serde_json::Value;

use crate::{
    config::JiraDeployment,
    error::{JiraError, Result},
};

/// Convert ADF to Jira wiki markup, warning and preserving readable text for unsupported features.
/// Mention IDs must be login names on Data Center or account IDs on Cloud.
/// Malformed ADF and excessive nesting still return an error.
pub fn adf_to_wiki(document: &Value, deployment: &JiraDeployment) -> Result<String> {
    if document.get("type").and_then(Value::as_str) != Some("doc") {
        return Err(unsupported("expected an ADF doc"));
    }
    render(document, "", 0, deployment)
}

fn unsupported(detail: &str) -> JiraError {
    JiraError::Config(format!(
        "Cannot convert rich text to Jira API v2 wiki markup: {detail}"
    ))
}

fn children(node: &Value) -> Result<&[Value]> {
    match node.get("content") {
        None => Ok(&[]),
        Some(Value::Array(content)) => Ok(content),
        _ => Err(unsupported("invalid ADF content")),
    }
}

fn escape(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::new();
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            out.push_str("\\\\\n");
        }
        if ["h1.", "h2.", "h3.", "h4.", "h5.", "h6.", "bq."]
            .iter()
            .any(|prefix| line.starts_with(prefix))
        {
            out.push('\\');
        }
        for ch in line.chars() {
            // Jira processes emoticons and citations even inside monospace spans.
            match ch {
                '\\' => out.push_str("&#92;"),
                '&' => out.push_str("&amp;"),
                '{' => out.push_str("&#123;"),
                '}' => out.push_str("&#125;"),
                '(' => out.push_str("&#40;"),
                ')' => out.push_str("&#41;"),
                ':' => out.push_str("&#58;"),
                '?' => out.push_str("&#63;"),
                _ => {
                    if "*_+^~[]!|#-".contains(ch) {
                        out.push('\\');
                    }
                    out.push(ch);
                }
            }
        }
    }
    out
}

fn render_children(
    node: &Value,
    prefix: &str,
    depth: usize,
    separator: &str,
    deployment: &JiraDeployment,
) -> Result<String> {
    let mut content = children(node)?.iter().peekable();
    let mut parts = Vec::new();
    let mut link_in_word = false;
    while let Some(child) = content.next() {
        if separator.is_empty() {
            if let Some(href) = link_target(child) {
                // Markdown entities and emphasis can split one link into adjacent ADF nodes.
                let mut group = vec![child];
                while let Some(next) = content.next_if(|next| link_target(next) == Some(href)) {
                    group.push(next);
                }
                let mut label = Vec::new();
                let mut text = String::new();
                for child in group {
                    let mut child = child.clone();
                    if let Some(marks) = child["marks"].as_array_mut() {
                        marks.retain(|mark| mark["type"] != "link");
                    }
                    label.push(render(&child, prefix, depth + 1, deployment)?);
                    text.push_str(child["text"].as_str().unwrap_or(""));
                }
                let label = join_inlines(label);
                let link = if link_in_word
                    && !label.starts_with(char::is_whitespace)
                    && !href.trim().is_empty()
                {
                    tracing::warn!("Flattening a link without a Jira wiki word boundary");
                    flatten_link(label, &text, href)
                } else {
                    render_link(label, &text, href)
                };
                if link.trim_start().starts_with('[') {
                    link_in_word = !link.ends_with(char::is_whitespace);
                } else if link.chars().any(char::is_whitespace) {
                    link_in_word = false;
                }
                parts.push(link);
                continue;
            }
        }
        let part = render(child, prefix, depth + 1, deployment)?;
        if part.chars().any(char::is_whitespace) {
            link_in_word = false;
        }
        parts.push(part);
    }
    Ok(if separator.is_empty() {
        join_inlines(parts)
    } else {
        parts.join(separator)
    })
}

fn join_inlines(mut parts: Vec<String>) -> String {
    for index in 0..parts.len() {
        let trimmed = parts[index].trim();
        let Some(delimiter) = trimmed.chars().next().filter(|ch| "*_+-".contains(*ch)) else {
            continue;
        };
        if trimmed.len() <= 2 || !trimmed.ends_with(delimiter) {
            continue;
        }
        let before = parts[index].starts_with(delimiter);
        let after = parts[index].ends_with(delimiter);
        // Entities supply wiki word boundaries without adding visible or hidden characters.
        if before && index > 0 {
            encode_word_edge(&mut parts[index - 1], false);
        }
        if after && index + 1 < parts.len() {
            encode_word_edge(&mut parts[index + 1], true);
        }
    }
    parts.concat()
}

fn encode_word_edge(text: &mut String, first: bool) {
    let ch = if first {
        text.chars().next()
    } else {
        text.chars().next_back()
    };
    if let Some(ch) = ch.filter(|ch| ch.is_alphanumeric()) {
        let start = if first { 0 } else { text.len() - ch.len_utf8() };
        text.replace_range(start..start + ch.len_utf8(), &format!("&#{};", ch as u32));
    }
}

fn link_target(node: &Value) -> Option<&str> {
    if node["type"] != "text" {
        return None;
    }
    let mark = node["marks"]
        .as_array()?
        .iter()
        .find(|mark| mark["type"] == "link")?;
    mark["attrs"]["href"].as_str()
}

fn render_link(label: String, text: &str, href: &str) -> String {
    if href.trim().is_empty() {
        tracing::warn!("Omitting link formatting without a URL for Jira API v2");
        return label;
    }
    let target = href.to_ascii_lowercase();
    let native_target = target.starts_with('#')
        || target.starts_with("//")
        || [
            "http://", "https://", "ftp://", "ftps://", "file://", "irc://", "mailto:",
        ]
        .iter()
        .any(|prefix| target.starts_with(prefix));
    if !native_target || href.contains(['|', '[', ']', '\r', '\n']) || label.contains('\n') {
        tracing::warn!("Flattening an unrepresentable link for Jira API v2");
        return flatten_link(label, text, href);
    }
    let label = label.replace("\\|", "&#124;");
    let start = label.len() - label.trim_start().len();
    let end = label.trim_end().len();
    if start >= end {
        tracing::warn!("Flattening a link with an empty label for Jira API v2");
        return format!("{label}{}", escape(href));
    }
    format!(
        "{}[{}|{href}]{}",
        &label[..start],
        &label[start..end],
        &label[end..]
    )
}

fn flatten_link(label: String, text: &str, href: &str) -> String {
    if text == href {
        label
    } else {
        format!("{label} {}", escape(&format!("({href})")))
    }
}

fn flatten(node: &Value, depth: usize, reason: &str) -> Result<String> {
    tracing::warn!(
        node_type = node["type"].as_str().unwrap_or("unknown"),
        reason,
        "Flattening unsupported rich text for Jira API v2"
    );
    Ok(escape(&plain(node, depth)?))
}

fn plain(node: &Value, depth: usize) -> Result<String> {
    if depth > super::MAX_RENDER_DEPTH {
        return Err(unsupported("ADF nesting exceeds the supported depth"));
    }
    let kind = node["type"].as_str().unwrap_or("unknown");
    if kind == "hardBreak" {
        return Ok("\n".into());
    }
    let content = children(node)?;
    let mut text = if let Some(text) = node["text"].as_str() {
        text.to_string()
    } else if !content.is_empty() {
        let parts = content
            .iter()
            .map(|child| plain(child, depth + 1))
            .collect::<Result<Vec<_>>>()?;
        if matches!(kind, "bulletList" | "orderedList") {
            let start = node["attrs"]["order"].as_u64().unwrap_or(1);
            parts
                .iter()
                .enumerate()
                .map(|(index, part)| {
                    if kind == "orderedList" {
                        format!("{}. {part}", start.saturating_add(index as u64))
                    } else {
                        format!("- {part}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            parts.join(match kind {
                "paragraph" | "heading" | "codeBlock" => "",
                "tableRow" => " | ",
                _ => "\n",
            })
        }
    } else if let Some(label) = [
        "text",
        "label",
        "shortName",
        "alt",
        "title",
        "name",
        "url",
        "timestamp",
        "id",
    ]
    .iter()
    .find_map(|attr| node["attrs"][attr].as_str().filter(|s| !s.is_empty()))
    {
        label.to_string()
    } else if matches!(
        kind,
        "doc"
            | "paragraph"
            | "heading"
            | "codeBlock"
            | "blockquote"
            | "bulletList"
            | "orderedList"
            | "listItem"
            | "table"
            | "tableRow"
            | "tableCell"
            | "tableHeader"
    ) {
        String::new()
    } else {
        format!("[{kind}]")
    };
    if let Some(url) = node["attrs"]["url"].as_str() {
        if !text.contains(url) {
            text.push_str(&format!(" ({url})"));
        }
    }
    if let Some(marks) = node["marks"].as_array() {
        for mark in marks {
            if mark["type"] == "link" {
                if let Some(href) = mark["attrs"]["href"].as_str() {
                    if text != href {
                        text.push_str(&format!(" ({href})"));
                    }
                }
            }
        }
    }
    Ok(text)
}

fn render(node: &Value, prefix: &str, depth: usize, deployment: &JiraDeployment) -> Result<String> {
    if depth > super::MAX_RENDER_DEPTH {
        return Err(unsupported("ADF nesting exceeds the supported depth"));
    }
    let kind = node.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "doc" => render_children(node, prefix, depth, "\n\n", deployment),
        "paragraph" => render_children(node, prefix, depth, "", deployment),
        "text" => render_text(node),
        "mention" => {
            let Some(id) = node["attrs"]["id"]
                .as_str()
                .filter(|id| !id.trim().is_empty())
            else {
                return flatten(node, depth, "mention is missing its identifier");
            };
            if id.contains(['[', ']', '|', '\\']) || id.chars().any(char::is_control) {
                return flatten(node, depth, "mention identifier contains wiki delimiters");
            }
            Ok(match deployment {
                JiraDeployment::DataCenter => format!("[~{id}]"),
                JiraDeployment::Cloud => format!("[~accountid:{id}]"),
            })
        }
        "hardBreak" => Ok("\\\\\n".into()),
        "heading" => {
            let Some(level) = node["attrs"]["level"]
                .as_u64()
                .filter(|level| (1..=6).contains(level))
            else {
                return flatten(node, depth, "invalid heading level");
            };
            Ok(format!(
                "h{level}. {}",
                render_children(node, prefix, depth, "", deployment)?
            ))
        }
        "bulletList" | "orderedList" => {
            if kind == "orderedList" && node["attrs"]["order"].as_u64().unwrap_or(1) != 1 {
                return flatten(node, depth, "wiki ordered lists always start at 1");
            }
            let marker = if kind == "orderedList" { '#' } else { '*' };
            let prefix = format!("{prefix}{marker}");
            render_children(node, &prefix, depth, "\n", deployment)
        }
        "listItem" => {
            let content = children(node)?;
            // Nested wiki markers already include the otherwise empty parent item.
            if content.len() == 1
                && (content[0]["type"] == "bulletList"
                    || (content[0]["type"] == "orderedList"
                        && content[0]["attrs"]["order"].as_u64().unwrap_or(1) == 1))
            {
                return render(&content[0], prefix, depth + 1, deployment);
            }
            let mut out = format!("{prefix} ");
            let mut previous_inline = false;
            for (index, child) in content.iter().enumerate() {
                let kind = child["type"].as_str().unwrap_or("");
                // Wiki continuation lines after a nested list belong to its last child.
                let flatten_list =
                    matches!(kind, "bulletList" | "orderedList") && index + 1 < content.len();
                if index > 0 {
                    out.push_str(
                        if flatten_list || (kind == "paragraph" && previous_inline) {
                            "\\\\\n"
                        } else {
                            "\n"
                        },
                    );
                }
                out.push_str(&if flatten_list {
                    flatten(child, depth + 1, "nested list has following parent content")?
                } else {
                    render(child, prefix, depth + 1, deployment)?
                });
                previous_inline = kind == "paragraph" || flatten_list;
            }
            Ok(out)
        }
        "codeBlock" => {
            let mut code = String::new();
            for child in children(node)? {
                if child["type"] != "text" {
                    return flatten(node, depth, "unsupported code block content");
                }
                code.push_str(
                    child["text"]
                        .as_str()
                        .ok_or_else(|| unsupported("code block text is missing"))?,
                );
            }
            let language = node["attrs"]["language"].as_str().unwrap_or("");
            if language.contains(['{', '}', '|', '\r', '\n']) || code.contains("{code}") {
                return flatten(node, depth, "code block contains wiki macro delimiters");
            }
            // Use bundled Jira formatters rather than assuming a Markdown language is installed.
            let language = language.to_ascii_lowercase();
            let language = match language.as_str() {
                "" | "actionscript" | "ada" | "applescript" | "bash" | "c" | "c#" | "c++"
                | "cpp" | "css" | "erlang" | "go" | "groovy" | "haskell" | "html" | "java"
                | "javascript" | "js" | "json" | "lua" | "none" | "nyan" | "objc" | "perl"
                | "php" | "python" | "r" | "rainbow" | "ruby" | "scala" | "sh" | "sql"
                | "swift" | "visualbasic" | "xml" | "yaml" => language.as_str(),
                "text" | "txt" | "plain" | "plaintext" => "none",
                language => {
                    tracing::warn!(
                        language,
                        "Omitting unsupported code language for Jira API v2"
                    );
                    "none"
                }
            };
            let open = if language.is_empty() {
                "{code}".to_string()
            } else {
                format!("{{code:{language}}}")
            };
            Ok(format!("{open}\n{code}\n{{code}}"))
        }
        "blockquote" => {
            let content = render_children(node, "", depth, "\n\n", deployment)?;
            if content.contains("{quote}") {
                return flatten(node, depth, "nested quote macros");
            }
            Ok(format!("{{quote}}\n{content}\n{{quote}}"))
        }
        "rule" => Ok("----".into()),
        "table" => render_children(node, prefix, depth, "\n", deployment),
        "tableRow" => {
            let cells = children(node)?;
            let header = cells
                .first()
                .is_some_and(|cell| cell["type"] == "tableHeader");
            let cell_type = if header { "tableHeader" } else { "tableCell" };
            if cells.iter().any(|cell| cell["type"] != cell_type) {
                return flatten(node, depth, "mixed header and data cells in a table row");
            }
            let separator = if header { "||" } else { "|" };
            Ok(format!(
                "{separator}{}{separator}",
                render_children(node, prefix, depth, separator, deployment)?
            ))
        }
        "tableCell" | "tableHeader" => {
            let content = if node["attrs"]["colspan"].as_u64().unwrap_or(1) != 1
                || node["attrs"]["rowspan"].as_u64().unwrap_or(1) != 1
                || children(node)?
                    .iter()
                    .any(|child| child["type"] != "paragraph")
            {
                flatten(
                    node,
                    depth,
                    "merged cell or non-paragraph table cell content",
                )?
            } else {
                render_children(node, prefix, depth, "\\\\", deployment)?
            }
            .replace('\n', "");
            Ok(if content.is_empty() {
                " ".into()
            } else {
                content
            })
        }
        _ => flatten(node, depth, "unsupported ADF node"),
    }
}

fn render_text(node: &Value) -> Result<String> {
    let text = node["text"]
        .as_str()
        .ok_or_else(|| unsupported("text node is missing text"))?;
    let marks = match node.get("marks") {
        None => &[][..],
        Some(Value::Array(marks)) => marks,
        _ => return Err(unsupported("invalid text marks")),
    };
    let mut code = marks.iter().any(|mark| mark["type"] == "code");
    if code && text.contains(['\r', '\n']) {
        tracing::warn!("Flattening multiline inline code for Jira API v2");
        code = false;
    }
    let mut out = if code {
        format!("{{{{{}}}}}", escape(text))
    } else {
        escape(text)
    };
    for mark in marks {
        let kind = mark["type"].as_str().unwrap_or("");
        let delimiter = match kind {
            "strong" => "*",
            "em" => "_",
            "strike" => "-",
            "underline" => "+",
            "code" => continue,
            "link" => {
                let Some(href) = mark["attrs"]["href"].as_str() else {
                    tracing::warn!("Omitting link formatting without a URL for Jira API v2");
                    continue;
                };
                out = render_link(out, text, href);
                continue;
            }
            _ => {
                tracing::warn!(
                    mark_type = kind,
                    "Omitting unsupported text formatting for Jira API v2"
                );
                continue;
            }
        };
        let start = out.len() - out.trim_start().len();
        let end = out.trim_end().len();
        if start < end {
            out = format!(
                "{}{delimiter}{}{delimiter}{}",
                &out[..start],
                &out[start..end],
                &out[end..]
            );
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adf::markdown_to_adf;
    use serde_json::json;

    fn dc_wiki(document: &Value) -> Result<String> {
        adf_to_wiki(document, &JiraDeployment::DataCenter)
    }

    #[test]
    fn markdown_rich_text_becomes_wiki() {
        let markdown = "# Heading\n\n**Bold**, *italic*, `code`, [Example](https://example.com).\n\n> Quote\n\n---\n\n```rust\nlet x = 1;\n```";
        assert_eq!(
            dc_wiki(&markdown_to_adf(markdown)).unwrap(),
            "h1. Heading\n\n*Bold*, _italic_, {{code}}, [Example|https://example.com].\n\n{quote}\nQuote\n{quote}\n\n----\n\n{code:none}\nlet x = 1;\n{code}"
        );
    }

    #[test]
    fn code_languages_keep_supported_highlighting_and_fall_back_to_plain_code() {
        for (language, expected) in [
            ("rust", "none"),
            ("typescript", "none"),
            ("custom-language", "none"),
            ("text", "none"),
            ("txt", "none"),
            ("plaintext", "none"),
            ("plain", "none"),
            ("python", "python"),
            ("PYTHON", "python"),
            ("bash", "bash"),
            ("java", "java"),
            ("", ""),
        ] {
            let markdown = format!("```{language}\nlet x = 1;\n```");
            let open = if expected.is_empty() {
                "{code}".to_string()
            } else {
                format!("{{code:{expected}}}")
            };
            assert_eq!(
                dc_wiki(&markdown_to_adf(&markdown)).unwrap(),
                format!("{open}\nlet x = 1;\n{{code}}"),
                "{language}"
            );
        }
    }

    #[test]
    fn nested_lists_and_table_headers_become_wiki() {
        assert_eq!(
            dc_wiki(&markdown_to_adf("- First\n  1. Nested\n- Second")).unwrap(),
            "* First\n*# Nested\n* Second"
        );
        assert_eq!(
            dc_wiki(&markdown_to_adf(
                "| Name | State |\n| --- | --- |\n| Item | **Ready** |"
            ))
            .unwrap(),
            "||Name||State||\n|Item|*Ready*|"
        );
    }

    #[test]
    fn list_items_support_code_quotes_headings_and_continuation_paragraphs() {
        let markdown = "1. Run:\n\n   ```bash\n   cargo build\n   ```\n\n   > Quote\n\n   ### Details\n\n   Continue.\n\n2. Next";
        assert_eq!(
            dc_wiki(&markdown_to_adf(markdown)).unwrap(),
            "# Run&#58;\n{code:bash}\ncargo build\n{code}\n{quote}\nQuote\n{quote}\nh3. Details\nContinue.\n# Next"
        );
        assert_eq!(
            dc_wiki(&markdown_to_adf("- Parent\n\n  > - Quoted child\n\n- Next")).unwrap(),
            "* Parent\n{quote}\n* Quoted child\n{quote}\n* Next"
        );
    }

    #[test]
    fn parent_content_after_nested_lists_stays_in_the_parent_item() {
        let markdown = "- Parent\n  - Child\n\n  Parent continuation\n- Next";
        assert_eq!(
            dc_wiki(&markdown_to_adf(markdown)).unwrap(),
            "* Parent\\\\\n\\- Child\\\\\nParent continuation\n* Next"
        );
    }

    #[test]
    fn nested_only_list_items_do_not_duplicate_the_parent_marker() {
        for (markdown, expected) in [
            ("- - x", "** x"),
            ("1. 1. x", "## x"),
            ("- 1. x", "*# x"),
            ("1. - x", "#* x"),
            ("- - - x", "*** x"),
            ("- - x\n\n  para", "* \\- x\\\\\npara"),
        ] {
            assert_eq!(dc_wiki(&markdown_to_adf(markdown)).unwrap(), expected);
        }
        let nested = json!({
            "type": "orderedList", "attrs": {"order": 3}, "content": [{
                "type": "listItem", "content": [{
                    "type": "paragraph", "content": [{"type": "text", "text": "Third"}]
                }]
            }]
        });
        let document = json!({
            "type": "doc", "content": [{
                "type": "bulletList", "content": [{"type": "listItem", "content": [nested]}]
            }]
        });
        assert_eq!(dc_wiki(&document).unwrap(), "* 3. Third");
    }

    #[test]
    fn literal_emoticons_and_citations_stay_literal_in_prose_and_inline_code() {
        assert_eq!(
            dc_wiki(&crate::adf::plain_text_to_adf(
                "f(x), O(n), :) :D ;) ??cite??"
            ))
            .unwrap(),
            "f&#40;x&#41;, O&#40;n&#41;, &#58;&#41; &#58;D ;&#41; &#63;&#63;cite&#63;&#63;"
        );
        assert_eq!(
            dc_wiki(&markdown_to_adf("`f(x) :P ??cite??`")).unwrap(),
            "{{f&#40;x&#41; &#58;P &#63;&#63;cite&#63;&#63;}}"
        );
    }

    #[test]
    fn literal_markup_is_escaped_in_plain_text_nodes() {
        let document = json!({
            "type": "doc", "content": [{
                "type": "paragraph", "content": [{
                    "type": "text", "text": "*literal* [not a link] {code}\nh1. Not a heading"
                }]
            }]
        });
        assert_eq!(
            dc_wiki(&document).unwrap(),
            "\\*literal\\* \\[not a link\\] &#123;code&#125;\\\\\n\\h1. Not a heading"
        );
    }

    #[test]
    fn literal_backslashes_and_entities_remain_literal() {
        assert_eq!(
            dc_wiki(&markdown_to_adf(r"C:\Users\me and \d+")).unwrap(),
            r"C&#58;&#92;Users&#92;me and &#92;d\+"
        );
        assert_eq!(
            dc_wiki(&crate::adf::plain_text_to_adf(r"\\ \* &#92;")).unwrap(),
            r"&#92;&#92; &#92;\* &amp;\#92;"
        );
    }

    #[test]
    fn empty_table_cells_keep_their_columns_and_cell_types() {
        assert_eq!(
            dc_wiki(&markdown_to_adf(
                "| | Middle | |\n| --- | --- | --- |\n| a | | b |\n| | | |"
            ))
            .unwrap(),
            "|| ||Middle|| ||\n|a| |b|\n| | | |"
        );
    }

    #[test]
    fn inline_code_escapes_wiki_markup_and_table_delimiters() {
        assert_eq!(
            dc_wiki(&markdown_to_adf(r"`arr[0] | **kwargs** \d+ &copy;`")).unwrap(),
            r"{{arr\[0\] \| \*\*kwargs\*\* &#92;d\+ &amp;copy;}}"
        );
        assert_eq!(
            dc_wiki(&markdown_to_adf(
                "| Code | Empty |\n| --- | --- |\n| `a\\|b` | |"
            ))
            .unwrap(),
            "||Code||Empty||\n|{{a\\|b}}| |"
        );
    }

    #[test]
    fn inline_code_braces_do_not_close_the_monospace_span() {
        assert_eq!(
            dc_wiki(&markdown_to_adf("`{x} {{value}}`")).unwrap(),
            "{{&#123;x&#125; &#123;&#123;value&#125;&#125;}}"
        );
    }

    #[test]
    fn nested_emphasis_keeps_spaces_outside_wiki_delimiters() {
        assert_eq!(
            dc_wiki(&markdown_to_adf("**bold and *italic***")).unwrap(),
            "*bold and* *_italic_*"
        );
    }

    #[test]
    fn link_fragments_share_one_wiki_link_and_literal_urls_remain_literal() {
        let target = "https://example.com/a-b_c?first=1&second=2";
        for (markdown, label) in [
            ("A & B", "A &amp; B"),
            ("**A** & B", "*A* &amp; B"),
            (" A & B ", "A &amp; B"),
        ] {
            let actual = dc_wiki(&markdown_to_adf(&format!("[{markdown}]({target})"))).unwrap();
            assert_eq!(actual.trim(), format!("[{label}|{target}]"));
        }
        assert_eq!(
            dc_wiki(&crate::adf::plain_text_to_adf(target)).unwrap(),
            "https&#58;//example.com/a\\-b\\_c&#63;first=1&amp;second=2"
        );
    }

    #[test]
    fn intra_word_emphasis_preserves_formatting_without_inserting_characters() {
        for (markdown, expected) in [
            ("foo**bar**baz", "fo&#111;*bar*&#98;az"),
            ("foo*bar*baz", "fo&#111;_bar_&#98;az"),
            ("foo**bar**", "fo&#111;*bar*"),
            ("**bar**baz", "*bar*&#98;az"),
            ("foo **bar** baz", "foo *bar* baz"),
            ("\u{e9}**x**\u{5b57}", "&#233;*x*&#23383;"),
            (
                "[foo**bar**baz](https://example.com)",
                "[fo&#111;*bar*&#98;az|https://example.com]",
            ),
        ] {
            assert_eq!(dc_wiki(&markdown_to_adf(markdown)).unwrap(), expected);
        }
    }

    #[test]
    fn links_without_word_boundaries_preserve_labels_and_targets() {
        for (markdown, expected) in [
            (
                "[A](https://a.example)[B](https://b.example)",
                "[A|https://a.example]B &#40;https&#58;//b.example&#41;",
            ),
            (
                "[A](https://a.example)-[B](https://b.example)",
                "[A|https://a.example]\\-B &#40;https&#58;//b.example&#41;",
            ),
            (
                "[A](https://a.example)[B](https://b.example) [C](https://c.example)",
                "[A|https://a.example]B &#40;https&#58;//b.example&#41; [C|https://c.example]",
            ),
            (
                "[A](https://a.example) [B](https://b.example)",
                "[A|https://a.example] [B|https://b.example]",
            ),
            (
                "[A](https://a.example)`x y`[B](https://b.example)",
                "[A|https://a.example]{{x y}}[B|https://b.example]",
            ),
            (
                "[A](https://a.example)**x y**[B](https://b.example)",
                "[A|https://a.example]*x y*[B|https://b.example]",
            ),
            (
                "[A](https://a.example)[B](https://a.example)",
                "[AB|https://a.example]",
            ),
            ("[A](https://a.example)[B]()", "[A|https://a.example]B"),
            (
                "pre[link](https://example.com)post",
                "pre[link|https://example.com]post",
            ),
            ("-[B](https://b.example)", "\\-[B|https://b.example]"),
        ] {
            assert_eq!(dc_wiki(&markdown_to_adf(markdown)).unwrap(), expected);
        }
    }

    #[test]
    fn flattened_relative_link_targets_do_not_become_emoticons() {
        for target in ["x", "y", "n", "i", "off", "docs/page.md"] {
            let markdown = format!("[A](https://a.example)[B]({target})");
            assert_eq!(
                dc_wiki(&markdown_to_adf(&markdown)).unwrap(),
                format!("[A|https://a.example]B &#40;{target}&#41;")
            );
        }
        assert_eq!(
            dc_wiki(&markdown_to_adf("[A](https://a.example)[x](x)")).unwrap(),
            "[A|https://a.example]x"
        );
    }

    #[test]
    fn link_labels_keep_literal_pipes_without_breaking_native_links() {
        for (markdown, expected) in [
            (
                "[a|b](https://example.com)",
                "[a&#124;b|https://example.com]",
            ),
            (
                "[`a|b`](https://example.com)",
                "[{{a&#124;b}}|https://example.com]",
            ),
            (
                "[**a|b**](https://example.com)",
                "[*a&#124;b*|https://example.com]",
            ),
            (
                r"[`a\|b`](https://example.com)",
                "[{{a&#92;&#124;b}}|https://example.com]",
            ),
            (
                "| Link |\n| --- |\n| [a\\|b](https://example.com) |",
                "||Link||\n|[a&#124;b|https://example.com]|",
            ),
        ] {
            assert_eq!(dc_wiki(&markdown_to_adf(markdown)).unwrap(), expected);
        }
    }

    #[test]
    fn unsupported_link_destinations_fall_back_without_losing_native_url_forms() {
        for (markdown, expected) in [
            ("[Docs](docs/page.md)", "Docs &#40;docs/page.md&#41;"),
            ("[Docs](/docs/page.md)", "Docs &#40;/docs/page.md&#41;"),
            ("[Call](tel:1234)", "Call &#40;tel&#58;1234&#41;"),
            (
                "[SSH](ssh://example.com/path)",
                "SSH &#40;ssh&#58;//example.com/path&#41;",
            ),
            (
                "[News](news:example.group)",
                "News &#40;news&#58;example.group&#41;",
            ),
            ("[Self](#anchor)", "[Self|#anchor]"),
            ("[Web](//example.com/path)", "[Web|//example.com/path]"),
            (
                "[Mail](mailto:reader@example.com)",
                "[Mail|mailto:reader@example.com]",
            ),
        ] {
            assert_eq!(dc_wiki(&markdown_to_adf(markdown)).unwrap(), expected);
        }
    }

    #[test]
    fn table_line_breaks_do_not_split_rows() {
        let document = json!({
            "type": "doc", "content": [{
                "type": "table", "content": [{
                    "type": "tableRow", "content": [{
                        "type": "tableCell", "content": [{
                            "type": "paragraph", "content": [
                                {"type": "text", "text": "first|part"},
                                {"type": "hardBreak"},
                                {"type": "text", "text": "second"}
                            ]
                        }]
                    }]
                }]
            }]
        });
        assert_eq!(dc_wiki(&document).unwrap(), "|first\\|part\\\\second|");
    }

    #[test]
    fn excessively_nested_adf_is_rejected() {
        for kind in ["blockquote", "panel"] {
            let mut node = json!({"type": "paragraph"});
            for _ in 0..super::super::MAX_RENDER_DEPTH {
                node = json!({"type": kind, "content": [node]});
            }
            let document = json!({"type": "doc", "content": [node]});
            assert!(dc_wiki(&document)
                .unwrap_err()
                .to_string()
                .contains("depth"));
        }
    }

    #[test]
    fn unsupported_adf_preserves_readable_text_labels_and_link_targets() {
        for (node, expected) in [
            (
                json!({"type": "inlineCard", "attrs": {"url": "https://example.com"}}),
                "https&#58;//example.com",
            ),
            (
                json!({"type": "mention", "attrs": {"id": "invalid]identifier"}}),
                "invalid\\]identifier",
            ),
            (
                json!({"type": "mention", "attrs": {"text": "@User"}}),
                "@User",
            ),
            (json!({"type": "mention", "attrs": {}}), "\\[mention\\]"),
            (
                json!({"type": "text", "text": "red", "marks": [{"type": "textColor"}, {"type": "strong"}]}),
                "*red*",
            ),
            (
                json!({"type": "codeBlock", "content": [{"type": "text", "text": "{code}"}]}),
                "&#123;code&#125;",
            ),
            (
                json!({"type": "text", "text": "bad", "marks": [{"type": "link", "attrs": {"href": "https://example.com/a|b"}}]}),
                "bad &#40;https&#58;//example.com/a\\|b&#41;",
            ),
            (
                json!({"type": "text", "text": "label", "marks": [{"type": "link", "attrs": {"href": ""}}]}),
                "label",
            ),
            (
                json!({"type": "text", "text": "first\nsecond", "marks": [{"type": "link", "attrs": {"href": "https://example.com"}}]}),
                "first\\\\\nsecond &#40;https&#58;//example.com&#41;",
            ),
            (
                json!({"type": "blockquote", "content": [{"type": "blockquote", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "nested"}]}]}]}),
                "nested",
            ),
            (
                json!({"type": "panel", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Example", "marks": [{"type": "link", "attrs": {"href": "https://example.com"}}]}]}]}),
                "Example &#40;https&#58;//example.com&#41;",
            ),
            (
                json!({"type": "status", "attrs": {"text": "In progress"}}),
                "In progress",
            ),
            (
                json!({"type": "emoji", "attrs": {"shortName": ":example:"}}),
                "&#58;example&#58;",
            ),
            (
                json!({"type": "media", "attrs": {"alt": "Screenshot"}}),
                "Screenshot",
            ),
            (
                json!({"type": "media", "attrs": {"alt": "Screenshot", "url": "https://example.com/image"}}),
                "Screenshot &#40;https&#58;//example.com/image&#41;",
            ),
            (json!({"type": "extension", "attrs": {}}), "\\[extension\\]"),
            (
                json!({"type": "text", "text": "first\nsecond", "marks": [{"type": "code"}]}),
                "first\\\\\nsecond",
            ),
            (
                json!({"type": "text", "text": "first\r\nsecond", "marks": [{"type": "code"}]}),
                "first\\\\\nsecond",
            ),
        ] {
            let document = json!({"type": "doc", "content": [node]});
            assert_eq!(dc_wiki(&document).unwrap(), expected, "{document}");
        }
    }

    #[test]
    fn complex_table_cells_and_ordered_starts_are_flattened_without_macro_injection() {
        let document = json!({
            "type": "doc", "content": [{
                "type": "table", "content": [{
                    "type": "tableRow", "content": [{
                        "type": "tableCell", "attrs": {"colspan": 2}, "content": [
                            {"type": "heading", "attrs": {"level": 2}, "content": [{"type": "text", "text": "Head|ing"}]},
                            {"type": "codeBlock", "content": [{"type": "text", "text": "{code}"}]}
                        ]
                    }, {
                        "type": "tableCell", "content": []
                    }]
                }]
            }, {
                "type": "orderedList", "attrs": {"order": 3}, "content": [{
                    "type": "listItem", "content": [{
                        "type": "paragraph", "content": [{"type": "text", "text": "Third"}]
                    }]
                }]
            }]
        });
        assert_eq!(
            dc_wiki(&document).unwrap(),
            "|Head\\|ing\\\\&#123;code&#125;| |\n\n3. Third"
        );
    }

    #[test]
    fn malformed_adf_still_returns_an_error() {
        for document in [
            json!({"type": "paragraph"}),
            json!({"type": "doc", "content": "not an array"}),
            json!({"type": "doc", "content": [{"type": "text", "marks": []}]}),
        ] {
            assert!(dc_wiki(&document).is_err());
        }
    }
}
