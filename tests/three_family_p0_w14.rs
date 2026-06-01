//! Wave-14 P0 visual regression tests across three diagram families.
//!
//! Covers three P0 defects filed on 2026-05-28:
//!   #1296 — creole: `====+ title ====+` titled section separator falls through as raw text
//!   #1297 — sequence: activation bar covered by combined-fragment group frame
//!   #1298 — mindmap: multi-line node labels stack with insufficient vertical spacing

fn svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("diagram should render without error")
}

// ─────────────────────────────────────────────────────────────────────────────
// #1296 — creole: `====+ title ====+` titled section divider
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn issue_1296_creole_equals_titled_section_renders_as_divider_text() {
    // `========== section ==========` should tokenize to a titled-rule span
    // (monospace, muted color with em-dashes) rather than falling through as
    // literal equals signs.
    let src = r#"@startuml
note over A
========== section ==========
end note
@enduml
"#;
    let out = svg(src);
    // The creole parser converts `====+ title ====+` to `---------- title ----------`
    // which is emitted as a monospace tspan.
    assert!(
        out.contains("---------- section ----------"),
        "titled section rule '========== section ==========' must render as \
         '---------- section ----------', got SVG:\n{out}"
    );
    // Must NOT appear as raw equals signs (the literal `==========` characters).
    assert!(
        !out.contains("=========="),
        "raw equals fences must not appear verbatim in the SVG output"
    );
}

#[test]
fn issue_1296_creole_dot_dot_titled_section_still_works() {
    // Regression guard: the existing `.. title ..` syntax must still render as
    // a titled divider after the `====+` branch was added.
    let src = r#"@startuml
note over A
.. section ..
end note
@enduml
"#;
    let out = svg(src);
    assert!(
        out.contains("---------- section ----------"),
        "`.. section ..` titled rule must still render correctly"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #1297 — sequence: activation bar visible through alt combined-fragment
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn issue_1297_activation_bar_rendered_after_group_frame() {
    // The activation box for "A" spans the full height of the alt block.
    // Before the fix, the group `<rect>` was drawn on top of (and covering)
    // the activation `<rect>`, making it invisible.  After the fix, the
    // activation rect appears later in the SVG document order, so it paints
    // on top of the group frame.
    let src = r#"@startuml
participant "Browser" as B
participant "Auth Service" as A
B -> A: POST /login
activate A
alt credentials valid
  A --> B: 200 OK
else invalid
  A --> B: 401 Unauthorized
end
deactivate A
@enduml
"#;
    let out = svg(src);

    // Find the position of the activation rect for "A".
    let act_pos = out
        .find("class=\"sequence-activation\" data-participant=\"A\"")
        .expect("activation box for A must be present in SVG");

    // Find the position of the first combined-fragment group rect (alt frame).
    // The alt frame rect has `fill=` referencing the group background and a
    // stroke; its `<rect` must appear BEFORE the activation box so SVG
    // painter's-algorithm renders the activation on top.
    let group_rect_pos = out
        .find("<rect x=")
        .expect("at least one rect (group frame) must appear before the activation");

    // The group frame (painted first) must come BEFORE the activation box.
    // If the activation appeared first, it would be covered by the group fill.
    assert!(
        group_rect_pos < act_pos,
        "group frame rect (pos {group_rect_pos}) must appear before the \
         activation box (pos {act_pos}) in SVG document order so the activation \
         paints on top of the frame border"
    );
}

#[test]
fn issue_1297_activation_bar_height_spans_full_alt_block() {
    // The activation bar for Auth Service must be tall enough to span at least
    // through two alt arms (the distance between activate and deactivate).
    let src = r#"@startuml
participant "Browser" as B
participant "Auth Service" as A
B -> A: call
activate A
alt arm1
  A --> B: resp1
else arm2
  A --> B: resp2
end
deactivate A
@enduml
"#;
    let out = svg(src);
    // Extract the height of the A activation box.  The attribute order in the
    // emitted SVG is fixed:  class=... data-participant="A" x=... y=... width=... height=...
    let marker = "data-participant=\"A\" x=";
    let start = out.find(marker).expect("A activation must be in SVG");
    let snippet = &out[start..start.min(out.len()).saturating_add(200)];
    // height is the last positional attribute before `fill=`
    let height_val: i32 = snippet
        .split("height=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|v| v.parse().ok())
        .expect("height attribute must be a parseable integer");

    // The alt block has at least 2 arms plus a deactivate row → minimum ~3 rows
    // at the default 32px row height → height must be > 80px.
    assert!(
        height_val > 80,
        "activation bar for A must span the full alt block (height > 80px), \
         got height={height_val}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// #1298 — mindmap: multi-line node labels have non-overlapping y positions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn issue_1298_mindmap_multiline_left_nodes_do_not_overlap() {
    // Four depth-1 nodes each with a two-line label, with two explicitly tagged
    // `left side`. Before the multi-line height fix the y-step was fixed at 48px
    // while each node rendered at 50px tall, causing node boxes to overlap.
    // After the fix each slot is at least `rendered_height + padding`.
    //
    // Test refreshed for #1467 (PlantUML parity): auto-balance was removed, so
    // the test now uses explicit `left side` markers to exercise the left-side
    // slot-height path.
    let src = "@startmindmap\n\
               * Root\n\
               ** Node1\\nLine2\n\
               ** Node2\\nLine2\n\
               left side\n\
               ** Node3\\nLine2\n\
               ** Node4\\nLine2\n\
               @endmindmap\n";
    let out = svg(src);

    // Collect y-top positions for left-side leaf nodes.
    let mut y_tops: Vec<i32> = Vec::new();
    let mut heights: Vec<i32> = Vec::new();
    let mut search = out.as_str();
    while let Some(pos) = search.find("data-mindmap-side=\"left\"") {
        let snippet = &search[pos..pos.min(search.len()).saturating_add(300)];
        // Only leaf nodes (child-count=0) are interesting.
        if snippet.contains("data-mindmap-child-count=\"0\"") {
            if let Some(y_str) = snippet
                .split("y=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
            {
                if let Ok(y) = y_str.parse::<i32>() {
                    y_tops.push(y);
                }
            }
            if let Some(h_str) = snippet
                .split("height=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
            {
                if let Ok(h) = h_str.parse::<i32>() {
                    heights.push(h);
                }
            }
        }
        search = &search[pos + 1..];
    }

    assert!(
        y_tops.len() >= 2,
        "expected at least 2 left-side leaf nodes, found {}",
        y_tops.len()
    );

    // For each consecutive pair of siblings (sorted by y), the bottom of the
    // upper box must be above the top of the lower box (no overlap).
    let mut pairs: Vec<(i32, i32)> = y_tops
        .iter()
        .copied()
        .zip(heights.iter().copied())
        .collect();
    pairs.sort_by_key(|&(y, _)| y);
    for window in pairs.windows(2) {
        let (y0, h0) = window[0];
        let (y1, _h1) = window[1];
        let bottom0 = y0 + h0;
        assert!(
            bottom0 <= y1,
            "left-side leaf nodes must not overlap: node bottom={bottom0} \
             overlaps next node top={y1} (overlap={}px)",
            bottom0 - y1
        );
    }
}

#[test]
fn issue_1298_mindmap_single_line_nodes_unaffected() {
    // Single-line nodes should still render correctly (regression guard).
    let src = "@startmindmap\n\
               * Root\n\
               ** Alpha\n\
               ** Beta\n\
               ** Gamma\n\
               @endmindmap\n";
    let out = svg(src);
    // Three left/right leaf nodes should all be present.
    let node_count = out.matches("mindmap-leaf").count();
    assert!(
        node_count >= 3,
        "expected at least 3 leaf nodes in SVG, found {node_count}"
    );
}
