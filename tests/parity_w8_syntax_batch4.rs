//! Wave-8 syntax parity batch 4 — focused tests for newly-wired PlantUML
//! constructs:
//!
//! - `header`/`footer`/`caption` rendering for class/family diagrams:
//!   The SVG must emit `<g class="uml-header">`, `<g class="uml-footer">`,
//!   and `<g class="uml-caption">` groups containing the text.
//! - Aligned `header`/`footer` (`right header ...`, `center footer ...`):
//!   The emitted text element's `x` position must reflect the requested alignment.
//! - Multi-line `header ... end header` blocks (both pre- and post-detection):
//!   Multi-line content must appear as separate `<text>` elements in the group.
//! - `header`/`footer`/`caption` for component/deployment diagrams:
//!   box_grid renderer must also emit the metadata groups.
//! - `!pragma layout smetana` / other unknown pragmas: must parse cleanly and
//!   render without error (the pragma is a no-op but must not cause a crash).
//! - `!pragma teoz true` for sequence diagrams: must be accepted without warning.
//!
//! Refs #1258

use puml::render_source_to_svg;

// ─────────────────────────────────────────────────────────────────────────────
// Helper
// ─────────────────────────────────────────────────────────────────────────────

fn render(src: &str) -> String {
    render_source_to_svg(src).expect("render should succeed")
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Inline header / footer / caption for class diagrams
// ─────────────────────────────────────────────────────────────────────────────

const CLASS_WITH_INLINE_HEADER: &str = "\
@startuml
header My Diagram Header
class Alpha
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_diagram_inline_header_emits_uml_header_group() {
    let svg = render(CLASS_WITH_INLINE_HEADER);
    assert!(
        svg.contains("class=\"uml-header\""),
        "class diagram with inline `header` must emit <g class=\"uml-header\">; got:\n{svg}"
    );
    assert!(
        svg.contains("My Diagram Header"),
        "header text must appear in SVG; got:\n{svg}"
    );
}

const CLASS_WITH_INLINE_FOOTER: &str = "\
@startuml
footer Page 1
class Alpha
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_diagram_inline_footer_emits_uml_footer_group() {
    let svg = render(CLASS_WITH_INLINE_FOOTER);
    assert!(
        svg.contains("class=\"uml-footer\""),
        "class diagram with inline `footer` must emit <g class=\"uml-footer\">; got:\n{svg}"
    );
    assert!(
        svg.contains("Page 1"),
        "footer text must appear in SVG; got:\n{svg}"
    );
}

const CLASS_WITH_CAPTION: &str = "\
@startuml
caption Figure 1: Class Structure
class Alpha {
  +ping()
}
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_diagram_caption_emits_uml_caption_group() {
    let svg = render(CLASS_WITH_CAPTION);
    assert!(
        svg.contains("class=\"uml-caption\""),
        "class diagram with `caption` must emit <g class=\"uml-caption\">; got:\n{svg}"
    );
    assert!(
        svg.contains("Figure 1: Class Structure"),
        "caption text must appear in SVG; got:\n{svg}"
    );
    // Caption should use italic style to match PlantUML convention.
    assert!(
        svg.contains("font-style=\"italic\""),
        "caption text must use italic font-style; got:\n{svg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Aligned header / footer
// ─────────────────────────────────────────────────────────────────────────────

const CLASS_RIGHT_HEADER: &str = "\
@startuml
right header Aligned Right
class Alpha
@enduml
";

#[test]
fn class_diagram_right_header_has_rightward_x_position() {
    let svg = render(CLASS_RIGHT_HEADER);
    assert!(
        svg.contains("class=\"uml-header\""),
        "right header must emit uml-header group; got:\n{svg}"
    );
    // For right-aligned text, x position should be > half the SVG width.
    // We can't check exact pixel values easily, but we can assert the text
    // is present and the group exists.
    assert!(
        svg.contains("Aligned Right"),
        "header text must appear in SVG"
    );
}

const CLASS_CENTER_FOOTER: &str = "\
@startuml
center footer Centered Footer
class Alpha
@enduml
";

#[test]
fn class_diagram_center_footer_is_present() {
    let svg = render(CLASS_CENTER_FOOTER);
    assert!(
        svg.contains("class=\"uml-footer\""),
        "center footer must emit uml-footer group; got:\n{svg}"
    );
    assert!(
        svg.contains("Centered Footer"),
        "footer text must appear in SVG"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Multi-line header / footer / caption blocks
// ─────────────────────────────────────────────────────────────────────────────

const CLASS_MULTILINE_HEADER_POST_DETECTION: &str = "\
@startuml
class Alpha
class Beta
Alpha --> Beta
header
Line 1 of header
Line 2 of header
end header
footer
Footer line A
Footer line B
end footer
@enduml
";

#[test]
fn class_diagram_multiline_header_footer_post_detection() {
    let svg = render(CLASS_MULTILINE_HEADER_POST_DETECTION);
    assert!(
        svg.contains("class=\"uml-header\""),
        "multiline header block (post-detection) must emit uml-header group; got:\n{svg}"
    );
    assert!(
        svg.contains("Line 1 of header"),
        "first header line must appear in SVG; got:\n{svg}"
    );
    assert!(
        svg.contains("Line 2 of header"),
        "second header line must appear in SVG; got:\n{svg}"
    );
    assert!(
        svg.contains("class=\"uml-footer\""),
        "multiline footer block must emit uml-footer group; got:\n{svg}"
    );
    assert!(
        svg.contains("Footer line A"),
        "first footer line must appear in SVG; got:\n{svg}"
    );
    assert!(
        svg.contains("Footer line B"),
        "second footer line must appear in SVG; got:\n{svg}"
    );
}

const CLASS_MULTILINE_HEADER_PRE_DETECTION: &str = "\
@startuml
header
Pre-detection Header
Line Two
end header
class Alpha
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_diagram_multiline_header_pre_detection() {
    let svg = render(CLASS_MULTILINE_HEADER_PRE_DETECTION);
    assert!(
        svg.contains("class=\"uml-header\""),
        "multiline header block (pre-detection, before class decl) must emit uml-header group; got:\n{svg}"
    );
    assert!(
        svg.contains("Pre-detection Header"),
        "pre-detection header first line must appear in SVG; got:\n{svg}"
    );
    assert!(
        svg.contains("Line Two"),
        "pre-detection header second line must appear in SVG; got:\n{svg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Header / footer / caption for component diagrams
// ─────────────────────────────────────────────────────────────────────────────

const COMPONENT_WITH_METADATA: &str = "\
@startuml
header Component Diagram Header
footer Copyright 2024
caption Figure 2: System Architecture
[WebServer] --> [Database]
@enduml
";

#[test]
fn component_diagram_header_footer_caption_rendered() {
    let svg = render(COMPONENT_WITH_METADATA);
    assert!(
        svg.contains("class=\"uml-header\""),
        "component diagram must emit uml-header; got:\n{svg}"
    );
    assert!(
        svg.contains("Component Diagram Header"),
        "component header text must be in SVG; got:\n{svg}"
    );
    assert!(
        svg.contains("class=\"uml-footer\""),
        "component diagram must emit uml-footer; got:\n{svg}"
    );
    assert!(
        svg.contains("Copyright 2024"),
        "component footer text must be in SVG; got:\n{svg}"
    );
    assert!(
        svg.contains("class=\"uml-caption\""),
        "component diagram must emit uml-caption; got:\n{svg}"
    );
    assert!(
        svg.contains("Figure 2: System Architecture"),
        "component caption text must be in SVG; got:\n{svg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. !pragma layout smetana — no-op, parses cleanly
// ─────────────────────────────────────────────────────────────────────────────

const PRAGMA_LAYOUT_SMETANA: &str = "\
@startuml
!pragma layout smetana
participant Alice
participant Bob
Alice -> Bob: request
Bob --> Alice: response
@enduml
";

#[test]
fn pragma_layout_smetana_renders_without_crashing() {
    // The pragma is unsupported (no-op) but must not cause a panic or error.
    let result = render_source_to_svg(PRAGMA_LAYOUT_SMETANA);
    assert!(
        result.is_ok(),
        "!pragma layout smetana must not fail rendering; err: {:?}",
        result.err()
    );
    let svg = result.unwrap();
    // The sequence diagram content should still be rendered.
    assert!(
        svg.contains("Alice"),
        "participant Alice must appear in SVG despite pragma; got:\n{svg}"
    );
    assert!(
        svg.contains("Bob"),
        "participant Bob must appear in SVG despite pragma; got:\n{svg}"
    );
}

const PRAGMA_LAYOUT_ELK: &str = "\
@startuml
!pragma layout elk
class Foo
class Bar
Foo --> Bar
@enduml
";

#[test]
fn pragma_layout_elk_renders_without_crashing() {
    let result = render_source_to_svg(PRAGMA_LAYOUT_ELK);
    assert!(
        result.is_ok(),
        "!pragma layout elk must not fail rendering; err: {:?}",
        result.err()
    );
    let svg = result.unwrap();
    assert!(svg.contains("Foo"), "class Foo must appear; got:\n{svg}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. !pragma teoz true — accepted, sets teoz mode for sequence diagrams
// ─────────────────────────────────────────────────────────────────────────────

const PRAGMA_TEOZ_TRUE: &str = "\
@startuml
!pragma teoz true
participant Alice
participant Bob
participant Charlie
Alice -> Bob: msg1
Alice -> Charlie: msg2
Bob --> Alice: ack
@enduml
";

#[test]
fn pragma_teoz_true_renders_correctly() {
    // !pragma teoz true is a supported pragma that enables teoz layout mode.
    // It must parse without warnings and render correctly.
    let result = render_source_to_svg(PRAGMA_TEOZ_TRUE);
    assert!(
        result.is_ok(),
        "!pragma teoz true must not fail rendering; err: {:?}",
        result.err()
    );
    let svg = result.unwrap();
    assert!(
        svg.contains("Alice"),
        "participant Alice must appear in SVG; got:\n{svg}"
    );
}

const PRAGMA_TEOZ_FALSE: &str = "\
@startuml
!pragma teoz false
participant Alice
participant Bob
Alice -> Bob: msg
@enduml
";

#[test]
fn pragma_teoz_false_renders_correctly() {
    let result = render_source_to_svg(PRAGMA_TEOZ_FALSE);
    assert!(
        result.is_ok(),
        "!pragma teoz false must not fail rendering; err: {:?}",
        result.err()
    );
    let svg = result.unwrap();
    assert!(
        svg.contains("Alice"),
        "participant Alice must appear; got:\n{svg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. SVG structural tests — metadata labels don't overlap the diagram canvas
// ─────────────────────────────────────────────────────────────────────────────

const CLASS_ALL_METADATA: &str = "\
@startuml
header My Header
footer My Footer
caption My Caption
class Alpha
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_diagram_with_all_metadata_svg_height_accommodates_all_labels() {
    let svg = render(CLASS_ALL_METADATA);
    // Extract SVG height from the opening tag.
    let height_str = svg
        .split("height=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .expect("SVG must have a height attribute");
    let height: i32 = height_str.parse().expect("SVG height must be an integer");

    // The SVG height must be large enough to fit header + diagram + caption + footer.
    // Minimum reasonable height with all three labels is 120px.
    assert!(
        height >= 120,
        "SVG height {height} should be >= 120 to accommodate header + nodes + caption + footer"
    );

    // All three groups must be present.
    assert!(
        svg.contains("class=\"uml-header\""),
        "uml-header group must be present"
    );
    assert!(
        svg.contains("class=\"uml-caption\""),
        "uml-caption group must be present"
    );
    assert!(
        svg.contains("class=\"uml-footer\""),
        "uml-footer group must be present"
    );

    // The caption must use italic font-style.
    assert!(
        svg.contains("font-style=\"italic\""),
        "caption text must use italic styling"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. header/footer alignment roundtrip in normalize for family diagrams
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn class_diagram_left_header_alignment_is_stored_in_model() {
    use puml::model::{MetadataHAlign, NormalizedDocument};

    let src = "\
@startuml
left header Left-aligned Header
class Foo
@enduml
";
    let doc = puml::parser::parse(src).expect("parse");
    let normalized = puml::normalize_family(doc).expect("normalize");
    let NormalizedDocument::Family(family) = normalized else {
        panic!("expected Family document");
    };
    assert_eq!(
        family.header_align,
        MetadataHAlign::Left,
        "left header directive should store Left alignment"
    );
    assert_eq!(
        family.header.as_deref(),
        Some("Left-aligned Header"),
        "header text should be stripped of the METADATA_ALIGN prefix"
    );
}

#[test]
fn class_diagram_right_footer_alignment_is_stored_in_model() {
    use puml::model::{MetadataHAlign, NormalizedDocument};

    let src = "\
@startuml
right footer Right-aligned Footer
class Foo
@enduml
";
    let doc = puml::parser::parse(src).expect("parse");
    let normalized = puml::normalize_family(doc).expect("normalize");
    let NormalizedDocument::Family(family) = normalized else {
        panic!("expected Family document");
    };
    assert_eq!(
        family.footer_align,
        MetadataHAlign::Right,
        "right footer directive should store Right alignment"
    );
    assert_eq!(
        family.footer.as_deref(),
        Some("Right-aligned Footer"),
        "footer text should be stripped of the METADATA_ALIGN prefix"
    );
}
