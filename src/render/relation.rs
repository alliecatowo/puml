/// Normalize relation endpoints when the parser stuffs arrow-head markers
/// (e.g. `<|`, `*`, `o`) into the trailing chars of `from` or the leading
/// chars of `to`. Returns (clean_from, clean_to, normalized_arrow).
pub(super) fn normalize_relation_endpoints(
    from: &str,
    to: &str,
    arrow: &str,
) -> (String, String, String) {
    let (clean_from, head_marker) = split_trailing_marker(from);
    let (clean_to, tail_marker) = split_leading_marker(to);
    let mut a = String::new();
    a.push_str(head_marker);
    a.push_str(arrow);
    a.push_str(tail_marker);
    (clean_from, clean_to, a)
}

fn split_trailing_marker(s: &str) -> (String, &'static str) {
    let trimmed = s.trim_end();
    if let Some(stripped) = trimmed.strip_suffix("<|") {
        return (stripped.trim_end().to_string(), "<|");
    }
    for m in ["*", "o", "<", "+"] {
        if let Some(stripped) = trimmed.strip_suffix(m) {
            // Require space between name and marker to avoid clobbering names
            // that legitimately end with these characters.
            if stripped.ends_with(' ') {
                return (
                    stripped.trim_end().to_string(),
                    match m {
                        "*" => "*",
                        "o" => "o",
                        "<" => "<",
                        "+" => "+",
                        _ => "",
                    },
                );
            }
        }
    }
    (trimmed.to_string(), "")
}

fn split_leading_marker(s: &str) -> (String, &'static str) {
    let trimmed = s.trim_start();
    if let Some(stripped) = trimmed.strip_prefix("|>") {
        return (stripped.trim_start().to_string(), "|>");
    }
    for m in ["*", "o", ">", "+"] {
        if let Some(stripped) = trimmed.strip_prefix(m) {
            if stripped.starts_with(' ') {
                return (
                    stripped.trim_start().to_string(),
                    match m {
                        "*" => "*",
                        "o" => "o",
                        ">" => ">",
                        "+" => "+",
                        _ => "",
                    },
                );
            }
        }
    }
    (trimmed.to_string(), "")
}

pub(super) struct ArrowStyle {
    pub(super) end_marker: Option<&'static str>,
    pub(super) start_marker: Option<&'static str>,
    pub(super) dashed: bool,
}

pub(super) fn arrow_style(arrow: &str) -> ArrowStyle {
    let trimmed = arrow.trim();
    let dashed = trimmed.contains("..");
    let ie_start_marker = ie_start_marker(trimmed);
    let ie_end_marker = ie_end_marker(trimmed);
    // Detect markers at each end
    let head = trimmed.chars().next().unwrap_or(' ');
    let tail = trimmed.chars().last().unwrap_or(' ');
    let start_marker = ie_start_marker.or(match head {
        '<' => {
            // inheritance reversed if starts with "<|"
            if trimmed.starts_with("<|") {
                Some("arrow-triangle")
            } else {
                Some("arrow-open")
            }
        }
        '*' => Some("arrow-diamond-filled"),
        'o' => Some("arrow-diamond-open"),
        '0' | '(' | ')' => Some("arrow-open"),
        _ => None,
    });
    let end_marker = ie_end_marker.or(match tail {
        '>' => {
            if trimmed.ends_with("|>") {
                Some("arrow-triangle")
            } else {
                Some("arrow-open")
            }
        }
        '*' => Some("arrow-diamond-filled"),
        'o' => Some("arrow-diamond-open"),
        '0' | '(' | ')' | '@' | '^' | '#' | '+' => Some("arrow-open"),
        _ => None,
    });
    ArrowStyle {
        end_marker,
        start_marker,
        dashed,
    }
}

pub(super) fn has_ie_endpoint_marker(arrow: &str) -> bool {
    ie_start_marker(arrow).is_some() || ie_end_marker(arrow).is_some()
}

fn ie_start_marker(arrow: &str) -> Option<&'static str> {
    if arrow.starts_with("}o") || arrow.starts_with("o{") {
        Some("arrow-ie-zero-many")
    } else if arrow.starts_with("}|") || arrow.starts_with("|{") {
        Some("arrow-ie-one-many")
    } else if arrow.starts_with("|o") || arrow.starts_with("o|") {
        Some("arrow-ie-zero-one")
    } else if arrow.starts_with("||") {
        Some("arrow-ie-one")
    } else {
        None
    }
}

fn ie_end_marker(arrow: &str) -> Option<&'static str> {
    if arrow.ends_with("o{") || arrow.ends_with("}o") {
        Some("arrow-ie-zero-many")
    } else if arrow.ends_with("|{") || arrow.ends_with("}|") {
        Some("arrow-ie-one-many")
    } else if arrow.ends_with("o|") || arrow.ends_with("|o") {
        Some("arrow-ie-zero-one")
    } else if arrow.ends_with("||") {
        Some("arrow-ie-one")
    } else {
        None
    }
}

pub(super) fn usecase_dependency_label(label: Option<&str>) -> Option<&'static str> {
    let normalized = label?.trim().to_ascii_lowercase();
    let compact = normalized.split_whitespace().collect::<String>();
    if matches!(compact.as_str(), "<<include>>" | "include" | "includes")
        || compact.contains("include")
    {
        Some("<<include>>")
    } else if matches!(compact.as_str(), "<<extend>>" | "extend" | "extends")
        || compact.contains("extend")
    {
        Some("<<extend>>")
    } else {
        None
    }
}

pub(crate) fn render_relation_marker_defs(out: &mut String, arrow_stroke: &str) {
    render_relation_marker_defs_with_prefix(out, arrow_stroke, "");
}

pub(crate) fn render_relation_marker_defs_with_prefix(
    out: &mut String,
    arrow_stroke: &str,
    prefix: &str,
) {
    // markerUnits="userSpaceOnUse" pins marker sizes in SVG user units so they
    // are NOT scaled by the parent element's stroke-width (fix #471 collision).
    // fill="#ffffff" instead of fill="white" avoids resvg color-keyword inheritance
    // rendering open markers as filled in PNG output (fix #467).
    out.push_str("<defs>");
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-open\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"10\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-triangle\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <polygon points=\"0,0 12,6 0,12\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\" fill-rule=\"nonzero\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-diamond-filled\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-diamond-open\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    out.push_str("</defs>");
}

pub(crate) fn render_ie_marker_defs(out: &mut String, arrow_stroke: &str) {
    render_ie_marker_defs_with_prefix(out, arrow_stroke, "");
}

fn render_ie_marker_defs_with_prefix(out: &mut String, arrow_stroke: &str, prefix: &str) {
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-ie-one\" viewBox=\"0 0 24 16\" refX=\"22\" refY=\"8\" \
         markerWidth=\"24\" markerHeight=\"16\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M16,2 L16,14\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-ie-zero-one\" viewBox=\"0 0 30 16\" refX=\"28\" refY=\"8\" \
         markerWidth=\"30\" markerHeight=\"16\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <circle cx=\"10\" cy=\"8\" r=\"4\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         <path d=\"M20,2 L20,14\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-ie-zero-many\" viewBox=\"0 0 34 18\" refX=\"32\" refY=\"9\" \
         markerWidth=\"34\" markerHeight=\"18\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <circle cx=\"8\" cy=\"9\" r=\"4\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         <path d=\"M18,9 L31,2 M18,9 L31,9 M18,9 L31,16\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.7\" stroke-linecap=\"round\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-ie-one-many\" viewBox=\"0 0 34 18\" refX=\"32\" refY=\"9\" \
         markerWidth=\"34\" markerHeight=\"18\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M8,2 L8,16 M18,9 L31,2 M18,9 L31,9 M18,9 L31,16\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.7\" stroke-linecap=\"round\"/>\
         </marker>",
    ));
}

pub(super) fn render_lollipop_endpoint(out: &mut String, x: i32, y: i32, stroke: &str) {
    out.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"white\" stroke=\"{}\" stroke-width=\"1.5\" class=\"uml-lollipop\"/>",
        x,
        y,
        stroke
    ));
}
