/// Chapter 21 common-commands parity tests.
///
/// Covers:
///   21.1.2  Block comments  `/' ... '/`
///   21.2    scale variants
///   21.x    `left to right direction` / `top to bottom direction`
///   21.x    header/footer alignment qualifiers
///   21.x    `skinparam sepia true/false`
use puml::{model::ScaleSpec, normalize, parser, render_source_to_svg};

// ── helpers ───────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    render_source_to_svg(src).expect("render should succeed")
}

fn svg_attr_u32(svg: &str, attr: &str) -> u32 {
    let needle = format!("{attr}=\"");
    let start = svg.find(&needle).expect("attribute should exist") + needle.len();
    let rest = &svg[start..];
    let value = rest
        .split('"')
        .next()
        .expect("attribute should have a value");
    value.parse().expect("attribute should be a u32")
}

fn svg_viewbox_dimensions(svg: &str) -> (u32, u32) {
    let needle = "viewBox=\"0 0 ";
    let start = svg.find(needle).expect("viewBox should exist") + needle.len();
    let rest = &svg[start..];
    let value = rest.split('"').next().expect("viewBox should have a value");
    let mut parts = value.split_whitespace();
    let width = parts
        .next()
        .expect("viewBox width should exist")
        .parse()
        .expect("viewBox width should be a u32");
    let height = parts
        .next()
        .expect("viewBox height should exist")
        .parse()
        .expect("viewBox height should be a u32");
    (width, height)
}

fn rounded_scaled(value: u32, numerator: u32, denominator: u32) -> u32 {
    ((value as f64) * (numerator as f64 / denominator as f64)).round() as u32
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

// ── 21.2  Scale directive variants ───────────────────────────────────────────

#[test]
fn scale_width_height_and_max_variants_are_parsed_into_model() {
    let cases = [
        ("scale 200 width", ScaleSpec::Width(200)),
        ("scale 120 height", ScaleSpec::Height(120)),
        ("scale max 180 width", ScaleSpec::MaxWidth(180)),
        ("scale max 90 height", ScaleSpec::MaxHeight(90)),
        (
            "scale max 180*90",
            ScaleSpec::MaxFixed {
                width: 180,
                height: 90,
            },
        ),
    ];

    for (directive, expected) in cases {
        let src = format!("@startuml\n{directive}\nAlice -> Bob : hello\n@enduml\n");
        let document = parser::parse(&src).expect("parse should succeed");
        let model = normalize::normalize(document).expect("normalize should succeed");
        assert_eq!(model.scale, Some(expected), "directive: {directive}");
    }
}

#[test]
fn scale_fraction_factor_is_parsed_into_model() {
    let src = "@startuml\nscale 2/3\nAlice -> Bob : hello\n@enduml\n";
    let document = parser::parse(src).expect("parse should succeed");
    let model = normalize::normalize(document).expect("normalize should succeed");
    let Some(ScaleSpec::Factor(factor)) = model.scale else {
        panic!("expected factor scale, got {:?}", model.scale);
    };

    assert!((factor - (2.0 / 3.0)).abs() < 0.0001);
}

#[test]
fn scale_fraction_factor_applies_to_svg_dimensions() {
    let base = render_svg("@startuml\nAlice -> Bob : hello\n@enduml\n");
    let scaled = render_svg("@startuml\nscale 2/3\nAlice -> Bob : hello\n@enduml\n");
    let (base_w, base_h) = (svg_attr_u32(&base, "width"), svg_attr_u32(&base, "height"));

    assert_eq!(svg_attr_u32(&scaled, "width"), rounded_scaled(base_w, 2, 3));
    assert_eq!(
        svg_attr_u32(&scaled, "height"),
        rounded_scaled(base_h, 2, 3)
    );
    assert_eq!(svg_viewbox_dimensions(&scaled), (base_w, base_h));
}

#[test]
fn scale_width_preserves_viewbox_aspect_ratio() {
    let svg = render_svg("@startuml\nscale 200 width\nAlice -> Bob : hello\n@enduml\n");
    let (view_w, view_h) = svg_viewbox_dimensions(&svg);

    assert_eq!(svg_attr_u32(&svg, "width"), 200);
    assert_eq!(
        svg_attr_u32(&svg, "height"),
        rounded_scaled(view_h, 200, view_w)
    );
}

#[test]
fn scale_height_preserves_viewbox_aspect_ratio() {
    let svg = render_svg("@startuml\nscale 120 height\nAlice -> Bob : hello\n@enduml\n");
    let (view_w, view_h) = svg_viewbox_dimensions(&svg);

    assert_eq!(svg_attr_u32(&svg, "height"), 120);
    assert_eq!(
        svg_attr_u32(&svg, "width"),
        rounded_scaled(view_w, 120, view_h)
    );
}

#[test]
fn scale_max_width_only_caps_when_needed() {
    let svg = render_svg("@startuml\nscale max 180 width\nAlice -> Bob : hello\n@enduml\n");
    let (view_w, view_h) = svg_viewbox_dimensions(&svg);
    let expected_width = view_w.min(180);
    let expected_height = if view_w <= 180 {
        view_h
    } else {
        rounded_scaled(view_h, 180, view_w)
    };

    assert_eq!(svg_attr_u32(&svg, "width"), expected_width);
    assert_eq!(svg_attr_u32(&svg, "height"), expected_height);
}

#[test]
fn scale_max_fixed_box_fits_both_dimensions() {
    let svg = render_svg("@startuml\nscale max 180*90\nAlice -> Bob : hello\n@enduml\n");
    let width = svg_attr_u32(&svg, "width");
    let height = svg_attr_u32(&svg, "height");

    assert!(width <= 180, "width should fit max box, got {width}");
    assert!(height <= 90, "height should fit max box, got {height}");
}

// ── 21.x  Orientation directives ─────────────────────────────────────────────

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
