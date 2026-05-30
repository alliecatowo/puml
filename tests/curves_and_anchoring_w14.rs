//! Wave-14 curves + endpoint anchoring regression suite (#1318, #1319).
//!
//! Covers four invariants in one focused test crate:
//!
//! 1. Channel-router edges report their polyline in the model's original
//!    `from → to` order even for edges that the rank-assignment phase
//!    reverses to break cycles.  This guarantees endpoint snapping in
//!    box_grid_edges and class_relations attaches the source-side anchor
//!    to the source bbox and the target-side anchor to the target bbox.
//! 2. Sequence self-message loops emit a rounded curve (quadratic-bezier
//!    corners) instead of a sharp 3-segment polyline.
//! 3. Class self-association edges emit a curved arc tagged with the
//!    `uml-self-association` class instead of falling through to the
//!    orthogonal router (which collapses to a zero-length line).
//! 4. State self-transitions emit a visible cubic-bezier arc, and
//!    activity back-edges emit rounded corner arcs while preserving an
//!    upward `<line>` segment for the vertical leg.

use puml::render_source_to_svg;

#[test]
fn part_a_component_diamond_arrow_lands_on_target_bbox_top_edge() {
    // Regression for #1318: a composition relation drawn from a node in a
    // lower rank to a node in a higher rank used to anchor its first
    // waypoint at the source bbox's TOP-LEFT corner instead of the
    // TOP-CENTER edge midpoint, because the router published the polyline
    // in layout-time (post-cycle-break) order while the consumer snapped
    // endpoints in original-model order.
    //
    // Verify the composition diamond now lands cleanly on A's bottom edge.
    let svg = render_source_to_svg(
        r#"@startuml
component A
component B
component C
component D
A --> B : calls
A ..> C : uses
B <|-- D : extends
C --* A : composed
@enduml
"#,
    )
    .expect("component-6 should render");

    // The composition arrow goes C → A, with the filled diamond at A.
    // After the router-order fix, the relation starts on C's TOP edge
    // (not its top-LEFT corner) and ends on A's BOTTOM edge midpoint.
    // Find the relation element with from=C and to=A (must scan SVG as
    // one stream; svg has no newlines in single-file output).
    // Under EdgeRouting::Splines (default) the element is a <path>; under
    // Polyline/Ortho it is a <polyline>.  Accept both.
    let from_c_idx = svg
        .match_indices("data-uml-from=\"C\"")
        .find(|(idx, _)| {
            let lookbehind = &svg[idx.saturating_sub(120)..*idx];
            lookbehind.contains("polyline class=\"uml-relation\"")
                || lookbehind.contains("path class=\"uml-relation\"")
        })
        .map(|(idx, _)| idx)
        .expect("expected uml-relation element with data-uml-from=C");
    let tag_end = svg[from_c_idx..]
        .find("/>")
        .map(|off| from_c_idx + off)
        .expect("relation element self-close");
    let rel_el = &svg[from_c_idx..tag_end];
    assert!(
        rel_el.contains("data-uml-to=\"A\""),
        "C→A relation should target A, got: {rel_el}",
    );
    // Extract coordinates from either `points="..."` (polyline) or `d="..."` (path).
    let coords: Vec<(i32, i32)> = if let Some(rest) = rel_el.split("points=\"").nth(1) {
        let pts_attr = rest.split('"').next().expect("points attr close quote");
        pts_attr
            .split_whitespace()
            .filter_map(|tok| {
                let mut it = tok.split(',');
                Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
            })
            .collect()
    } else {
        // Parse coordinate pairs from SVG path d="..." data.
        // Command letters (M/C/L/Z) are stripped; consecutive float pairs become waypoints.
        let d_attr = rel_el
            .split("d=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .expect("path d attribute");
        let mut result: Vec<(i32, i32)> = Vec::new();
        let mut pending: Vec<f64> = Vec::new();
        for tok in d_attr.split(|c: char| c == ',' || c.is_whitespace()) {
            if tok.is_empty() {
                continue;
            }
            if let Ok(n) = tok.parse::<f64>() {
                pending.push(n);
                if pending.len() == 2 {
                    result.push((pending[0].round() as i32, pending[1].round() as i32));
                    pending.clear();
                }
            } else {
                pending.clear();
            }
        }
        result
    };
    assert!(
        coords.len() >= 3,
        "composition relation should have at least 3 waypoints, got {coords:?}",
    );

    // Find A's bbox by scanning for the `uml-component` rect that follows
    // `data-uml-id="A"`.  We grab the substring between that descriptor
    // and the next `<rect …/>` close (the A node itself) so we never
    // pick up the small port stubs or the SVG <defs> markers that come
    // earlier in the document.
    let a_node_chunk = svg
        .split("data-uml-id=\"A\"")
        .nth(1)
        .expect("A descriptor")
        .split("class=\"uml-node uml-component\"")
        .nth(1)
        .expect("A uml-component rect")
        .split("/>")
        .next()
        .expect("rect close");
    let a_x: i32 = extract_attr(a_node_chunk, "x=\"").expect("A.x");
    let a_y: i32 = extract_attr(a_node_chunk, "y=\"").expect("A.y");
    let a_w: i32 = extract_attr(a_node_chunk, "width=\"").expect("A.w");
    let a_h: i32 = extract_attr(a_node_chunk, "height=\"").expect("A.h");

    let (end_x, end_y) = coords.last().copied().unwrap();
    assert_eq!(
        end_y,
        a_y + a_h,
        "composition arrow should END on A's bottom edge (y={}), got y={}; coords={:?}",
        a_y + a_h,
        end_y,
        coords,
    );
    assert!(
        end_x >= a_x && end_x <= a_x + a_w,
        "composition arrow X={end_x} should fall within A's bbox [{}, {}]",
        a_x,
        a_x + a_w,
    );

    // Also assert no waypoint lies at C's top-left CORNER.  Before #1318,
    // the source-side snap collapsed onto (392, 392) — C's top-left
    // corner — instead of C's top-edge midpoint.
    let (start_x, start_y) = coords.first().copied().unwrap();
    let c_node_chunk = svg
        .split("data-uml-id=\"C\"")
        .nth(1)
        .expect("C descriptor")
        .split("class=\"uml-node uml-component\"")
        .nth(1)
        .expect("C uml-component rect")
        .split("/>")
        .next()
        .expect("rect close");
    let c_x: i32 = extract_attr(c_node_chunk, "x=\"").expect("C.x");
    let c_y: i32 = extract_attr(c_node_chunk, "y=\"").expect("C.y");
    let c_w: i32 = extract_attr(c_node_chunk, "width=\"").expect("C.w");
    assert_eq!(
        start_y, c_y,
        "composition arrow should START on C's top edge (y={c_y}), got y={start_y}; coords={coords:?}",
    );
    let c_cx = c_x + c_w / 2;
    assert!(
        (start_x - c_cx).abs() <= 16,
        "composition arrow should start near C's top-center x≈{c_cx}, got x={start_x}; was on the corner before #1318",
    );
}

fn extract_attr(chunk: &str, key: &str) -> Option<i32> {
    chunk
        .split(key)
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse().ok())
}

#[test]
fn part_b_sequence_self_message_emits_rounded_curve() {
    // Regression for #1319: sequence self-messages used to emit a single
    // <path d="M ... L ... L ... L ..."> with only L commands (3 sharp
    // 90° corners).  After the curve refresh the path uses Q-style
    // quadratic-bezier joints so the loop reads as a rounded "D" shape.
    let svg = render_source_to_svg(
        r#"@startuml
participant Alice
Alice -> Alice : process
@enduml
"#,
    )
    .expect("self-message should render");

    let self_loop_path = svg
        .split('<')
        .find(|el| el.starts_with("path") && el.contains("M ") && el.contains(" Q "))
        .expect("sequence self-message loop should use Q-style curves now");
    assert!(
        self_loop_path.contains(" Q "),
        "expected quadratic-bezier corner joints in the self-message path, got: {self_loop_path}",
    );
}

#[test]
fn part_b_class_self_association_emits_arc_path() {
    // Regression for #1319: a class self-association (Node --> Node) now
    // emits a dedicated `<path class="uml-relation uml-self-association">`
    // hugging the top-right corner instead of falling through to the
    // orthogonal router, which collapsed to a zero-length line.
    let svg = render_source_to_svg(
        r#"@startuml
class Node {
  +value: int
  +next: Node
}
Node --> Node : next
@enduml
"#,
    )
    .expect("class self-association should render");

    assert!(
        svg.contains("uml-self-association"),
        "class self-association should emit uml-self-association class, got: {svg}",
    );
    assert!(
        svg.contains(">next</text>") || svg.contains(">next<"),
        "class self-association label `next` should be present",
    );
    // Two Q-bezier joints in the arc:
    let arc_block = svg
        .split("uml-self-association")
        .nth(1)
        .and_then(|s| s.split("/>").next())
        .expect("arc element");
    let q_count = arc_block.matches(" Q ").count();
    assert!(
        q_count >= 2,
        "class self-association arc should have ≥2 Q corners, got {q_count} in: {arc_block}",
    );
}

#[test]
fn part_b_state_self_transition_emits_visible_cubic_arc() {
    // Regression for #1319: state self-transitions used to render a
    // collapsed quadratic curve (start == end), which was invisible.
    // After the curve refresh the path uses a cubic-bezier (C ...) whose
    // start and end anchors are on different edges of the node.
    let svg = render_source_to_svg(
        r#"@startuml
[*] --> Idle
Idle --> Working : start
Working --> Working : retry
Working --> Idle : done
@enduml
"#,
    )
    .expect("state self-transition should render");

    let self_path = svg
        .split('<')
        .find(|el| {
            el.starts_with("path")
                && el.contains("data-state-from=\"Working\"")
                && el.contains("data-state-to=\"Working\"")
        })
        .expect("state self-transition should be present");
    assert!(
        self_path.contains(" C "),
        "state self-transition should use a cubic Bézier (C), got: {self_path}",
    );
    // Extract M and final L/C coordinates — they must NOT be equal anchor.
    let d_attr = self_path
        .split("d=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("path d attribute");
    let m_coord = d_attr.split_whitespace().take(3).collect::<Vec<_>>(); // "M" "x" "y"
    let end_coord: Vec<&str> = d_attr.split_whitespace().rev().take(2).collect();
    assert_ne!(
        format!("{},{}", m_coord[1], m_coord[2]),
        format!("{},{}", end_coord[1], end_coord[0]),
        "state self-transition start ({:?}) and end ({:?}) anchors must differ",
        m_coord,
        end_coord,
    );
}

#[test]
fn part_b_activity_while_back_edge_uses_rounded_corner_arcs() {
    // Regression for #1319: while-loop back-edges previously rendered as
    // a stack of three straight <line> elements with sharp 90° joints.
    // After the curve refresh, the two corners are emitted as <path>
    // elements with Q-style quadratic-bezier joints, while the long
    // vertical segment stays a <line> so existing tests/grep-tooling
    // that detect upward back-edges (y2 < y1) still work.
    let svg = render_source_to_svg(
        r#"@startuml
start
while (keep going?) is (yes)
  :loop body;
endwhile (stop)
:finished;
stop
@enduml
"#,
    )
    .expect("while-loop activity should render");

    // At least one upward <line> segment (y2 < y1).
    let has_upward_line = svg
        .split('<')
        .filter(|el| el.starts_with("line"))
        .any(|el| {
            let y1 = el
                .split("y1=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .and_then(|s| s.parse::<i32>().ok());
            let y2 = el
                .split("y2=\"")
                .nth(1)
                .and_then(|s| s.split('"').next())
                .and_then(|s| s.parse::<i32>().ok());
            matches!((y1, y2), (Some(a), Some(b)) if b < a)
        });
    assert!(
        has_upward_line,
        "while-loop back-edge must keep one upward <line> segment for tooling",
    );

    // At least one corner-arc <path> with a Q command.
    let q_corner_count = svg
        .split('<')
        .filter(|el| {
            el.starts_with("path")
                && el.contains("M ")
                && el.contains(" Q ")
                && !el.contains("data-state-from")
                && !el.contains("data-uml-from")
        })
        .count();
    assert!(
        q_corner_count >= 2,
        "back-edge should emit ≥2 rounded corner arcs, got {q_corner_count}",
    );
}
