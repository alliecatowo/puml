//! Canonical semantic SVG hook emitters.
//!
//! Family renderers may keep their visual classes (`uml-*`, `chart-*`,
//! `chen-*`, ...), but primary geometry should also carry these `puml-*`
//! hooks so validation and tests can reason about roles instead of private
//! renderer class names.

use super::scene_graph::Rect;
use super::svg::escape_text;

pub fn bbox_value(bbox: Rect) -> String {
    bbox.as_puml_bbox()
}

pub fn node_attrs(id: &str, family: &str, kind: &str, bbox: Rect) -> String {
    format!(
        "data-puml-id=\"{}\" data-puml-kind=\"{}\" data-puml-family=\"{}\" data-puml-bbox=\"{}\"",
        escape_text(id),
        escape_text(kind),
        escape_text(family),
        bbox_value(bbox)
    )
}

pub fn edge_attrs(id: &str, family: &str, kind: &str, from: &str, to: &str) -> String {
    format!(
        "data-puml-edge-id=\"{}\" data-puml-family=\"{}\" data-puml-edge-kind=\"{}\" data-puml-from=\"{}\" data-puml-to=\"{}\"",
        escape_text(id),
        escape_text(family),
        escape_text(kind),
        escape_text(from),
        escape_text(to)
    )
}

pub fn label_attrs(owner: &str, kind: &str, bbox: Rect) -> String {
    format!(
        "data-puml-owner=\"{}\" data-puml-label-kind=\"{}\" data-puml-bbox=\"{}\"",
        escape_text(owner),
        escape_text(kind),
        bbox_value(bbox)
    )
}

pub fn container_attrs(id: &str, family: &str, kind: &str, frame_bbox: Rect) -> String {
    format!(
        "data-puml-id=\"{}\" data-puml-kind=\"{}\" data-puml-family=\"{}\" data-puml-bbox=\"{}\"",
        escape_text(id),
        escape_text(kind),
        escape_text(family),
        bbox_value(frame_bbox)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_attrs_escape_and_format_canonical_bbox() {
        let attrs = node_attrs("A&B", "class", "entity", Rect::new(1.25, 2.0, 30.0, 40.5));
        assert!(attrs.contains("data-puml-id=\"A&amp;B\""));
        assert!(attrs.contains("data-puml-family=\"class\""));
        assert!(attrs.contains("data-puml-bbox=\"1.2 2.0 30.0 40.5\""));
    }
}
