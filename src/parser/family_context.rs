fn later_lines_contain_class_family_declaration(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("abstract class ")
            || line.starts_with("abstract ")
            || line.starts_with("annotation ")
            || line.starts_with("circle ")
            || line.starts_with("class ")
            || line.starts_with("diamond ")
            || line.starts_with("enum ")
            || line.starts_with("exception ")
            || line.starts_with("metaclass ")
            || line.starts_with("protocol ")
            || line.starts_with("stereotype ")
            || line.starts_with("struct ")
            || (line.starts_with("entity ") && line.ends_with('{'))
    })
}

fn later_lines_contain_ie_family_context(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("entity ") && line.ends_with('{') || line_contains_ie_relation_token(line)
    })
}

fn line_contains_ie_relation_token(line: &str) -> bool {
    [
        "||--", "||..", "|o--", "|o..", "}o--", "}o..", "}|--", "}|..", "--||", "..||", "--o|",
        "..o|", "--o{", "..o{", "--|{", "..|{",
    ]
    .iter()
    .any(|token| line.contains(token))
}

fn later_lines_contain_usecase_family_declaration(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("usecase ")
            || line.starts_with("usecase(")
            || line.starts_with('(')
            || line.starts_with("actor ")
    })
}

/// Returns `true` if any subsequent line is an unambiguous sequence-diagram keyword.
/// Used to suppress the component-family heuristic when `actor` appears in a context
/// that is clearly a sequence diagram (fixes #776).
fn later_lines_contain_sequence_family_keywords(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        let lower = line.to_ascii_lowercase();
        // Sequence arrows: ->, ->>, -->, -->>, <-, <<-, <--, etc.
        let has_sequence_arrow = line.contains("->") || line.contains("<-");
        // Unambiguous sequence keywords (not shared with component/class)
        let is_sequence_keyword = lower.starts_with("activate ")
            || lower == "activate"
            || lower.starts_with("deactivate ")
            || lower == "deactivate"
            || lower.starts_with("destroy ")
            || lower == "destroy"
            || lower.starts_with("autonumber")
            || lower.starts_with("participant ")
            || lower.starts_with("boundary ")
            || lower.starts_with("control ")
            || lower.starts_with("entity ")
            || lower.starts_with("collections ")
            || lower.starts_with("queue ")
            || lower.starts_with("alt ")
            || lower == "alt"
            || lower.starts_with("opt ")
            || lower == "opt"
            || lower.starts_with("loop ")
            || lower == "loop"
            || lower.starts_with("par ")
            || lower == "par"
            || lower.starts_with("also ")
            || lower == "also"
            || lower.starts_with("critical ")
            || lower == "critical"
            || lower.starts_with("ref over ")
            || lower.starts_with("ref over\t")
            || (lower.starts_with("==") && lower.ends_with("==") && lower.len() >= 4);
        has_sequence_arrow || is_sequence_keyword
    })
}

fn later_lines_contain_activity_context(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        looks_like_old_activity_flow(line) || parse_activity_step(line).is_some()
    })
}
