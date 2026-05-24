use super::super::{FamilyDocument, FamilyNode};

const MINDMAP_PALETTE: &[&str] = &[
    "#fde68a", // depth 0 — root amber
    "#bfdbfe", // depth 1 — sky blue
    "#bbf7d0", // depth 2 — mint
    "#fecaca", // depth 3 — rose
    "#e9d5ff", // depth 4 — lavender
    "#fed7aa", // depth 5 — peach
];

fn mindmap_node_fill(depth: usize) -> &'static str {
    MINDMAP_PALETTE[depth % MINDMAP_PALETTE.len()]
}

pub(super) fn mindmap_style(doc: &FamilyDocument) -> Option<&crate::theme::MindMapStyle> {
    match &doc.family_style {
        Some(crate::model::FamilyStyle::MindMap(style)) => Some(style),
        _ => None,
    }
}

pub(super) fn mindmap_node_fill_resolved(
    node: &FamilyNode,
    style: Option<&crate::theme::MindMapStyle>,
) -> String {
    tree_node_fill_resolved(node, style, mindmap_node_fill(node.depth))
}

pub(super) fn tree_node_fill_resolved(
    node: &FamilyNode,
    style: Option<&crate::theme::MindMapStyle>,
    fallback: &str,
) -> String {
    node.fill_color
        .clone()
        .or_else(|| {
            style
                .and_then(|s| s.depth_styles.get(&node.depth))
                .and_then(|s| s.background_color.clone())
        })
        .unwrap_or_else(|| fallback.to_string())
}

pub(super) fn mindmap_node_font_color<'a>(
    depth: usize,
    style: Option<&'a crate::theme::MindMapStyle>,
    fallback: &'a str,
) -> &'a str {
    style
        .and_then(|s| s.depth_styles.get(&depth))
        .and_then(|s| s.font_color.as_deref())
        .unwrap_or(fallback)
}

pub(super) fn mindmap_node_border_color<'a>(
    depth: usize,
    style: Option<&'a crate::theme::MindMapStyle>,
    fallback: &'a str,
) -> &'a str {
    style
        .and_then(|s| s.depth_styles.get(&depth))
        .and_then(|s| s.border_color.as_deref())
        .unwrap_or(fallback)
}
