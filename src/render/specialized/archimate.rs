use super::*;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, LaneFrame, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge,
    SceneNode,
};

pub fn render_archimate_svg(document: &ArchimateDocument) -> String {
    render_archimate_artifact(document).svg
}

pub fn render_archimate_artifact(document: &ArchimateDocument) -> crate::output::RenderArtifact {
    let svg = render_archimate_svg_inner(document);
    let scene = build_archimate_scene(document);
    crate::output::RenderArtifact::with_scene(svg, scene)
}

fn render_archimate_svg_inner(document: &ArchimateDocument) -> String {
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
    // Count only layers that have content (#502) to avoid blank bands.
    let populated_layer_count = layers
        .iter()
        .filter(|&&layer| document.elements.iter().any(|e| e.layer == layer))
        .count()
        .max(1);
    let height = 80 + (populated_layer_count as i32) * lane_height;
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
        // Only render layers that have at least one element (#502).
        let layer_elements: Vec<_> = document
            .elements
            .iter()
            .filter(|e| e.layer == *layer)
            .collect();
        if layer_elements.is_empty() {
            continue;
        }
        let layer_y = y;
        // ArchiMate standard layer colours (#529).
        let bg = match *layer {
            "strategy" => "#F5DEAA",
            "business" => "#FFFFB0",
            "application" => "#D5E8F0",
            "technology" => "#D5F5DD",
            "motivation" => "#E0D5F5",
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
        for elem in layer_elements {
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
            render_archimate_role_icon(out, kind, x, y, w, stroke);
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

fn render_archimate_role_icon(out: &mut String, kind: &str, x: i32, y: i32, w: i32, stroke: &str) {
    let icon_x = x + w - 22;
    let icon_y = y + 8;
    let lower = kind.to_ascii_lowercase();
    if lower.contains("service") {
        out.push_str(&format!(
            "<polygon class=\"archimate-role-icon\" data-archimate-role-icon=\"service\" points=\"{},{} {},{} {},{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" stroke-linejoin=\"round\"/>",
            icon_x,
            icon_y,
            icon_x + 10,
            icon_y + 5,
            icon_x,
            icon_y + 10,
            escape_text(stroke)
        ));
    } else if lower.contains("process") || lower.contains("function") || lower.contains("event") {
        let center_x = icon_x + 5;
        let center_y = icon_y + 5;
        out.push_str(&format!(
            "<g class=\"archimate-role-icon\" data-archimate-role-icon=\"process\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.2\" stroke-linecap=\"round\"><circle cx=\"{}\" cy=\"{}\" r=\"3\"/><circle cx=\"{}\" cy=\"{}\" r=\"6\" stroke-dasharray=\"0.5 5.5\"/></g>",
            escape_text(stroke),
            center_x,
            center_y,
            center_x,
            center_y
        ));
    }
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

/// Build a typed `RenderScene` that mirrors the geometry emitted by
/// [`render_archimate_svg_inner`].
///
/// Nodes are keyed by element **name** (the same string used in
/// `element_bounds.insert(elem.name.clone(), ...)`).  Alias → name resolution
/// is applied so that edge `from`/`to` always refer to a real node id even
/// when the relation was written using an alias.
fn build_archimate_scene(document: &ArchimateDocument) -> RenderScene {
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
    let populated_layer_count = layers
        .iter()
        .filter(|&&layer| document.elements.iter().any(|e| e.layer == layer))
        .count()
        .max(1);
    let height = 80 + (populated_layer_count as i32) * lane_height;

    let viewport = Rect::new(0.0, 0.0, width as f64, height as f64);
    let mut scene = RenderScene::new(viewport);

    // alias → element name resolution map
    let mut alias_to_name: BTreeMap<String, String> = BTreeMap::new();
    for elem in &document.elements {
        if let Some(alias) = &elem.alias {
            alias_to_name.insert(alias.clone(), elem.name.clone());
        }
    }

    // geometry mirrors render_archimate_svg_inner exactly
    let mut y = 44i32; // 28 + 16 as in the SVG emitter
    let mut element_bounds: BTreeMap<String, (i32, i32, i32, i32)> = BTreeMap::new();

    for layer in layers.iter() {
        let layer_elements: Vec<_> = document
            .elements
            .iter()
            .filter(|e| e.layer == *layer)
            .collect();
        if layer_elements.is_empty() {
            continue;
        }
        let layer_y = y;

        // Lane for this layer band
        scene.add_lane(LaneFrame {
            id: format!("lane:{layer}"),
            bounds: Rect::new(24.0, layer_y as f64, 712.0, lane_height as f64),
            header: Some(Rect::new(24.0, layer_y as f64, 712.0, 14.0)),
            child_node_ids: layer_elements.iter().map(|e| e.name.clone()).collect(),
            labels: vec![LabelBox {
                id: format!("lane:{layer}:label"),
                text: layer.to_string(),
                bounds: Rect::new(32.0, (layer_y + 14) as f64, 80.0, 12.0),
                owner_id: Some(format!("lane:{layer}")),
                role: LabelRole::Lane,
            }],
        });

        let mut x = 100i32;
        for elem in layer_elements {
            let elem_y = layer_y + 22;
            let bounds = Rect::new(x as f64, elem_y as f64, 140.0, 40.0);
            let label = LabelBox {
                id: format!("{}:label", elem.name),
                text: elem.name.clone(),
                bounds: Rect::new((x + 8) as f64, (layer_y + 46) as f64, 100.0, 14.0),
                owner_id: Some(elem.name.clone()),
                role: LabelRole::Node,
            };
            scene.add_node(SceneNode {
                id: elem.name.clone(),
                node_box: NodeBox {
                    id: elem.name.clone(),
                    bounds,
                    ports: vec![],
                    labels: vec![label],
                },
            });
            element_bounds.insert(elem.name.clone(), (x, elem_y, 140, 40));
            if let Some(alias) = &elem.alias {
                element_bounds.insert(alias.clone(), (x, elem_y, 140, 40));
            }
            x += 150;
            if x + 140 > 736 {
                break;
            }
        }
        y += lane_height;
    }

    // Resolve alias or name to node id (element name).
    let resolve_id = |id: &str| -> String {
        alias_to_name
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    };

    for (rel_idx, rel) in document.relations.iter().enumerate() {
        let Some(&from_rect) = element_bounds.get(&rel.from) else {
            continue;
        };
        let Some(&to_rect) = element_bounds.get(&rel.to) else {
            continue;
        };
        let (x1, y1, x2, y2) =
            compute_edge_anchors_for_direction(from_rect, to_rect, rel.direction.as_deref());
        let from_id = resolve_id(&rel.from);
        let to_id = resolve_id(&rel.to);
        let edge_id = format!("rel:{rel_idx}");
        let src_pos = Point::new(x1 as f64, y1 as f64);
        let tgt_pos = Point::new(x2 as f64, y2 as f64);
        scene.add_edge(SceneEdge {
            id: edge_id.clone(),
            from: from_id.clone(),
            to: to_id.clone(),
            route: Polyline::from_tuples(&[(x1 as f64, y1 as f64), (x2 as f64, y2 as f64)]),
            route_channel_ids: vec![],
            source_anchor: Anchor {
                id: format!("{edge_id}:source"),
                owner_id: from_id,
                position: src_pos,
                port: None,
            },
            target_anchor: Anchor {
                id: format!("{edge_id}:target"),
                owner_id: to_id,
                position: tgt_pos,
                port: None,
            },
            labels: vec![],
        });
    }

    scene
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ArchimateElement, ArchimateRelation};

    fn make_doc_with_alias_relation() -> ArchimateDocument {
        ArchimateDocument {
            title: Some("Test".to_string()),
            elements: vec![
                ArchimateElement {
                    name: "Order Service".to_string(),
                    alias: Some("svc".to_string()),
                    layer: "application".to_string(),
                    kind: "component".to_string(),
                    fill: None,
                    stroke: None,
                },
                ArchimateElement {
                    name: "Customer".to_string(),
                    alias: Some("cust".to_string()),
                    layer: "business".to_string(),
                    kind: "actor".to_string(),
                    fill: None,
                    stroke: None,
                },
            ],
            relations: vec![ArchimateRelation {
                from: "svc".to_string(),
                to: "cust".to_string(),
                kind: "serving".to_string(),
                label: None,
                direction: None,
                style: None,
            }],
            warnings: vec![],
        }
    }

    #[test]
    fn archimate_artifact_scene_has_no_geometry_issues() {
        let doc = make_doc_with_alias_relation();
        let artifact = render_archimate_artifact(&doc);
        let scene = artifact.scene.expect("archimate scene must be present");

        // Two elements → two nodes keyed by display name
        assert_eq!(scene.nodes.len(), 2, "expected 2 nodes");
        assert!(
            scene.nodes.contains_key("Order Service"),
            "node 'Order Service' must be present"
        );
        assert!(
            scene.nodes.contains_key("Customer"),
            "node 'Customer' must be present"
        );

        // One relation → one edge
        assert_eq!(scene.edges.len(), 1, "expected 1 edge");
        let edge = scene.edges.values().next().unwrap();
        assert_eq!(
            edge.from, "Order Service",
            "edge.from must resolve alias 'svc' → 'Order Service'"
        );
        assert_eq!(
            edge.to, "Customer",
            "edge.to must resolve alias 'cust' → 'Customer'"
        );

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "archimate scene must have no geometry issues: {issues:?}"
        );
    }
}
