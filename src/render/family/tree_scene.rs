use crate::model::{FamilyDocument, FamilyOrientation};
use crate::output::RenderArtifact;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge, SceneNode,
};

use super::tree::{wrap_text, NodeLayout};

/// Build a typed `RenderScene` for a family-tree diagram.
///
/// Nodes are placed with the same geometry as `render_family_tree_svg`.
/// Edges are routed as orthogonal polylines
/// `(x1,y1) → (x1,mid_y) → (x2,mid_y) → (x2,y2)` so they travel
/// horizontally only at the midpoint channel and never cut through sibling
/// node boxes.
pub(super) fn build_family_tree_scene(document: &FamilyDocument) -> RenderScene {
    const MARGIN: i32 = 24;
    const CHAR_WIDTH: i32 = 7;
    const NODE_MIN_WIDTH: i32 = 220;
    const NODE_MAX_WIDTH: i32 = 360;
    const NODE_PADDING_X: i32 = 12;
    const NODE_PADDING_Y: i32 = 12;
    const MIN_SPACING_X: i32 = 80;
    const MIN_SPACING_Y: i32 = 48;
    const MAX_LINE_CHARS: usize = 24;

    let title_lines = document
        .title
        .as_deref()
        .map(|v| v.lines().collect::<Vec<_>>())
        .unwrap_or_default();

    let hide_empty_members = document.hide_options.contains("empty members")
        || document.hide_options.contains("empty methods")
        || document.hide_options.contains("empty fields");

    let mut layouts = Vec::with_capacity(document.nodes.len());
    for node in &document.nodes {
        let raw_label = node.alias.as_ref().map_or_else(
            || node.name.clone(),
            |alias| format!("{} as {}", node.name, alias),
        );
        let lines = wrap_text(raw_label, MAX_LINE_CHARS, document.text_overflow_policy);
        let width_chars = lines
            .iter()
            .map(|line| line.chars().count() as i32)
            .max()
            .unwrap_or(1);
        let width =
            (width_chars * CHAR_WIDTH + (NODE_PADDING_X * 2)).clamp(NODE_MIN_WIDTH, NODE_MAX_WIDTH);
        let member_count = if hide_empty_members && node.members.is_empty() {
            0
        } else {
            node.members.len() as i32
        };
        let height = (lines.len() as i32 * 18) + (NODE_PADDING_Y * 2) + (member_count * 16);
        layouts.push(NodeLayout {
            label_lines: lines,
            width,
            height,
            x: 0,
            y: 0,
        });
    }

    let mut levels = Vec::<Vec<usize>>::new();
    let mut max_depth = 0usize;
    for (idx, node) in document.nodes.iter().enumerate() {
        let depth = node.depth;
        if depth > max_depth {
            max_depth = depth;
        }
        if levels.len() <= depth {
            levels.resize_with(depth + 1, Vec::new);
        }
        levels[depth].push(idx);
    }

    let mut depth_slot = vec![0usize; document.nodes.len()];
    for level_nodes in &levels {
        for (slot, idx) in level_nodes.iter().copied().enumerate() {
            depth_slot[idx] = slot;
        }
    }

    let max_node_width = layouts
        .iter()
        .map(|layout| layout.width)
        .max()
        .unwrap_or(NODE_MIN_WIDTH);
    let max_node_height = layouts
        .iter()
        .map(|layout| layout.height)
        .max()
        .unwrap_or(58);

    let x_step = max_node_width + MIN_SPACING_X;
    let y_step = max_node_height + MIN_SPACING_Y;

    let mut y_offsets = vec![0i32; levels.len()];
    for i in 1..levels.len() {
        let prev = y_offsets[i - 1] + y_step;
        y_offsets[i] = prev;
    }

    let vertical = matches!(
        document.orientation,
        FamilyOrientation::TopToBottom | FamilyOrientation::BottomToTop
    );

    let mut height_offset = MARGIN;
    if !title_lines.is_empty() {
        height_offset += (title_lines.len() as i32) * 24;
        height_offset += 12;
    }
    height_offset += (document.groups.len() as i32) * 48;

    for (depth, level_nodes) in levels.iter().enumerate() {
        for &node_idx in level_nodes {
            let slot = depth_slot[node_idx] as i32;
            let display_depth = match document.orientation {
                FamilyOrientation::TopToBottom => depth,
                FamilyOrientation::BottomToTop => max_depth.saturating_sub(depth),
                FamilyOrientation::LeftToRight => depth,
                FamilyOrientation::RightToLeft => max_depth.saturating_sub(depth),
            };

            if vertical {
                layouts[node_idx].x = MARGIN + (slot * x_step);
                layouts[node_idx].y = height_offset + (display_depth as i32 * y_step);
            } else {
                layouts[node_idx].x = MARGIN + (display_depth as i32 * x_step);
                layouts[node_idx].y = MARGIN + (slot * y_step);
            }
        }
    }

    let mut max_x = MARGIN;
    let mut max_y = height_offset;
    for layout in &layouts {
        max_x = max_x.max(layout.x + layout.width);
        max_y = max_y.max(layout.y + layout.height);
    }
    let width = (max_x + MARGIN).max(760);
    let height = (max_y + MARGIN).max(180);

    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width as f64, height as f64));

    // Add nodes
    for (idx, layout) in layouts.iter().enumerate() {
        let node = &document.nodes[idx];
        let node_id = node.name.clone();
        let bounds = Rect::new(
            layout.x as f64,
            layout.y as f64,
            layout.width as f64,
            layout.height as f64,
        );
        let label = LabelBox {
            id: format!("{node_id}:label"),
            text: node_id.clone(),
            bounds: Rect::new(
                (layout.x + NODE_PADDING_X) as f64,
                (layout.y + NODE_PADDING_Y) as f64,
                (layout.width - NODE_PADDING_X * 2) as f64,
                (layout.label_lines.len() as i32 * 18) as f64,
            ),
            owner_id: Some(node_id.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: node_id.clone(),
            node_box: NodeBox {
                id: node_id.clone(),
                bounds,
                ports: vec![],
                labels: vec![label],
            },
        });
    }

    // Add edges with orthogonal routes
    for (rel_idx, relation) in document.relations.iter().enumerate() {
        let from_idx = document
            .nodes
            .iter()
            .position(|node| node.name == relation.from)
            .or_else(|| {
                document
                    .nodes
                    .iter()
                    .position(|node| node.alias.as_deref() == Some(relation.from.as_str()))
            });
        let to_idx = document
            .nodes
            .iter()
            .position(|node| node.name == relation.to)
            .or_else(|| {
                document
                    .nodes
                    .iter()
                    .position(|node| node.alias.as_deref() == Some(relation.to.as_str()))
            });

        let (Some(from), Some(to)) = (from_idx, to_idx) else {
            continue;
        };

        let from_layout = &layouts[from];
        let to_layout = &layouts[to];
        let from_node = &document.nodes[from];
        let to_node = &document.nodes[to];
        let from_id = from_node.name.clone();
        let to_id = to_node.name.clone();

        // Determine endpoint coordinates (same as the SVG line, but route
        // is orthogonal to avoid crossing sibling node boxes).
        let (x1, y1, x2, y2) = match document.orientation {
            FamilyOrientation::TopToBottom => (
                from_layout.x + from_layout.width / 2,
                from_layout.y + from_layout.height,
                to_layout.x + to_layout.width / 2,
                to_layout.y,
            ),
            FamilyOrientation::BottomToTop => (
                from_layout.x + from_layout.width / 2,
                from_layout.y,
                to_layout.x + to_layout.width / 2,
                to_layout.y + to_layout.height,
            ),
            FamilyOrientation::LeftToRight => (
                from_layout.x + from_layout.width,
                from_layout.y + from_layout.height / 2,
                to_layout.x,
                to_layout.y + to_layout.height / 2,
            ),
            FamilyOrientation::RightToLeft => (
                from_layout.x,
                from_layout.y + from_layout.height / 2,
                to_layout.x + to_layout.width,
                to_layout.y + to_layout.height / 2,
            ),
        };

        // Orthogonal route: go out from source, jog to target column/row,
        // then enter target. For vertical layouts this is a 4-point U-channel
        // at the midpoint between the two depth levels.
        let route = if x1 == x2 {
            // Same column: straight vertical — no siblings to cross
            Polyline::from_tuples(&[(x1 as f64, y1 as f64), (x2 as f64, y2 as f64)])
        } else if vertical {
            // Different columns in a vertical layout: use midpoint channel
            let mid_y = (y1 + y2) / 2;
            Polyline::from_tuples(&[
                (x1 as f64, y1 as f64),
                (x1 as f64, mid_y as f64),
                (x2 as f64, mid_y as f64),
                (x2 as f64, y2 as f64),
            ])
        } else {
            // Horizontal layout: midpoint column channel
            let mid_x = (x1 + x2) / 2;
            Polyline::from_tuples(&[
                (x1 as f64, y1 as f64),
                (mid_x as f64, y1 as f64),
                (mid_x as f64, y2 as f64),
                (x2 as f64, y2 as f64),
            ])
        };

        let src_pos = Point::new(x1 as f64, y1 as f64);
        let tgt_pos = Point::new(x2 as f64, y2 as f64);
        let edge_id = format!("tree:rel:{rel_idx}");
        scene.add_edge(SceneEdge {
            id: edge_id.clone(),
            from: from_id.clone(),
            to: to_id.clone(),
            route,
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

pub(super) fn render_family_tree_artifact_inner(document: &FamilyDocument) -> RenderArtifact {
    let svg = super::tree::render_family_tree_svg_inner(document);
    let scene = build_family_tree_scene(document);
    RenderArtifact::with_scene(svg, scene)
}

#[cfg(test)]
mod tests {
    use crate::ast::DiagramKind;
    use crate::model::{
        FamilyDocument, FamilyNode, FamilyNodeKind, FamilyOrientation, FamilyRelation,
        FamilyRelationArrow, MindMapSide,
    };
    use crate::scene::TextOverflowPolicy;
    use crate::theme::SequenceStyle;
    use std::collections::BTreeSet;

    use super::render_family_tree_artifact_inner;

    fn make_family_tree_doc() -> FamilyDocument {
        // Three nodes: Root at depth=0, ChildA at depth=1 (slot 0), ChildB at depth=1 (slot 1).
        // With TopToBottom orientation, ChildA and ChildB are side by side at the same y-level.
        // A straight diagonal line from Root to ChildA (when x1 != x2) would clip through
        // ChildB's box without orthogonal routing.
        let nodes = vec![
            FamilyNode {
                kind: FamilyNodeKind::Class,
                name: "Root".to_string(),
                alias: None,
                members: vec![],
                depth: 0,
                label: None,
                mindmap_side: MindMapSide::Right,
                wbs_checkbox: None,
                fill_color: None,
            },
            FamilyNode {
                kind: FamilyNodeKind::Class,
                name: "ChildA".to_string(),
                alias: None,
                members: vec![],
                depth: 1,
                label: None,
                mindmap_side: MindMapSide::Right,
                wbs_checkbox: None,
                fill_color: None,
            },
            FamilyNode {
                kind: FamilyNodeKind::Class,
                name: "ChildB".to_string(),
                alias: None,
                members: vec![],
                depth: 1,
                label: None,
                mindmap_side: MindMapSide::Right,
                wbs_checkbox: None,
                fill_color: None,
            },
        ];
        let arrow = FamilyRelationArrow::parse("-->").unwrap();
        let relations = vec![
            FamilyRelation {
                from: "Root".to_string(),
                to: "ChildA".to_string(),
                arrow: arrow.clone(),
                label: None,
                stereotype: None,
                left_cardinality: None,
                right_cardinality: None,
                left_role: None,
                right_role: None,
                line_color: None,
                dashed: false,
                hidden: false,
                thickness: None,
                direction: None,
                left_lollipop: false,
                right_lollipop: false,
            },
            FamilyRelation {
                from: "Root".to_string(),
                to: "ChildB".to_string(),
                arrow,
                label: None,
                stereotype: None,
                left_cardinality: None,
                right_cardinality: None,
                left_role: None,
                right_role: None,
                line_color: None,
                dashed: false,
                hidden: false,
                thickness: None,
                direction: None,
                left_lollipop: false,
                right_lollipop: false,
            },
        ];
        FamilyDocument {
            kind: DiagramKind::Class,
            nodes,
            relations,
            groups: vec![],
            json_projections: vec![],
            hide_options: BTreeSet::new(),
            namespace_separator: None,
            title: None,
            header: None,
            footer: None,
            caption: None,
            legend: None,
            mainframe: None,
            scale: None,
            orientation: FamilyOrientation::TopToBottom,
            style: SequenceStyle::default(),
            family_style: None,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
            maximum_width: None,
            sprites: Default::default(),
            list_sprites: false,
            warnings: vec![],
        }
    }

    #[test]
    fn family_tree_artifact_scene_node_count_matches_tree_nodes_and_geometry_is_valid() {
        // Root → ChildA and Root → ChildB: siblings at depth 1.
        // With a straight diagonal line Root→ChildA would clip through ChildB's box
        // when they're at different x positions.  The orthogonal route avoids this.
        let family = make_family_tree_doc();
        let artifact = render_family_tree_artifact_inner(&family);
        let scene = artifact.scene.expect("family_tree scene must be present");

        assert_eq!(
            scene.nodes.len(),
            family.nodes.len(),
            "scene node count must match document node count"
        );
        assert_eq!(
            scene.edges.len(),
            family.relations.len(),
            "scene edge count must match document relation count"
        );

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "family_tree scene must have no geometry issues: {issues:?}"
        );
    }
}
