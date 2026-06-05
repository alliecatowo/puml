//! Structural dimension tests for wave-15 density follow-ups.
//! Covers three bespoke-slot families whose constants were NOT touched by the
//! universal layout retune in PR #1357. Each assertion enforces the target
//! canvas size agreed in issues #1358, #1359, #1360.

use puml::render_source_to_svg;
use std::fs;

fn svg_dimensions(svg: &str) -> (u32, u32) {
    // Extract width and height from the opening <svg ...> tag.
    let tag_end = svg.find('>').expect("malformed SVG: no closing >");
    let tag = &svg[..tag_end];

    let width = tag
        .split("width=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse::<u32>().ok())
        .expect("SVG missing width attribute");

    let height = tag
        .split("height=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse::<u32>().ok())
        .expect("SVG missing height attribute");

    (width, height)
}

fn render_fixture(path: &str) -> String {
    let src = fs::read_to_string(path).unwrap_or_else(|_| panic!("fixture missing: {path}"));
    render_source_to_svg(&src).unwrap_or_else(|e| panic!("render failed for {path}: {e:?}"))
}

/// #1358 — timing bespoke slots: 01_concise.puml must be ≤ 500 px wide.
#[test]
fn timing_concise_width_le_500() {
    let svg = render_fixture("docs/examples/timing/01_concise.puml");
    let (w, _h) = svg_dimensions(&svg);
    assert!(
        w <= 500,
        "timing 01_concise width {w} exceeds 500 px (issue #1358)"
    );
}

/// #1359 — usecase bespoke slots: 06_multi_system_boundary.puml must be ≤ 1500 px wide.
#[test]
fn usecase_multi_system_boundary_width_le_1500() {
    let svg = render_fixture("docs/examples/usecase/06_multi_system_boundary.puml");
    let (w, _h) = svg_dimensions(&svg);
    assert!(
        w <= 1500,
        "usecase 06_multi_system_boundary width {w} exceeds 1500 px (issue #1359)"
    );
}

/// #1484 — usecase/06 routing artifacts: no waypoint x-coordinate may exceed the
/// canvas width + 20px margin (guards against the x=1352 U-turn regression).
#[test]
fn usecase_multi_system_boundary_no_extreme_waypoints() {
    let svg = render_fixture("docs/examples/usecase/06_multi_system_boundary.puml");
    let (w, _h) = svg_dimensions(&svg);
    let max_allowed = w + 20;
    // Scan all polyline `points="..."` attributes for individual x values.
    let mut pos = 0;
    while let Some(start) = svg[pos..].find("points=\"") {
        let abs_start = pos + start + 8; // skip 'points="'
        let end = svg[abs_start..].find('"').unwrap_or(0) + abs_start;
        let points_str = &svg[abs_start..end];
        for token in points_str.split_whitespace() {
            if let Some((x_str, _y_str)) = token.split_once(',') {
                if let Ok(x) = x_str.parse::<u32>() {
                    assert!(
                        x <= max_allowed,
                        "usecase/06 has waypoint x={x} which exceeds canvas+20 ({max_allowed}); \
                         U-turn routing artifact regression (#1484)"
                    );
                }
            }
        }
        pos = end + 1;
        if pos >= svg.len() {
            break;
        }
    }
}

/// #1360 — activity bespoke slots: 02_if_then_else.puml must be ≤ 500 px wide and ≤ 500 px tall.
#[test]
fn activity_if_then_else_fits_500x500() {
    let svg = render_fixture("docs/examples/activity/02_if_then_else.puml");
    let (w, h) = svg_dimensions(&svg);
    assert!(
        w <= 500,
        "activity 02_if_then_else width {w} exceeds 500 px (issue #1360)"
    );
    assert!(
        h <= 500,
        "activity 02_if_then_else height {h} exceeds 500 px (issue #1360)"
    );
}
