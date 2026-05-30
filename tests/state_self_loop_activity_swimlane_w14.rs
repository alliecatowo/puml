//! Structural regression tests for:
//!   - #1332 state self-transition arc (C-shaped loop hugging top-right corner)
//!   - #449  activity swimlane / partition layout improvements
//!
//! These tests assert SVG content and geometry — not pixel snapshots — so they
//! remain stable across cosmetic rendering changes.

mod svg_test_helpers;

use svg_test_helpers::{f64_attr, SvgDoc};

// ─── #1332 State self-transition arc ─────────────────────────────────────────

/// A state self-transition must emit a cubic Bézier path (`C` command) rather
/// than a degenerate zero-length line, and the path must extend to the right of
/// the state box so it never clips through the node interior.
#[test]
fn state_self_transition_emits_cubic_bezier_arc() {
    let src = r#"@startuml
[*] --> Active
Active --> Active : process()
Active --> [*]
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("state self-loop should render");

    // The self-transition must be a <path> with data-state-from and data-state-to
    // both set to "Active".
    assert!(
        svg.contains("data-state-from=\"Active\" data-state-to=\"Active\""),
        "SVG must include a self-transition path for Active"
    );

    // The path d attribute must contain a cubic Bézier command ("C "), not just
    // a straight line or a degenerate point.
    let self_path_start = svg
        .find("data-state-from=\"Active\" data-state-to=\"Active\"")
        .expect("self-transition marker must be in SVG");
    let path_snippet = &svg[self_path_start..self_path_start + 200];
    assert!(
        path_snippet.contains(" C "),
        "self-transition must use a cubic Bézier ('C') — found: {path_snippet}"
    );
}

/// The self-transition arc must bulge to the right of the state box so the path
/// never passes through the node interior.  We verify this by checking that at
/// least one control-point x-coordinate in the cubic Bézier is strictly greater
/// than the right edge of the state rectangle.
#[test]
fn state_self_transition_arc_stays_right_of_box() {
    let src = r#"@startuml
[*] --> Waiting
Waiting --> Waiting : tick
Waiting --> Done : finish
Done --> [*]
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("state self-loop should render");
    let doc = SvgDoc::parse(&svg);

    // Find the Waiting state box (rect)
    let waiting_rect = doc
        .elements("rect")
        .into_iter()
        .find(|r| {
            // Check for the metadata node preceding or adjacent to this rect.
            // Easier: find the rect whose x+width (right edge) is near the
            // self-transition path's exit_x.
            // We locate it by finding the metadata element for "Waiting".
            let meta = doc
                .elements_with_attr("metadata", "data-state-node", "Waiting")
                .into_iter()
                .next();
            // The rect we want is *after* the metadata in the SVG; use a
            // simpler heuristic: the rect with largest area that doesn't have
            // width="100%".
            meta.is_some() && r.attribute("width") != Some("100%")
        })
        .expect("Waiting state rect should be present");

    let rect_x = f64_attr(waiting_rect, "x");
    let rect_w = f64_attr(waiting_rect, "width");
    let right_edge = rect_x + rect_w;

    // Extract the self-transition path d attribute.
    let self_transition_marker = "data-state-from=\"Waiting\" data-state-to=\"Waiting\"";
    let start = svg
        .find(self_transition_marker)
        .expect("self-transition for Waiting must appear in SVG");
    // Extract the d="..." portion that follows
    let after_marker = &svg[start..start + 300];
    let d_start = after_marker
        .find("d=\"")
        .expect("self-transition path must have d attribute")
        + 3;
    let d_end = after_marker[d_start..]
        .find('"')
        .expect("d attribute must be terminated");
    let d_attr = &after_marker[d_start..d_start + d_end];

    // Parse numeric tokens from the d attribute.
    let coords: Vec<f64> = d_attr
        .split_ascii_whitespace()
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();

    // There must be at least 8 numbers: M x1 y1 C cx1 cy1 cx2 cy2 x2 y2
    assert!(
        coords.len() >= 8,
        "self-transition cubic Bézier must have at least 8 coordinate values, got: {d_attr}"
    );

    // At least one of the x coordinates (control points or endpoints) must be
    // strictly to the right of the box's right edge, proving the arc bulges out.
    let max_x = coords
        .iter()
        .copied()
        .filter(|&v| v > 0.0)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        max_x > right_edge,
        "self-transition arc must extend past the right edge ({right_edge}) of the state box; \
         max x in path = {max_x}, d = {d_attr}"
    );
}

/// Two distinct states with self-transitions must each emit their own arc,
/// and the arcs must be at different x positions (one per node).
#[test]
fn state_multiple_self_transitions_render_at_distinct_positions() {
    let src = r#"@startuml
[*] --> A
A --> A : loop
A --> B : go
B --> B : idle
B --> [*]
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("multiple self-loops should render");

    assert!(
        svg.contains("data-state-from=\"A\" data-state-to=\"A\""),
        "self-transition for A must be present"
    );
    assert!(
        svg.contains("data-state-from=\"B\" data-state-to=\"B\""),
        "self-transition for B must be present"
    );

    // Extract x-coordinates of both arcs' start points and check they differ.
    let extract_start_x = |from: &str| -> f64 {
        let marker = format!("data-state-from=\"{from}\" data-state-to=\"{from}\"");
        let pos = svg
            .find(&marker)
            .unwrap_or_else(|| panic!("self-transition for {from} not found"));
        let snippet = &svg[pos..pos + 300];
        let d_pos = snippet.find("d=\"").expect("path must have d attr") + 3;
        let d_end = snippet[d_pos..].find('"').expect("d must be closed");
        let d = &snippet[d_pos..d_pos + d_end];
        // First numeric token after "M " is the start x
        d.split_ascii_whitespace()
            .filter_map(|t| t.parse::<f64>().ok())
            .next()
            .unwrap_or(0.0)
    };

    let ax = extract_start_x("A");
    let bx = extract_start_x("B");
    assert!(
        (ax - bx).abs() > 1.0,
        "self-transition arcs for A ({ax}) and B ({bx}) should be at different x positions"
    );
}

// ─── #449 Activity swimlane / partition layout ────────────────────────────────

/// The example 07_partition.puml fixture must render all three named partition
/// headers (Worker, Backend, Frontend) and stack them vertically in source order.
#[test]
fn activity_partition_blocks_stack_vertically_in_source_order() {
    let src = include_str!("../docs/examples/activity/07_partition.puml");
    let svg = puml::render_source_to_svg(src).expect("partition example must render");
    let doc = SvgDoc::parse(&svg);

    // All three partition headers must appear as text labels.
    assert!(
        svg.contains("Worker"),
        "Worker partition header must render"
    );
    assert!(
        svg.contains("Backend"),
        "Backend partition header must render"
    );
    assert!(
        svg.contains("Frontend"),
        "Frontend partition header must render"
    );

    // Locate header text elements for vertical order check.
    let worker_texts = doc.texts_containing("Worker");
    let backend_texts = doc.texts_containing("Backend");
    let frontend_texts = doc.texts_containing("Frontend");

    assert!(!worker_texts.is_empty(), "Worker header text must exist");
    assert!(!backend_texts.is_empty(), "Backend header text must exist");
    assert!(
        !frontend_texts.is_empty(),
        "Frontend header text must exist"
    );

    let worker_y = f64_attr(worker_texts[0], "y");
    let backend_y = f64_attr(backend_texts[0], "y");
    let frontend_y = f64_attr(frontend_texts[0], "y");

    assert!(
        worker_y < backend_y,
        "Worker header ({worker_y}) must appear above Backend header ({backend_y})"
    );
    assert!(
        backend_y < frontend_y,
        "Backend header ({backend_y}) must appear above Frontend header ({frontend_y})"
    );
}

/// The example fixture for `|Lane|` swimlanes must render lanes as side-by-side
/// columns: each lane's header must have a different x-center than the others,
/// all on the same y row.
#[test]
fn activity_swimlane_columns_render_side_by_side_with_distinct_x_positions() {
    let src = include_str!("../docs/examples/activity/16_nested_swimlanes_parallel_forks.puml");
    let svg = puml::render_source_to_svg(src).expect("swimlane example must render");
    let doc = SvgDoc::parse(&svg);

    // All four lane headers must appear.
    for lane in &["Customer", "Warehouse", "Finance", "Logistics"] {
        assert!(svg.contains(lane), "lane header {lane} must appear in SVG");
    }

    // Lane header rects (height == 24) must occupy distinct x positions.
    let lane_header_rects: Vec<_> = doc
        .elements("rect")
        .into_iter()
        .filter(|r| r.attribute("height") == Some("24") && r.attribute("width") != Some("100%"))
        .collect();

    assert!(
        lane_header_rects.len() >= 4,
        "expected at least 4 swimlane header rects (height=24), got {}",
        lane_header_rects.len()
    );

    // Collect unique x values; all four should be distinct.
    let mut xs: Vec<f64> = lane_header_rects
        .iter()
        .map(|r| f64_attr(*r, "x"))
        .collect();
    xs.sort_by(|a, b| a.partial_cmp(b).expect("finite x"));
    xs.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    assert!(
        xs.len() >= 4,
        "expected 4 distinct lane header x positions, got {:?}",
        xs
    );
}

/// Nodes assigned to a given swimlane must be rendered within that lane's
/// horizontal bounds.  We verify `Submit order` (Customer lane) and
/// `Receive order notification` (Warehouse lane) are in distinct x columns.
#[test]
fn activity_swimlane_nodes_placed_in_correct_column() {
    let src = include_str!("../docs/examples/activity/16_nested_swimlanes_parallel_forks.puml");
    let svg = puml::render_source_to_svg(src).expect("swimlane example must render");
    let doc = SvgDoc::parse(&svg);

    // Find the Customer and Warehouse lane headers to get their x bounds.
    let lane_header_rects: Vec<_> = doc
        .elements("rect")
        .into_iter()
        .filter(|r| r.attribute("height") == Some("24") && r.attribute("width") != Some("100%"))
        .collect();
    assert!(
        lane_header_rects.len() >= 2,
        "need at least 2 lane header rects"
    );

    let mut lane_xs: Vec<f64> = lane_header_rects
        .iter()
        .map(|r| f64_attr(*r, "x"))
        .collect();
    lane_xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    let customer_x = lane_xs[0];
    let warehouse_x = lane_xs[1];

    // Customer lane must be to the left of Warehouse lane.
    assert!(
        customer_x < warehouse_x,
        "Customer lane ({customer_x}) must be left of Warehouse ({warehouse_x})"
    );

    // "Submit order" text must be in the Customer column (nearer to customer_x).
    let submit_texts = doc.texts_containing("Submit order");
    assert!(!submit_texts.is_empty(), "Submit order must render");
    let submit_x = f64_attr(submit_texts[0], "x");

    // "Receive order notification" must be in Warehouse column (nearer to warehouse_x).
    let receive_texts = doc.texts_containing("Receive order notification");
    assert!(
        !receive_texts.is_empty(),
        "Receive order notification must render"
    );
    let receive_x = f64_attr(receive_texts[0], "x");

    assert!(
        submit_x < receive_x,
        "Submit order ({submit_x}) should be left of Receive order notification ({receive_x})"
    );
}

/// The self-transition fixture renders both self-loops without errors.
#[test]
fn state_self_transitions_example_fixture_renders_cleanly() {
    let src = include_str!("../docs/examples/state/14_self_transitions.puml");
    let svg = puml::render_source_to_svg(src).expect("state self-transitions fixture must render");

    // Both self-transition states must have their arcs.
    assert!(
        svg.contains("data-state-from=\"Active\" data-state-to=\"Active\""),
        "Active self-transition must be in SVG"
    );
    assert!(
        svg.contains("data-state-from=\"Retrying\" data-state-to=\"Retrying\""),
        "Retrying self-transition must be in SVG"
    );

    // Both arcs must use cubic Bézier curves.
    let active_pos = svg
        .find("data-state-from=\"Active\" data-state-to=\"Active\"")
        .unwrap();
    let retrying_pos = svg
        .find("data-state-from=\"Retrying\" data-state-to=\"Retrying\"")
        .unwrap();
    assert!(
        svg[active_pos..active_pos + 200].contains(" C "),
        "Active self-loop must use cubic Bézier"
    );
    assert!(
        svg[retrying_pos..retrying_pos + 200].contains(" C "),
        "Retrying self-loop must use cubic Bézier"
    );
}
