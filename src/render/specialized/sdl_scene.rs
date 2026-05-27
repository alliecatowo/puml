use super::*;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge, SceneNode,
};

use super::sdl::{sdl_transition_endpoints, SdlNodeBox};

/// Build a typed [`RenderScene`] from SDL's laid-out geometry. Node bounds use
/// the same `SdlNodeBox` positions the SVG draws; edge routes use the same
/// `sdl_transition_endpoints` border-to-border segments, so scene and SVG never
/// diverge.
pub(super) fn build_sdl_scene(
    positions: &BTreeMap<&str, SdlNodeBox>,
    transitions: &[crate::model::SdlTransition],
    width: f64,
    height: f64,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width, height));

    for (&name, &node) in positions {
        let bounds = Rect::new(node.x as f64, node.y as f64, node.w as f64, node.h as f64);
        let label = LabelBox {
            id: format!("{name}::label"),
            text: name.to_string(),
            bounds,
            owner_id: Some(name.to_string()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: name.to_string(),
            node_box: NodeBox {
                id: name.to_string(),
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    for (idx, tr) in transitions.iter().enumerate() {
        let id = format!("tr{idx}");
        push_sdl_scene_edge(&mut scene, positions, id, &tr.from, &tr.to);
    }

    scene
}

pub(super) fn push_sdl_scene_edge(
    scene: &mut RenderScene,
    positions: &BTreeMap<&str, SdlNodeBox>,
    id: String,
    from: &str,
    to: &str,
) {
    let (Some(&from_box), Some(&to_box)) = (positions.get(from), positions.get(to)) else {
        return;
    };
    let (x1, y1, x2, y2) = sdl_transition_endpoints(from_box, to_box);
    let x1 = x1 as f64;
    let y1 = y1 as f64;
    let x2 = x2 as f64;
    let y2 = y2 as f64;
    let source_anchor = Anchor {
        id: format!("{id}::src"),
        owner_id: from.to_string(),
        position: Point::new(x1, y1),
        port: None,
    };
    let target_anchor = Anchor {
        id: format!("{id}::tgt"),
        owner_id: to.to_string(),
        position: Point::new(x2, y2),
        port: None,
    };
    scene.add_edge(SceneEdge {
        id,
        from: from.to_string(),
        to: to.to_string(),
        route: Polyline::from_tuples(&[(x1, y1), (x2, y2)]),
        route_channel_ids: Vec::new(),
        source_anchor,
        target_anchor,
        labels: Vec::new(),
    });
}

#[cfg(test)]
mod tests {
    use crate::{normalize_family, parser, NormalizedDocument};

    use super::super::sdl::{render_sdl_artifact, render_sdl_svg};

    #[test]
    fn sdl_artifact_scene_node_count_matches_state_count_and_geometry_is_valid() {
        // Three states, two transitions.
        let src = "@startsdl\nstate Ready\nstate Running\nstate Done\nReady -> Running : start\nRunning -> Done : finish\n@endsdl\n";
        let parsed = parser::parse(src).expect("parse sdl");
        let NormalizedDocument::Sdl(doc) = normalize_family(parsed).expect("normalize sdl") else {
            panic!("expected sdl model");
        };

        let artifact = render_sdl_artifact(&doc);

        // SVG must still be produced.
        assert!(artifact.svg.contains("sdl-node"), "SVG contains sdl nodes");

        let scene = artifact.scene.expect("sdl scene must be present");

        // One SceneNode per state.
        assert_eq!(
            scene.nodes.len(),
            doc.states.len(),
            "scene node count == state count"
        );

        // One SceneEdge per transition.
        assert_eq!(
            scene.edges.len(),
            doc.transitions.len(),
            "scene edge count == transition count"
        );

        // Geometry must be valid (no out-of-viewport nodes or degenerate edges).
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "validate_geometry() must be clean; got: {issues:?}"
        );
    }

    #[test]
    fn sdl_svg_and_artifact_svg_are_byte_identical() {
        let src = "@startsdl\nstate A\nstate B\nA -> B : go\n@endsdl\n";
        let parsed = parser::parse(src).expect("parse sdl");
        let NormalizedDocument::Sdl(doc) = normalize_family(parsed).expect("normalize sdl") else {
            panic!("expected sdl model");
        };
        let svg_direct = render_sdl_svg(&doc);
        let artifact = render_sdl_artifact(&doc);
        assert_eq!(
            svg_direct, artifact.svg,
            "render_sdl_svg and render_sdl_artifact must produce identical SVG"
        );
    }
}
