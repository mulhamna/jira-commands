const MAX_DEPTH: usize = 32;

/// Render common Jira wiki markup as readable text. Unknown macros remain visible;
/// rendering never executes macros or fetches external content.
pub fn wiki_to_text(wiki: &str) -> String {
    render_blocks(&wiki.replace("\r\n", "\n").replace('\r', "\n"), 0)
        .trim_end()
        .to_string()
}

fn render_blocks(mut source: &str, depth: usize) -> String {
    if depth >= MAX_DEPTH {
        return source.to_string();
    }
    let mut out = String::new();
    let mut search = BlockSearch::default();
    while let Some((start, open_end, close_start, close_end, kind, options)) =
        next_block(source, &mut search)
    {
        out.push_str(&render_lines(&source[..start], depth));
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        let body = &source[open_end..close_start];
        let body = body.strip_prefix('\n').unwrap_or(body);
        let body = body.strip_suffix('\n').unwrap_or(body);
        if kind == "quote" {
            let quote = render_blocks(body, depth + 1);
            for (index, line) in quote.lines().enumerate() {
                if index > 0 {
                    out.push('\n');
                }
                out.push_str("> ");
                out.push_str(line);
            }
        } else {
            out.push_str("```");
            if kind == "code" {
                let language = options
                    .split('|')
                    .find_map(|option| option.strip_prefix("language="))
                    .or_else(|| {
                        options
                            .split('|')
                            .next()
                            .filter(|option| !option.contains('='))
                    })
                    .unwrap_or("");
                if language
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || "+#-".contains(ch))
                {
                    out.push_str(language);
                }
            }
            out.push('\n');
            out.push_str(body);
            out.push_str("\n```");
        }
        source = &source[close_end..];
        if !source.is_empty() && !source.starts_with('\n') {
            out.push('\n');
        }
    }
    out.push_str(&render_lines(source, depth));
    out
}

type Block<'a> = (usize, usize, usize, usize, &'a str, &'a str);

#[derive(Default)]
struct BlockSearch {
    missing_closers: [bool; 3],
    #[cfg(test)]
    searched_bytes: usize,
}

impl BlockSearch {
    fn find_closing(&mut self, source: &str, kind: &str) -> Option<usize> {
        let (index, closing) = match kind {
            "code" => (0, "{code}"),
            "noformat" => (1, "{noformat}"),
            "quote" => (2, "{quote}"),
            _ => return None,
        };
        if self.missing_closers[index] {
            return None;
        }
        let found = find_unescaped(source, closing);
        #[cfg(test)]
        {
            self.searched_bytes += found.map_or(source.len(), |offset| offset + closing.len());
        }
        // Later suffixes cannot contain a closer that is absent from this suffix.
        self.missing_closers[index] = found.is_none();
        found
    }
}

fn next_block<'a>(source: &'a str, search: &mut BlockSearch) -> Option<Block<'a>> {
    let mut skip_until = 0;
    for (start, _) in source.match_indices('{') {
        if start < skip_until || escaped(source, start) {
            continue;
        }
        if source[start..].starts_with("{{") {
            skip_until = find_unescaped(&source[start + 2..], "}}")
                .map_or(source.len(), |end| start + 2 + end + 2);
            continue;
        }
        let token_start = start + 1;
        let Some(end) = source[token_start..].find(['{', '}']) else {
            break;
        };
        if source.as_bytes()[token_start + end] != b'}' {
            continue;
        }
        let open_end = token_start + end + 1;
        let token = &source[start + 1..open_end - 1];
        let (kind, options) = token.split_once(':').unwrap_or((token, ""));
        if !matches!(kind, "code" | "noformat" | "quote")
            || source[..start].ends_with('{')
            || source[open_end..].starts_with('}')
        {
            continue;
        }
        if let Some(offset) = search.find_closing(&source[open_end..], kind) {
            let close_start = open_end + offset;
            return Some((
                start,
                open_end,
                close_start,
                close_start + kind.len() + 2,
                kind,
                options,
            ));
        }
    }
    None
}

fn render_lines(source: &str, depth: usize) -> String {
    let mut out = String::new();
    let mut lists: Vec<(char, usize)> = Vec::new();
    let mut in_table = false;
    for line in source.split_inclusive('\n') {
        let text = line.strip_suffix('\n').unwrap_or(line);
        if let Some(cells) = table_cells(text) {
            let cells: Vec<String> = cells
                .iter()
                .map(|cell| render_inline(cell.trim(), depth).replace('\n', " <br> "))
                .collect();
            out.push_str(&format!("| {} |", cells.join(" | ")));
            if !in_table {
                out.push_str(&format!("\n| {} |", vec!["---"; cells.len()].join(" | ")));
            }
            in_table = true;
            lists.clear();
        } else {
            in_table = false;
            let text = text.trim_start();
            let markers = text
                .bytes()
                .take_while(|ch| matches!(ch, b'*' | b'#'))
                .count();
            if markers > 0 && text[markers..].starts_with(char::is_whitespace) {
                let prefix = &text[..markers];
                let common = prefix
                    .chars()
                    .zip(&lists)
                    .take_while(|(a, (b, _))| a == b)
                    .count();
                lists.truncate(common);
                for marker in prefix.chars().skip(common) {
                    lists.push((marker, 0));
                }
                let (marker, count) = lists.last_mut().expect("nonempty list prefix");
                *count += 1;
                out.push_str(&"  ".repeat(markers - 1));
                if *marker == '#' {
                    out.push_str(&format!("{count}. "));
                } else {
                    out.push_str("- ");
                }
                out.push_str(&render_inline(text[markers..].trim_start(), depth));
            } else {
                lists.clear();
                if text.len() >= 4
                    && text.starts_with('h')
                    && matches!(text.as_bytes()[1], b'1'..=b'6')
                    && text[2..].starts_with(". ")
                {
                    out.push_str(&"#".repeat((text.as_bytes()[1] - b'0') as usize));
                    out.push(' ');
                    out.push_str(&render_inline(&text[4..], depth));
                } else if let Some(quote) = text.strip_prefix("bq. ") {
                    out.push_str("> ");
                    out.push_str(&render_inline(quote, depth));
                } else if text == "----" {
                    out.push_str("---");
                } else {
                    // Keep indentation for plain text and unsupported constructs.
                    out.push_str(&render_inline(
                        line.strip_suffix('\n').unwrap_or(line),
                        depth,
                    ));
                }
            }
        }
        let hard_break = text.ends_with("\\\\") && !escaped(text, text.len() - 2);
        if line.ends_with('\n') && !hard_break {
            out.push('\n');
        }
    }
    out
}

fn table_cells(line: &str) -> Option<Vec<&str>> {
    let line = line.trim();
    let inner = if line.starts_with("||") && line.ends_with("||") && line.len() >= 4 {
        &line[2..line.len() - 2]
    } else {
        line.strip_prefix('|')?.strip_suffix('|')?
    };
    let mut cells = Vec::new();
    let mut start = 0;
    let mut brackets = false;
    let mut image = false;
    let mut monospace = false;
    let mut chars = inner.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if escaped(inner, index) {
            continue;
        }
        match ch {
            '{' if inner[index..].starts_with("{{") => {
                monospace = true;
                chars.next();
            }
            '}' if inner[index..].starts_with("}}") => {
                monospace = false;
                chars.next();
            }
            '[' if !monospace => brackets = true,
            ']' if !monospace => brackets = false,
            '!' if !monospace && !brackets => image = !image,
            '|' if !monospace && !brackets && !image => {
                cells.push(&inner[start..index]);
                start = index + 1;
                if chars.peek().is_some_and(|(_, ch)| *ch == '|') {
                    chars.next();
                    start += 1;
                }
            }
            _ => {}
        }
    }
    cells.push(&inner[start..]);
    Some(cells)
}

fn render_inline(source: &str, depth: usize) -> String {
    if depth >= MAX_DEPTH {
        return source.to_string();
    }
    let mut out = String::new();
    let mut rest = source;
    while !rest.is_empty() {
        if let Some(escaped) = rest.strip_prefix('\\') {
            if let Some(next) = escaped.chars().next() {
                if next == '\\' {
                    out.push('\n');
                    rest = escaped[1..].strip_prefix('\n').unwrap_or(&escaped[1..]);
                    continue;
                }
                if "*_+^~[]!|#-{}.".contains(next)
                    || ["h1.", "h2.", "h3.", "h4.", "h5.", "h6.", "bq."]
                        .iter()
                        .any(|prefix| escaped.starts_with(prefix))
                {
                    out.push(next);
                    rest = &escaped[next.len_utf8()..];
                    continue;
                }
            }
        }
        if rest.starts_with('&') {
            if let Some((entity, len)) = entity(rest) {
                out.push(entity);
                rest = &rest[len..];
                continue;
            }
        }
        if let Some(code) = rest.strip_prefix("{{") {
            if let Some(end) = find_unescaped(code, "}}") {
                out.push_str(&literal(&code[..end]));
                rest = &code[end + 2..];
                continue;
            }
        }
        if let Some(link) = rest.strip_prefix('[') {
            if let Some(end) = find_unescaped(link, "]") {
                let target = &link[..end];
                if let Some(user) = target.strip_prefix('~') {
                    out.push('@');
                    out.push_str(user.strip_prefix("accountid:").unwrap_or(user));
                } else {
                    let label =
                        find_unescaped(target, "|").map_or(target, |index| &target[..index]);
                    out.push_str(&render_inline(
                        label.strip_prefix('^').unwrap_or(label),
                        depth + 1,
                    ));
                }
                rest = &link[end + 1..];
                continue;
            }
        }
        if let Some(image) = rest.strip_prefix('!') {
            if let Some(end) = find_unescaped(image, "!") {
                let name = image[..end].split('|').next().unwrap_or("");
                if !name.is_empty() && !name.chars().any(char::is_whitespace) {
                    out.push_str(&format!("[image: {}]", literal(name)));
                    rest = &image[end + 1..];
                    continue;
                }
            }
        }
        if rest.starts_with("{color:") {
            if let Some(open_end) = rest.find('}') {
                if let Some(close) = rest[open_end + 1..].find("{color}") {
                    let close = open_end + 1 + close;
                    out.push_str(&render_inline(&rest[open_end + 1..close], depth + 1));
                    rest = &rest[close + "{color}".len()..];
                    continue;
                }
            }
        }
        let ch = rest.chars().next().expect("nonempty input");
        if "*_+-^~".contains(ch) {
            let body = &rest[ch.len_utf8()..];
            let before = source[..source.len() - rest.len()].chars().next_back();
            if !before.is_some_and(char::is_alphanumeric) && !body.starts_with(char::is_whitespace)
            {
                if let Some(end) = find_unescaped(body, &ch.to_string()) {
                    let after = &body[end + ch.len_utf8()..];
                    if end > 0
                        && !body[..end].ends_with(char::is_whitespace)
                        && !after.starts_with(char::is_alphanumeric)
                    {
                        out.push_str(&render_inline(&body[..end], depth + 1));
                        rest = after;
                        continue;
                    }
                }
            }
        }
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

fn literal(source: &str) -> String {
    let mut out = String::new();
    let mut rest = source;
    while !rest.is_empty() {
        if rest.starts_with('&') {
            if let Some((ch, len)) = entity(rest) {
                out.push(ch);
                rest = &rest[len..];
                continue;
            }
        }
        if let Some(tail) = rest.strip_prefix('\\') {
            if let Some(ch) = tail
                .chars()
                .next()
                .filter(|ch| "*_+^~[]!|#-{}\\".contains(*ch))
            {
                out.push(ch);
                rest = &tail[ch.len_utf8()..];
                continue;
            }
        }
        let ch = rest.chars().next().expect("nonempty input");
        out.push(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

fn entity(source: &str) -> Option<(char, usize)> {
    let end = source.bytes().take(16).position(|ch| ch == b';')?;
    let name = &source[1..end];
    let ch = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" | "#39" => '\'',
        "nbsp" => ' ',
        _ => {
            let code =
                if let Some(hex) = name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                    u32::from_str_radix(hex, 16).ok()?
                } else {
                    name.strip_prefix('#')?.parse().ok()?
                };
            char::from_u32(code)?
        }
    };
    Some((ch, end + 1))
}

fn escaped(source: &str, index: usize) -> bool {
    source[..index]
        .bytes()
        .rev()
        .take_while(|ch| *ch == b'\\')
        .count()
        % 2
        == 1
}

fn find_unescaped(source: &str, needle: &str) -> Option<usize> {
    source
        .match_indices(needle)
        .map(|(index, _)| index)
        .find(|index| !escaped(source, *index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_lists_quotes_and_rules() {
        let wiki = "h2. Plan\n* First\n** Nested\n*# Numbered\n*# Second\n* Next\n\n# One\n# Two\nbq. Quoted *text*\n----";
        assert_eq!(wiki_to_text(wiki), "## Plan\n- First\n  - Nested\n  1. Numbered\n  2. Second\n- Next\n\n1. One\n2. Two\n> Quoted text\n---");
    }

    #[test]
    fn inline_styles_links_mentions_and_images() {
        assert_eq!(
            wiki_to_text("*Bold* _italic_ +underline+ -strike- ^up^ ~down~ {{a*b}} {color:red}red{color}\n[Docs|https://example.com] [https://example.com] [^report.pdf] [~alice] [~accountid:123]\n!shot.png|thumbnail!"),
            "Bold italic underline strike up down a*b red\nDocs https://example.com report.pdf @alice @123\n[image: shot.png]"
        );
    }

    #[test]
    fn preserves_literal_punctuation_and_decodes_entities_once() {
        assert_eq!(
            wiki_to_text(
                r"\*literal\* foo_bar_baz C:\Users\me C:\home &#123;code&#125; &amp;lt; &#x1F600; &unknown;"
            ),
            "*literal* foo_bar_baz C:\\Users\\me C:\\home {code} &lt; \u{1f600} &unknown;"
        );
        assert_eq!(wiki_to_text(r"\h2. Not a heading"), "h2. Not a heading");
        assert_eq!(wiki_to_text("one\\\\\ntwo"), "one\ntwo");
    }

    #[test]
    fn code_and_noformat_do_not_interpret_their_content() {
        let wiki = "{code:java|title=Example}\n*literal* [~alice] &amp;\n\n{code}\n{noformat}h2. raw{noformat}";
        assert_eq!(
            wiki_to_text(wiki),
            "```java\n*literal* [~alice] &amp;\n\n```\n```\nh2. raw\n```"
        );
        assert_eq!(
            wiki_to_text("{code:title=Example|language=java}*raw*{code}"),
            "```java\n*raw*\n```"
        );
        assert_eq!(
            wiki_to_text("{{{code}}} literal {{{code}}}"),
            "{code} literal {code}"
        );
        assert_eq!(
            wiki_to_text("{{before {code} after}} and {{{code}}}"),
            "before {code} after and {code}"
        );
    }

    #[test]
    fn quote_macros_render_their_contents() {
        assert_eq!(
            wiki_to_text("{quote}\nh2. Quote\n* item\n{quote}"),
            "> ## Quote\n> - item"
        );
    }

    #[test]
    fn tables_keep_inline_pipes_inside_cells() {
        assert_eq!(
            wiki_to_text("||Name||Value||\n|*Link*|[A|https://a.example]|\n|Escaped|a\\|b|\n|Code|{{x|y}}|\n|Image|!shot.png|thumbnail!|"),
            "| Name | Value |\n| --- | --- |\n| Link | A |\n| Escaped | a|b |\n| Code | x|y |\n| Image | [image: shot.png] |"
        );
    }

    #[test]
    fn unknown_and_unclosed_constructs_remain_visible() {
        for wiki in [
            "{custom:option=value}body{custom}",
            "{code}unfinished",
            "[unfinished",
            "*unfinished",
            "plain \u{e9} text",
        ] {
            assert_eq!(wiki_to_text(wiki), wiki);
        }
    }

    #[test]
    fn repeated_unmatched_macros_only_scan_the_suffix_once() {
        for opener in ["{code:java}", "{noformat:unused}", "{quote:unused}"] {
            let wiki = opener.repeat(1_000);
            let mut search = BlockSearch::default();

            assert!(next_block(&wiki, &mut search).is_none());
            assert!(search.searched_bytes <= wiki.len());
            assert_eq!(wiki_to_text(&wiki), wiki);
        }
    }

    #[test]
    fn unmatched_macro_searches_stay_cached_across_completed_blocks() {
        let wiki = "{code:java}\n{noformat:unused}\n{quote}text{quote}\n".repeat(1_000);
        let mut source = wiki.as_str();
        let mut search = BlockSearch::default();
        let mut blocks = 0;

        while let Some((_, _, _, close_end, kind, _)) = next_block(source, &mut search) {
            assert_eq!(kind, "quote");
            blocks += 1;
            source = &source[close_end..];
        }

        assert_eq!(blocks, 1_000);
        assert!(search.searched_bytes <= 3 * wiki.len());
        assert_eq!(
            wiki_to_text(&wiki),
            "{code:java}\n{noformat:unused}\n> text\n"
                .repeat(1_000)
                .trim_end()
        );
    }

    #[test]
    fn deeply_nested_formatting_keeps_the_content() {
        let wiki = format!("{}text{}", "{color:red}".repeat(64), "{color}".repeat(64));
        assert!(wiki_to_text(&wiki).contains("text"));
    }
}
