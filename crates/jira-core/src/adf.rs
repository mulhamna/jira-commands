use serde_json::{json, Value};

/// Convert an ADF (Atlassian Document Format) JSON value to plain text.
pub fn adf_to_text(value: &Value) -> String {
    let mut output = String::new();
    render_node(value, &mut output, 0);
    output.trim_end().to_string()
}

fn render_node(node: &Value, out: &mut String, depth: usize) {
    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("");

    match node_type {
        "doc" => render_children(node, out, depth),
        "paragraph" => {
            render_children(node, out, depth);
            out.push('\n');
        }
        "text" => {
            if let Some(text) = node.get("text").and_then(|v| v.as_str()) {
                out.push_str(text);
            }
        }
        "hardBreak" => {
            out.push('\n');
        }
        "heading" => {
            let level = node
                .get("attrs")
                .and_then(|a| a.get("level"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1);
            let hashes = "#".repeat(level as usize);
            out.push_str(&hashes);
            out.push(' ');
            render_children(node, out, depth);
            out.push('\n');
        }
        "bulletList" => {
            render_list(node, out, depth, false);
        }
        "orderedList" => {
            render_list(node, out, depth, true);
        }
        "listItem" => {
            render_children(node, out, depth);
        }
        "codeBlock" => {
            let language = node
                .get("attrs")
                .and_then(|a| a.get("language"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            out.push_str("```");
            out.push_str(language);
            out.push('\n');
            render_children(node, out, depth);
            out.push_str("```\n");
        }
        "blockquote" => {
            let mut inner = String::new();
            render_children(node, &mut inner, depth);
            for line in inner.lines() {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
        }
        "rule" => {
            out.push_str("---\n");
        }
        _ => {
            // Unknown node — try to render children
            render_children(node, out, depth);
        }
    }
}

fn render_children(node: &Value, out: &mut String, depth: usize) {
    if let Some(children) = node.get("content").and_then(|v| v.as_array()) {
        for child in children {
            render_node(child, out, depth);
        }
    }
}

fn render_list(node: &Value, out: &mut String, depth: usize, ordered: bool) {
    let indent = "  ".repeat(depth);
    if let Some(items) = node.get("content").and_then(|v| v.as_array()) {
        for (i, item) in items.iter().enumerate() {
            let bullet = if ordered {
                format!("{}{}. ", indent, i + 1)
            } else {
                format!("{}- ", indent)
            };
            out.push_str(&bullet);
            // Render list item content inline
            let mut item_text = String::new();
            render_node(item, &mut item_text, depth + 1);
            out.push_str(item_text.trim());
            out.push('\n');
        }
    }
}

/// Convert plain text to ADF JSON — each non-empty line becomes a paragraph.
pub fn plain_text_to_adf(text: &str) -> Value {
    let mut content: Vec<Value> = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|line| {
            json!({
                "type": "paragraph",
                "content": [{ "type": "text", "text": line }]
            })
        })
        .collect();

    if content.is_empty() {
        content.push(json!({ "type": "paragraph", "content": [] }));
    }

    json!({ "version": 1, "type": "doc", "content": content })
}

/// Convert Markdown text to ADF JSON.
pub fn markdown_to_adf(markdown: &str) -> Value {
    use comrak::{parse_document, Arena, Options};

    let arena = Arena::new();
    let options = Options::default();
    let root = parse_document(&arena, markdown, &options);

    let mut content: Vec<Value> = Vec::new();
    convert_node(root, &mut content);

    json!({
        "version": 1,
        "type": "doc",
        "content": content
    })
}

fn convert_node<'a>(node: &'a comrak::nodes::AstNode<'a>, out: &mut Vec<Value>) {
    use comrak::nodes::{ListType, NodeValue};

    match &node.data.borrow().value {
        NodeValue::Document => {
            for child in node.children() {
                convert_node(child, out);
            }
        }
        NodeValue::Paragraph => {
            let mut inline_content: Vec<Value> = Vec::new();
            for child in node.children() {
                collect_inline(child, &mut inline_content);
            }
            if !inline_content.is_empty() {
                out.push(json!({
                    "type": "paragraph",
                    "content": inline_content
                }));
            }
        }
        NodeValue::Heading(heading) => {
            let mut inline_content: Vec<Value> = Vec::new();
            for child in node.children() {
                collect_inline(child, &mut inline_content);
            }
            out.push(json!({
                "type": "heading",
                "attrs": { "level": heading.level },
                "content": inline_content
            }));
        }
        NodeValue::List(list) => {
            let mut items: Vec<Value> = Vec::new();
            for child in node.children() {
                let mut item_content: Vec<Value> = Vec::new();
                convert_node(child, &mut item_content);
                items.push(json!({
                    "type": "listItem",
                    "content": item_content
                }));
            }
            let list_type = match list.list_type {
                ListType::Ordered => "orderedList",
                ListType::Bullet => "bulletList",
            };
            out.push(json!({
                "type": list_type,
                "content": items
            }));
        }
        NodeValue::Item(_) => {
            // Handled by parent list node
            let mut para_content: Vec<Value> = Vec::new();
            for child in node.children() {
                convert_node(child, &mut para_content);
            }
            out.extend(para_content);
        }
        NodeValue::CodeBlock(code) => {
            let language = code.info.trim().to_string();
            let text = code.literal.trim_end_matches('\n').to_string();
            out.push(json!({
                "type": "codeBlock",
                "attrs": { "language": language },
                "content": [{ "type": "text", "text": text }]
            }));
        }
        NodeValue::BlockQuote => {
            let mut inner: Vec<Value> = Vec::new();
            for child in node.children() {
                convert_node(child, &mut inner);
            }
            out.push(json!({
                "type": "blockquote",
                "content": inner
            }));
        }
        NodeValue::ThematicBreak => {
            out.push(json!({ "type": "rule" }));
        }
        NodeValue::LineBreak | NodeValue::SoftBreak => {
            // handled inline
        }
        _ => {
            // For other block-level nodes, recurse
            let mut child_content: Vec<Value> = Vec::new();
            for child in node.children() {
                convert_node(child, &mut child_content);
            }
            out.extend(child_content);
        }
    }
}

fn collect_inline<'a>(node: &'a comrak::nodes::AstNode<'a>, out: &mut Vec<Value>) {
    use comrak::nodes::NodeValue;

    match &node.data.borrow().value {
        NodeValue::Text(text) => {
            out.push(json!({ "type": "text", "text": text }));
        }
        NodeValue::SoftBreak => {
            out.push(json!({ "type": "text", "text": " " }));
        }
        NodeValue::LineBreak => {
            out.push(json!({ "type": "hardBreak" }));
        }
        NodeValue::Code(code) => {
            out.push(json!({
                "type": "text",
                "text": code.literal,
                "marks": [{ "type": "code" }]
            }));
        }
        NodeValue::Strong => {
            let mut inner: Vec<Value> = Vec::new();
            for child in node.children() {
                collect_inline(child, &mut inner);
            }
            for mut item in inner {
                let marks = item.get("marks").cloned().unwrap_or_else(|| json!([]));
                let mut marks_arr = marks.as_array().cloned().unwrap_or_default();
                marks_arr.push(json!({ "type": "strong" }));
                item["marks"] = json!(marks_arr);
                out.push(item);
            }
        }
        NodeValue::Emph => {
            let mut inner: Vec<Value> = Vec::new();
            for child in node.children() {
                collect_inline(child, &mut inner);
            }
            for mut item in inner {
                let marks = item.get("marks").cloned().unwrap_or_else(|| json!([]));
                let mut marks_arr = marks.as_array().cloned().unwrap_or_default();
                marks_arr.push(json!({ "type": "em" }));
                item["marks"] = json!(marks_arr);
                out.push(item);
            }
        }
        NodeValue::Link(link) => {
            let mut inner: Vec<Value> = Vec::new();
            for child in node.children() {
                collect_inline(child, &mut inner);
            }
            let url = link.url.clone();
            for mut item in inner {
                let marks = item.get("marks").cloned().unwrap_or_else(|| json!([]));
                let mut marks_arr = marks.as_array().cloned().unwrap_or_default();
                marks_arr.push(json!({
                    "type": "link",
                    "attrs": { "href": url }
                }));
                item["marks"] = json!(marks_arr);
                out.push(item);
            }
        }
        _ => {
            for child in node.children() {
                collect_inline(child, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adf_to_text_paragraph() {
        let adf = json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{ "type": "text", "text": "Hello world" }]
            }]
        });
        assert_eq!(adf_to_text(&adf), "Hello world");
    }

    #[test]
    fn test_adf_to_text_heading() {
        let adf = json!({
            "type": "doc",
            "content": [{
                "type": "heading",
                "attrs": { "level": 2 },
                "content": [{ "type": "text", "text": "Title" }]
            }]
        });
        assert!(adf_to_text(&adf).starts_with("## Title"));
    }

    #[test]
    fn test_markdown_to_adf_paragraph() {
        let adf = markdown_to_adf("Hello world");
        assert_eq!(adf["type"], "doc");
        assert_eq!(adf["content"][0]["type"], "paragraph");
    }

    #[test]
    fn test_markdown_to_adf_heading() {
        let adf = markdown_to_adf("# My Heading");
        assert_eq!(adf["content"][0]["type"], "heading");
        assert_eq!(adf["content"][0]["attrs"]["level"], 1);
    }
}
