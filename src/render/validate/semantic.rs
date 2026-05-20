use super::edge::{extract_relation_segments_with_class, Segment};
use super::svg::{
    extract_attr_str, extract_text_content_at, parse_attr_i32, parse_bbox, parse_points_bbox,
    svg_element_tags, svg_element_tags_with_pos, sync_svg_dimensions, tag_has_class,
};
use super::{GraphValidationProfile, InvariantKind, InvariantViolation, SemanticRole};

/// A node bounding box scraped from SVG semantic hook attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeBbox {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A canonical semantic node extracted from a `puml-node` SVG hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticNode {
    pub id: String,
    pub kind: Option<String>,
    pub bbox: NodeBbox,
}

/// A canonical semantic edge extracted from a `puml-edge` SVG hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticEdge {
    pub from: String,
    pub to: String,
    pub segments: Vec<Segment>,
}

/// A canonical semantic label extracted from a `puml-label` SVG hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticLabel {
    pub owner: String,
    pub label_kind: String,
    pub bbox: NodeBbox,
    pub text: Option<String>,
}

/// Extract semantic nodes from canonical `puml-node` hooks only.
pub fn extract_semantic_nodes(svg: &str) -> Vec<SemanticNode> {
    let mut result = Vec::new();

    for tag in svg_element_tags(svg) {
        if !tag_has_class(tag, "puml-node") {
            continue;
        }

        let Some((x, y, w, h)) = extract_node_bbox_from_tag(tag) else {
            continue;
        };
        if w <= 0 || h <= 0 {
            continue;
        }

        let id = extract_attr_str(tag, "data-puml-id").unwrap_or_else(|| format!("node@{x},{y}"));
        let kind = extract_attr_str(tag, "data-puml-kind");
        let bbox = NodeBbox {
            id: id.clone(),
            x,
            y,
            w,
            h,
        };
        result.push(SemanticNode { id, kind, bbox });
    }

    result
}

/// Extract semantic edges from canonical `puml-edge` hooks only.
pub fn extract_semantic_edges(svg: &str) -> Vec<SemanticEdge> {
    extract_relation_segments_with_class(svg, "puml-edge", "data-puml-from", "data-puml-to")
        .into_iter()
        .map(|(from, to, segments)| SemanticEdge { from, to, segments })
        .collect()
}

/// Extract semantic labels from canonical `puml-label` hooks only.
pub fn extract_semantic_labels(svg: &str) -> Vec<SemanticLabel> {
    let mut result = Vec::new();

    for (tag_start, tag) in svg_element_tags_with_pos(svg) {
        if !tag_has_class(tag, "puml-label") {
            continue;
        }

        let Some((x, y, w, h)) =
            extract_attr_str(tag, "data-puml-bbox").and_then(|raw| parse_bbox(&raw))
        else {
            continue;
        };
        if w <= 0 || h <= 0 {
            continue;
        }

        let owner = extract_attr_str(tag, "data-puml-owner").unwrap_or_default();
        let label_kind = extract_attr_str(tag, "data-puml-label-kind").unwrap_or_default();
        let text = extract_text_content_at(svg, tag_start);
        let id = format!("{owner}:{label_kind}@{x},{y}");
        let bbox = NodeBbox { id, x, y, w, h };
        result.push(SemanticLabel {
            owner,
            label_kind,
            bbox,
            text,
        });
    }

    result
}

/// Extract node bounding boxes from canonical `puml-node` hooks, falling back
/// to legacy `uml-node` hooks while renderer families migrate.
pub(crate) fn extract_node_bboxes(svg: &str) -> Vec<NodeBbox> {
    let mut result = Vec::new();

    for tag in svg_element_tags(svg) {
        if !tag_has_class(tag, "puml-node") && !tag_has_class(tag, "uml-node") {
            continue;
        }

        let Some((x, y, w, h)) = extract_node_bbox_from_tag(tag) else {
            continue;
        };
        let id = extract_attr_str(tag, "data-puml-id")
            .or_else(|| extract_attr_str(tag, "data-uml-id"))
            .or_else(|| extract_attr_str(tag, "id"))
            .unwrap_or_else(|| format!("node@{x},{y}"));

        if w > 0 && h > 0 {
            result.push(NodeBbox { id, x, y, w, h });
        }
    }
    result
}

/// Check that all canonical semantic bounding boxes fit inside the SVG viewBox.
pub fn check_semantic_bboxes_inside_viewbox(svg: &str) -> Vec<InvariantViolation> {
    let Some(viewbox) = super::parse_viewbox(svg) else {
        return Vec::new();
    };
    let mut violations = Vec::new();

    for node in extract_semantic_nodes(svg) {
        if let Some(overflow_px) = bbox_viewbox_overflow(&node.bbox, viewbox) {
            violations.push(InvariantViolation {
                kind: InvariantKind::SemanticBBoxOutsideViewbox {
                    role: SemanticRole::Node,
                    id: node.id.clone(),
                    overflow_px,
                },
                corrected: false,
                message: format!(
                    "[INV-PUML-BBOX] puml-node {:?} bbox ({},{},{},{}) overflows viewBox by {}px",
                    node.id, node.bbox.x, node.bbox.y, node.bbox.w, node.bbox.h, overflow_px
                ),
            });
        }
    }

    for label in extract_semantic_labels(svg) {
        if let Some(overflow_px) = bbox_viewbox_overflow(&label.bbox, viewbox) {
            let id = semantic_label_id(&label);
            violations.push(InvariantViolation {
                kind: InvariantKind::SemanticBBoxOutsideViewbox {
                    role: SemanticRole::Label,
                    id: id.clone(),
                    overflow_px,
                },
                corrected: false,
                message: format!(
                    "[INV-PUML-BBOX] puml-label {id:?} bbox ({},{},{},{}) overflows viewBox by {}px",
                    label.bbox.x, label.bbox.y, label.bbox.w, label.bbox.h, overflow_px
                ),
            });
        }
    }

    violations
}

/// Expand the root SVG viewBox/intrinsic size until every canonical semantic
/// bbox fits. Returns the corrected overflow diagnostics.
pub fn expand_viewbox_to_semantic_bboxes(svg: &mut String) -> Vec<InvariantViolation> {
    let Some((vb_x, vb_y, mut vb_w, mut vb_h)) = super::parse_viewbox(svg) else {
        return Vec::new();
    };

    let mut violations = Vec::new();
    let mut expanded = false;
    let semantic_boxes = extract_semantic_nodes(svg)
        .into_iter()
        .map(|node| (SemanticRole::Node, node.id, node.bbox))
        .chain(
            extract_semantic_labels(svg)
                .into_iter()
                .map(|label| (SemanticRole::Label, semantic_label_id(&label), label.bbox)),
        )
        .collect::<Vec<_>>();

    for (role, id, bbox) in semantic_boxes {
        let left_overflow = (vb_x - bbox.x).max(0);
        let top_overflow = (vb_y - bbox.y).max(0);
        let right_overflow = (bbox.x + bbox.w - (vb_x + vb_w)).max(0);
        let bottom_overflow = (bbox.y + bbox.h - (vb_y + vb_h)).max(0);
        let overflow_px = left_overflow
            .max(top_overflow)
            .max(right_overflow)
            .max(bottom_overflow);
        if overflow_px == 0 {
            continue;
        }

        violations.push(InvariantViolation {
            kind: InvariantKind::SemanticBBoxOutsideViewbox {
                role,
                id: id.clone(),
                overflow_px,
            },
            corrected: true,
            message: format!(
                "[INV-PUML-BBOX] {role:?} {id:?} bbox ({},{},{},{}) overflows viewBox by {}px",
                bbox.x, bbox.y, bbox.w, bbox.h, overflow_px
            ),
        });

        if left_overflow > 0 {
            vb_w += left_overflow + 8;
        }
        if top_overflow > 0 {
            vb_h += top_overflow + 8;
        }
        vb_w = vb_w.max(bbox.x + bbox.w - vb_x + 8);
        vb_h = vb_h.max(bbox.y + bbox.h - vb_y + 8);
        expanded = true;
    }

    if expanded {
        *svg = sync_svg_dimensions(svg, vb_x, vb_y, vb_w, vb_h);
    }

    violations
}

/// Check that canonical primary `puml-node` bounding boxes do not overlap.
pub fn check_primary_node_non_overlap(svg: &str) -> Vec<InvariantViolation> {
    let nodes = extract_semantic_nodes(svg);
    let mut violations = Vec::new();

    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let a = &nodes[i];
            let b = &nodes[j];
            if bboxes_overlap(&a.bbox, &b.bbox) {
                violations.push(InvariantViolation {
                    kind: InvariantKind::PrimaryNodeOverlap {
                        a: a.id.clone(),
                        b: b.id.clone(),
                    },
                    corrected: false,
                    message: format!(
                        "[INV-PUML-NODE] puml-node {:?} bbox overlaps puml-node {:?} bbox",
                        a.id, b.id
                    ),
                });
            }
        }
    }

    violations
}

/// Check that canonical labels do not overlap nodes other than their owner.
pub fn check_labels_clear_non_owner_nodes(svg: &str) -> Vec<InvariantViolation> {
    let nodes = extract_semantic_nodes(svg);
    let labels = extract_semantic_labels(svg);
    let mut violations = Vec::new();

    for label in &labels {
        for node in &nodes {
            if node.id == label.owner {
                continue;
            }
            if bboxes_overlap(&label.bbox, &node.bbox) {
                violations.push(InvariantViolation {
                    kind: InvariantKind::LabelOverlapsNonOwnerNode {
                        owner: label.owner.clone(),
                        label_kind: label.label_kind.clone(),
                        node_id: node.id.clone(),
                    },
                    corrected: false,
                    message: format!(
                        "[INV-PUML-LABEL] puml-label owner={:?} kind={:?} overlaps non-owner puml-node {:?}",
                        label.owner, label.label_kind, node.id
                    ),
                });
            }
        }
    }

    violations
}

/// Check that graph-profile SVGs expose the canonical `puml-*` hooks.
pub fn check_canonical_graph_hooks(
    svg: &str,
    profile: GraphValidationProfile,
) -> Vec<InvariantViolation> {
    if matches!(profile, GraphValidationProfile::None) {
        return Vec::new();
    }

    let mut violations = Vec::new();
    let tags = svg_element_tags(svg);
    let node_tags: Vec<&str> = tags
        .iter()
        .copied()
        .filter(|tag| tag_has_class(tag, "puml-node"))
        .collect();
    let edge_tags: Vec<&str> = tags
        .iter()
        .copied()
        .filter(|tag| tag_has_class(tag, "puml-edge"))
        .collect();
    let label_tags: Vec<&str> = tags
        .iter()
        .copied()
        .filter(|tag| tag_has_class(tag, "puml-label"))
        .collect();

    if node_tags.is_empty() {
        push_missing_graph_hook(&mut violations, "svg", "class=puml-node");
    }
    if edge_tags.is_empty() {
        push_missing_graph_hook(&mut violations, "svg", "class=puml-edge");
    }
    if label_tags.is_empty() {
        push_missing_graph_hook(&mut violations, "svg", "class=puml-label");
    }

    for tag in node_tags {
        require_graph_attr(&mut violations, tag, "puml-node", "data-puml-id");
        require_graph_attr(&mut violations, tag, "puml-node", "data-puml-bbox");
    }
    for tag in edge_tags {
        require_graph_attr(&mut violations, tag, "puml-edge", "data-puml-from");
        require_graph_attr(&mut violations, tag, "puml-edge", "data-puml-to");
    }
    for tag in label_tags {
        require_graph_attr(&mut violations, tag, "puml-label", "data-puml-owner");
        require_graph_attr(&mut violations, tag, "puml-label", "data-puml-label-kind");
        require_graph_attr(&mut violations, tag, "puml-label", "data-puml-bbox");
    }

    violations
}

/// Check canonical hook attributes without requiring that every role exists.
///
/// This is the public render-boundary contract: if a renderer emits a canonical
/// `puml-*` hook, the hook must be complete enough for downstream geometry
/// checks. Fixture manifests remain responsible for asserting role counts when
/// a specific diagram should contain nodes, edges, or labels.
pub fn check_canonical_semantic_hook_attrs(svg: &str) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();
    let tags = svg_element_tags(svg);

    for tag in tags
        .iter()
        .copied()
        .filter(|tag| tag_has_class(tag, "puml-node"))
    {
        require_graph_attr(&mut violations, tag, "puml-node", "data-puml-id");
        require_graph_attr(&mut violations, tag, "puml-node", "data-puml-kind");
        require_graph_attr(&mut violations, tag, "puml-node", "data-puml-bbox");
    }
    for tag in tags
        .iter()
        .copied()
        .filter(|tag| tag_has_class(tag, "puml-edge"))
    {
        require_graph_attr(&mut violations, tag, "puml-edge", "data-puml-from");
        require_graph_attr(&mut violations, tag, "puml-edge", "data-puml-to");
    }
    for tag in tags
        .iter()
        .copied()
        .filter(|tag| tag_has_class(tag, "puml-label"))
    {
        require_graph_attr(&mut violations, tag, "puml-label", "data-puml-owner");
        require_graph_attr(&mut violations, tag, "puml-label", "data-puml-label-kind");
        require_graph_attr(&mut violations, tag, "puml-label", "data-puml-bbox");
    }

    violations
}

fn bbox_viewbox_overflow(bbox: &NodeBbox, viewbox: (i32, i32, i32, i32)) -> Option<i32> {
    let (vb_x, vb_y, vb_w, vb_h) = viewbox;
    let left = (vb_x - bbox.x).max(0);
    let top = (vb_y - bbox.y).max(0);
    let right = (bbox.x + bbox.w - (vb_x + vb_w)).max(0);
    let bottom = (bbox.y + bbox.h - (vb_y + vb_h)).max(0);
    let overflow = left.max(top).max(right).max(bottom);
    (overflow > 0).then_some(overflow)
}

fn bboxes_overlap(a: &NodeBbox, b: &NodeBbox) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

fn semantic_label_id(label: &SemanticLabel) -> String {
    format!("{}:{}", label.owner, label.label_kind)
}

fn require_graph_attr(
    violations: &mut Vec<InvariantViolation>,
    tag: &str,
    element: &str,
    attr: &str,
) {
    if extract_attr_str(tag, attr).is_none() {
        push_missing_graph_hook(violations, element, attr);
    }
}

fn push_missing_graph_hook(violations: &mut Vec<InvariantViolation>, element: &str, hook: &str) {
    violations.push(InvariantViolation {
        kind: InvariantKind::CanonicalGraphHookMissing {
            element: element.to_string(),
            hook: hook.to_string(),
        },
        corrected: false,
        message: format!("[INV-PUML-HOOK] graph profile requires {hook:?} on {element}"),
    });
}

fn extract_node_bbox_from_tag(tag: &str) -> Option<(i32, i32, i32, i32)> {
    if let Some(bbox) = extract_attr_str(tag, "data-puml-bbox").and_then(|raw| parse_bbox(&raw)) {
        return Some(bbox);
    }

    if tag.starts_with("<rect ") {
        return Some((
            parse_attr_i32(tag, "x").unwrap_or(0),
            parse_attr_i32(tag, "y").unwrap_or(0),
            parse_attr_i32(tag, "width").unwrap_or(0),
            parse_attr_i32(tag, "height").unwrap_or(0),
        ));
    }
    if tag.starts_with("<circle ") {
        let cx = parse_attr_i32(tag, "cx")?;
        let cy = parse_attr_i32(tag, "cy")?;
        let r = parse_attr_i32(tag, "r")?;
        return Some((cx - r, cy - r, r * 2, r * 2));
    }
    if tag.starts_with("<ellipse ") {
        let cx = parse_attr_i32(tag, "cx")?;
        let cy = parse_attr_i32(tag, "cy")?;
        let rx = parse_attr_i32(tag, "rx")?;
        let ry = parse_attr_i32(tag, "ry")?;
        return Some((cx - rx, cy - ry, rx * 2, ry * 2));
    }
    if tag.starts_with("<polygon ") || tag.starts_with("<polyline ") {
        let points = extract_attr_str(tag, "points")?;
        return parse_points_bbox(&points);
    }
    None
}
