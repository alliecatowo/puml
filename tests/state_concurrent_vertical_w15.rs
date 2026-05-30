//! Wave-15 structural regression: concurrent state regions stack top-to-bottom.
//!
//! PlantUML 1.2026.5 and the UML 2.x spec place concurrent regions vertically
//! (top-to-bottom) separated by a horizontal dashed divider.  Prior to this fix,
//! PUML placed regions side-by-side (left-to-right) with a vertical dashed divider.
//!
//! These tests assert the structural invariant: child nodes in region 0 must have
//! smaller `y` positions than child nodes in region 1 (top-to-bottom stacking), not
//! smaller `x` positions (left-to-right stacking).  The divider line must be
//! horizontal (y1 == y2, x1 != x2).
//!
//! Closes #1351.

mod svg_test_helpers;

use puml::render_source_to_svg;
use svg_test_helpers::{f64_attr, SvgDoc};

/// Fixture 03_concurrent: Processing composite with two concurrent regions.
/// Region 0: Parse → Validate
/// Region 1: Log → Audit
#[test]
fn concurrent_regions_stack_top_to_bottom_03() {
    let src = include_str!("../docs/examples/state/03_concurrent.puml");
    let svg = render_source_to_svg(src).expect("state/03 svg should render");
    let doc = SvgDoc::parse(&svg);

    // Find rect elements for region-0 children (Parse, Validate) and
    // region-1 children (Log, Audit) by looking for text labels nearby.
    let parse_rect = rect_below_text(&doc, "Parsing");
    let validate_rect = rect_below_text(&doc, "Validating");
    let log_rect = rect_below_text(&doc, "Logging");
    let audit_rect = rect_below_text(&doc, "Auditing");

    // Within each region children should stack vertically.
    assert!(
        parse_rect.y < validate_rect.y,
        "Parsing (y={}) should be above Validating (y={}) within region 0",
        parse_rect.y,
        validate_rect.y
    );
    assert!(
        log_rect.y < audit_rect.y,
        "Logging (y={}) should be above Auditing (y={}) within region 1",
        log_rect.y,
        audit_rect.y
    );

    // Region 0 must be above region 1 (top-to-bottom stacking, NOT side-by-side).
    assert!(
        validate_rect.y < log_rect.y,
        "region 0 bottom child Validating (y={}) should be above region 1 top child Logging (y={})",
        validate_rect.y,
        log_rect.y
    );

    // Regions must share approximately the same x centre (same column, not side-by-side).
    let parse_cx = parse_rect.x + parse_rect.width / 2.0;
    let log_cx = log_rect.x + log_rect.width / 2.0;
    assert!(
        (parse_cx - log_cx).abs() < 40.0,
        "Parsing centre-x ({parse_cx}) and Logging centre-x ({log_cx}) should be aligned (top-to-bottom layout)"
    );

    // Dashed divider line must be horizontal (y1 == y2).
    assert_divider_is_horizontal(&doc);
}

/// Fixture 10_parallel_regions_shared_events: MediaPlayer with three concurrent regions.
/// Region 0: Playing / Paused / Stopped
/// Region 1: Normal / Muted
/// Region 2: EQOff / EQOn
#[test]
fn concurrent_regions_stack_top_to_bottom_10() {
    let src = include_str!("../docs/examples/state/10_parallel_regions_shared_events.puml");
    let svg = render_source_to_svg(src).expect("state/10 svg should render");
    let doc = SvgDoc::parse(&svg);

    let playing_rect = rect_below_text(&doc, "Playing");
    let normal_rect = rect_below_text(&doc, "Normal");
    let eq_off_rect = rect_below_text(&doc, "EQOff");

    // Region 0 must be above region 1, which must be above region 2.
    assert!(
        playing_rect.y < normal_rect.y,
        "Playing (region 0, y={}) should be above Normal (region 1, y={})",
        playing_rect.y,
        normal_rect.y
    );
    assert!(
        normal_rect.y < eq_off_rect.y,
        "Normal (region 1, y={}) should be above EQOff (region 2, y={})",
        normal_rect.y,
        eq_off_rect.y
    );

    // Regions share the same column.
    let playing_cx = playing_rect.x + playing_rect.width / 2.0;
    let normal_cx = normal_rect.x + normal_rect.width / 2.0;
    assert!(
        (playing_cx - normal_cx).abs() < 40.0,
        "Playing centre-x ({playing_cx}) and Normal centre-x ({normal_cx}) should be in the same column"
    );

    // Dashed divider(s) must be horizontal.
    assert_divider_is_horizontal(&doc);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the bounding box of the `<rect>` whose sibling `<text>` contains
/// `label`.  We locate the text element and then find the nearest preceding
/// `<rect>` at approximately the same position.
fn rect_below_text(doc: &SvgDoc<'_>, label: &str) -> svg_test_helpers::Bounds {
    let texts = doc.texts_containing(label);
    let text_node = texts
        .first()
        .unwrap_or_else(|| panic!("text {label:?} not found in SVG"));
    let tx = f64_attr(*text_node, "x");
    let ty = f64_attr(*text_node, "y");

    // Find a rect whose centre-x is close to the text x and whose y-range
    // contains ty (text typically sits inside the rect).  Skip the background
    // rect which has `width="100%"` and no `x` attribute.
    let rects = doc.elements("rect");
    rects
        .into_iter()
        .filter(|r| {
            // Skip background rect (has no numeric x).
            let rx: f64 = match r.attribute("x").and_then(|v| v.parse().ok()) {
                Some(v) => v,
                None => return false,
            };
            let ry: f64 = match r.attribute("y").and_then(|v| v.parse().ok()) {
                Some(v) => v,
                None => return false,
            };
            let rw: f64 = match r.attribute("width").and_then(|v| v.parse().ok()) {
                Some(v) => v,
                None => return false,
            };
            let rh: f64 = match r.attribute("height").and_then(|v| v.parse().ok()) {
                Some(v) => v,
                None => return false,
            };
            let cx = rx + rw / 2.0;
            // Must share approximate centre-x and contain ty in its y-range.
            (cx - tx).abs() < 30.0 && ry <= ty && ty <= ry + rh && rh < 80.0
        })
        .map(|r| svg_test_helpers::bounds(r))
        .next()
        .unwrap_or_else(|| panic!("rect for label {label:?} not found"))
}

/// Assert that at least one dashed `<line>` in the SVG is horizontal
/// (y1 == y2, meaning width > 0 and height == 0).
fn assert_divider_is_horizontal(doc: &SvgDoc<'_>) {
    let dashed_lines: Vec<_> = doc
        .elements("line")
        .into_iter()
        .filter(|line| {
            line.attribute("stroke-dasharray")
                .map(|v| !v.is_empty())
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !dashed_lines.is_empty(),
        "expected at least one dashed divider line in SVG"
    );

    let has_horizontal = dashed_lines.iter().any(|line| {
        let y1 = f64_attr(*line, "y1");
        let y2 = f64_attr(*line, "y2");
        let x1 = f64_attr(*line, "x1");
        let x2 = f64_attr(*line, "x2");
        // Horizontal: same y, different x.
        (y1 - y2).abs() < 1.0 && (x1 - x2).abs() > 10.0
    });

    assert!(
        has_horizontal,
        "concurrent region divider should be a horizontal dashed line (y1==y2); \
         found dashed lines: {:?}",
        dashed_lines
            .iter()
            .map(|l| {
                format!(
                    "x1={} y1={} x2={} y2={}",
                    l.attribute("x1").unwrap_or("?"),
                    l.attribute("y1").unwrap_or("?"),
                    l.attribute("x2").unwrap_or("?"),
                    l.attribute("y2").unwrap_or("?"),
                )
            })
            .collect::<Vec<_>>()
    );
}
