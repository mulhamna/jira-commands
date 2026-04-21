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
        "table" => {
            render_table(node, out, depth);
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
            let mut item_text = String::new();
            render_node(item, &mut item_text, depth + 1);
            out.push_str(item_text.trim());
            out.push('\n');
        }
    }
}

fn render_table(node: &Value, out: &mut String, depth: usize) {
    let indent = "  ".repeat(depth);
    let mut rows: Vec<Vec<String>> = Vec::new();

    if let Some(row_nodes) = node.get("content").and_then(|v| v.as_array()) {
        for row in row_nodes {
            let mut cells: Vec<String> = Vec::new();
            if let Some(cell_nodes) = row.get("content").and_then(|v| v.as_array()) {
                for cell in cell_nodes {
                    let mut cell_text = String::new();
                    render_children(cell, &mut cell_text, depth + 1);
                    let normalized = cell_text
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join(" <br> ");
                    cells.push(normalized);
                }
            }
            if !cells.is_empty() {
                rows.push(cells);
            }
        }
    }

    if rows.is_empty() {
        return;
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    for row in &mut rows {
        while row.len() < col_count {
            row.push(String::new());
        }
    }

    let mut widths = vec![3usize; col_count];
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }

    for (idx, row) in rows.iter().enumerate() {
        out.push_str(&indent);
        out.push('|');
        for (i, cell) in row.iter().enumerate() {
            out.push(' ');
            out.push_str(cell);
            let pad = widths[i].saturating_sub(cell.len());
            out.push_str(&" ".repeat(pad));
            out.push(' ');
            out.push('|');
        }
        out.push('\n');

        if idx == 0 {
            out.push_str(&indent);
            out.push('|');
            for width in &widths {
                out.push(' ');
                out.push_str(&"-".repeat(*width));
                out.push(' ');
                out.push('|');
            }
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
    let mut unsupported: Vec<&'static str> = Vec::new();
    convert_node(root, &mut content, &mut unsupported);

    if !unsupported.is_empty() {
        content.push(json!({
            "type": "blockquote",
            "content": [{
                "type": "paragraph",
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Unsupported Markdown constructs were flattened during conversion: {}",
                        unsupported.join(", ")
                    )
                }]
            }]
        }));
    }

    json!({
        "version": 1,
        "type": "doc",
        "content": content
    })
}

fn convert_node<'a>(
    node: &'a comrak::nodes::AstNode<'a>,
    out: &mut Vec<Value>,
    unsupported: &mut Vec<&'static str>,
) {
    use comrak::nodes::{ListType, NodeValue};

    match &node.data.borrow().value {
        NodeValue::Document => {
            for child in node.children() {
                convert_node(child, out, unsupported);
            }
        }
        NodeValue::Paragraph => {
            let inline_content = collect_inline_children(node, unsupported);
            if !inline_content.is_empty() {
                out.push(json!({
                    "type": "paragraph",
                    "content": inline_content
                }));
            }
        }
        NodeValue::Heading(heading) => {
            let inline_content = collect_inline_children(node, unsupported);
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
                convert_node(child, &mut item_content, unsupported);
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
            for child in node.children() {
                convert_node(child, out, unsupported);
            }
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
                convert_node(child, &mut inner, unsupported);
            }
            out.push(json!({
                "type": "blockquote",
                "content": inner
            }));
        }
        NodeValue::Table(_) => {
            out.push(render_markdown_table(node, unsupported));
        }
        NodeValue::ThematicBreak => {
            out.push(json!({ "type": "rule" }));
        }
        NodeValue::HtmlBlock(html) => {
            note_unsupported(unsupported, "html block");
            let text = html.literal.trim_end_matches('\n');
            if !text.is_empty() {
                out.push(json!({
                    "type": "codeBlock",
                    "attrs": { "language": "html" },
                    "content": [{ "type": "text", "text": text }]
                }));
            }
        }
        NodeValue::DescriptionList
        | NodeValue::DescriptionItem(_)
        | NodeValue::DescriptionTerm
        | NodeValue::DescriptionDetails => {
            note_unsupported(unsupported, "description list");
            for child in node.children() {
                convert_node(child, out, unsupported);
            }
        }
        NodeValue::FootnoteDefinition(_) => {
            note_unsupported(unsupported, "footnote");
            for child in node.children() {
                convert_node(child, out, unsupported);
            }
        }
        NodeValue::LineBreak | NodeValue::SoftBreak => {}
        _ => {
            for child in node.children() {
                convert_node(child, out, unsupported);
            }
        }
    }
}

fn collect_inline_children<'a>(
    node: &'a comrak::nodes::AstNode<'a>,
    unsupported: &mut Vec<&'static str>,
) -> Vec<Value> {
    let mut inline_content: Vec<Value> = Vec::new();
    for child in node.children() {
        collect_inline(child, &mut inline_content, unsupported);
    }
    inline_content
}

fn collect_inline<'a>(
    node: &'a comrak::nodes::AstNode<'a>,
    out: &mut Vec<Value>,
    unsupported: &mut Vec<&'static str>,
) {
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
            apply_mark(node, out, unsupported, json!({ "type": "strong" }));
        }
        NodeValue::Emph => {
            apply_mark(node, out, unsupported, json!({ "type": "em" }));
        }
        NodeValue::Strikethrough => {
            apply_mark(node, out, unsupported, json!({ "type": "strike" }));
        }
        NodeValue::Link(link) => {
            apply_mark(
                node,
                out,
                unsupported,
                json!({
                    "type": "link",
                    "attrs": { "href": link.url.clone() }
                }),
            );
        }
        NodeValue::Image(link) => {
            let mut inner: Vec<Value> = Vec::new();
            for child in node.children() {
                collect_inline(child, &mut inner, unsupported);
            }
            let label = inner
                .iter()
                .filter_map(|item| item.get("text").and_then(|value| value.as_str()))
                .collect::<Vec<_>>()
                .join("");
            let text = if label.trim().is_empty() {
                link.url.clone()
            } else {
                format!("{} ({})", label.trim(), link.url)
            };
            out.push(json!({ "type": "text", "text": text }));
        }
        NodeValue::HtmlInline(html) => {
            note_unsupported(unsupported, "inline html");
            out.push(json!({ "type": "text", "text": html }));
        }
        _ => {
            for child in node.children() {
                collect_inline(child, out, unsupported);
            }
        }
    }
}

fn apply_mark<'a>(
    node: &'a comrak::nodes::AstNode<'a>,
    out: &mut Vec<Value>,
    unsupported: &mut Vec<&'static str>,
    mark: Value,
) {
    let mut inner: Vec<Value> = Vec::new();
    for child in node.children() {
        collect_inline(child, &mut inner, unsupported);
    }
    for mut item in inner {
        let marks = item.get("marks").cloned().unwrap_or_else(|| json!([]));
        let mut marks_arr = marks.as_array().cloned().unwrap_or_default();
        marks_arr.push(mark.clone());
        item["marks"] = json!(marks_arr);
        out.push(item);
    }
}

fn render_markdown_table<'a>(
    table: &'a comrak::nodes::AstNode<'a>,
    unsupported: &mut Vec<&'static str>,
) -> Value {
    let mut rows: Vec<Value> = Vec::new();

    for (row_idx, row) in table.children().enumerate() {
        let mut cells: Vec<Value> = Vec::new();
        for cell in row.children() {
            let mut block_content: Vec<Value> = Vec::new();
            for child in cell.children() {
                convert_node(child, &mut block_content, unsupported);
            }
            if block_content.is_empty() {
                block_content.push(json!({ "type": "paragraph", "content": [] }));
            }
            cells.push(json!({
                "type": if row_idx == 0 { "tableHeader" } else { "tableCell" },
                "content": block_content
            }));
        }
        if !cells.is_empty() {
            rows.push(json!({ "type": "tableRow", "content": cells }));
        }
    }

    json!({ "type": "table", "content": rows })
}

fn note_unsupported(unsupported: &mut Vec<&'static str>, value: &'static str) {
    if !unsupported.contains(&value) {
        unsupported.push(value);
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

    #[test]
    fn test_markdown_to_adf_table_falls_back_without_table_extension() {
        let adf = markdown_to_adf("| Name | Status |\n| --- | --- |\n| API | Done |");
        assert_eq!(adf["content"][0]["type"], "paragraph");
    }

    #[test]
    fn test_markdown_to_adf_list_link_and_code_block() {
        let adf = markdown_to_adf("- [docs](https://example.com)\n\n```rust\nfn main() {}\n```");
        assert_eq!(adf["content"][0]["type"], "bulletList");
        assert_eq!(adf["content"][1]["type"], "codeBlock");
    }

    #[test]
    fn test_markdown_to_adf_unsupported_note() {
        let adf = markdown_to_adf("<div>raw html</div>");
        assert_eq!(adf["content"][0]["type"], "codeBlock");
        assert_eq!(adf["content"][1]["type"], "blockquote");
    }

    #[test]
    fn test_adf_table_to_markdown_table() {
        let adf = json!({
            "type": "table",
            "content": [
                {
                    "type": "tableRow",
                    "content": [
                        {"type": "tableHeader", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Name"}]}]},
                        {"type": "tableHeader", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Status"}]}]}
                    ]
                },
                {
                    "type": "tableRow",
                    "content": [
                        {"type": "tableCell", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "API"}]}]},
                        {"type": "tableCell", "content": [{"type": "paragraph", "content": [{"type": "text", "text": "Done"}]}]}
                    ]
                }
            ]
        });
        let text = adf_to_text(&adf);
        assert!(text.contains("| Name "));
        assert!(text.contains("| API"));
    }
}
