//! Wave-9 batch D: timing diagram parity tests for
//! - analog participant with `between MIN and MAX` range
//! - per-event inline color (`@T SIG is VAL #color`)
//! - `mode compact` reducing row height
//! - initial-state events before the first `@T` block

fn svg_attr(tag: &str, key: &str) -> Option<String> {
    let needle = format!("{key}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')?;
    Some(tag[start..start + end].to_string())
}

fn parse_i32_attr(tag: &str, key: &str) -> Option<i32> {
    svg_attr(tag, key)?.parse::<i32>().ok()
}

fn svg_viewbox(svg: &str) -> Option<(i32, i32)> {
    let tag = svg.split("<svg ").nth(1)?.split('>').next()?;
    let viewbox = svg_attr(tag, "viewBox")?;
    let parts = viewbox
        .split_whitespace()
        .filter_map(|v| v.parse::<i32>().ok())
        .collect::<Vec<_>>();
    Some((*parts.get(2)?, *parts.get(3)?))
}

#[derive(Debug, Clone)]
struct SvgRect {
    class: Option<String>,
    height: i32,
}

fn svg_rects(svg: &str) -> Vec<SvgRect> {
    svg.split("<rect ")
        .skip(1)
        .filter_map(|chunk| {
            let tag = chunk.split('>').next()?;
            Some(SvgRect {
                class: svg_attr(tag, "class"),
                height: parse_i32_attr(tag, "height")?,
            })
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: analog participant renders a waveform (polyline + analog-point circles)
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn timing_analog_participant_renders_waveform() {
    let src = r#"@startuml
analog "Voltage" between 0 and 5 as VCC
@50
VCC is 0
@100
VCC is 3.3
@150
VCC is 5
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("render analog timing diagram");

    // The analog renderer emits a `class="timing-analog"` polyline.
    assert!(
        svg.contains("class=\"timing-analog\""),
        "analog participant must emit a timing-analog polyline"
    );
    // Each event becomes a `class="timing-analog-point"` circle.
    let point_count = svg.matches("class=\"timing-analog-point\"").count();
    assert!(
        point_count >= 3,
        "expected at least 3 analog-point circles (one per event), got {point_count}"
    );
    // The row label should say "Voltage".
    assert!(
        svg.contains("Voltage"),
        "analog signal label 'Voltage' must appear in SVG"
    );
    // Kind-tag suppression (#1372): per-lane kind sub-labels ("analog", "concise",
    // "robust", "binary", "clock") are no longer emitted; PlantUML does not render them.
    assert!(
        !svg.contains(">analog<"),
        "kind-tag suppression removes the 'analog' sub-label"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: per-event inline color is applied as a fill on the analog point
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn timing_event_inline_color_applied() {
    let src = r#"@startuml
analog "Voltage" between 0 and 5 as VCC
@50
VCC is 3.3 #palegreen
@100
VCC is 0 #pink
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("render analog timing with inline color");

    // The analog renderer should include the per-event fill colors.
    assert!(
        svg.contains("palegreen") || svg.contains("PaleGreen"),
        "per-event color #palegreen must appear in SVG output"
    );
    assert!(
        svg.contains("pink") || svg.contains("Pink"),
        "per-event color #pink must appear in SVG output"
    );
    // The analog polyline must still be rendered.
    assert!(
        svg.contains("class=\"timing-analog\""),
        "analog polyline must still be present when per-event colors are used"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: `mode compact` reduces the row height
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn timing_mode_compact_reduces_row_height() {
    let src_normal = r#"@startuml
concise "Sig" as S
@0
S is Low
@10
S is High
@enduml
"#;
    let src_compact = r#"@startuml
mode compact
concise "Sig" as S
@0
S is Low
@10
S is High
@enduml
"#;

    let svg_normal = puml::render_source_to_svg(src_normal).expect("render normal timing");
    let svg_compact = puml::render_source_to_svg(src_compact).expect("render compact timing");

    let (_, h_normal) = svg_viewbox(&svg_normal).expect("normal diagram has viewBox");
    let (_, h_compact) = svg_viewbox(&svg_compact).expect("compact diagram has viewBox");

    assert!(
        h_compact < h_normal,
        "mode compact must produce a shorter diagram: normal={h_normal}, compact={h_compact}"
    );

    // The row background rect height should be 48 in compact mode vs 64 in normal.
    let rects_normal = svg_rects(&svg_normal);
    let rects_compact = svg_rects(&svg_compact);

    let max_row_h_normal = rects_normal
        .iter()
        .filter(|r| r.class.is_none()) // row bg rects have no class
        .map(|r| r.height)
        .max()
        .unwrap_or(0);
    let max_row_h_compact = rects_compact
        .iter()
        .filter(|r| r.class.is_none())
        .map(|r| r.height)
        .max()
        .unwrap_or(0);

    assert!(
        max_row_h_compact < max_row_h_normal,
        "compact mode must halve row height: normal={max_row_h_normal}, compact={max_row_h_compact}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: initial state before any `@T` block renders without crashing
// The `@WU` (wake-up / initial) time marker is non-numeric; the event should
// either be silently skipped or treated as time 0, but must never panic.
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn timing_initial_state_before_at_block_renders() {
    let src = r#"@startuml
mode compact
concise "Sig" as S
@WU
S is Low #SlateGrey
@0
S is Low
@10
S is High
@20
S is Low
@enduml
"#;
    // Must not panic.
    let svg = puml::render_source_to_svg(src).expect("render timing with @WU initial state");

    // The explicitly-timed events must be present (normalized to lowercase by the renderer).
    assert!(
        svg.contains("low") || svg.contains("high"),
        "timing states low/high must appear in SVG"
    );
    // Compact mode must still apply (shorter diagram than without compact).
    let (_, h_compact) = svg_viewbox(&svg).expect("diagram has viewBox");
    let src_normal = r#"@startuml
concise "Sig" as S
@0
S is Low
@10
S is High
@20
S is Low
@enduml
"#;
    let svg_normal =
        puml::render_source_to_svg(src_normal).expect("render timing without compact mode");
    let (_, h_normal) = svg_viewbox(&svg_normal).expect("normal diagram has viewBox");
    assert!(
        h_compact < h_normal,
        "compact mode should still apply even with @WU event: compact={h_compact}, normal={h_normal}"
    );
}
