use super::*;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, LaneFrame, Point, Polyline, Rect, RenderScene, SceneEdge,
};
use model::timing_relation_endpoint;

mod axes;
mod messages;
mod model;
mod rows;
mod svg_emit;

use axes::render_timing_axis;
use messages::render_timing_relations;
use model::{TimingLayout, TimingModel};
use rows::{render_timing_rows, signal_row_midpoints};
use svg_emit::{render_timing_footer_caption, render_timing_svg_header};

pub fn render_timing_svg(doc: &FamilyDocument) -> String {
    render_timing_artifact(doc).svg
}

pub fn render_timing_artifact(doc: &FamilyDocument) -> crate::output::RenderArtifact {
    let default_timing_style;
    let style = match &doc.family_style {
        Some(crate::model::FamilyStyle::Timing(style)) => style,
        _ => {
            default_timing_style = crate::theme::TimingStyle::default();
            &default_timing_style
        }
    };

    let model = TimingModel::from_document(doc);
    let layout = TimingLayout::new(doc, &model, style);

    let mut out = render_timing_svg_header(doc, style, &layout);

    render_timing_axis(&mut out, &model, &layout, style);

    let signal_row_mid = signal_row_midpoints(&model.signals, &layout);
    render_timing_rows(&mut out, &model, &layout, style);

    render_timing_relations(
        &mut out,
        doc,
        &signal_row_mid,
        layout.axis_top,
        layout.signals_top + layout.rows_h(),
        &|time| layout.time_to_x(time),
        style,
    );

    render_timing_footer_caption(&mut out, doc, style, &layout);

    out.push_str("</svg>");

    let scene = build_timing_scene(doc, &model, &layout, &signal_row_mid);
    crate::output::RenderArtifact::with_scene(out, scene)
}

/// Build a typed `RenderScene` that mirrors timing diagram geometry.
///
/// Signal rows are modelled as `LaneFrame`s (not `SceneNode`s) so that
/// message arrows between rows do not trigger `EdgeCrossesNode` in the
/// geometry validator.  Edge `from`/`to` are the lane ids (signal names),
/// which the endpoint-exclusion logic skips correctly.
fn build_timing_scene(
    doc: &FamilyDocument,
    model: &TimingModel<'_>,
    layout: &TimingLayout,
    signal_row_mid: &BTreeMap<String, i32>,
) -> RenderScene {
    let viewport = Rect::new(0.0, 0.0, layout.width as f64, layout.height as f64);
    let mut scene = RenderScene::new(viewport);

    // Each signal row is a lane.
    for (idx, signal) in model.signals.iter().enumerate() {
        let row_y = layout.signals_top + (idx as i32) * layout.row_h;
        let lane_id = signal.name.clone();
        let label_text = signal.label.as_deref().unwrap_or(&signal.name).to_string();
        scene.add_lane(LaneFrame {
            id: lane_id.clone(),
            bounds: Rect::new(0.0, row_y as f64, layout.width as f64, layout.row_h as f64),
            header: None,
            child_node_ids: vec![],
            labels: vec![LabelBox {
                id: format!("{lane_id}:label"),
                text: label_text,
                bounds: Rect::new(
                    0.0,
                    row_y as f64,
                    (layout.left_pad - 8) as f64,
                    layout.row_h as f64,
                ),
                owner_id: Some(lane_id.clone()),
                role: LabelRole::Lane,
            }],
        });
        // Register alias and label as lookups pointing to the same lane id.
        // (Not needed for scene geometry, but useful for debugging.)
        let _ = signal.alias.as_deref();
    }

    // Each relation becomes an edge between two lane ids.
    for (rel_idx, relation) in doc.relations.iter().enumerate() {
        let Some((from_signal, from_time)) = timing_relation_endpoint(&relation.from) else {
            continue;
        };
        let Some((to_signal, to_time)) = timing_relation_endpoint(&relation.to) else {
            continue;
        };

        // Resolve to the canonical signal name used as the lane id.
        let from_lookup = from_signal.to_ascii_lowercase();
        let to_lookup = to_signal.to_ascii_lowercase();
        let Some(&y1_mid) = signal_row_mid.get(&from_lookup) else {
            continue;
        };
        let Some(&y2_mid) = signal_row_mid.get(&to_lookup) else {
            continue;
        };

        // Find the canonical lane id (signal.name, not the lowercased lookup).
        let from_lane_id = model
            .signals
            .iter()
            .find(|s| {
                s.name.to_ascii_lowercase() == from_lookup
                    || s.alias
                        .as_deref()
                        .map(|a| a.to_ascii_lowercase())
                        .as_deref()
                        == Some(from_lookup.as_str())
                    || s.label
                        .as_deref()
                        .map(|l| l.to_ascii_lowercase())
                        .as_deref()
                        == Some(from_lookup.as_str())
            })
            .map(|s| s.name.clone())
            .unwrap_or_else(|| from_signal.to_string());
        let to_lane_id = model
            .signals
            .iter()
            .find(|s| {
                s.name.to_ascii_lowercase() == to_lookup
                    || s.alias
                        .as_deref()
                        .map(|a| a.to_ascii_lowercase())
                        .as_deref()
                        == Some(to_lookup.as_str())
                    || s.label
                        .as_deref()
                        .map(|l| l.to_ascii_lowercase())
                        .as_deref()
                        == Some(to_lookup.as_str())
            })
            .map(|s| s.name.clone())
            .unwrap_or_else(|| to_signal.to_string());

        let x1 = layout.time_to_x(from_time);
        let x2 = layout.time_to_x(to_time);
        let lane_inset = 16i32;
        let (y1, y2) = if y2_mid > y1_mid {
            (y1_mid + lane_inset, y2_mid - lane_inset)
        } else if y2_mid < y1_mid {
            (y1_mid - lane_inset, y2_mid + lane_inset)
        } else {
            (y1_mid, y2_mid)
        };

        let edge_id = format!("timing:rel:{rel_idx}");
        let src_pos = Point::new(x1 as f64, y1 as f64);
        let tgt_pos = Point::new(x2 as f64, y2 as f64);
        scene.add_edge(SceneEdge {
            id: edge_id.clone(),
            from: from_lane_id.clone(),
            to: to_lane_id.clone(),
            route: Polyline::from_tuples(&[(x1 as f64, y1 as f64), (x2 as f64, y2 as f64)]),
            route_channel_ids: vec![],
            source_anchor: Anchor {
                id: format!("{edge_id}:source"),
                owner_id: from_lane_id,
                position: src_pos,
                port: None,
            },
            target_anchor: Anchor {
                id: format!("{edge_id}:target"),
                owner_id: to_lane_id,
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
    use crate::{normalize_family, parser, NormalizedDocument};

    #[test]
    fn timing_artifact_with_relations_has_correct_edge_count() {
        let src = "@startuml\n\
            robust \"Bus\" as B\n\
            concise \"CPU\" as C\n\
            @0\n\
            B is Idle\n\
            C is Idle\n\
            @10\n\
            B is Busy\n\
            @20\n\
            C is Active\n\
            B@10 -> C@20 : req\n\
            @enduml\n";
        let parsed = parser::parse(src).expect("parse timing");
        let NormalizedDocument::Family(family) =
            normalize_family(parsed).expect("normalize timing")
        else {
            panic!("expected family model for timing");
        };

        let artifact = render_timing_artifact(&family);
        let scene = artifact.scene.expect("timing scene must be present");

        // The two signals become lanes, not nodes
        assert_eq!(scene.nodes.len(), 0, "timing rows must be lanes, not nodes");
        assert_eq!(scene.lanes.len(), 2, "expected 2 signal lanes");

        // One relation → one edge
        assert_eq!(
            scene.edges.len(),
            family.relations.len(),
            "edge count must equal relation count"
        );

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "timing scene must have no geometry issues: {issues:?}"
        );
    }
}
