use super::*;

pub fn render_archimate_svg(document: &ArchimateDocument) -> String {
    let width = 760;
    let layers = [
        "strategy",
        "business",
        "application",
        "technology",
        "motivation",
        "junction",
    ];
    let lane_height = 80;
    let height = 80 + (layers.len() as i32) * lane_height;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    render_archimate_relation_marker_defs(&mut out, "#475569");
    let mut y = 28;
    out.push_str(&format!(
        "<text x=\"24\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\">{}</text>",
        y,
        escape_text(document.title.as_deref().unwrap_or("Archimate"))
    ));
    y += 16;
    let mut element_bounds: BTreeMap<String, (i32, i32, i32, i32)> = BTreeMap::new();
    let mut element_markup = String::new();
    for layer in layers.iter() {
        let layer_y = y;
        let bg = match *layer {
            "strategy" => "#fee2e2",
            "business" => "#fef3c7",
            "application" => "#dbeafe",
            "technology" => "#dcfce7",
            "motivation" => "#ede9fe",
            "junction" => "#f1f5f9",
            _ => "#f1f5f9",
        };
        out.push_str(&format!(
            "<rect x=\"24\" y=\"{}\" width=\"712\" height=\"{}\" fill=\"{}\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
            layer_y, lane_height, bg
        ));
        out.push_str(&format!(
            "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            layer_y + 14,
            escape_text(layer)
        ));
        let mut x = 100;
        for elem in document.elements.iter().filter(|e| e.layer == *layer) {
            let fill = elem.fill.as_deref().unwrap_or("white");
            let stroke = elem.stroke.as_deref().unwrap_or("#334155");
            let elem_y = layer_y + 22;
            render_archimate_element_shape(
                &mut element_markup,
                ArchimateElementRender {
                    layer: &elem.layer,
                    kind: &elem.kind,
                    alias: elem.alias.as_deref().unwrap_or(""),
                    x,
                    y: elem_y,
                    w: 140,
                    h: 40,
                    fill,
                    stroke,
                },
            );
            element_bounds.insert(elem.name.clone(), (x, elem_y, 140, 40));
            if let Some(alias) = &elem.alias {
                element_bounds.insert(alias.clone(), (x, elem_y, 140, 40));
            }
            element_markup.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
                x + 8,
                layer_y + 46,
                escape_text(&elem.name)
            ));
            x += 150;
            if x + 140 > 736 {
                break;
            }
        }
        y += lane_height;
    }
    for rel in &document.relations {
        let Some(&from) = element_bounds.get(&rel.from) else {
            continue;
        };
        let Some(&to) = element_bounds.get(&rel.to) else {
            continue;
        };
        let (x1, y1, x2, y2) =
            compute_edge_anchors_for_direction(from, to, rel.direction.as_deref());
        let relation_style = archimate_relation_style(rel.kind.as_str(), rel.style.as_deref());
        out.push_str(&format!(
            "<line class=\"archimate-relation-edge\" data-archimate-kind=\"{}\" data-archimate-direction=\"{}\" data-archimate-style=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} />",
            escape_text(&rel.kind),
            escape_text(rel.direction.as_deref().unwrap_or("")),
            escape_text(rel.style.as_deref().unwrap_or("")),
            x1,
            y1,
            x2,
            y2,
            escape_text(relation_style.color),
            relation_style.stroke_width,
            relation_style.dash,
            relation_style.marker_start,
            relation_style.marker_end
        ));
        if let Some(label) = rel.label.as_deref().filter(|label| !label.is_empty()) {
            out.push_str(&format!(
                "<text class=\"archimate-relation-label\" data-archimate-kind=\"{}\" x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">{}</text>",
                escape_text(&rel.kind),
                (x1 + x2) / 2 + 6,
                (y1 + y2) / 2 - 4,
                escape_text(label)
            ));
        }
    }
    out.push_str(&element_markup);
    out.push_str("</svg>");
    out
}

struct ArchimateRelationStyle<'a> {
    color: &'a str,
    stroke_width: f64,
    dash: &'static str,
    marker_start: &'static str,
    marker_end: &'static str,
}

struct ArchimateElementRender<'a> {
    layer: &'a str,
    kind: &'a str,
    alias: &'a str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    fill: &'a str,
    stroke: &'a str,
}

fn archimate_relation_style<'a>(
    kind: &str,
    inline_style: Option<&'a str>,
) -> ArchimateRelationStyle<'a> {
    let lower_style = inline_style.unwrap_or("").to_ascii_lowercase();
    let color = inline_style
        .filter(|style| style.starts_with('#') || style.starts_with('$'))
        .unwrap_or("#475569");
    let bold = lower_style.contains("bold");
    let dashed = lower_style.contains("dashed")
        || matches!(
            kind,
            "access" | "flow" | "influence" | "realization" | "used_by"
        );
    let marker_start = match kind {
        "aggregation" => " marker-start=\"url(#arrow-diamond-open)\"",
        "composition" => " marker-start=\"url(#arrow-diamond-filled)\"",
        "assignment" => " marker-start=\"url(#archimate-assignment)\"",
        _ => "",
    };
    let marker_end = match kind {
        "association" => "",
        "realization" | "specialization" => " marker-end=\"url(#arrow-triangle)\"",
        _ => " marker-end=\"url(#arrow-open)\"",
    };
    ArchimateRelationStyle {
        color,
        stroke_width: if bold { 2.5 } else { 1.5 },
        dash: if dashed {
            " stroke-dasharray=\"5 3\""
        } else {
            ""
        },
        marker_start,
        marker_end,
    }
}

fn render_archimate_element_shape(out: &mut String, element: ArchimateElementRender<'_>) {
    let ArchimateElementRender {
        layer,
        kind,
        alias,
        x,
        y,
        w,
        h,
        fill,
        stroke,
    } = element;
    out.push_str(&format!(
        "<g class=\"archimate-element\" data-archimate-layer=\"{}\" data-archimate-kind=\"{}\" data-archimate-alias=\"{}\">",
        escape_text(layer),
        escape_text(kind),
        escape_text(alias)
    ));
    match archimate_shape_for(layer, kind) {
        "junction" => {
            out.push_str(&format!(
                "<circle class=\"archimate-junction\" cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"#334155\"/>",
                x + w / 2,
                y + h / 2
            ));
        }
        "component" => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x,
                y,
                w,
                h,
                escape_text(fill),
                escape_text(stroke)
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"15\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4,
                y + 10,
                escape_text(fill),
                escape_text(stroke)
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"15\" height=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x - 4,
                y + h - 18,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
        "service" | "process" => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"18\" ry=\"18\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x,
                y,
                w,
                h,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
        "node" => {
            out.push_str(&format!(
                "<path d=\"M{x},{front_y} H{front_right} L{right},{top} V{back_bottom} L{front_right},{bottom} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                escape_text(fill),
                escape_text(stroke),
                front_y = y + 8,
                front_right = x + w - 12,
                right = x + w,
                top = y,
                back_bottom = y + h - 8,
                bottom = y + h
            ));
            out.push_str(&format!(
                "<path d=\"M{} {} V{} M{} {} L{} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + w - 12,
                y + 8,
                y + h,
                x + w - 12,
                y + 8,
                x + w,
                y,
                escape_text(stroke)
            ));
        }
        "data-object" => {
            out.push_str(&format!(
                "<path d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x + w - 18,
                x + w,
                y + 18,
                y + h,
                escape_text(fill),
                escape_text(stroke)
            ));
            out.push_str(&format!(
                "<path d=\"M{} {y} V{} H{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + w - 18,
                y + 18,
                x + w,
                escape_text(stroke)
            ));
        }
        "motivation" => {
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x + 14,
                y,
                x + w - 14,
                y,
                x + w,
                y + h / 2,
                x + w - 14,
                y + h,
                x + 14,
                y + h,
                x,
                y + h / 2,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
        "strategy" => {
            out.push_str(&format!(
                "<path d=\"M{x},{y} H{} L{} {} H{x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x + w - 18,
                x + w,
                y + h,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
        _ => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                x,
                y,
                w,
                h,
                escape_text(fill),
                escape_text(stroke)
            ));
        }
    }
    out.push_str("</g>");
}

fn archimate_shape_for(layer: &str, kind: &str) -> &'static str {
    let lower = kind.to_ascii_lowercase();
    if layer == "junction" || lower.starts_with("and") || lower.starts_with("or") {
        "junction"
    } else if lower.contains("component") {
        "component"
    } else if lower.contains("service") {
        "service"
    } else if lower.contains("process") || lower.contains("function") || lower.contains("event") {
        "process"
    } else if lower.contains("node")
        || lower.contains("device")
        || lower.contains("system-software")
    {
        "node"
    } else if lower.contains("data-object") || lower.contains("artifact") {
        "data-object"
    } else if layer == "motivation" {
        "motivation"
    } else if layer == "strategy" {
        "strategy"
    } else {
        "box"
    }
}

fn render_archimate_relation_marker_defs(out: &mut String, arrow_stroke: &str) {
    render_relation_marker_defs(out, arrow_stroke);
    out.push_str(&format!(
        "<defs><marker id=\"archimate-assignment\" viewBox=\"0 0 10 10\" refX=\"1\" refY=\"5\" markerWidth=\"8\" markerHeight=\"8\" orient=\"auto-start-reverse\"><circle cx=\"5\" cy=\"5\" r=\"3\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/></marker></defs>",
        escape_text(arrow_stroke),
        escape_text(arrow_stroke)
    ));
}
