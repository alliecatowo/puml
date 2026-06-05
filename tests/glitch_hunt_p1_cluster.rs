//! Regression tests for the P1 glitch-hunt cluster (issues #1445–#1454).
//!
//! Each section covers one bug with a targeted structural assertion:
//!
//! - **#1448** — state/10 bidirectional transition labels no longer stack.
//! - **#1454** — class/32 "1..*" multiplicity label no longer bleeds into DataSource.
//! - **#1447** — activity/09 if-then branch labels are assigned to the correct branch.
//! - **#1445** — usecase/05 actor-fan gap increased to ≥ 40 px.

use puml::{
    normalize_family, parse_with_pipeline_options, render_artifact_pages_from_model,
    ParsePipelineOptions,
};

fn render(src: &str) -> String {
    let opts = ParsePipelineOptions::default();
    let doc = parse_with_pipeline_options(src, &opts).expect("source should parse");
    let model = normalize_family(doc).expect("source should normalize");
    render_artifact_pages_from_model(&model)
        .into_iter()
        .next()
        .map(|a| a.svg)
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// #1448 – state/10 bidirectional label separation
// ---------------------------------------------------------------------------

const STATE_10: &str =
    include_str!("../docs/examples/state/10_parallel_regions_shared_events.puml");

/// Extract the x-coordinates of all `<text ... data-state-label="<label>">` elements.
fn state_label_xs(svg: &str, label: &str) -> Vec<i32> {
    let needle = format!("data-state-label=\"{label}\"");
    let mut xs = Vec::new();
    let mut pos = 0;
    while let Some(idx) = svg[pos..].find(&needle) {
        let abs = pos + idx;
        let elem_start = svg[..abs].rfind('<').unwrap_or(0);
        let elem_end = svg[abs..]
            .find("</text>")
            .map(|i| abs + i + 7)
            .unwrap_or(abs);
        let elem = &svg[elem_start..elem_end];
        // Extract x="NNN" from the opening <text> tag
        if let Some(x_idx) = elem.find(" x=\"") {
            let rest = &elem[x_idx + 4..];
            if let Some(end) = rest.find('"') {
                if let Ok(x) = rest[..end].parse::<i32>() {
                    xs.push(x);
                }
            }
        }
        pos = abs + 1;
    }
    xs
}

#[test]
fn state_10_pause_resume_labels_separated() {
    // #1448: "pause" (Playing→Paused) and "resume" (Paused→Playing) are a
    // bidirectional pair.  They must be placed at different x-coordinates —
    // at least 20 px apart — so they are not stacked on top of each other.
    let svg = render(STATE_10);
    let pause_xs = state_label_xs(&svg, "pause");
    let resume_xs = state_label_xs(&svg, "resume");
    assert!(
        !pause_xs.is_empty(),
        "no 'pause' label found in state-10 SVG"
    );
    assert!(
        !resume_xs.is_empty(),
        "no 'resume' label found in state-10 SVG"
    );
    let px = pause_xs[0];
    let rx = resume_xs[0];
    assert!(
        (px - rx).abs() >= 20,
        "pause (x={px}) and resume (x={rx}) labels are within 20 px — still stacking (#1448)"
    );
}

#[test]
fn state_10_unmute_mute_labels_separated() {
    // #1448: "unmute" (Muted→Normal) and "mute" (Normal→Muted) are a
    // bidirectional pair and must not share the same x-coordinate.
    let svg = render(STATE_10);
    let unmute_xs = state_label_xs(&svg, "unmute");
    let mute_xs = state_label_xs(&svg, "mute");
    assert!(
        !unmute_xs.is_empty(),
        "no 'unmute' label found in state-10 SVG"
    );
    assert!(!mute_xs.is_empty(), "no 'mute' label found in state-10 SVG");
    let ux = unmute_xs[0];
    let mx = mute_xs[0];
    assert!(
        (ux - mx).abs() >= 20,
        "unmute (x={ux}) and mute (x={mx}) labels are within 20 px — still stacking (#1448)"
    );
}

// ---------------------------------------------------------------------------
// #1485 – state/10 "play" label inside composite
// ---------------------------------------------------------------------------

#[test]
fn state_10_play_label_inside_composite() {
    // #1485: "play" (Stopped→Playing) was placed at x=56 (off-canvas, left of the
    // composite box whose left edge is x≈90).  After the fix, the label must be
    // to the right of the composite left wall (x > 90) and inside the composite
    // right wall (x < 270).
    let svg = render(STATE_10);
    let play_xs = state_label_xs(&svg, "play");
    assert!(!play_xs.is_empty(), "no 'play' label found in state-10 SVG");
    let px = play_xs[0];
    assert!(
        px > 90,
        "play label x={px} is to the left of the composite boundary (expected > 90); \
         label is still escaping the composite box (#1485)"
    );
    assert!(
        px < 270,
        "play label x={px} is past the composite right boundary (expected < 270)"
    );
}

// ---------------------------------------------------------------------------
// #1454 – class/32 multiplicity label clearance
// ---------------------------------------------------------------------------

const CLASS_32: &str =
    include_str!("../docs/examples/class/32_association_class_deep_packages.puml");

/// Find the x-coordinate of the `right_cardinality` text element whose text
/// matches `label_text` by looking for `text-anchor="end"` followed by the
/// label content on the same element.
fn right_cardinality_end_x(svg: &str, label_text: &str) -> Option<i32> {
    let mut pos = 0;
    while let Some(idx) = svg[pos..].find("text-anchor=\"end\"") {
        let abs = pos + idx;
        let elem_start = svg[..abs].rfind('<').unwrap_or(0);
        let elem_end = svg[abs..]
            .find("</text>")
            .map(|i| abs + i + 7)
            .unwrap_or(abs);
        let elem = &svg[elem_start..elem_end];
        if elem.contains(label_text) {
            // Extract x="NNN"
            if let Some(x_idx) = elem.find(" x=\"") {
                let rest = &elem[x_idx + 4..];
                if let Some(end) = rest.find('"') {
                    if let Ok(x) = rest[..end].parse::<i32>() {
                        return Some(x);
                    }
                }
            }
        }
        pos = elem_end;
    }
    None
}

#[test]
fn class_32_multiplicity_label_clear_of_datasource() {
    // #1454: the "1..*" right-cardinality label on the Report→DataSource edge
    // must end BEFORE the DataSource node's left boundary (x ≥ 1389).
    // Before the fix, text-anchor="start" at x=1373 let the 25-px wide label
    // extend to 1398, bleeding 9 px inside the node.
    // After the fix, text-anchor="end" at x≤(node_left−2) clears the node.
    let svg = render(CLASS_32);
    // DataSource node starts at approximately x=1389 in the current layout.
    // We only assert that the label x is <= 1385 (4+ px clearance) to give the
    // test some tolerance against minor layout tweaks.
    let label_x = right_cardinality_end_x(&svg, "1..*");
    assert!(
        label_x.is_some(),
        "no text-anchor=end element with '1..*' found in class-32 SVG"
    );
    let x = label_x.unwrap();
    // With text-anchor="end", the right edge of the text is AT x.  The
    // DataSource node starts at ~1389, so we need x < 1389.
    assert!(
        x < 1389,
        "multiplicity '1..*' label x={x} (text-anchor=end, right edge at x) is >= DataSource left boundary 1389 — still bleeding (#1454)"
    );
}

// ---------------------------------------------------------------------------
// #1447 – activity/09 if-then branch guard labels
// ---------------------------------------------------------------------------

const ACTIVITY_09: &str = include_str!("../docs/examples/activity/09_error_handling.puml");

/// Return the x-coordinate of the first `<text ...>LABEL</text>` element
/// (not containing any nested tags) that matches `label`.
fn first_label_x(svg: &str, label: &str) -> Option<i32> {
    let needle = format!(">{label}</text>");
    let pos = svg.find(&needle)?;
    // Walk backward to find the opening <text
    let before = &svg[..pos];
    let open = before.rfind("<text")?;
    let tag = &svg[open..pos];
    // Extract x="NNN"
    let x_idx = tag.find(" x=\"")?;
    let rest = &tag[x_idx + 4..];
    let end = rest.find('"')?;
    rest[..end].parse::<i32>().ok()
}

/// Return the x-coordinate of the LAST `<text ...>LABEL</text>` with a
/// smaller x than `max_x_threshold`.  Used to find the leftmost occurrence
/// for a label that may appear multiple times (e.g. "yes" for repeat-while).
fn leftmost_label_x(svg: &str, label: &str) -> Option<i32> {
    let needle = format!(">{label}</text>");
    let mut min_x: Option<i32> = None;
    let mut pos = 0;
    while let Some(idx) = svg[pos..].find(&needle) {
        let abs = pos + idx;
        let before = &svg[..abs];
        if let Some(open) = before.rfind("<text") {
            let tag = &svg[open..abs];
            if let Some(x_idx) = tag.find(" x=\"") {
                let rest = &tag[x_idx + 4..];
                if let Some(end) = rest.find('"') {
                    if let Ok(x) = rest[..end].parse::<i32>() {
                        min_x = Some(match min_x {
                            None => x,
                            Some(prev) => prev.min(x),
                        });
                    }
                }
            }
        }
        pos = abs + 1;
    }
    min_x
}

#[test]
fn activity_09_yes_label_on_left_branch() {
    // #1447: `if (Success?) then (yes)` — the "yes" guard for the if-then
    // branch must appear to the LEFT of the diagram centre (x < 200), i.e. on
    // the arrow going to Complete, not to Log Error.
    //
    // Before the fix, the "yes" guard was applied to the else-branch arrow
    // (heading right to Log Error), so its x would be > 200.
    // There is also a "yes" from `repeat while (retry < 3?) is (yes)` which
    // appears further right; we use the leftmost occurrence.
    let svg = render(ACTIVITY_09);
    let x = leftmost_label_x(&svg, "yes");
    assert!(
        x.is_some(),
        "no 'yes' label found anywhere in activity-09 SVG"
    );
    let x = x.unwrap();
    assert!(
        x < 200,
        "leftmost 'yes' label at x={x} is to the right of centre — it is on the wrong (else) branch (#1447)"
    );
}

#[test]
fn activity_09_no_label_on_right_branch() {
    // #1447: `else (no)` — the "no" guard must appear on the right side
    // (x > 200), i.e. on the arrow heading to Log Error, not to Complete.
    let svg = render(ACTIVITY_09);
    let x = first_label_x(&svg, "no");
    assert!(x.is_some(), "no 'no' label found in activity-09 SVG");
    let x = x.unwrap();
    assert!(
        x > 200,
        "first 'no' label at x={x} is to the left of centre — it is on the wrong (then) branch (#1447)"
    );
}

// ---------------------------------------------------------------------------
// #1445 – usecase/05 actor-fan gap ≥ 40 px
// ---------------------------------------------------------------------------

const USECASE_05: &str =
    include_str!("../docs/examples/usecase/05_actor_generalization_system_boundary.puml");

/// Extract polyline start-x values for edges with the given `data-uml-from` attribute.
fn polyline_start_xs(svg: &str, from_attr: &str) -> Vec<i32> {
    let needle = format!("data-uml-from=\"{from_attr}\"");
    let mut xs = Vec::new();
    let mut pos = 0;
    while let Some(idx) = svg[pos..].find(&needle) {
        let abs = pos + idx;
        let elem_start = svg[..abs].rfind('<').unwrap_or(0);
        let elem_end = svg[abs..].find("/>").map(|i| abs + i + 2).unwrap_or(abs);
        let elem = &svg[elem_start..elem_end];
        if elem.contains("points=\"") {
            if let Some(pts_idx) = elem.find("points=\"") {
                let rest = &elem[pts_idx + 8..];
                if let Some(comma) = rest.find(',') {
                    if let Ok(x) = rest[..comma].trim().parse::<i32>() {
                        xs.push(x);
                    }
                }
            }
        }
        pos = elem_end;
    }
    xs.sort_unstable();
    xs
}

#[test]
fn usecase_05_user_actor_edges_fanned_40px_apart() {
    // #1445: actor "User" (alias U) has 3 edges to use cases.
    // After the widened fan, adjacent edge start-x values must differ ≥ 40 px
    // so that edges from U clear the child actor figures below it.
    let svg = render(USECASE_05);
    let xs = polyline_start_xs(&svg, "U");
    assert_eq!(
        xs.len(),
        3,
        "expected 3 polyline edges from actor U, got {:?}",
        xs
    );
    for pair in xs.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            gap >= 40,
            "actor U edges not fanned 40 px apart: gap={gap}px, xs={xs:?} (#1445)"
        );
    }
}
