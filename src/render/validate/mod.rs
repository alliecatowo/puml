//! Render-time invariants pass: makes visually-broken diagrams structurally impossible.
//!
//! This module enforces correctness invariants on the completed SVG output.
//! Each invariant either auto-corrects (mutating the SVG string in place) or
//! emits a structured diagnostic explaining what would have been broken.
//!
//! Priority order (from the issue body):
//!   1. Edge-vs-node non-intersection  [auto-correct: segment re-route]
//!   2. Label-inside-viewBox           [auto-correct: expand viewBox]
//!   3. Label-vs-edge-stroke clearance [auto-correct: background rect]
//!   4. Package-header reservation     [auto-correct: segment re-route]
//!   5. Pseudo-state deduplication     [normalization assertion — see normalize/state.rs]
//!   6. Edge endpoint connectivity     [diagnostic only]
//!   7. Self-loop row allocation       [diagnostic only]
//!
//! The main entry point is [`run`].

mod geometry;
mod invariants;
mod metrics;
mod report;
mod svg_hooks;
mod types;

pub use geometry::{extract_node_bboxes, extract_package_frames, NodeBbox, PackageFrame};
pub use invariants::{
    check_edge_label_proximity, check_edge_node_clearance, check_endpoint_connectivity,
    check_label_edge_clearance, check_labels_inside_viewbox, check_package_headers,
    check_package_headers_from_svg, check_pseudo_state_dedup,
};
pub use metrics::{collect_quality_metrics, ContentBounds, QualityMetrics};
pub use report::{run, run_with_scene, InvariantReport};
pub use svg_hooks::parse_viewbox;
pub use types::{AutoCorrect, EndpointSide, InvariantKind, InvariantViolation, PseudoStateKind};

#[cfg(test)]
use geometry::{parse_polyline_segments, segment_crosses_rect, Segment};
#[cfg(test)]
use svg_hooks::replace_viewbox;

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_viewbox_roundtrip() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#;
        assert_eq!(parse_viewbox(svg), Some((0, 0, 400, 300)));
    }

    #[test]
    fn replace_viewbox_works() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300"></svg>"#;
        let out = replace_viewbox(svg, 0, 0, 500, 400);
        assert!(out.contains("viewBox=\"0 0 500 400\""), "got: {out}");
    }

    #[test]
    fn check_labels_inside_viewbox_expands_on_overflow() {
        // A text element at x=390, content 10 chars → right edge ≈ 390+70=460 > viewBox width 400.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300"><rect width="100%" height="100%" fill="white"/><text x="390" y="150" text-anchor="middle" font-family="monospace">0123456789</text></svg>"#;
        let mut svg = svg.to_string();
        let v = check_labels_inside_viewbox(&mut svg, AutoCorrect::Apply);
        assert!(
            !v.is_empty(),
            "expected at least one label-overflow violation"
        );
        let (_, _, vb_w, _) = parse_viewbox(&svg).expect("viewBox should be present");
        assert!(
            vb_w > 400,
            "viewBox width should have been expanded; got {vb_w}"
        );
    }

    #[test]
    fn check_labels_inside_viewbox_no_false_positive() {
        // Text well within viewBox.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300"><text x="100" y="100">Hi</text></svg>"#;
        let mut svg = svg.to_string();
        let v = check_labels_inside_viewbox(&mut svg, AutoCorrect::Apply);
        assert!(v.is_empty(), "no violations expected for in-bounds text");
    }

    #[test]
    fn segment_crosses_rect_basic() {
        // Horizontal segment crossing a rect.
        let seg = Segment {
            x1: 0,
            y1: 50,
            x2: 200,
            y2: 50,
        };
        assert!(segment_crosses_rect(seg, 80, 30, 60, 50)); // rect at (80,30) 60×50
                                                            // Segment that passes above the rect.
        let seg2 = Segment {
            x1: 0,
            y1: 10,
            x2: 200,
            y2: 10,
        };
        assert!(!segment_crosses_rect(seg2, 80, 30, 60, 50));
    }

    #[test]
    fn parse_polyline_segments_basic() {
        let tag = r#"<polyline class="uml-relation" points="10,20 50,20 50,80 100,80""#;
        let segs = parse_polyline_segments(tag);
        assert_eq!(segs.len(), 3, "expected 3 segments from 4 points");
        assert_eq!(segs[0].x1, 10);
        assert_eq!(segs[0].y1, 20);
        assert_eq!(segs[0].x2, 50);
    }

    #[test]
    fn check_pseudo_state_dedup_no_violation_when_normalized() {
        use crate::model::{StateNode, StateNodeKind};
        let nodes = vec![
            StateNode {
                name: "[*]".to_string(),
                display: None,
                kind: StateNodeKind::StartEnd,
                stereotype: None,
                style: Default::default(),
                internal_actions: vec![],
                regions: vec![],
            },
            StateNode {
                name: "Active".to_string(),
                display: None,
                kind: StateNodeKind::Normal,
                stereotype: None,
                style: Default::default(),
                internal_actions: vec![],
                regions: vec![],
            },
        ];
        let violations = check_pseudo_state_dedup(&nodes, "root");
        assert!(violations.is_empty(), "single [*] should not violate");
    }

    #[test]
    fn check_pseudo_state_dedup_catches_duplicates() {
        use crate::model::{StateNode, StateNodeKind};
        let nodes = vec![
            StateNode {
                name: "[*]".to_string(),
                display: None,
                kind: StateNodeKind::StartEnd,
                stereotype: None,
                style: Default::default(),
                internal_actions: vec![],
                regions: vec![],
            },
            StateNode {
                name: "[*]_dup".to_string(),
                display: None,
                kind: StateNodeKind::StartEnd,
                stereotype: None,
                style: Default::default(),
                internal_actions: vec![],
                regions: vec![],
            },
        ];
        let violations = check_pseudo_state_dedup(&nodes, "root");
        assert_eq!(
            violations.len(),
            1,
            "expected one duplicate-initial violation"
        );
        assert!(matches!(
            violations[0].kind,
            InvariantKind::DuplicatePseudoState {
                kind: PseudoStateKind::Initial,
                ..
            }
        ));
    }

    #[test]
    fn check_label_edge_clearance_adds_background_rect() {
        // An edge that passes directly under a label.
        // Using format! to avoid raw-string # ambiguity in concat!.
        let svg = format!(
            concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="200" viewBox="0 0 300 200">"#,
                r#"<polyline class="uml-relation" data-uml-from="A" data-uml-to="B" points="50,100 250,100" fill="none" stroke="{}" stroke-width="2"/>"#,
                r#"<text x="150" y="100" text-anchor="middle" font-family="monospace">label</text>"#,
                r#"</svg>"#
            ),
            "#333"
        );
        let mut svg = svg.to_string();
        let v = check_label_edge_clearance(&mut svg, AutoCorrect::Apply);
        // Should detect clearance issue (label sits exactly on the stroke).
        // The text y=100 and the segment y=100 → clearance=0 < 4.
        assert!(
            !v.is_empty() || svg.contains("<rect"),
            "expected either a violation or a background rect to be inserted"
        );
    }

    #[test]
    fn run_entry_point_returns_report() {
        let svg = concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="150" viewBox="0 0 200 150">"#,
            r#"<rect width="100%" height="100%" fill="white"/>"#,
            r#"<text x="10" y="50">hello</text>"#,
            r#"</svg>"#
        );
        let mut svg = svg.to_string();
        let report = run(&mut svg, AutoCorrect::Apply);
        // No violations expected for a simple, well-formed SVG.
        assert!(
            report.violations.is_empty(),
            "expected no violations: {:?}",
            report.violations
        );
    }
}
