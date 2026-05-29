use std::collections::{BTreeMap, BTreeSet};

use super::{creole_text, escape_text, RenderArtifact};
use crate::model::{WireComponent, WireDocument, WireEndpoint, WirePort, WirePortSide};
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Port, PortSide, Rect, RenderScene,
    SceneEdge, SceneNode, Size,
};

const MARGIN: f64 = 28.0;
const TITLE_H: f64 = 28.0;
const PORT_R: f64 = 4.0;

#[derive(Debug, Clone)]
struct WireLayout {
    width: f64,
    height: f64,
    title_offset: f64,
    ports: BTreeMap<String, Point>,
}

pub fn render_wire_svg(document: &WireDocument) -> String {
    render_wire_artifact(document).svg
}

pub fn render_wire_artifact(document: &WireDocument) -> RenderArtifact {
    let layout = compute_layout(document);
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.0}\" height=\"{:.0}\" viewBox=\"0 0 {:.0} {:.0}\">",
        layout.width, layout.height, layout.width, layout.height
    ));
    out.push_str(&format!(
        "<rect width=\"{:.0}\" height=\"{:.0}\" fill=\"#ffffff\"/>",
        layout.width, layout.height
    ));
    if let Some(header) = &document.header {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"18\" font-family=\"monospace\" font-size=\"11\" fill=\"#64748b\">{}</text>",
            MARGIN,
            escape_text(header)
        ));
    }
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"15\" font-weight=\"700\" fill=\"#0f172a\">{}</text>",
            layout.width / 2.0,
            if document.header.is_some() { 38 } else { 22 },
            escape_text(title)
        ));
    }
    out.push_str("<defs><marker id=\"wire-arrow\" markerWidth=\"8\" markerHeight=\"8\" refX=\"7\" refY=\"4\" orient=\"auto\"><path d=\"M0,0 L8,4 L0,8 Z\" fill=\"#334155\"/></marker></defs>");

    for link in &document.links {
        let Some(points) = link_points(&link.from, &link.to, &layout.ports, document) else {
            continue;
        };
        let attr = if link.directed {
            " marker-end=\"url(#wire-arrow)\""
        } else {
            ""
        };
        out.push_str(&format!(
            "<polyline class=\"wire-link\" points=\"{}\" fill=\"none\" stroke=\"#334155\" stroke-width=\"1.5\"{} />",
            points
                .iter()
                .map(|p| format!("{:.1},{:.1}", p.x, p.y + layout.title_offset))
                .collect::<Vec<_>>()
                .join(" "),
            attr
        ));
        if let Some(label) = &link.label {
            let mut mid = link_label_point(&points);
            mid.y += if link.directed { 22.0 } else { -18.0 };
            out.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{}\" height=\"16\" rx=\"2\" fill=\"#ffffff\" opacity=\"0.9\"/>",
                mid.x - text_width(label) / 2.0 - 4.0,
                mid.y + layout.title_offset - 14.0,
                text_width(label) + 8.0
            ));
            out.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#334155\">{}</text>",
                mid.x,
                mid.y + layout.title_offset - 2.0,
                escape_text(label)
            ));
        }
    }

    for label in &document.labels {
        out.push_str(&creole_text(
            (label.x + MARGIN) as i32,
            (label.y + layout.title_offset) as i32,
            "font-family=\"monospace\" font-size=\"11\"",
            &label.text,
            "#475569",
        ));
    }

    // Collect port-label identifiers that are "covered" by a same-named edge
    // label so port labels are suppressed in those cases (#1301).
    // A port label is considered covered when every link that touches it
    // carries an identical `label` — i.e. the wire already names the signal.
    let covered_ports: BTreeSet<String> = {
        let mut candidate_port_labels: BTreeMap<String, bool> = BTreeMap::new();
        for link in &document.links {
            let edge_label = link.label.as_deref().unwrap_or("");
            for endpoint in [&link.from, &link.to] {
                if let Some(port_label) = &endpoint.port {
                    let key = format!("{}::{}", endpoint.component, port_label);
                    let covered = edge_label == port_label;
                    candidate_port_labels
                        .entry(key)
                        .and_modify(|v| *v = *v && covered)
                        .or_insert(covered);
                }
            }
        }
        candidate_port_labels
            .into_iter()
            .filter_map(|(k, v)| if v { Some(k) } else { None })
            .collect()
    };
    for component in &document.components {
        render_component_svg(&mut out, component, &layout, &covered_ports);
    }

    if let Some(caption) = &document.caption {
        out.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#64748b\">{}</text>",
            layout.width / 2.0,
            layout.height - 10.0,
            escape_text(caption)
        ));
    }
    if let Some(footer) = &document.footer {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{:.1}\" font-family=\"monospace\" font-size=\"11\" fill=\"#64748b\">{}</text>",
            MARGIN,
            layout.height - 10.0,
            escape_text(footer)
        ));
    }
    out.push_str("</svg>");

    RenderArtifact::with_scene(out, build_scene(document, &layout))
}

fn render_component_svg(
    out: &mut String,
    component: &WireComponent,
    layout: &WireLayout,
    covered_ports: &BTreeSet<String>,
) {
    let x = component.x + MARGIN;
    let y = component.y + layout.title_offset;
    let fill = component.color.as_deref().unwrap_or("#f8fafc");
    out.push_str(&format!(
        "<rect class=\"wire-component\" data-wire-id=\"{}\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"4\" fill=\"{}\" stroke=\"#475569\" stroke-width=\"1.5\"/>",
        escape_text(&component.id),
        component.width,
        component.height,
        fill
    ));
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#0f172a\">{}</text>",
        x + component.width / 2.0,
        y + component.height / 2.0 + 4.0,
        escape_text(&component.label)
    ));
    for port in &component.ports {
        let p = port_position(component, port);
        let px = p.x + MARGIN;
        let py = p.y + layout.title_offset;
        out.push_str(&format!(
            "<circle class=\"wire-port\" data-wire-port=\"{}\" cx=\"{px:.1}\" cy=\"{py:.1}\" r=\"{PORT_R}\" fill=\"#ffffff\" stroke=\"#0f766e\" stroke-width=\"1.5\"/>",
            escape_text(&port.id)
        ));
        // Suppress port label when an edge already carries the same text on
        // every wire that touches this port — avoids strikethrough overlap
        // (issue #1301).
        let port_key = format!("{}::{}", component.id, port.label);
        if covered_ports.contains(&port_key) {
            continue;
        }
        let (tx, anchor) = match port.side {
            WirePortSide::Left => (px - 8.0, "end"),
            WirePortSide::Right => (px + 8.0, "start"),
            WirePortSide::Top | WirePortSide::Bottom => (px, "middle"),
        };
        let ty = match port.side {
            WirePortSide::Top => py - 7.0,
            WirePortSide::Bottom => py + 14.0,
            WirePortSide::Left | WirePortSide::Right => py + 4.0,
        };
        out.push_str(&format!(
            "<text x=\"{tx:.1}\" y=\"{ty:.1}\" text-anchor=\"{anchor}\" font-family=\"monospace\" font-size=\"10\" fill=\"#0f766e\">{}</text>",
            escape_text(&port.label)
        ));
    }
}

fn compute_layout(document: &WireDocument) -> WireLayout {
    let title_offset = if document.title.is_some() || document.header.is_some() {
        TITLE_H + if document.header.is_some() { 18.0 } else { 0.0 }
    } else {
        0.0
    };
    let mut max_x: f64 = 160.0;
    let mut max_y: f64 = 80.0;
    let mut ports = BTreeMap::new();
    for component in &document.components {
        max_x = max_x.max(component.x + component.width + MARGIN * 2.0);
        max_y = max_y.max(component.y + component.height + MARGIN * 2.0 + title_offset);
        for port in &component.ports {
            ports.insert(
                port_key(&component.id, &port.label),
                port_position(component, port),
            );
        }
    }
    for label in &document.labels {
        max_x = max_x.max(label.x + text_width(&label.text) + MARGIN * 2.0);
        max_y = max_y.max(label.y + 24.0 + title_offset);
    }
    if document.caption.is_some() || document.footer.is_some() {
        max_y += 20.0;
    }
    WireLayout {
        width: max_x.ceil(),
        height: max_y.ceil(),
        title_offset,
        ports,
    }
}

fn port_position(component: &WireComponent, port: &WirePort) -> Point {
    let side_count = component
        .ports
        .iter()
        .filter(|candidate| candidate.side == port.side)
        .count()
        .max(1);
    let slot = (port.order + 1) as f64 / (side_count + 1) as f64;
    match port.side {
        WirePortSide::Left => Point::new(component.x, component.y + component.height * slot),
        WirePortSide::Right => Point::new(
            component.x + component.width,
            component.y + component.height * slot,
        ),
        WirePortSide::Top => Point::new(component.x + component.width * slot, component.y),
        WirePortSide::Bottom => Point::new(
            component.x + component.width * slot,
            component.y + component.height,
        ),
    }
}

fn link_points(
    from: &WireEndpoint,
    to: &WireEndpoint,
    ports: &BTreeMap<String, Point>,
    document: &WireDocument,
) -> Option<Vec<Point>> {
    let start = endpoint_point(from, ports, document)?;
    let end = endpoint_point(to, ports, document)?;
    let mid_x = (start.x + end.x) / 2.0;
    Some(vec![
        Point::new(start.x + MARGIN, start.y),
        Point::new(mid_x + MARGIN, start.y),
        Point::new(mid_x + MARGIN, end.y),
        Point::new(end.x + MARGIN, end.y),
    ])
}

fn endpoint_point(
    endpoint: &WireEndpoint,
    ports: &BTreeMap<String, Point>,
    document: &WireDocument,
) -> Option<Point> {
    if let Some(port) = &endpoint.port {
        if let Some(point) = ports.get(&port_key(&endpoint.component, port)) {
            return Some(*point);
        }
    }
    let component = document
        .components
        .iter()
        .find(|component| component.id == endpoint.component)?;
    Some(Point::new(
        component.x + component.width / 2.0,
        component.y + component.height / 2.0,
    ))
}

fn build_scene(document: &WireDocument, layout: &WireLayout) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, layout.width, layout.height));
    for component in &document.components {
        let bounds = Rect::new(
            component.x + MARGIN,
            component.y + layout.title_offset,
            component.width,
            component.height,
        );
        let label = LabelBox {
            id: format!("{}:label", component.id),
            text: component.label.clone(),
            bounds: Rect::new(
                bounds.center().x - text_width(&component.label) / 2.0,
                bounds.center().y - 8.0,
                text_width(&component.label),
                14.0,
            ),
            owner_id: Some(component.id.clone()),
            role: LabelRole::Node,
        };
        let ports = component
            .ports
            .iter()
            .map(|port| {
                let p = port_position(component, port);
                Port {
                    id: port.id.clone(),
                    node_id: component.id.clone(),
                    side: port_side(port.side),
                    position: Point::new(p.x + MARGIN, p.y + layout.title_offset),
                }
            })
            .collect::<Vec<_>>();
        scene.add_node(SceneNode {
            id: component.id.clone(),
            node_box: NodeBox {
                id: component.id.clone(),
                bounds,
                ports,
                labels: vec![label],
            },
        });
    }
    for label in &document.labels {
        scene.add_label_box(LabelBox {
            id: label.id.clone(),
            text: label.text.clone(),
            bounds: Rect::new(
                label.x + MARGIN,
                label.y + layout.title_offset - 12.0,
                text_width(&label.text),
                16.0,
            ),
            owner_id: None,
            role: LabelRole::Other,
        });
    }
    for link in &document.links {
        let Some(points) = link_points(&link.from, &link.to, &layout.ports, document) else {
            continue;
        };
        let shifted = points
            .iter()
            .map(|point| Point::new(point.x, point.y + layout.title_offset))
            .collect::<Vec<_>>();
        let source = *shifted.first().unwrap_or(&Point::new(0.0, 0.0));
        let target = *shifted.last().unwrap_or(&Point::new(0.0, 0.0));
        let labels = link
            .label
            .as_ref()
            .map(|text| {
                let mut mid = link_label_point(&shifted);
                mid.y += if link.directed { 22.0 } else { -18.0 };
                vec![LabelBox {
                    id: format!("{}:label", link.id),
                    text: text.clone(),
                    bounds: Rect::new(
                        mid.x - text_width(text) / 2.0,
                        mid.y - 16.0,
                        text_width(text),
                        14.0,
                    ),
                    owner_id: Some(link.id.clone()),
                    role: LabelRole::Edge,
                }]
            })
            .unwrap_or_default();
        scene.add_edge(SceneEdge {
            id: link.id.clone(),
            from: link.from.component.clone(),
            to: link.to.component.clone(),
            route: Polyline::new(shifted),
            route_channel_ids: Vec::new(),
            source_anchor: Anchor {
                id: format!("{}:source", link.id),
                owner_id: link.from.component.clone(),
                position: source,
                port: None,
            },
            target_anchor: Anchor {
                id: format!("{}:target", link.id),
                owner_id: link.to.component.clone(),
                position: target,
                port: None,
            },
            labels,
        });
    }
    scene
}

fn port_side(side: WirePortSide) -> PortSide {
    match side {
        WirePortSide::Top => PortSide::Top,
        WirePortSide::Right => PortSide::Right,
        WirePortSide::Bottom => PortSide::Bottom,
        WirePortSide::Left => PortSide::Left,
    }
}

fn port_key(component: &str, port: &str) -> String {
    format!("{component}:{}", port.trim())
}

fn text_width(text: &str) -> f64 {
    text.lines()
        .map(|line| crate::render_core::text_metrics::estimate_text_width_f64(line, 14.0))
        .fold(8.0, f64::max)
}

fn link_label_point(points: &[Point]) -> Point {
    let first = points.first().copied().unwrap_or(Point::new(0.0, 0.0));
    let last = points.last().copied().unwrap_or(first);
    Point::new((first.x + last.x) / 2.0, (first.y + last.y) / 2.0)
}

#[allow(dead_code)]
fn _size(width: f64, height: f64) -> Size {
    Size::new(width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{normalize_family, parser, NormalizedDocument};

    #[test]
    fn wire_renderer_exposes_typed_scene_nodes_edges_and_ports() {
        let src = "@startwire\ncomponent Panel [100x80] right:A\n--\ncomponent FPGA [120x80] left:A\nPanel.A -- FPGA.A : bus\n@endwire\n";
        let parsed = parser::parse(src).expect("parse wire");
        let NormalizedDocument::Wire(wire) = normalize_family(parsed).expect("normalize wire")
        else {
            panic!("expected wire model");
        };
        let artifact = render_wire_artifact(&wire);

        assert!(artifact.svg.contains("wire-component"));
        let scene = artifact.scene.expect("wire scene");
        assert_eq!(scene.nodes.len(), 2);
        assert_eq!(scene.edges.len(), 1);
        assert_eq!(scene.nodes["Panel"].node_box.ports.len(), 1);
        assert!(scene.validate_geometry().is_empty());
    }
}
