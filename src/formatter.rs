#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatResult {
    pub formatted: String,
    pub changed: bool,
}

pub fn format_source(source: &str) -> FormatResult {
    let formatted = format_source_inner(source);
    FormatResult {
        changed: formatted != source,
        formatted,
    }
}

fn format_source_inner(source: &str) -> String {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let had_final_newline = normalized.ends_with('\n');
    let mut lines = normalized.split('\n').collect::<Vec<_>>();
    if had_final_newline {
        lines.pop();
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut indent = 0usize;
    let mut verbatim_until: Option<&'static str> = None;

    for raw_line in lines {
        let trailing_trimmed = trim_trailing_whitespace(raw_line);
        let trimmed = trailing_trimmed.trim_start_matches([' ', '\t']);

        if trimmed.is_empty() {
            out.push(String::new());
            continue;
        }

        if let Some(end_marker) = verbatim_until {
            if is_verbatim_end(trimmed, end_marker) {
                indent = indent.saturating_sub(1);
                let line = format!(
                    "{}{}",
                    " ".repeat(indent * 2),
                    canonicalize_code_arrows(trimmed)
                );
                out.push(line);
                verbatim_until = None;
            } else {
                out.push(trailing_trimmed.to_string());
            }
            continue;
        }

        let kind = classify_line(trimmed);
        if kind.pre_dedent {
            indent = indent.saturating_sub(1);
        }
        if kind.force_zero_indent {
            indent = 0;
        }

        let formatted_line = format!(
            "{}{}",
            " ".repeat(indent * 2),
            canonicalize_code_arrows(trimmed)
        );
        out.push(formatted_line);

        if let Some(end_marker) = starts_verbatim_block(trimmed) {
            indent += 1;
            verbatim_until = Some(end_marker);
            continue;
        }
        if kind.post_indent {
            indent += 1;
        }
    }

    let mut formatted = out.join("\n");
    if had_final_newline {
        formatted.push('\n');
    }
    formatted
}

fn trim_trailing_whitespace(line: &str) -> &str {
    line.trim_end_matches([' ', '\t'])
}

#[derive(Debug, Clone, Copy, Default)]
struct LineKind {
    pre_dedent: bool,
    post_indent: bool,
    force_zero_indent: bool,
}

fn classify_line(line: &str) -> LineKind {
    let code = code_before_comment(line).trim();
    let lower = code.to_ascii_lowercase();

    if lower.starts_with("@end") {
        return LineKind {
            force_zero_indent: true,
            ..LineKind::default()
        };
    }

    if lower == "}" || lower.starts_with("} ") {
        return LineKind {
            pre_dedent: true,
            ..LineKind::default()
        };
    }

    if is_branch(&lower) {
        return LineKind {
            pre_dedent: true,
            post_indent: true,
            ..LineKind::default()
        };
    }

    if is_closer(&lower) {
        return LineKind {
            pre_dedent: true,
            ..LineKind::default()
        };
    }

    if is_opener(&lower) || lower.ends_with('{') {
        return LineKind {
            post_indent: true,
            ..LineKind::default()
        };
    }

    LineKind::default()
}

fn is_opener(lower: &str) -> bool {
    starts_with_keyword(
        lower,
        &[
            "group",
            "alt",
            "opt",
            "loop",
            "par",
            "break",
            "critical",
            "box",
            "ref",
            "if",
            "!if",
            "while",
            "!while",
            "fork",
            "split",
            "partition",
            "repeat",
        ],
    )
}

fn is_closer(lower: &str) -> bool {
    lower == "end"
        || starts_with_keyword(
            lower,
            &[
                "end group",
                "end box",
                "end ref",
                "endif",
                "!endif",
                "endwhile",
                "!endwhile",
                "end fork",
                "end split",
                "end partition",
                "repeat while",
            ],
        )
}

fn is_branch(lower: &str) -> bool {
    starts_with_keyword(
        lower,
        &[
            "else",
            "elseif",
            "else if",
            "!else",
            "!elseif",
            "also",
            "and",
            "fork again",
            "split again",
        ],
    )
}

fn starts_with_keyword(line: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| {
        line == *keyword
            || line
                .strip_prefix(keyword)
                .map(|rest| rest.starts_with(char::is_whitespace) || rest.starts_with('('))
                .unwrap_or(false)
    })
}

fn starts_verbatim_block(line: &str) -> Option<&'static str> {
    let code = code_before_comment(line).trim();
    let lower = code.to_ascii_lowercase();
    if has_colon_outside_quotes(code) {
        return None;
    }

    if starts_with_keyword(&lower, &["note", "hnote", "rnote"]) {
        return Some("end note");
    }
    if starts_with_keyword(&lower, &["legend"]) {
        return Some("end legend");
    }
    if starts_with_keyword(&lower, &["title"]) {
        return Some("end title");
    }
    if starts_with_keyword(&lower, &["caption"]) {
        return Some("end caption");
    }
    if starts_with_keyword(
        &lower,
        &[
            "header",
            "footer",
            "center header",
            "center footer",
            "left header",
            "left footer",
            "right header",
            "right footer",
        ],
    ) {
        return if lower.contains("header") {
            Some("end header")
        } else {
            Some("end footer")
        };
    }
    None
}

fn is_verbatim_end(line: &str, end_marker: &str) -> bool {
    code_before_comment(line)
        .trim()
        .eq_ignore_ascii_case(end_marker)
}

fn canonicalize_code_arrows(line: &str) -> String {
    let code_end = code_segment_end(line);
    let code = &line[..code_end];
    let suffix = &line[code_end..];
    let mut out = String::with_capacity(line.len());
    let mut in_quotes = false;

    for ch in code.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            out.push(ch);
            continue;
        }
        if in_quotes {
            out.push(ch);
            continue;
        }
        match ch {
            '→' | '⟶' | '➡' => out.push_str("->"),
            '←' | '⟵' | '⬅' => out.push_str("<-"),
            '⇒' | '⟹' => out.push_str("-->"),
            '⇐' | '⟸' => out.push_str("<--"),
            '↔' | '⟷' => out.push_str("<->"),
            '⇔' | '⟺' => out.push_str("<-->"),
            _ => out.push(ch),
        }
    }
    out.push_str(suffix);
    out
}

fn code_segment_end(line: &str) -> usize {
    let mut in_quotes = false;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if !in_quotes && (ch == '\'' || ch == ':') {
            return idx;
        }
    }
    line.len()
}

fn code_before_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == '\'' && !in_quotes {
            return &line[..idx];
        }
    }
    line
}

fn has_colon_outside_quotes(line: &str) -> bool {
    let mut in_quotes = false;
    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == ':' && !in_quotes {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::format_source;

    #[test]
    fn formatter_is_idempotent_for_nested_sequence_blocks() {
        let input = "@startuml\r\nAlice → Bob: keep label → intact   \r\nalt ok\r\nBob ← Alice: yep\r\nelse nope\r\nloop retry\r\nAlice ⇒ Bob: again\r\nend\r\nend\r\n@enduml\r\n";
        let expected = "@startuml\nAlice -> Bob: keep label → intact\nalt ok\n  Bob <- Alice: yep\nelse nope\n  loop retry\n    Alice --> Bob: again\n  end\nend\n@enduml\n";

        let once = format_source(input);
        assert_eq!(once.formatted, expected);
        assert!(once.changed);

        let twice = format_source(&once.formatted);
        assert_eq!(twice.formatted, expected);
        assert!(!twice.changed);
    }

    #[test]
    fn formatter_preserves_multiline_note_body_indentation_and_arrows() {
        let input = "@startuml\nnote right\n  A → B stays prose  \nend note\nAlice → Bob: message\n@enduml\n";
        let expected =
            "@startuml\nnote right\n  A → B stays prose\nend note\nAlice -> Bob: message\n@enduml\n";

        assert_eq!(format_source(input).formatted, expected);
    }
}
