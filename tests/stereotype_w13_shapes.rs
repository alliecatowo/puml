//! Stereotype-as-shape / stereotype-as-guillemets tests (wave-13).
//!
//! Covers:
//! - Non-built-in stereotypes like <<controller>>, <<service>>, <<repository>>
//!   render as «foo» guillemet labels in the class-box header (NOT as raw
//!   `<<foo>>` double-angle-bracket text).
//! - Built-in UML type stereotypes render with canonical «…» header labels and
//!   visually distinct header fill colors.
//! - The <<exception>> keyword maps to a reddish header fill.
//! - The <<enumeration>> word form is synonymous with <<enum>> and produces
//!   «enumeration» label with lemon header.
//! - The <<entity>> stereotype continues to render with the wave-9-A tan header.
//! - Abstract class names are italicized per UML convention.
//!
//! Refs #88

/// Render a source string to SVG, panicking on failure.
fn svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

// ── Non-built-in stereotypes render as «...» guillemets, NOT <<...>> text ──────

/// <<controller>>, <<service>>, <<repository>> are DDD/Spring conventions not
/// built into PlantUML's class grammar.  They must appear as «foo» guillemet
/// labels above the class name, not as raw `<<foo>>` angle-bracket characters.
#[test]
fn stereotype_unknown_renders_as_guillemets_not_double_angle_brackets() {
    let s = svg(
        r#"@startuml
class UserController <<controller>>
class UserService <<service>>
class UserRepository <<repository>>
@enduml
"#,
    );
    // Guillemet forms must be present.
    assert!(
        s.contains("\u{ab}controller\u{bb}"),
        "<<controller>> must render as «controller» guillemets: {s}"
    );
    assert!(
        s.contains("\u{ab}service\u{bb}"),
        "<<service>> must render as «service» guillemets: {s}"
    );
    assert!(
        s.contains("\u{ab}repository\u{bb}"),
        "<<repository>> must render as «repository» guillemets: {s}"
    );
    // Raw double-angle-bracket text must NOT appear in the SVG output.
    // SVG-escapes: `<` → `&lt;`, `>` → `&gt;`.  We check both the literal
    // form (blocked by the XML escaper) and the escaped entity form.
    assert!(
        !s.contains("&lt;&lt;controller&gt;&gt;"),
        "raw <<controller>> must not appear escaped in SVG: {s}"
    );
    assert!(
        !s.contains("&lt;&lt;service&gt;&gt;"),
        "raw <<service>> must not appear escaped in SVG: {s}"
    );
    assert!(
        !s.contains("&lt;&lt;repository&gt;&gt;"),
        "raw <<repository>> must not appear escaped in SVG: {s}"
    );
}

/// A completely arbitrary user stereotype must also render as guillemets.
#[test]
fn stereotype_arbitrary_user_label_renders_as_guillemets() {
    let s = svg(
        r#"@startuml
class Widget <<myCustomStereotype>>
@enduml
"#,
    );
    assert!(
        s.contains("\u{ab}myCustomStereotype\u{bb}"),
        "arbitrary user stereotype must render as «myCustomStereotype»: {s}"
    );
    assert!(
        !s.contains("&lt;&lt;myCustomStereotype"),
        "raw <<…>> text must not appear in SVG: {s}"
    );
}

// ── <<entity>> continues to render with the wave-9-A tan header ────────────────

/// The <<entity>> stereotype must produce «entity» guillemet label and the
/// characteristic warm-tan (#fde68a) header introduced in wave-9-A.
#[test]
fn stereotype_entity_continues_to_render_with_tan_header_after_w9a() {
    let s = svg(
        r#"@startuml
class User <<entity>>
@enduml
"#,
    );
    assert!(
        s.contains("\u{ab}entity\u{bb}"),
        "<<entity>> must render «entity» guillemet label: {s}"
    );
    assert!(
        s.contains("#fde68a"),
        "<<entity>> must carry the IE tan header fill #fde68a: {s}"
    );
}

// ── <<enumeration>> word form is equivalent to <<enum>> ───────────────────────

/// The word `enumeration` used as a stereotype (e.g. in generated diagrams
/// from some tools) must behave identically to the `enum` keyword form:
/// «enumeration» label and lemon-yellow (#ffffcc) header.
#[test]
fn stereotype_enumeration_word_form_maps_to_enum_header() {
    let s = svg(
        r#"@startuml
class Color <<enumeration>>
@enduml
"#,
    );
    assert!(
        s.contains("\u{ab}enumeration\u{bb}"),
        "<<enumeration>> must render «enumeration» guillemet label: {s}"
    );
    assert!(
        s.contains("#ffffcc"),
        "<<enumeration>> must carry lemon-yellow header fill #ffffcc: {s}"
    );
}

/// The `enum` keyword form must continue to produce the same canonical label.
#[test]
fn stereotype_enumeration_renders_class_box_with_enumeration_header() {
    let s = svg(
        r#"@startuml
enum Status {
  ACTIVE
  INACTIVE
}
@enduml
"#,
    );
    assert!(
        s.contains("\u{ab}enumeration\u{bb}"),
        "enum keyword must render «enumeration» guillemet label: {s}"
    );
    assert!(
        s.contains("#ffffcc"),
        "enum keyword must carry lemon-yellow header fill #ffffcc: {s}"
    );
}

// ── <<exception>> gets a reddish header ───────────────────────────────────────

/// The `exception` keyword produces <<exception>> which must render with the
/// canonical reddish (#fecaca) header fill.
#[test]
fn stereotype_exception_renders_with_reddish_header() {
    let s = svg(
        r#"@startuml
exception IOException
@enduml
"#,
    );
    assert!(
        s.contains("\u{ab}exception\u{bb}"),
        "exception keyword must render «exception» guillemet label: {s}"
    );
    assert!(
        s.contains("#fecaca"),
        "exception keyword must carry reddish header fill #fecaca: {s}"
    );
}

// ── <<abstract>> italicizes the class name ────────────────────────────────────

/// Abstract classes must have the class name rendered in italic font-style per
/// UML convention (fix #767).
#[test]
fn stereotype_abstract_renders_class_name_in_italic() {
    let s = svg(
        r#"@startuml
abstract class Vehicle
@enduml
"#,
    );
    assert!(
        s.contains("font-style=\"italic\""),
        "abstract class name must be rendered in italic: {s}"
    );
    assert!(
        s.contains("\u{ab}abstract\u{bb}"),
        "abstract class must carry «abstract» guillemet label: {s}"
    );
}

// ── Custom controller renders only as guillemet label (no shape change) ────────

/// <<controller>> is NOT a PlantUML built-in shape — it must stay as a plain
/// class box with a «controller» guillemet label; no shape geometry should
/// change.  The rendered SVG must contain a `<rect>` element (the class box).
#[test]
fn stereotype_custom_controller_renders_as_guillemets_label() {
    let s = svg(
        r#"@startuml
class Auth <<controller>>
@enduml
"#,
    );
    // Must still be a rect-based class box.
    assert!(
        s.contains("<rect"),
        "<<controller>> must render as a plain class box (rect): {s}"
    );
    // Must show the guillemet label.
    assert!(
        s.contains("\u{ab}controller\u{bb}"),
        "<<controller>> must render «controller» guillemet label in the header: {s}"
    );
    // The class name must still appear.
    assert!(
        s.contains(">Auth<") || s.contains("Auth"),
        "class name Auth must appear in the output: {s}"
    );
}

// ── Multiple stereotypes in one declaration ───────────────────────────────────

/// A class can carry both a built-in type marker and a user stereotype.
/// Both must appear in the header as guillemet labels.
#[test]
fn stereotype_multiple_on_same_class_all_render_in_header() {
    let s = svg(
        r#"@startuml
class UserService <<service>> <<internal>>
@enduml
"#,
    );
    assert!(
        s.contains("\u{ab}service\u{bb}"),
        "first stereotype «service» must appear: {s}"
    );
    assert!(
        s.contains("\u{ab}internal\u{bb}"),
        "second stereotype «internal» must appear: {s}"
    );
}
