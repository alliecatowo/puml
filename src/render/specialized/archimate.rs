use super::*;
use crate::output::RenderArtifact;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge, SceneNode,
};

pub fn render_archimate_svg(document: &ArchimateDocument) -> String {
    render_archimate_artifact(document).svg
}

/// Render an ArchiMate diagram into a typed [`RenderArtifact`].
///
/// The SVG is still emitted directly (ArchiMate draws bespoke layer bands, shaped
/// element boxes, and straight border-to-border relationship lines), but we also
/// build a [`RenderScene`] from the *actual* drawn geometry — node boxes at their
/// computed positions/sizes and edges along the same anchor segments the SVG uses —
/// so the scene stays consistent with the output.  SVG output is byte-identical to
/// the legacy `render_archimate_svg`; the scene is attached for the typed-geometry
/// validation path.
pub fn render_archimate_artifact(document: &ArchimateDocument) -> RenderArtifact {
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

    let scene = build_archimate_scene(
        &element_bounds,
        &document.relations,
        width as f64,
        height as f64,
    );
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from ArchiMate's laid-out geometry.  Node boxes
/// use the same `(x, y, w, h)` tuples stored in `element_bounds`; edge routes use
/// the same `compute_edge_anchors_for_direction` anchor points the SVG draws, so
/// scene and SVG never diverge.
fn build_archimate_scene(
    element_bounds: &BTreeMap<String, (i32, i32, i32, i32)>,
    relations: &[crate::model::ArchimateRelation],
    width: f64,
    height: f64,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width, height));

    // Add a scene node for every element that has a known bounding box.
    // Because element_bounds may contain both the canonical name and an alias
    // pointing at the same rectangle, we deduplicate by (x, y, w, h) to avoid
    // registering the same box twice.
    let mut seen_bounds: BTreeMap<(i32, i32, i32, i32), String> = BTreeMap::new();
    for (name, &(x, y, w, h)) in element_bounds {
        if seen_bounds.contains_key(&(x, y, w, h)) {
            // This entry is the alias — the canonical name was inserted first;
            // skip to avoid duplicate nodes in the scene.
            continue;
        }
        seen_bounds.insert((x, y, w, h), name.clone());
        let bounds = Rect::new(x as f64, y as f64, w as f64, h as f64);
        let label = LabelBox {
            id: format!("{name}::label"),
            text: name.clone(),
            bounds,
            owner_id: Some(name.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: name.clone(),
            node_box: NodeBox {
                id: name.clone(),
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    // Add a scene edge for every relation whose endpoints are in the bounds map.
    for (idx, rel) in relations.iter().enumerate() {
        let Some(&from) = element_bounds.get(&rel.from) else {
            continue;
        };
        let Some(&to) = element_bounds.get(&rel.to) else {
            continue;
        };
        let (x1, y1, x2, y2) =
            compute_edge_anchors_for_direction(from, to, rel.direction.as_deref());
        let edge_id = format!("rel{idx}");
        let source_anchor = Anchor {
            id: format!("{edge_id}::src"),
            owner_id: rel.from.clone(),
            position: Point::new(x1 as f64, y1 as f64),
            port: None,
        };
        let target_anchor = Anchor {
            id: format!("{edge_id}::tgt"),
            owner_id: rel.to.clone(),
            position: Point::new(x2 as f64, y2 as f64),
            port: None,
        };
        scene.add_edge(SceneEdge {
            id: edge_id,
            from: rel.from.clone(),
            to: rel.to.clone(),
            route: Polyline::from_tuples(&[(x1 as f64, y1 as f64), (x2 as f64, y2 as f64)]),
            route_channel_ids: Vec::new(),
            source_anchor,
            target_anchor,
            labels: Vec::new(),
        });
    }

    scene
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ArchimateElement, ArchimateRelation};

    fn make_doc() -> ArchimateDocument {
        ArchimateDocument {
            title: Some("Test".to_string()),
            elements: vec![
                ArchimateElement {
                    name: "Customer".to_string(),
                    alias: Some("cust".to_string()),
                    layer: "business".to_string(),
                    kind: "actor".to_string(),
                    fill: None,
                    stroke: None,
                },
                ArchimateElement {
                    name: "Order Service".to_string(),
                    alias: Some("svc".to_string()),
                    layer: "application".to_string(),
                    kind: "service".to_string(),
                    fill: None,
                    stroke: None,
                },
                ArchimateElement {
                    name: "Database".to_string(),
                    alias: Some("db".to_string()),
                    layer: "technology".to_string(),
                    kind: "node".to_string(),
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
            warnings: Vec::new(),
        }
    }

    #[test]
    fn archimate_artifact_scene_node_count_matches_element_count() {
        let doc = make_doc();
        let artifact = render_archimate_artifact(&doc);

        // SVG must be present and non-empty.
        assert!(
            artifact.svg.contains("<svg"),
            "artifact must contain SVG markup"
        );

        // The scene must have exactly as many nodes as there are unique elements
        // in the document.  Aliases map to the same box so the canonical name is
        // the one inserted; we therefore expect one node per element.
        let scene = artifact
            .typed_scene()
            .expect("archimate artifact must carry a typed RenderScene");
        assert_eq!(
            scene.nodes.len(),
            doc.elements.len(),
            "scene node count must equal element count (got {} nodes for {} elements)",
            scene.nodes.len(),
            doc.elements.len()
        );
    }

    #[test]
    fn archimate_artifact_scene_has_no_geometry_issues() {
        let doc = make_doc();
        let artifact = render_archimate_artifact(&doc);
        let scene = artifact
            .typed_scene()
            .expect("archimate artifact must carry a typed RenderScene");
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene geometry validation must pass, got issues: {issues:?}"
        );
    }

    #[test]
    fn archimate_artifact_svg_is_byte_identical_to_render_archimate_svg() {
        let doc = make_doc();
        let svg_direct = render_archimate_svg(&doc);
        let artifact = render_archimate_artifact(&doc);
        assert_eq!(
            artifact.svg, svg_direct,
            "render_archimate_svg must be byte-identical to render_archimate_artifact(...).svg"
        );
    }
}
