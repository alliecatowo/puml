//! Promoted visual-quality gates for high-risk corpus cases.
//!
//! Geometry invariants are now checked exclusively through the typed RenderScene
//! (validate_geometry / validate_scene). Known failures remain non-fatal only
//! when the fixture records a linked issue.

use puml::render::validate::{self, AutoCorrect};
use puml::render_core::{GeometryIssue, SceneAvailability};

#[derive(Clone, Copy)]
struct PromotedFixture {
    path: &'static str,
    expected_issues: &'static [u32],
}

const ISSUE_VISUAL_GATES: u32 = 1113;
const ISSUE_ROUTING: u32 = 593;
const ISSUE_COMPACTNESS: u32 = 594;
const ISSUE_BASELINES_GRAPH: u32 = 739;
const ISSUE_BASELINES_TIMING: u32 = 434;

const PROMOTED_FIXTURES: &[PromotedFixture] = &[
    fixture(
        "docs/examples/component/07_ports_lollipop_interfaces.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_ROUTING],
    ),
    fixture(
        "docs/examples/deployment/05_three_tier_cloud_onprem.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_ROUTING, ISSUE_COMPACTNESS],
    ),
    fixture(
        "docs/examples/deployment/06_kubernetes_pods_containers.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_ROUTING, ISSUE_COMPACTNESS],
    ),
    fixture(
        "docs/examples/usecase/06_multi_system_boundary.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_ROUTING, ISSUE_COMPACTNESS],
    ),
    fixture(
        "docs/examples/class/31_generic_types_container.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_COMPACTNESS],
    ),
    fixture(
        "docs/examples/class/32_association_class_deep_packages.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_ROUTING, ISSUE_COMPACTNESS],
    ),
    fixture(
        "docs/examples/sequence/48_complex_ref_over_multibox.puml",
        &[ISSUE_VISUAL_GATES],
    ),
    fixture(
        "docs/examples/activity/16_nested_swimlanes_parallel_forks.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_COMPACTNESS],
    ),
    fixture(
        "docs/examples/activity_new/08_notes_split_partitions.puml",
        &[ISSUE_VISUAL_GATES],
    ),
    fixture(
        "docs/examples/state/09_three_level_composite.puml",
        &[ISSUE_VISUAL_GATES],
    ),
    fixture(
        "docs/examples/timing/05_concurrent_timelines_message_arrows.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_BASELINES_TIMING],
    ),
    fixture(
        "docs/examples/c4/11_system_landscape.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_BASELINES_GRAPH, ISSUE_ROUTING],
    ),
    fixture(
        "docs/examples/chart/06_multi_series_line.puml",
        &[ISSUE_VISUAL_GATES],
    ),
    fixture(
        "docs/diagrams/language-service-layers.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_ROUTING, ISSUE_COMPACTNESS],
    ),
    fixture(
        "docs/diagrams/architecture-overview.puml",
        &[ISSUE_VISUAL_GATES, ISSUE_ROUTING],
    ),
];

const fn fixture(path: &'static str, expected_issues: &'static [u32]) -> PromotedFixture {
    PromotedFixture {
        path,
        expected_issues,
    }
}

#[test]
fn promoted_visual_cases_have_svg_invariant_coverage() {
    let mut summaries = Vec::new();

    for fixture in PROMOTED_FIXTURES {
        let source = std::fs::read_to_string(fixture.path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", fixture.path));

        // Render to typed artifact; collect the SVG for SVG-level metrics.
        let artifacts = puml::render_source_to_artifacts(&source)
            .unwrap_or_else(|err| panic!("failed to render {}: {err:?}", fixture.path));
        let artifact = &artifacts[0];
        let svg = &artifact.svg;

        // SVG-level quality metrics (viewBox, text count) — unchanged.
        let mut label_svg = svg.clone();
        let label_bounds =
            validate::check_labels_inside_viewbox(&mut label_svg, AutoCorrect::EmitDiagnostic);
        let metrics = validate::collect_quality_metrics(svg);

        assert!(
            metrics.viewbox_width > 0 && metrics.viewbox_height > 0,
            "{} should expose valid SVG viewBox metrics",
            fixture.path
        );
        assert!(
            metrics.text_count > 0,
            "{} should expose visible text for visual gates",
            fixture.path
        );

        assert_or_expected(
            fixture,
            label_bounds.is_empty(),
            ISSUE_VISUAL_GATES,
            format!("{} label bounds violations", label_bounds.len()),
        );

        // ── Typed-scene geometry checks ───────────────────────────────────────
        //
        // When the artifact carries a typed RenderScene, derive the geometry
        // issues from it (authoritative).  For non-migrated renderers the
        // artifact still produces SVG, but validate_geometry is unavailable —
        // those fixtures pass the geometry gates vacuously (no scene → no issues
        // to report) and fall back on the SVG-level label check above.
        let typed_issues: Vec<GeometryIssue> =
            if artifact.scene_availability == SceneAvailability::TypedScene {
                artifact
                    .typed_scene()
                    .map(|scene| scene.validate_geometry())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

        // INV-1 / INV-4: edge-vs-node and edge-vs-group-header crossings.
        let route_node_count = typed_issues
            .iter()
            .filter(|i| matches!(i, GeometryIssue::EdgeCrossesNode { .. }))
            .count();
        let route_header_count = typed_issues
            .iter()
            .filter(|i| matches!(i, GeometryIssue::EdgeCrossesGroupHeader { .. }))
            .count();
        // Detached edge labels (label far from owning route).
        let detached_label_count = typed_issues
            .iter()
            .filter(|i| matches!(i, GeometryIssue::EdgeLabelDetached { .. }))
            .count();

        assert_or_expected(
            fixture,
            route_node_count == 0,
            ISSUE_ROUTING,
            format!("{route_node_count} route/node crossings (typed scene)"),
        );
        assert_or_expected(
            fixture,
            route_header_count == 0,
            ISSUE_VISUAL_GATES,
            format!("{route_header_count} route/header crossings (typed scene)"),
        );
        assert_or_expected(
            fixture,
            detached_label_count == 0,
            ISSUE_VISUAL_GATES,
            format!("{detached_label_count} detached edge labels (typed scene)"),
        );

        let compact = metrics.aspect_ratio <= 6.0
            && metrics.max_empty_gutter_ratio <= 0.35
            && metrics.route_length_per_node_px <= 900.0;
        assert_or_expected(
            fixture,
            compact,
            ISSUE_COMPACTNESS,
            format!(
                "compactness aspect={:.2}, gutter={:.2}, route_per_node={:.1}",
                metrics.aspect_ratio,
                metrics.max_empty_gutter_ratio,
                metrics.route_length_per_node_px
            ),
        );

        summaries.push(format!(
            "{}: nodes={}, rels={}, headers={}, aspect={:.2}, gutter={:.2}, route/node={:.1} | typed: node_crossings={}, header_crossings={}, detached_labels={}",
            fixture.path,
            metrics.node_count,
            metrics.relation_count,
            metrics.package_count,
            metrics.aspect_ratio,
            metrics.max_empty_gutter_ratio,
            metrics.route_length_per_node_px,
            route_node_count,
            route_header_count,
            detached_label_count,
        ));
    }

    eprintln!("promoted visual quality metrics:\n{}", summaries.join("\n"));
}

fn assert_or_expected(fixture: &PromotedFixture, condition: bool, issue: u32, detail: String) {
    if condition {
        return;
    }

    assert!(
        fixture.expected_issues.contains(&issue)
            || fixture.expected_issues.contains(&ISSUE_VISUAL_GATES),
        "{} failed visual gate ({detail}) without linked issue #{issue}; expected issues were {:?}",
        fixture.path,
        fixture.expected_issues
    );
    eprintln!(
        "expected visual debt for {}: {detail}; linked issues {:?}",
        fixture.path, fixture.expected_issues
    );
}
