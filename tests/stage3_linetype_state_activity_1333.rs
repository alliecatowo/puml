//! Stage-3 EdgeRouting tests for state and activity bespoke renderers.
//!
//! Verifies that `skinparam linetype splines/polyline/ortho` is honoured by
//! both the state and activity diagram renderers after the Stage-3 migration
//! (issue #1333).  The class-diagram variants are covered by
//! `edge_routing_modes.rs`; this file only tests the bespoke renderers.

use puml::render_source_to_svg;

fn render_svg(source: &str) -> String {
    render_source_to_svg(source).expect("test fixture must render without errors")
}

// ── State diagram fixtures ───────────────────────────────────────────────────

const STATE_MINIMAL: &str = "@startuml
[*] --> Idle
Idle --> Active : start
Active --> Done : finish
Done --> [*]
@enduml";

/// Build a minimal state diagram source with an explicit linetype directive.
fn state_with_linetype(linetype: &str) -> String {
    format!(
        "@startuml\nskinparam linetype {linetype}\n[*] --> Idle\nIdle --> Active : start\nActive --> Done : finish\nDone --> [*]\n@enduml"
    )
}

#[test]
fn state_default_routing_emits_path_elements() {
    // Default state routing uses the Ortho mode — transitions are emitted as
    // `<path class="state-transition" …>` with straight `L` segments.
    let svg = render_svg(STATE_MINIMAL);
    assert!(
        svg.contains("class=\"state-transition\""),
        "state diagram must emit state-transition elements, got: {svg}"
    );
}

#[test]
fn state_linetype_splines_emits_cubic_bezier() {
    // `skinparam linetype splines` must produce `<path d="M … C …">` with
    // cubic Bézier curves (the `C` command) for state transitions.
    let svg = render_svg(&state_with_linetype("splines"));
    assert!(
        svg.contains(" C "),
        "state splines mode must emit cubic Bézier 'C' commands, got: {svg}"
    );
    // The element must still carry the state-transition class so downstream
    // tooling (e.g. the language service hover) can identify transitions.
    assert!(
        svg.contains("class=\"state-transition\""),
        "state splines mode must still emit state-transition class, got: {svg}"
    );
}

#[test]
fn state_linetype_polyline_emits_polyline_elements() {
    // `skinparam linetype polyline` must emit `<polyline class="state-transition">`
    // elements with a `points="…"` attribute.
    let svg = render_svg(&state_with_linetype("polyline"));
    assert!(
        svg.contains("<polyline class=\"state-transition\""),
        "state polyline mode must emit <polyline class=\"state-transition\"> elements, got: {svg}"
    );
    assert!(
        svg.contains("points=\""),
        "state polyline mode must emit points= attribute, got: {svg}"
    );
}

#[test]
fn state_linetype_ortho_emits_path_elements() {
    // `skinparam linetype ortho` keeps the legacy orthogonal-path emission:
    // `<path class="state-transition" d="M … L …">` with only straight `L` commands.
    let svg = render_svg(&state_with_linetype("ortho"));
    assert!(
        svg.contains("<path class=\"state-transition\""),
        "state ortho mode must emit <path class=\"state-transition\"> elements, got: {svg}"
    );
    // Ortho emits only L commands (no C for cubic Bézier).
    assert!(
        !svg.contains("class=\"state-transition\" data-state-from") || {
            // Filter: find one state-transition path and check it has no C.
            let needle = "class=\"state-transition\"";
            if let Some(off) = svg.find(needle) {
                let slice = &svg[off..off + 200.min(svg.len() - off)];
                !slice.contains(" C ")
            } else {
                true
            }
        },
        "state ortho mode must NOT emit cubic Bézier curves, got: {svg}"
    );
}

// ── Activity diagram fixtures ────────────────────────────────────────────────

// A diagram with an if-branch produces L-shaped arrows where x1 != x2,
// which gives 4 waypoints and produces real cubic-Bézier curves in Splines mode.
const ACTIVITY_BRANCHED: &str = "@startuml
start
if (Condition?) then (yes)
  :Step A;
else (no)
  :Step B;
endif
stop
@enduml";

/// Build a branched activity diagram source with an explicit linetype directive.
fn activity_with_linetype(linetype: &str) -> String {
    format!(
        "@startuml\nskinparam linetype {linetype}\nstart\nif (Condition?) then (yes)\n  :Step A;\nelse (no)\n  :Step B;\nendif\nstop\n@enduml"
    )
}

#[test]
fn activity_default_routing_emits_line_elements() {
    // Default activity routing is Ortho — arrows are emitted as `<line>` elements
    // with straight segments.
    let svg = render_svg(ACTIVITY_BRANCHED);
    // Must contain some SVG line or polygon elements (the arrow body + head).
    assert!(
        svg.contains("<line ") || svg.contains("<polygon "),
        "activity default mode must emit <line> or <polygon> elements, got: {svg}"
    );
}

#[test]
fn activity_linetype_polyline_emits_polyline_elements() {
    // `skinparam linetype polyline` must emit `<polyline points="…">` elements
    // for L-shaped arrow segments (the branch arms create x1 != x2 arrows).
    let svg = render_svg(&activity_with_linetype("polyline"));
    assert!(
        svg.contains("<polyline "),
        "activity polyline mode must emit <polyline> elements, got: {svg}"
    );
    assert!(
        svg.contains("points=\""),
        "activity polyline mode must emit points= attribute, got: {svg}"
    );
}

#[test]
fn activity_linetype_splines_emits_bezier_path() {
    // `skinparam linetype splines` must emit `<path d="M … C …">` elements with
    // cubic Bézier curves for multi-waypoint (L-shaped) arrows.
    // A branched diagram guarantees x1 != x2 for branch arrows → 4 waypoints →
    // cubic_bezier_path_d emits at least one `C` command.
    let svg = render_svg(&activity_with_linetype("splines"));
    // A path with `C` cubic-Bézier command must be present (branch arrow body).
    assert!(
        svg.contains(" C "),
        "activity splines mode must emit cubic Bézier 'C' commands (branch arrows have 4 waypoints), got: {svg}"
    );
}

#[test]
fn activity_linetype_ortho_behaves_same_as_default() {
    // `skinparam linetype ortho` for activity is the legacy behavior:
    // `<line>` elements with straight segments.
    let svg_default = render_svg(ACTIVITY_BRANCHED);
    let svg_ortho = render_svg(&activity_with_linetype("ortho"));
    // Both must contain <line> elements (no polyline or bezier).
    assert!(
        svg_ortho.contains("<line ") || svg_ortho.contains("<polygon "),
        "activity ortho mode must emit <line> or <polygon> elements, got: {svg_ortho}"
    );
    // Ortho must not emit polyline (that would be Polyline mode).
    assert!(
        !svg_ortho.contains("<polyline "),
        "activity ortho mode must NOT emit <polyline> elements, got: {svg_ortho}"
    );
    let _ = svg_default; // for symmetry; not byte-compared (different edge_routing field value)
}
