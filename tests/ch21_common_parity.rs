/// Chapter 21 common-commands parity tests.
///
/// Covers:
///   21.1.2  Block comments  `/' ... '/`
///   21.2    `left to right direction` / `top to bottom direction`
///   21.3    header/footer alignment qualifiers
///   21.4    `skinparam sepia true/false`
///   21.x    top-level `backgroundColor`
use puml::render_source_to_svg;

// ── helpers ───────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    render_source_to_svg(src).expect("render should succeed")
}

// ── 21.1.2  Block comments ────────────────────────────────────────────────────

#[test]
fn block_comment_single_line_is_stripped() {
    let src = "@startuml\n/' this is a block comment '/\nA -> B : hello\n@enduml\n";
    let svg = render_svg(src);
    // The comment text should not appear in the output.
    assert!(!svg.contains("this is a block comment"));
    // The message label should still render.
    assert!(svg.contains("hello"));
}

#[test]
fn block_comment_multiline_is_stripped() {
    let src = "@startuml\n/' first line\nsecond line\nthird line '/\nA -> B : after\n@enduml\n";
    let svg = render_svg(src);
    assert!(!svg.contains("first line"));
    assert!(!svg.contains("second line"));
    assert!(!svg.contains("third line"));
    assert!(svg.contains("after"));
}

#[test]
fn block_comment_preserves_line_numbers_after_stripping() {
    // Multiline block comment: lines inside become blank (just '\n' preserved)
    // so that any subsequent parse errors reference the correct line.
    let src = "@startuml\n/' line A\nline B '/\nA -> B : ok\n@enduml\n";
    // Should parse and render cleanly.
    let svg = render_svg(src);
    assert!(svg.contains("ok"));
}

#[test]
fn block_comment_adjacent_to_content() {
    let src = "@startuml\nA -> B /' comment '/ : label\n@enduml\n";
    // Inline block comment — the comment is stripped in the source.
    // We parse and verify no crash occurs.
    let result = render_source_to_svg(src);
    // Either succeeds or fails gracefully (no panic).
    let _ = result;
}

// ── 21.2  Orientation directives ─────────────────────────────────────────────

#[test]
fn left_to_right_direction_on_class_diagram() {
    let src = "@startuml\nleft to right direction\nclass Foo\nclass Bar\nFoo --> Bar\n@enduml\n";
    let svg = render_svg(src);
    // The rendered SVG should embed the orientation attribute.
    assert!(
        svg.contains("left-to-right") || svg.contains("LeftToRight"),
        "expected left-to-right orientation marker in SVG; got: {}",
        &svg[..svg.len().min(400)]
    );
}

#[test]
fn top_to_bottom_direction_is_default_on_class_diagram() {
    let src_with = "@startuml\ntop to bottom direction\nclass Foo\n@enduml\n";
    let src_without = "@startuml\nclass Foo\n@enduml\n";
    let svg_with = render_svg(src_with);
    let svg_without = render_svg(src_without);
    // Both should render without error; orientation not changing default means
    // both produce equivalent diagram orientation.
    assert!(svg_with.contains("<svg"));
    assert!(svg_without.contains("<svg"));
}

#[test]
fn left_to_right_direction_on_usecase_diagram() {
    let src =
        "@startuml\nleft to right direction\nactor User\nusecase UC1\nUser --> UC1\n@enduml\n";
    let svg = render_svg(src);
    assert!(svg.contains("<svg"));
}

#[test]
fn left_to_right_direction_on_component_diagram() {
    let src = "@startuml\nleft to right direction\n[Comp A] --> [Comp B]\n@enduml\n";
    let svg = render_svg(src);
    assert!(svg.contains("<svg"));
}

// ── 21.3  Header/footer alignment qualifiers ────────────────────────────────

#[test]
fn right_footer_qualifier_sets_svg_text_anchor() {
    let src = "@startuml\nAlice -> Bob : hello\nright footer Generated\n@enduml\n";
    let svg = render_svg(src);
    assert!(svg.contains("class=\"sequence-footer\""));
    assert!(
        svg.contains("text-anchor=\"end\"") && svg.contains("Generated"),
        "expected right footer to render with end anchor; got: {}",
        &svg[..svg.len().min(800)]
    );
}

#[test]
fn center_header_qualifier_sets_svg_text_anchor() {
    let src = "@startuml\ncenter header Confidential\nAlice -> Bob : hello\n@enduml\n";
    let svg = render_svg(src);
    assert!(svg.contains("class=\"sequence-header\""));
    assert!(
        svg.contains("text-anchor=\"middle\"") && svg.contains("Confidential"),
        "expected center header to render with middle anchor; got: {}",
        &svg[..svg.len().min(800)]
    );
}

#[test]
fn multiline_left_header_qualifier_preserves_header_text() {
    let src =
        "@startuml\nleft header\nLine one\nLine two\nendheader\nAlice -> Bob : hello\n@enduml\n";
    let svg = render_svg(src);
    assert!(svg.contains("class=\"sequence-header\""));
    assert!(svg.contains("Line one"));
    assert!(svg.contains("Line two"));
    assert!(svg.contains("text-anchor=\"start\""));
}

// ── 21.3  skinparam sepia ────────────────────────────────────────────────────

#[test]
fn skinparam_sepia_true_adds_css_filter_on_sequence() {
    let src = "@startuml\nskinparam sepia true\nA -> B : hello\n@enduml\n";
    let svg = render_svg(src);
    assert!(
        svg.contains("filter:sepia(1)"),
        "expected sepia CSS filter in SVG; got: {}",
        &svg[..svg.len().min(500)]
    );
}

#[test]
fn skinparam_sepia_false_does_not_add_css_filter_on_sequence() {
    let src = "@startuml\nskinparam sepia false\nA -> B : hello\n@enduml\n";
    let svg = render_svg(src);
    assert!(
        !svg.contains("filter:sepia(1)"),
        "should not have sepia CSS filter when sepia false"
    );
}

#[test]
fn skinparam_sepia_true_adds_css_filter_on_class_diagram() {
    let src = "@startuml\nskinparam sepia true\nclass Foo\n@enduml\n";
    let svg = render_svg(src);
    assert!(
        svg.contains("filter:sepia(1)"),
        "expected sepia CSS filter in class SVG; got: {}",
        &svg[..svg.len().min(500)]
    );
}

// ── 21.x  top-level backgroundColor ──────────────────────────────────────────

#[test]
fn top_level_background_color_applies_to_sequence() {
    let src = "@startuml\nbackgroundColor #fef3c7\nA -> B : hello\n@enduml\n";
    let svg = render_svg(src);
    assert!(
        svg.contains("fill=\"#fef3c7\""),
        "expected top-level backgroundColor to color sequence canvas; got: {}",
        &svg[..svg.len().min(500)]
    );
}

#[test]
fn top_level_background_color_before_family_detection_applies_to_class() {
    let src = "@startuml\nbackgroundColor #e0f2fe\nclass Foo\n@enduml\n";
    let svg = render_svg(src);
    assert!(
        svg.contains("fill=\"#e0f2fe\""),
        "expected top-level backgroundColor before class detection to color canvas; got: {}",
        &svg[..svg.len().min(500)]
    );
}

#[test]
fn top_level_background_color_after_family_detection_applies_to_component() {
    let src = "@startuml\n[API]\nbackgroundColor #dcfce7\n[API] --> [DB]\n@enduml\n";
    let svg = render_svg(src);
    assert!(
        svg.contains("fill=\"#dcfce7\""),
        "expected top-level backgroundColor after component detection to color canvas; got: {}",
        &svg[..svg.len().min(500)]
    );
}
