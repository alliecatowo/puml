/// Result of a full invariant run.
use crate::render_core::{validate::GeometryMetric, GeometryIssue, RenderScene};

use super::invariants::{
    check_edge_label_proximity, check_edge_node_clearance, check_endpoint_connectivity,
    check_label_edge_clearance, check_labels_inside_viewbox, check_package_headers_from_svg,
};
use super::types::{AutoCorrect, InvariantViolation};

#[derive(Debug, Default)]
pub struct InvariantReport {
    pub violations: Vec<InvariantViolation>,
    pub typed_issues: Vec<GeometryIssue>,
    pub typed_metrics: Vec<GeometryMetric>,
    pub expansions: usize,
    pub background_rects_added: usize,
}

impl InvariantReport {
    fn attach_typed_scene(&mut self, scene: &RenderScene) {
        let typed = scene.validate_scene();
        self.typed_issues = typed.issues;
        self.typed_metrics = typed.metrics;
    }
}

impl From<InvariantReport> for crate::output::RenderInvariantReport {
    fn from(report: InvariantReport) -> Self {
        Self {
            svg_violations: report.violations.len(),
            typed_issues: report.typed_issues,
            typed_metrics: report.typed_metrics,
            expansions: report.expansions,
            background_rects_added: report.background_rects_added,
        }
    }
}

/// Run all applicable SVG-level invariants on a completed SVG render.
///
/// `mode` controls whether auto-corrections are applied to the SVG string.
///
/// This is the main entry point; call it at the end of every render function.
pub fn run(svg: &mut String, mode: AutoCorrect) -> InvariantReport {
    run_with_scene(svg, None, mode)
}

/// Run typed pre-SVG validation when a scene is available, then fall back to
/// SVG-string checks for unmigrated artifacts.
///
/// SVG auto-corrections (viewBox expansion, label-background `<rect>`s) always
/// run regardless of scene availability — they mutate the SVG string and must
/// not be skipped.
///
/// When a typed [`RenderScene`] is present, the four **check-only** (no SVG
/// mutation) passes are superseded by the scene's own geometry validation:
///
/// | SVG-regex helper             | Role        | Status with TypedScene        |
/// |------------------------------|-------------|-------------------------------|
/// | `check_labels_inside_viewbox`| CORRECTION  | always runs (viewBox expand)  |
/// | `check_label_edge_clearance` | CORRECTION  | always runs (bg rect insert)  |
/// | `check_edge_label_proximity` | CHECK-only  | superseded by typed scene     |
/// | `check_edge_node_clearance`  | CHECK-only  | superseded by typed scene     |
/// | `check_package_headers_from_svg` | CHECK-only | superseded by typed scene  |
/// | `check_endpoint_connectivity`| CHECK-only  | superseded by typed scene     |
///
/// The four CHECK-only helpers are safe to delete in a later visually-gated
/// pass once every renderer has migrated to a typed scene (Refs #1258).
pub fn run_with_scene(
    svg: &mut String,
    scene: Option<&RenderScene>,
    mode: AutoCorrect,
) -> InvariantReport {
    let mut report = InvariantReport::default();

    // ── CORRECTIONS (always run) ─────────────────────────────────────────────
    //
    // Invariant #2: labels inside viewBox (auto-correct: expand viewBox).
    // This is safe to auto-correct at render time because it only expands the
    // viewBox dimensions — it never changes any element positions.
    {
        let v = check_labels_inside_viewbox(svg, mode);
        let expansions = v.iter().filter(|x| x.corrected).count();
        report.expansions += expansions;
        report.violations.extend(v);
    }

    // Invariant #3: label-vs-edge-stroke clearance. Renderers now mark graph
    // relation labels, so this pass can avoid node/header text false positives.
    {
        let before = svg.matches("class=\"uml-edge-label-bg\"").count();
        let v = check_label_edge_clearance(svg, mode);
        let after = svg.matches("class=\"uml-edge-label-bg\"").count();
        report.background_rects_added += after.saturating_sub(before);
        report.violations.extend(v);
    }

    // ── GEOMETRY CHECKS ──────────────────────────────────────────────────────
    //
    // When a typed scene is present it is the authoritative source of geometry
    // issue reporting. The SVG-regex check-only helpers are skipped to avoid
    // duplicated diagnostics and unnecessary regex work. For non-migrated and
    // unsupported artifacts the SVG-string helpers remain as the fallback.
    if let Some(scene) = scene {
        // Typed scene is authoritative: source all geometry checks from it.
        report.attach_typed_scene(scene);
    } else {
        // SVG-string fallback for non-migrated / unsupported renderers.

        // Edge labels should stay near visible routes. Diagnostic-only until
        // typed label ownership is available before SVG emission.
        report
            .violations
            .extend(check_edge_label_proximity(svg, 96));

        // Invariant #1: edge-vs-node intersection (diagnostic; layout engine
        // auto-corrects upstream at layout time).
        report.violations.extend(check_edge_node_clearance(svg));

        // Invariant #4: group/package header reservation (diagnostic fallback).
        report
            .violations
            .extend(check_package_headers_from_svg(svg));

        // Invariant #6: edge endpoint connectivity (diagnostic).
        report.violations.extend(check_endpoint_connectivity(svg));
    }

    report
}
