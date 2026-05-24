fn group_body_contains_component_family(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> bool {
    let mut idx = start + 1;
    while idx < end_idx {
        let line = strip_inline_plantuml_comment(lines[idx].0).trim();
        let lower = line.to_ascii_lowercase();
        if (lower.starts_with("package ")
            || lower.starts_with("namespace ")
            || lower.starts_with("rectangle "))
            && line.trim_end().ends_with('{')
        {
            let nested_end = find_scoping_block_end(lines, idx).min(end_idx);
            if nested_end > idx && group_body_contains_component_family(lines, idx, nested_end) {
                return true;
            }
            idx = nested_end.saturating_add(1);
            continue;
        }
        if component_decl_keyword(line).is_some() {
            return true;
        }
        idx += 1;
    }
    false
}

fn group_body_contains_class_family(lines: &[(&str, Span)], start: usize, end_idx: usize) -> bool {
    let mut idx = start + 1;
    while idx < end_idx {
        let line = strip_inline_plantuml_comment(lines[idx].0).trim();
        let lower = line.to_ascii_lowercase();
        if (lower.starts_with("package ")
            || lower.starts_with("namespace ")
            || lower.starts_with("rectangle "))
            && line.trim_end().ends_with('{')
        {
            let nested_end = find_scoping_block_end(lines, idx).min(end_idx);
            if nested_end > idx && group_body_contains_class_family(lines, idx, nested_end) {
                return true;
            }
            idx = nested_end.saturating_add(1);
            continue;
        }
        if lower.starts_with("abstract class ")
            || lower.starts_with("annotation ")
            || lower.starts_with("interface ")
            || lower.starts_with("abstract ")
            || lower.starts_with("enum ")
            || lower.starts_with("exception ")
            || lower.starts_with("metaclass ")
            || lower.starts_with("stereotype ")
            || lower.starts_with("circle ")
            || lower.starts_with("diamond ")
            || lower.starts_with("protocol ")
            || lower.starts_with("struct ")
            || lower.starts_with("class ")
            || (lower.starts_with("entity ") && lower.ends_with('{'))
        {
            return true;
        }
        idx += 1;
    }
    false
}

fn group_body_contains_object_family(lines: &[(&str, Span)], start: usize, end_idx: usize) -> bool {
    let mut idx = start + 1;
    while idx < end_idx {
        let line = strip_inline_plantuml_comment(lines[idx].0).trim();
        let lower = line.to_ascii_lowercase();
        if (lower.starts_with("package ")
            || lower.starts_with("namespace ")
            || lower.starts_with("rectangle "))
            && line.trim_end().ends_with('{')
        {
            let nested_end = find_scoping_block_end(lines, idx).min(end_idx);
            if nested_end > idx && group_body_contains_object_family(lines, idx, nested_end) {
                return true;
            }
            idx = nested_end.saturating_add(1);
            continue;
        }
        if lower.starts_with("object ") || lower.starts_with("map ") || lower.starts_with("diamond ")
        {
            return true;
        }
        idx += 1;
    }
    false
}

fn group_body_contains_usecase_family(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> bool {
    let mut idx = start + 1;
    while idx < end_idx {
        let line = strip_inline_plantuml_comment(lines[idx].0).trim();
        let lower = line.to_ascii_lowercase();
        if (lower.starts_with("package ")
            || lower.starts_with("namespace ")
            || lower.starts_with("rectangle "))
            && line.trim_end().ends_with('{')
        {
            let nested_end = find_scoping_block_end(lines, idx).min(end_idx);
            if nested_end > idx && group_body_contains_usecase_family(lines, idx, nested_end) {
                return true;
            }
            idx = nested_end.saturating_add(1);
            continue;
        }
        if lower.starts_with("usecase ")
            || lower.starts_with("usecase(")
            || lower.starts_with("actor ")
            || parse_parenthesized_usecase_decl(line).is_some()
        {
            return true;
        }
        idx += 1;
    }
    false
}

fn scoped_family_kind_for_block(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> DiagramKind {
    if group_body_contains_object_family(lines, start, end_idx) {
        DiagramKind::Object
    } else if group_body_contains_usecase_family(lines, start, end_idx) {
        DiagramKind::UseCase
    } else {
        DiagramKind::Class
    }
}
