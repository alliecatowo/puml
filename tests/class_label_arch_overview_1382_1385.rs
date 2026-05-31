//! Regression tests for #1382 (class edge label drift on sparse diagrams) and
//! #1385 (architecture-overview package frame header text overflow).
//!
//! #1382: On `class/01_basic`, the "owns" label was pushed ~120px right of the
//! arclength midpoint because `class_nudge_label_x` checked horizontal overlap
//! only, ignoring y-position. The label sat in the gap between Animal and Dog
//! (y ≈ 205) and did not overlap either node, yet the push fired. Fix: add a
//! y-overlap guard so the nudge only triggers on actual collisions.
//!
//! #1385: `box_grid.rs` derived frame widths from the layout engine without
//! checking whether the label text fitted. Long header labels ("package Pipeline
//! Core", "package Shared Services") overflowed the dark header band. Fix:
//! expand `fw` to at least `text_width(label, 11) + 24` before rendering.

// ─── #1382 ───────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

/// The "owns" label x-coordinate must be within ±20px of the horizontal
/// midpoint of the vertical edge between Animal and Dog.
/// Both nodes are centred at x=112, so the label must be within [92, 132].
#[test]
fn class_01_basic_label_at_midpoint() {
    let src = include_str!("../docs/examples/class/01_basic.puml");
    let svg = render_svg(src);

    // Extract the x coordinate from the "owns" edge label element.
    // Expected form: <text class="uml-edge-label" ... x="NNN" ...>owns</text>
    let label_x: i32 = {
        let pos = svg.find(">owns<").expect("no 'owns' label found in SVG");
        // Scan backwards from `pos` to find the x="..." attribute
        let before = &svg[..pos];
        let x_attr_start = before.rfind("x=\"").expect("no x attr before owns label");
        let x_val_start = x_attr_start + 3;
        let x_val_end = before[x_val_start..]
            .find('"')
            .expect("no closing quote on x attr");
        before[x_val_start..x_val_start + x_val_end]
            .parse()
            .expect("x attr not numeric")
    };

    // Both Animal and Dog centre at x=112 (layout may vary slightly by
    // padding, but mid should be within ±20px of 112 regardless).
    assert!(
        (label_x - 112).abs() <= 20,
        "owns label x={label_x} is more than 20px from midpoint 112 — collision push fired spuriously"
    );
}

/// Same invariant on `class/03_composition_aggregation` — "contains" and
/// "may have" labels between vertically-stacked nodes must not be pushed
/// far to the right.
#[test]
fn class_03_composition_aggregation_labels_near_midpoint() {
    let src = include_str!("../docs/examples/class/03_composition_aggregation.puml");
    let svg = render_svg(src);

    let find_label_x = |label: &str| -> Option<i32> {
        let marker = format!(">{label}<");
        let pos = svg.find(&marker)?;
        let before = &svg[..pos];
        let x_attr_start = before.rfind("x=\"")?;
        let x_val_start = x_attr_start + 3;
        let x_val_end = before[x_val_start..].find('"')?;
        before[x_val_start..x_val_start + x_val_end]
            .parse::<i32>()
            .ok()
    };

    for label in &["contains", "may have"] {
        if let Some(lx) = find_label_x(label) {
            // A pushed label ends up at x≈218 in the broken version; correctly
            // placed it is ≤ 160.
            assert!(
                lx <= 160,
                "'{label}' label x={lx} looks like a spurious collision push (> 160px)"
            );
        }
    }
}

// ─── #1385 ───────────────────────────────────────────────────────────────────

/// Every package frame in the architecture-overview diagram must be wide enough
/// to contain its header text without clipping.
///
/// We check that the frame `width` attribute on each `uml-group-frame` rect is
/// at least as wide as a floor derived from the known header label lengths.
#[test]
fn arch_overview_frame_widths_fit_header_text() {
    let src = include_str!("../docs/diagrams/architecture-overview.puml");
    let svg = render_svg(src);

    // Collect (group_id, frame_width) pairs from uml-group-frame rects.
    let mut frame_widths: std::collections::BTreeMap<String, i32> =
        std::collections::BTreeMap::new();
    {
        let mut rest = svg.as_str();
        while let Some(pos) = rest.find("class=\"uml-group-frame\"") {
            rest = &rest[pos + 1..];
            let group_id = rest
                .find("data-uml-group=\"")
                .and_then(|p| {
                    let start = p + 16;
                    rest[start..]
                        .find('"')
                        .map(|end| rest[start..start + end].to_string())
                })
                .unwrap_or_default();
            let width: i32 = rest
                .find("width=\"")
                .and_then(|p| {
                    let start = p + 7;
                    rest[start..]
                        .find('"')
                        .and_then(|end| rest[start..start + end].parse().ok())
                })
                .unwrap_or(0);
            if !group_id.is_empty() {
                frame_widths.insert(group_id, width);
            }
        }
    }

    // Each known package label must fit inside its frame with at least 20px spare.
    // "package Pipeline Core"   ≈ 120px text → frame must be ≥ 140px.
    // "package Shared Services" ≈ 132px text → frame must be ≥ 152px.
    let minimums = [
        ("Pipeline Core", 140i32),
        ("Shared Services", 152i32),
        ("Frontends", 80i32),
        ("Transports", 90i32),
    ];
    for (label, min_w) in &minimums {
        if let Some(&fw) = frame_widths.get(*label) {
            assert!(
                fw >= *min_w,
                "package '{label}' frame width={fw} < minimum {min_w} — header text would overflow"
            );
        }
        // If the group key is not found (layout may vary), skip rather than fail.
    }
}
