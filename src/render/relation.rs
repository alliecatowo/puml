/// Normalize relation endpoints when the parser stuffs arrow-head markers
/// (e.g. `<|`, `*`, `o`) into the trailing chars of `from` or the leading
/// chars of `to`. Returns (clean_from, clean_to, normalized_arrow).
pub(super) fn normalize_relation_endpoints(
    from: &str,
    to: &str,
    arrow: &str,
) -> (String, String, String) {
    normalize_relation(from, to, arrow).into_parts()
}

pub(super) fn normalize_relation(from: &str, to: &str, arrow: &str) -> NormalizedRelation {
    let (clean_from, head_marker) = split_trailing_marker(from);
    let (clean_to, tail_marker) = split_leading_marker(to);
    let mut a = String::new();
    a.push_str(head_marker);
    a.push_str(arrow);
    a.push_str(tail_marker);
    NormalizedRelation {
        from: clean_from,
        to: clean_to,
        arrow: RelationArrow::new(a),
    }
}

pub(super) struct NormalizedRelation {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) arrow: RelationArrow,
}

impl NormalizedRelation {
    fn into_parts(self) -> (String, String, String) {
        (self.from, self.to, self.arrow.into_raw())
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RelationLineKind {
    Solid,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RelationEndpointMarker {
    None,
    Open,
    Triangle,
    DiamondFilled,
    DiamondOpen,
    CircleOpen,
    CircleFilled,
    TriangleFilled,
    BoxFilled,
    Plus,
    Slash,
    DoubleOpen,
    InformationEngineeringOne,
    InformationEngineeringZeroOne,
    InformationEngineeringOneMany,
    InformationEngineeringZeroMany,
}

impl RelationEndpointMarker {
    pub(super) const fn svg_marker_id(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Open => Some("arrow-open"),
            Self::Triangle => Some("arrow-triangle"),
            Self::DiamondFilled => Some("arrow-diamond-filled"),
            Self::DiamondOpen => Some("arrow-diamond-open"),
            Self::CircleOpen => Some("arrow-circle-open"),
            Self::CircleFilled => Some("arrow-circle-filled"),
            Self::TriangleFilled => Some("arrow-triangle-filled"),
            Self::BoxFilled => Some("arrow-box-filled"),
            Self::Plus => Some("arrow-plus"),
            Self::Slash => Some("arrow-slash"),
            Self::DoubleOpen => Some("arrow-double-open"),
            Self::InformationEngineeringOne => Some("arrow-ie-one"),
            Self::InformationEngineeringZeroOne => Some("arrow-ie-zero-one"),
            Self::InformationEngineeringOneMany => Some("arrow-ie-one-many"),
            Self::InformationEngineeringZeroMany => Some("arrow-ie-zero-many"),
        }
    }

    const fn is_information_engineering(self) -> bool {
        matches!(
            self,
            Self::InformationEngineeringOne
                | Self::InformationEngineeringZeroOne
                | Self::InformationEngineeringOneMany
                | Self::InformationEngineeringZeroMany
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RelationArrow {
    raw: String,
    line: RelationLineKind,
    start_marker: RelationEndpointMarker,
    end_marker: RelationEndpointMarker,
}

impl RelationArrow {
    pub(super) fn new(raw: String) -> Self {
        let trimmed = raw.trim();
        let line = if trimmed.contains("..") {
            RelationLineKind::Dotted
        } else {
            RelationLineKind::Solid
        };
        let start_marker = relation_start_marker(trimmed);
        let end_marker = relation_end_marker(trimmed);
        Self {
            raw,
            line,
            start_marker,
            end_marker,
        }
    }

    pub(super) fn as_str(&self) -> &str {
        &self.raw
    }

    fn into_raw(self) -> String {
        self.raw
    }

    pub(super) fn style(&self) -> ArrowStyle {
        ArrowStyle {
            end_marker: self.end_marker,
            start_marker: self.start_marker,
            dashed: matches!(self.line, RelationLineKind::Dotted),
        }
    }

    pub(super) fn has_information_engineering_endpoint(&self) -> bool {
        self.start_marker.is_information_engineering()
            || self.end_marker.is_information_engineering()
    }
}

pub(super) struct ArrowStyle {
    pub(super) end_marker: RelationEndpointMarker,
    pub(super) start_marker: RelationEndpointMarker,
    pub(super) dashed: bool,
}

fn relation_start_marker(trimmed: &str) -> RelationEndpointMarker {
    let ie_start_marker = ie_start_marker(trimmed);
    // Detect markers at each end
    let head = trimmed.chars().next().unwrap_or(' ');
    ie_start_marker.unwrap_or(match head {
        '<' => {
            if trimmed.starts_with("<<") {
                RelationEndpointMarker::DoubleOpen
            } else if trimmed.starts_with("<|") {
                RelationEndpointMarker::Triangle
            } else {
                RelationEndpointMarker::Open
            }
        }
        '*' => RelationEndpointMarker::DiamondFilled,
        'o' => RelationEndpointMarker::DiamondOpen,
        '0' | '(' | ')' => RelationEndpointMarker::CircleOpen,
        '@' => RelationEndpointMarker::CircleFilled,
        '^' => RelationEndpointMarker::TriangleFilled,
        '#' => RelationEndpointMarker::BoxFilled,
        '+' => RelationEndpointMarker::Plus,
        '\\' | '/' => RelationEndpointMarker::Slash,
        _ => RelationEndpointMarker::None,
    })
}

fn relation_end_marker(trimmed: &str) -> RelationEndpointMarker {
    let ie_end_marker = ie_end_marker(trimmed);
    let tail = trimmed.chars().last().unwrap_or(' ');
    ie_end_marker.unwrap_or(match tail {
        '>' => {
            if trimmed.ends_with(">>") {
                RelationEndpointMarker::DoubleOpen
            } else if trimmed.ends_with("|>") {
                RelationEndpointMarker::Triangle
            } else {
                RelationEndpointMarker::Open
            }
        }
        '*' => RelationEndpointMarker::DiamondFilled,
        'o' => RelationEndpointMarker::DiamondOpen,
        '0' | '(' | ')' => RelationEndpointMarker::CircleOpen,
        '@' => RelationEndpointMarker::CircleFilled,
        '^' => RelationEndpointMarker::TriangleFilled,
        '#' => RelationEndpointMarker::BoxFilled,
        '+' => RelationEndpointMarker::Plus,
        '\\' | '/' => RelationEndpointMarker::Slash,
        _ => RelationEndpointMarker::None,
    })
}

fn ie_start_marker(arrow: &str) -> Option<RelationEndpointMarker> {
    if arrow.starts_with("}o") || arrow.starts_with("o{") {
        Some(RelationEndpointMarker::InformationEngineeringZeroMany)
    } else if arrow.starts_with("}|") || arrow.starts_with("|{") {
        Some(RelationEndpointMarker::InformationEngineeringOneMany)
    } else if arrow.starts_with("|o") || arrow.starts_with("o|") {
        Some(RelationEndpointMarker::InformationEngineeringZeroOne)
    } else if arrow.starts_with("||") {
        Some(RelationEndpointMarker::InformationEngineeringOne)
    } else {
        None
    }
}

fn ie_end_marker(arrow: &str) -> Option<RelationEndpointMarker> {
    if arrow.ends_with("o{") || arrow.ends_with("}o") {
        Some(RelationEndpointMarker::InformationEngineeringZeroMany)
    } else if arrow.ends_with("|{") || arrow.ends_with("}|") {
        Some(RelationEndpointMarker::InformationEngineeringOneMany)
    } else if arrow.ends_with("o|") || arrow.ends_with("|o") {
        Some(RelationEndpointMarker::InformationEngineeringZeroOne)
    } else if arrow.ends_with("||") {
        Some(RelationEndpointMarker::InformationEngineeringOne)
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

#[cfg(test)]
mod tests {
    use super::{normalize_relation, RelationEndpointMarker, RelationLineKind};

    #[test]
    fn normalized_relation_arrow_preserves_svg_string_and_exposes_typed_markers() {
        let relation = normalize_relation("Order *", "|> LineItem", "..");

        assert_eq!(relation.from, "Order");
        assert_eq!(relation.to, "LineItem");
        assert_eq!(relation.arrow.as_str(), "*..|>");
        assert_eq!(relation.arrow.line, RelationLineKind::Dotted);
        assert_eq!(
            relation.arrow.start_marker,
            RelationEndpointMarker::DiamondFilled
        );
        assert_eq!(relation.arrow.end_marker, RelationEndpointMarker::Triangle);
        assert!(relation.arrow.style().dashed);
    }

    #[test]
    fn information_engineering_markers_are_typed_not_stringly() {
        let relation = normalize_relation("Customer", "Order", "||--o{");

        assert!(relation.arrow.has_information_engineering_endpoint());
        assert_eq!(
            relation.arrow.style().start_marker,
            RelationEndpointMarker::InformationEngineeringOne
        );
        assert_eq!(
            relation.arrow.style().end_marker,
            RelationEndpointMarker::InformationEngineeringZeroMany
        );
    }
}
