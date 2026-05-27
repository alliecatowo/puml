use super::*;
use crate::output::RenderArtifact;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge, SceneNode,
};

mod axes;
mod messages;
mod model;
mod rows;
mod svg_emit;

use axes::render_timing_axis;
use messages::render_timing_relations;
use model::{timing_relation_endpoint, TimingLayout, TimingModel};
use rows::{render_timing_rows, signal_row_midpoints};
use svg_emit::{render_timing_footer_caption, render_timing_svg_header};

pub fn render_timing_svg(doc: &FamilyDocument) -> String {
    render_timing_artifact(doc).svg
}

/// Render a timing diagram into a typed [`RenderArtifact`].
///
/// The SVG is emitted exactly as before (byte-identical to `render_timing_svg`).
/// In addition, a [`RenderScene`] is built from the *same* laid-out geometry the
/// SVG uses — one `SceneNode` per signal row at its exact row rectangle, and one
/// `SceneEdge` per timing relation along the same line segment the SVG draws —
/// so the scene stays consistent with the output without any SVG drift.
pub fn render_timing_artifact(doc: &FamilyDocument) -> RenderArtifact {
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

    let scene = build_timing_scene(&model, &layout, doc, &signal_row_mid);
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from timing's laid-out geometry.
///
/// Each signal row becomes a `SceneNode` whose bounds exactly match the row
/// rectangle drawn by `render_row_background` (`x=0, y=row_y, width, row_h`).
/// Each timing relation becomes a `SceneEdge` along the same line segment that
/// `render_timing_relations` draws (using the identical coordinate computation).
fn build_timing_scene(
    model: &TimingModel<'_>,
    layout: &TimingLayout,
    doc: &FamilyDocument,
    signal_row_mid: &BTreeMap<String, i32>,
) -> RenderScene {
    let viewport = Rect::new(0.0, 0.0, layout.width as f64, layout.height as f64);
    let mut scene = RenderScene::new(viewport);

    // One SceneNode per signal row — bounds match render_row_background exactly.
    for (row_idx, signal) in model.signals.iter().enumerate() {
        let row_y = layout.signals_top + (row_idx as i32) * layout.row_h;
        let id = format!("row{row_idx}");
        let bounds = Rect::new(
            0.0,
            row_y as f64,
            layout.width as f64,
            layout.row_h as f64,
        );
        let signal_label = signal.label.as_deref().unwrap_or(&signal.name);
        let label = LabelBox {
            id: format!("{id}::label"),
            text: signal_label.to_string(),
            bounds,
            owner_id: Some(id.clone()),
            role: LabelRole::Lane,
        };
        scene.add_node(SceneNode {
            id: id.clone(),
            node_box: NodeBox {
                id: id.clone(),
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    // One SceneEdge per timing relation — coordinates mirror render_timing_relations.
    for (rel_idx, relation) in doc.relations.iter().enumerate() {
        let Some((from_signal, from_time)) = timing_relation_endpoint(&relation.from) else {
            continue;
        };
        let Some((to_signal, to_time)) = timing_relation_endpoint(&relation.to) else {
            continue;
        };
        let from_lookup = from_signal.to_ascii_lowercase();
        let to_lookup = to_signal.to_ascii_lowercase();
        let Some(&y1_mid) = signal_row_mid.get(&from_lookup) else {
            continue;
        };
        let Some(&y2_mid) = signal_row_mid.get(&to_lookup) else {
            continue;
        };
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
        let id = format!("rel{rel_idx}");
        let (x1f, y1f, x2f, y2f) = (x1 as f64, y1 as f64, x2 as f64, y2 as f64);
        let source_anchor = Anchor {
            id: format!("{id}::src"),
            owner_id: from_signal.to_string(),
            position: Point::new(x1f, y1f),
            port: None,
        };
        let target_anchor = Anchor {
            id: format!("{id}::tgt"),
            owner_id: to_signal.to_string(),
            position: Point::new(x2f, y2f),
            port: None,
        };
        scene.add_edge(SceneEdge {
            id,
            from: from_signal.to_string(),
            to: to_signal.to_string(),
            route: Polyline::from_tuples(&[(x1f, y1f), (x2f, y2f)]),
            route_channel_ids: Vec::new(),
            source_anchor,
            target_anchor,
            labels: Vec::new(),
        });
    }

    scene
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_core::SceneAvailability;

    fn parse_timing_doc(src: &str) -> FamilyDocument {
        use crate::normalize::normalize_family;
        use crate::parser::parse;
        let parsed = parse(src).expect("parse failed");
        let normalized = normalize_family(parsed).expect("normalize failed");
        match normalized {
            crate::model::NormalizedDocument::Family(doc) => doc,
            other => panic!("expected Family document, got {other:?}"),
        }
    }

    #[test]
    fn timing_artifact_has_typed_scene_with_correct_node_count() {
        let src = r#"
@startuml
robust "Bus" as B
robust "Clock" as C
robust "Data" as D

@0
B is Idle
C is Low
D is Wait

@5
B is Active
C is High

@10
B is Idle
C is Low
D is Done

@enduml
"#;
        let doc = parse_timing_doc(src);
        let n_signals = doc
            .nodes
            .iter()
            .filter(|n| {
                matches!(
                    n.kind,
                    crate::model::FamilyNodeKind::TimingConcise
                        | crate::model::FamilyNodeKind::TimingRobust
                        | crate::model::FamilyNodeKind::TimingClock
                        | crate::model::FamilyNodeKind::TimingBinary
                )
            })
            .count();

        let artifact = render_timing_artifact(&doc);

        // Scene must be typed.
        assert_eq!(
            artifact.scene_availability,
            SceneAvailability::TypedScene,
            "timing artifact must report TypedScene"
        );

        let scene = artifact.scene.as_ref().expect("scene must be present");

        // One SceneNode per signal row.
        assert_eq!(
            scene.nodes.len(),
            n_signals,
            "scene node count ({}) must match signal count ({})",
            scene.nodes.len(),
            n_signals
        );

        // Geometry must be valid — no issues tolerated.
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene reported geometry issues: {issues:?}"
        );
    }

    #[test]
    fn timing_svg_is_byte_identical_after_migration() {
        let src = r#"
@startuml
concise "Input" as I
binary "Enable" as E

@0
I is Idle
E is 0

@10
I is Active
E is 1

@20
I is Idle
E is 0

@enduml
"#;
        let doc = parse_timing_doc(src);
        // SVG from the old path (via legacy entry point) must equal artifact.svg.
        let legacy_svg = render_timing_svg(&doc);
        let artifact = render_timing_artifact(&doc);
        assert_eq!(
            legacy_svg, artifact.svg,
            "render_timing_svg must produce byte-identical output to render_timing_artifact"
        );
    }

    #[test]
    fn timing_artifact_with_relations_has_correct_edge_count() {
        let src = r#"
@startuml
robust "Signal A" as A
robust "Signal B" as B

@0
A is Low
B is Low

@5
A is High

@10
B is High

A@5 -> B@10 : trigger

@enduml
"#;
        let doc = parse_timing_doc(src);
        let artifact = render_timing_artifact(&doc);
        let scene = artifact.scene.as_ref().expect("scene must be present");

        // We have 1 relation defined; expect 1 edge.
        assert_eq!(
            scene.edges.len(),
            doc.relations.len(),
            "edge count ({}) must match relation count ({})",
            scene.edges.len(),
            doc.relations.len()
        );

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene with relation reported geometry issues: {issues:?}"
        );
    }
}
