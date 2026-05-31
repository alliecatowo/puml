//! `skinparam linetype ortho` for the Chen / IE entity-relationship family.
//!
//! PlantUML §20.3 documents `skinparam linetype ortho` as a workaround for
//! angled crow's-feet in IE-notation ER diagrams: "Currently the crows feet
//! do not look very good when the relationship is drawn at an angle to the
//! entity. This can be avoided by using the linetype ortho skinparam."
//!
//! This is the ONLY documented use case for `skinparam linetype` in the
//! PlantUML 1.2025 Language Reference Guide (confirmed in the 2026-05-31
//! edge-routing forensic at
//! `docs/internal/forensics/2026-05-31-plantuml-edge-routing-investigation.md`
//! Appendix B, lines 18875 and 18960 of the reference-raw text).
//!
//! Tests:
//! 1. Default mode emits straight `<line>` edges (no orthogonal elbow).
//! 2. `skinparam linetype ortho` emits `<polyline>` edges (right-angle elbow).
//! 3. Ortho polyline contains only axis-aligned segments (two-segment elbow:
//!    the mid-point shares either x or y with one of the endpoints).
//! 4. `skinparam linetype polyline` keeps the default straight-line behavior.
//!
//! Refs #1392

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

/// The §20.3 specimen fixture — a simple IE-style ER diagram with two entities
/// and one relationship, mirroring the crow's-feet example from the spec.
const CHEN_FIXTURE: &str = r#"
@startchen
entity Student {
  ID <<key>>
  Name
}
entity Course {
  Code <<key>>
  Title
}
relationship Enrolled {
}
Student =N= Enrolled
Enrolled =M= Course
@endchen
"#;

/// Same fixture with `skinparam linetype ortho` prepended.
const CHEN_ORTHO_FIXTURE: &str = r#"
@startchen
skinparam linetype ortho
entity Student {
  ID <<key>>
  Name
}
entity Course {
  Code <<key>>
  Title
}
relationship Enrolled {
}
Student =N= Enrolled
Enrolled =M= Course
@endchen
"#;

/// Same fixture with `skinparam linetype polyline` — should keep straight lines.
const CHEN_POLYLINE_FIXTURE: &str = r#"
@startchen
skinparam linetype polyline
entity Student {
  ID <<key>>
  Name
}
entity Course {
  Code <<key>>
  Title
}
relationship Enrolled {
}
Student =N= Enrolled
Enrolled =M= Course
@endchen
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

/// Without `skinparam linetype ortho` the renderer emits straight `<line>`
/// elements, not `<polyline>` elements, for chen-edge connections.
#[test]
fn chen_default_emits_straight_lines() {
    let svg = render_svg(CHEN_FIXTURE);
    assert!(
        svg.contains("class=\"chen-edge\""),
        "SVG must contain chen-edge elements"
    );
    // Default uses <line> not <polyline>
    assert!(
        svg.contains("<line class=\"chen-edge\""),
        "Default routing should emit <line> for chen edges, got:\n{svg}"
    );
    // No ortho polylines in default mode
    assert!(
        !svg.contains("<polyline class=\"chen-edge\""),
        "Default routing must NOT emit <polyline> for chen edges, got:\n{svg}"
    );
}

/// `skinparam linetype ortho` causes the renderer to emit `<polyline>` elements
/// for all chen-edge connections (right-angle elbow paths) instead of `<line>`.
#[test]
fn chen_linetype_ortho_emits_polyline_edges() {
    let svg = render_svg(CHEN_ORTHO_FIXTURE);
    assert!(
        svg.contains("class=\"chen-edge\""),
        "SVG must contain chen-edge elements"
    );
    // Ortho mode uses <polyline> not <line>
    assert!(
        svg.contains("<polyline class=\"chen-edge\""),
        "skinparam linetype ortho must emit <polyline> for chen edges, got:\n{svg}"
    );
    // No straight lines in ortho mode
    assert!(
        !svg.contains("<line class=\"chen-edge\""),
        "skinparam linetype ortho must NOT emit straight <line> for chen edges, got:\n{svg}"
    );
}

/// Every `<polyline class="chen-edge">` emitted in ortho mode must have
/// exactly three points — `x1,y1 mx,my x2,y2` — and the elbow mid-point must
/// share either the x-coordinate with one anchor or the y-coordinate with the
/// other (i.e., the path is a 2-segment right-angle elbow).
#[test]
fn chen_linetype_ortho_polylines_are_axis_aligned() {
    let svg = render_svg(CHEN_ORTHO_FIXTURE);

    // Extract all polyline points= attributes for chen-edge elements
    let mut found_polyline = false;
    for segment in svg.split("<polyline class=\"chen-edge\"") {
        if segment.starts_with('<') {
            continue; // skip before first match
        }
        // Find points="..." in this segment
        let Some(pts_start) = segment.find("points=\"") else {
            continue;
        };
        let after = &segment[pts_start + 8..];
        let Some(pts_end) = after.find('"') else {
            continue;
        };
        let pts_str = &after[..pts_end];

        // Parse the three coordinate pairs
        let pts: Vec<(f64, f64)> = pts_str
            .split_whitespace()
            .filter_map(|pair| {
                let mut it = pair.splitn(2, ',');
                let x: f64 = it.next()?.parse().ok()?;
                let y: f64 = it.next()?.parse().ok()?;
                Some((x, y))
            })
            .collect();

        assert_eq!(
            pts.len(),
            3,
            "Ortho chen-edge polyline must have exactly 3 points (start, elbow, end), got {}: {:?}",
            pts.len(),
            pts
        );

        let (x1, y1) = pts[0];
        let (mx, my) = pts[1];
        let (x2, y2) = pts[2];

        // The elbow point must share x with start OR y with start (horizontal-first elbow)
        // OR share x with end OR y with end (vertical-first elbow).
        let axis_aligned = (mx == x2 && my == y1)  // horizontal first: elbow at (x2, y1)
            || (mx == x1 && my == y2); // vertical first: elbow at (x1, y2)

        assert!(
            axis_aligned,
            "Ortho elbow mid-point ({mx},{my}) is not axis-aligned relative to \
             start ({x1},{y1}) and end ({x2},{y2}). Expected (x2,y1)=({x2},{y1}) \
             or (x1,y2)=({x1},{y2})."
        );

        found_polyline = true;
    }

    assert!(
        found_polyline,
        "Expected at least one <polyline class=\"chen-edge\"> in ortho SVG"
    );
}

/// `skinparam linetype polyline` is treated the same as the default — straight
/// `<line>` edges, not right-angle elbows.
#[test]
fn chen_linetype_polyline_keeps_straight_lines() {
    let svg = render_svg(CHEN_POLYLINE_FIXTURE);
    assert!(
        svg.contains("<line class=\"chen-edge\""),
        "skinparam linetype polyline should keep <line> edges, got:\n{svg}"
    );
    assert!(
        !svg.contains("<polyline class=\"chen-edge\""),
        "skinparam linetype polyline must NOT emit <polyline> for chen edges, got:\n{svg}"
    );
}

/// Ortho mode must not drop cardinality labels — relationship edges that carry
/// a label should still render the `chen-cardinality` text in ortho mode.
#[test]
fn chen_linetype_ortho_preserves_cardinality_labels() {
    let src = r#"
@startchen
skinparam linetype ortho
entity Employee {
  ID <<key>>
}
entity Department {
  Name <<key>>
}
relationship WorksIn {
}
Employee -N- WorksIn
WorksIn -1- Department
@endchen
"#;
    let svg = render_svg(src);
    // cardinality labels must still be present
    assert!(
        svg.contains("class=\"chen-cardinality\""),
        "Ortho mode must preserve cardinality labels, got:\n{svg}"
    );
    // and the polyline elbow must be used
    assert!(
        svg.contains("<polyline class=\"chen-edge\""),
        "Ortho mode must emit polyline edges, got:\n{svg}"
    );
}
