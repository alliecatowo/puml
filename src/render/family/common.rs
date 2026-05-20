use super::*;

/// Emit a centered SVG `<text>` element for a relation label.
///
/// Labels may contain `\n` after normalization merges multiple Rel() calls on
/// the same source→target pair into a single coalesced label (#425).  Each
/// logical line is emitted as a `<tspan>` so they stack visually instead of
/// being run together as a single string of whitespace.
pub(super) fn diagram_family_id(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::Sequence => "sequence",
        DiagramKind::Salt => "salt",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Json => "json",
        DiagramKind::Yaml => "yaml",
        DiagramKind::Nwdiag => "nwdiag",
        DiagramKind::Archimate => "archimate",
        DiagramKind::Regex => "regex",
        DiagramKind::Ebnf => "ebnf",
        DiagramKind::Math => "math",
        DiagramKind::Sdl => "sdl",
        DiagramKind::Ditaa => "ditaa",
        DiagramKind::Chart => "chart",
        DiagramKind::Chen => "chen",
        DiagramKind::Unknown => "unknown",
    }
}

pub(super) fn geometry_bbox(x: i32, y: i32, w: i32, h: i32) -> SceneRect {
    SceneRect::new(x as f64, y as f64, w as f64, h as f64)
}

pub(super) fn semantic_node_rect(
    id: &str,
    family: &str,
    kind: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> String {
    let attrs = crate::render::puml_node_attrs(id, family, kind, geometry_bbox(x, y, w, h));
    format!(
        "<rect class=\"puml-node\" data-uml-id=\"{}\" data-uml-kind=\"{}\" {} x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"none\" pointer-events=\"none\"/>",
        escape_text(id),
        escape_text(kind),
        attrs,
        x,
        y,
        w,
        h
    )
}

pub(super) fn text_semantic_attrs(
    owner: &str,
    label_kind: &str,
    x: i32,
    y: i32,
    text: &str,
    font_size: i32,
    middle_anchor: bool,
) -> String {
    let bbox = estimate_text_bbox(x as f64, y as f64, text, font_size as f64, middle_anchor);
    crate::render::puml_label_attrs(owner, label_kind, bbox)
}

pub(super) fn relation_label_svg(
    x: i32,
    y: i32,
    label: &str,
    font_size: i32,
    fill: &str,
    owner: &str,
    label_kind: &str,
) -> String {
    let lines: Vec<&str> = label.split('\n').collect();
    let label_attrs = if lines.len() <= 1 {
        text_semantic_attrs(owner, label_kind, x, y, label, font_size, true)
    } else {
        let longest = lines
            .iter()
            .copied()
            .max_by_key(|line| line.chars().count())
            .unwrap_or(label);
        let mut bbox = estimate_text_bbox(x as f64, y as f64, longest, font_size as f64, true);
        bbox.h = (lines.len() as i32 * (font_size + 2)) as f64;
        bbox.y = (y - bbox.h as i32 / 2) as f64;
        crate::render::puml_label_attrs(owner, label_kind, bbox)
    };
    if lines.len() <= 1 {
        // Fast path – no newline, emit plain text element.
        return format!(
            "<text class=\"puml-label\" {} x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"{}\" fill=\"{}\">{}</text>",
            label_attrs,
            x, y, font_size, escape_text(fill), escape_text(label)
        );
    }
    // Multiline: emit one <tspan> per logical line, each shifted down by
    // (font_size + 2) pixels so lines are clearly separated.
    let line_h = font_size + 2;
    let total_h = (lines.len() as i32 - 1) * line_h;
    // Start above the anchor so the block is centred on y.
    let start_y = y - total_h / 2;
    let mut buf = format!(
        "<text class=\"puml-label\" {} text-anchor=\"middle\" font-family=\"monospace\" font-size=\"{}\" fill=\"{}\">",
        label_attrs,
        font_size,
        escape_text(fill)
    );
    for (i, line) in lines.iter().enumerate() {
        let ty = start_y + (i as i32) * line_h;
        buf.push_str(&format!(
            "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
            x,
            ty,
            escape_text(line)
        ));
    }
    buf.push_str("</text>");
    buf
}

pub(super) fn relation_pair_label_lane_map(
    document: &FamilyDocument,
) -> std::collections::BTreeMap<usize, i32> {
    let mut pair_counts: std::collections::BTreeMap<(String, String), i32> =
        std::collections::BTreeMap::new();
    let mut pair_seen: std::collections::BTreeMap<(String, String), i32> =
        std::collections::BTreeMap::new();
    let mut lanes = std::collections::BTreeMap::new();

    for relation in &document.relations {
        let key = if relation.from <= relation.to {
            (relation.from.clone(), relation.to.clone())
        } else {
            (relation.to.clone(), relation.from.clone())
        };
        *pair_counts.entry(key).or_insert(0) += 1;
    }

    for (idx, relation) in document.relations.iter().enumerate() {
        let key = if relation.from <= relation.to {
            (relation.from.clone(), relation.to.clone())
        } else {
            (relation.to.clone(), relation.from.clone())
        };
        let count = pair_counts.get(&key).copied().unwrap_or(1);
        let seen = pair_seen.entry(key).or_insert(0);
        let lane = if count <= 1 {
            0
        } else {
            (*seen * 2 - (count - 1)) * 14
        };
        *seen += 1;
        lanes.insert(idx, lane);
    }

    lanes
}
