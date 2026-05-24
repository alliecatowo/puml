use super::*;

pub(super) fn normalize_family_tree_warnings(warnings: &mut [Diagnostic]) {
    warnings.sort_by(|a, b| {
        let sa = a.span.map(|s| s.start).unwrap_or_default();
        let sb = b.span.map(|s| s.start).unwrap_or_default();
        (a.message.as_str(), sa).cmp(&(b.message.as_str(), sb))
    });
}

pub(super) struct MindmapMultilineDraft {
    pub(super) kind: FamilyNodeKind,
    pub(super) depth: usize,
    pub(super) name: String,
    pub(super) alias: Option<String>,
    pub(super) side: MindMapSide,
    pub(super) checkbox: Option<WbsCheckbox>,
    pub(super) fill_color: Option<String>,
}

impl MindmapMultilineDraft {
    /// Append `line` to the in-progress multiline body. Returns `Some(node)` when the
    /// block ends on a line containing `;` (PlantUML ch17.4 / ch18.4).
    pub(super) fn append_line(&mut self, line: &str) -> Option<FamilyNode> {
        let trimmed_end = line.trim_end();
        if trimmed_end.ends_with(';') {
            let tail = trimmed_end.trim_end_matches(';').trim_end();
            if !tail.is_empty() {
                if !self.name.is_empty() {
                    self.name.push('\n');
                }
                self.name.push_str(tail);
            }
            return Some(FamilyNode {
                kind: self.kind,
                name: self.name.clone(),
                alias: self.alias.clone(),
                members: Vec::new(),
                depth: self.depth,
                label: None,
                mindmap_side: self.side,
                wbs_checkbox: self.checkbox.clone(),
                fill_color: self.fill_color.clone(),
            });
        }
        let piece = line.trim();
        if !piece.is_empty() {
            if !self.name.is_empty() {
                self.name.push('\n');
            }
            self.name.push_str(piece);
        }
        None
    }
}

pub(super) fn handle_mindmap_maximum_width_skinparam(
    key: &str,
    value: &str,
    maximum_width: &mut Option<i32>,
    warnings: &mut Vec<Diagnostic>,
    span: crate::source::Span,
) -> bool {
    if !key.trim().eq_ignore_ascii_case("maximumwidth") {
        return false;
    }
    match value.trim().parse::<i32>() {
        Ok(n) if n > 0 => *maximum_width = Some(n),
        _ => warnings.push(
            Diagnostic::warning(format!(
                "[W_SKINPARAM_UNSUPPORTED_VALUE] unsupported value `{}` for skinparam `{}`",
                value, key
            ))
            .with_span(span),
        ),
    }
    true
}

pub(super) fn collect_mindmap_style_line(
    line: &str,
    block: &mut Option<String>,
    style: &mut MindMapStyle,
) -> bool {
    let lower = line.trim_start().to_ascii_lowercase();
    if let Some(source) = block {
        if let Some((before_end, _)) = split_style_end(line) {
            source.push('\n');
            source.push_str(before_end);
            parse_mindmap_style_source(source, style);
            *block = None;
        } else {
            source.push('\n');
            source.push_str(line);
        }
        return true;
    }

    if !lower.starts_with("<style") {
        return false;
    }

    let after_start = line
        .split_once('>')
        .map(|(_, after)| after)
        .unwrap_or_default();
    if let Some((before_end, _)) = split_style_end(after_start) {
        parse_mindmap_style_source(before_end, style);
    } else {
        *block = Some(after_start.to_string());
    }
    true
}

pub(super) fn split_style_end(line: &str) -> Option<(&str, &str)> {
    let lower = line.to_ascii_lowercase();
    lower.find("</style>").map(|idx| {
        let end = idx + "</style>".len();
        (&line[..idx], &line[end..])
    })
}

pub(super) fn parse_mindmap_style_source(source: &str, style: &mut MindMapStyle) {
    let prepared = source.replace('{', "\n{\n").replace('}', "\n}\n");
    let mut stack: Vec<String> = Vec::new();
    let mut pending_selector: Option<String> = None;
    for raw in prepared.lines() {
        let line = raw.trim().trim_end_matches(';').trim();
        if line.is_empty() {
            continue;
        }
        if line == "{" {
            if let Some(selector) = pending_selector.take() {
                stack.push(selector);
            }
            continue;
        }
        if line == "}" {
            stack.pop();
            continue;
        }
        if apply_mindmap_style_property(line, &stack, style) {
            continue;
        }
        pending_selector = Some(line.to_string());
    }
}

pub(super) fn apply_mindmap_style_property(
    line: &str,
    stack: &[String],
    style: &mut MindMapStyle,
) -> bool {
    let mut parts = line.splitn(2, char::is_whitespace);
    let Some(raw_key) = parts.next() else {
        return false;
    };
    let key = raw_key.trim_end_matches(':').to_ascii_lowercase();
    if !matches!(key.as_str(), "backgroundcolor" | "fontcolor" | "linecolor") {
        return false;
    }
    let value = parts
        .next()
        .unwrap_or_default()
        .trim()
        .trim_start_matches(':')
        .trim()
        .trim_end_matches(';')
        .trim();
    if value.is_empty() {
        return true;
    }

    let in_mindmap = stack
        .iter()
        .any(|selector| selector.eq_ignore_ascii_case("mindmapDiagram"));
    let depth = stack.iter().rev().find_map(|selector| {
        let selector = selector.trim();
        let inner = selector.strip_prefix(":depth(")?.strip_suffix(')')?;
        inner.trim().parse::<usize>().ok()
    });
    if in_mindmap {
        if let Some(depth) = depth {
            apply_mindmap_depth_property(style.depth_styles.entry(depth).or_default(), &key, value);
        }
    }
    true
}

pub(super) fn apply_mindmap_depth_property(patch: &mut MindMapDepthStyle, key: &str, value: &str) {
    match key {
        "backgroundcolor" => patch.background_color = Some(value.to_string()),
        "fontcolor" => patch.font_color = Some(value.to_string()),
        "linecolor" => patch.border_color = Some(value.to_string()),
        _ => {}
    }
}

pub(super) struct MindMapWbsNode {
    pub(super) depth: usize,
    pub(super) name: String,
    pub(super) alias: Option<String>,
    pub(super) side: MindMapSide,
    pub(super) checkbox: Option<WbsCheckbox>,
    pub(super) fill_color: Option<String>,
}

/// Parse a MindMap / WBS node line. Handles:
///
/// - `* Root`, `** Child`, `*** Grandchild` — star-depth (depth = stars - 1)
/// - `*[#Orange] Root`, `**[#fef3c7] Child` — PlantUML-style node color tags
/// - `** Left child` after a `left side` keyword (tracked externally)
/// - `+** Right`, `-** Left` — explicit side prefix on first depth-2+ star
/// - WBS annotations: `[x]` checked, `[ ]` unchecked, `[%NN]` progress
pub(super) fn parse_mindmap_or_wbs_node(line: &str) -> Option<MindMapWbsNode> {
    let trimmed = line.trim_start();

    // Detect optional side prefix: `+` = right, `-` = left (only matters at
    // depth >= 1 in MindMap, but we parse it universally and let the renderer
    // decide what to do with it).
    let (side_prefix, rest) = if let Some(s) = trimmed.strip_prefix('+') {
        (Some(MindMapSide::Right), s)
    } else if let Some(s) = trimmed.strip_prefix('-') {
        (Some(MindMapSide::Left), s)
    } else {
        (None, trimmed)
    };

    let star_prefix = rest.bytes().take_while(|c| *c == b'*').count();
    if star_prefix == 0 {
        return None;
    }

    let mut label = rest[star_prefix..].trim().to_string();
    let alias = parse_mindmap_wbs_alias(&mut label);
    let fill_color = parse_mindmap_wbs_color_tag(&mut label);
    if label.is_empty() {
        return None;
    }
    // PlantUML interprets `\n` in label text as a line break (#560).
    // Convert the literal backslash-n sequence to an actual newline so the
    // renderer's multi-line text emission path can wrap it.
    label = label.replace("\\n", "\n");

    // Parse WBS checkbox suffix: `[x]`, `[ ]`, `[%NN]` at end of label.
    let checkbox = parse_wbs_checkbox(&mut label);

    // Side defaults to Right unless explicitly prefixed.
    let side = side_prefix.unwrap_or(MindMapSide::Right);
    let depth = star_prefix.saturating_sub(1);

    Some(MindMapWbsNode {
        depth,
        name: label,
        alias,
        side,
        checkbox,
        fill_color,
    })
}

pub(super) fn parse_mindmap_wbs_alias(label: &mut String) -> Option<String> {
    let trimmed = label.trim_start();
    if !trimmed.starts_with('(') {
        return None;
    }
    let close = trimmed.find(')')?;
    if close <= 1 {
        return None;
    }
    let alias = trimmed[1..close].trim().to_string();
    if alias.is_empty() {
        return None;
    }
    let remainder = &trimmed[close + 1..];
    if !remainder.is_empty() && !remainder.starts_with(char::is_whitespace) {
        return None;
    }
    *label = remainder.trim_start().to_string();
    Some(alias)
}

/// Parse a leading PlantUML color tag from MindMap/WBS labels.
///
/// PlantUML examples use tags such as `[#Orange]` and `[#lightgreen]`; SVG
/// accepts named colors without the leading `#`, while hex colors keep it.
pub(super) fn parse_mindmap_wbs_color_tag(label: &mut String) -> Option<String> {
    let trimmed = label.trim_start();
    let rest = trimmed.strip_prefix('[')?;
    let close = rest.find(']')?;
    let raw = rest[..close].trim();
    let value = raw.strip_prefix('#')?.trim();
    if value.is_empty() {
        return None;
    }
    let normalized =
        if matches!(value.len(), 3 | 6 | 8) && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
            format!("#{value}")
        } else {
            value.to_string()
        };
    *label = rest[close + 1..].trim_start().to_string();
    Some(normalized)
}

/// Try to parse a WBS checkbox annotation from the end of a label, stripping it
/// from the label string if found.
pub(super) fn parse_wbs_checkbox(label: &mut String) -> Option<WbsCheckbox> {
    let trimmed = label.trim_end();
    if let Some(inner) = trimmed.strip_suffix(']') {
        if let Some(bracket_start) = inner.rfind('[') {
            let content = &inner[bracket_start + 1..];
            let checkbox = if content == "x" || content == "X" {
                Some(WbsCheckbox::Checked)
            } else if content == " " || content.is_empty() {
                Some(WbsCheckbox::Unchecked)
            } else if let Some(pct_str) = content.strip_prefix('%') {
                pct_str
                    .trim()
                    .parse::<u8>()
                    .ok()
                    .filter(|&n| n <= 100)
                    .map(WbsCheckbox::Progress)
            } else {
                None
            };
            if checkbox.is_some() {
                let prefix = &inner[..bracket_start].trim_end().to_string();
                *label = prefix.to_string();
                return checkbox;
            }
        }
    }
    None
}
