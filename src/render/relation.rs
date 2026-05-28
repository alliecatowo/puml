/// Normalize relation endpoints when the parser stuffs arrow-head markers
/// (e.g. `<|`, `*`, `o`) into the trailing chars of `from` or the leading
/// chars of `to`. Returns (clean_from, clean_to, normalized_arrow).
pub(super) fn normalize_relation_endpoints(
    from: &str,
    to: &str,
    arrow: &crate::model::FamilyRelationArrow,
) -> (String, String, crate::model::FamilyRelationArrow) {
    let (clean_from, head_marker) = split_trailing_marker(from);
    let (clean_to, tail_marker) = split_leading_marker(to);
    let normalized_arrow = arrow
        .with_endpoint_markers(head_marker, tail_marker)
        .expect("endpoint marker normalization preserves a valid relation arrow");
    (clean_from, clean_to, normalized_arrow)
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

pub(super) fn arrow_style(arrow: &crate::model::FamilyRelationArrow) -> ArrowStyle {
    let start_marker = arrow.start_marker().map(relation_marker_id);
    let end_marker = arrow.end_marker().map(relation_marker_id);
    ArrowStyle {
        end_marker,
        start_marker,
        dashed: arrow.is_dashed(),
    }
}

pub(super) fn has_ie_endpoint_marker(arrow: &crate::model::FamilyRelationArrow) -> bool {
    matches!(
        arrow.start_marker(),
        Some(
            crate::model::FamilyRelationEndpointMarker::IeZeroMany
                | crate::model::FamilyRelationEndpointMarker::IeOneMany
                | crate::model::FamilyRelationEndpointMarker::IeZeroOne
                | crate::model::FamilyRelationEndpointMarker::IeOne
        )
    ) || matches!(
        arrow.end_marker(),
        Some(
            crate::model::FamilyRelationEndpointMarker::IeZeroMany
                | crate::model::FamilyRelationEndpointMarker::IeOneMany
                | crate::model::FamilyRelationEndpointMarker::IeZeroOne
                | crate::model::FamilyRelationEndpointMarker::IeOne
        )
    )
}

fn relation_marker_id(marker: crate::model::FamilyRelationEndpointMarker) -> &'static str {
    match marker {
        crate::model::FamilyRelationEndpointMarker::Open => "arrow-open",
        crate::model::FamilyRelationEndpointMarker::DoubleOpen => "arrow-double-open",
        crate::model::FamilyRelationEndpointMarker::Triangle => "arrow-triangle",
        crate::model::FamilyRelationEndpointMarker::DiamondFilled => "arrow-diamond-filled",
        crate::model::FamilyRelationEndpointMarker::DiamondOpen => "arrow-diamond-open",
        crate::model::FamilyRelationEndpointMarker::CircleOpen => "arrow-circle-open",
        crate::model::FamilyRelationEndpointMarker::CircleFilled => "arrow-circle-filled",
        crate::model::FamilyRelationEndpointMarker::TriangleFilled => "arrow-triangle-filled",
        crate::model::FamilyRelationEndpointMarker::BoxFilled => "arrow-box-filled",
        crate::model::FamilyRelationEndpointMarker::Plus => "arrow-plus",
        crate::model::FamilyRelationEndpointMarker::Slash => "arrow-slash",
        crate::model::FamilyRelationEndpointMarker::IeZeroMany => "arrow-ie-zero-many",
        crate::model::FamilyRelationEndpointMarker::IeOneMany => "arrow-ie-one-many",
        crate::model::FamilyRelationEndpointMarker::IeZeroOne => "arrow-ie-zero-one",
        crate::model::FamilyRelationEndpointMarker::IeOne => "arrow-ie-one",
    }
}

pub(super) fn usecase_dependency_label(label: Option<&str>) -> Option<&'static str> {
    let normalized = label?.trim().to_ascii_lowercase();
    // Collect whitespace-collapsed form for matching exact stereotype tokens.
    // Substring `.contains()` matching is intentionally NOT used here: labels
    // like "inheritance (B extends A)" contain "extend" as a sub-word and must
    // not be reclassified as `<<extend>>` dependency edges (#1261).
    // Only the canonical UML stereotype spellings are recognised.
    let compact = normalized.split_whitespace().collect::<String>();
    if matches!(compact.as_str(), "<<include>>" | "include" | "includes") {
        Some("<<include>>")
    } else if matches!(compact.as_str(), "<<extend>>" | "extend" | "extends") {
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
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-double-open\" viewBox=\"0 0 16 12\" refX=\"15\" refY=\"6\" \
         markerWidth=\"16\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,1 L8,6 L0,11 M7,1 L15,6 L7,11\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\" stroke-linejoin=\"round\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-circle-open\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <circle cx=\"6\" cy=\"6\" r=\"4\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-circle-filled\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <circle cx=\"6\" cy=\"6\" r=\"4\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-triangle-filled\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <polygon points=\"0,0 12,6 0,12\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-box-filled\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <rect x=\"2\" y=\"2\" width=\"8\" height=\"8\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-plus\" viewBox=\"0 0 12 12\" refX=\"11\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M6,1 L6,11 M1,6 L11,6\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"{prefix}arrow-slash\" viewBox=\"0 0 12 12\" refX=\"10\" refY=\"6\" \
         markerWidth=\"12\" markerHeight=\"12\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M3,2 L9,10\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.8\" stroke-linecap=\"round\"/>\
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
