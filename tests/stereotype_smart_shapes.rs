//! Smart-default stereotype-to-shape mapping tests (issue #1285).
//!
//! Covers:
//! - Each of the 8 new DDD / architectural stereotypes (controller, service,
//!   repository, value, aggregate, factory, datatype, utility) renders with the
//!   correct SVG element class AND header fill colour.
//! - The wave-9-A `<<entity>>` mapping is NOT regressed.
//! - The wave-13 `<<enumeration>>` / `<<exception>>` mappings are NOT regressed.
//! - User-defined stereotypes that are NOT in the smart-default table continue
//!   to render as plain class boxes with a guillemet label.
//!
//! Refs #1285

/// Render a source string to SVG, panicking on failure.
fn svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

// ── <<controller>> — hexagon, light-blue header ──────────────────────────────

/// `<<controller>>` must render as a flat-top hexagon with a light-blue
/// (#bfdbfe) header and «controller» guillemet label.
#[test]
fn stereotype_controller_renders_as_hexagon_with_blue_header() {
    let s = svg(r#"@startuml
class AuthController <<controller>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}controller\u{bb}"),
        "<<controller>> must render «controller» guillemet label: {s}"
    );
    assert!(
        s.contains("#bfdbfe"),
        "<<controller>> must carry light-blue header fill #bfdbfe: {s}"
    );
    assert!(
        s.contains("uml-stereotype-controller"),
        "<<controller>> must carry the uml-stereotype-controller CSS class: {s}"
    );
    // Hexagon is rendered as a <polygon>
    assert!(
        s.contains("<polygon"),
        "<<controller>> must render as a <polygon> (hexagon): {s}"
    );
}

// ── <<service>> — pill / rounded rect, light-green header ────────────────────

/// `<<service>>` must render as a pill (rounded-rect with high rx/ry) with a
/// light-green (#bbf7d0) header and «service» guillemet label.
#[test]
fn stereotype_service_renders_as_pill_with_green_header() {
    let s = svg(r#"@startuml
class UserService <<service>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}service\u{bb}"),
        "<<service>> must render «service» guillemet label: {s}"
    );
    assert!(
        s.contains("#bbf7d0"),
        "<<service>> must carry light-green header fill #bbf7d0: {s}"
    );
    assert!(
        s.contains("uml-stereotype-service"),
        "<<service>> must carry the uml-stereotype-service CSS class: {s}"
    );
    // Pill is a <rect> with high rx/ry
    assert!(
        s.contains("<rect"),
        "<<service>> must render as a <rect> (pill): {s}"
    );
}

// ── <<repository>> — cylinder, light-tan header ───────────────────────────────

/// `<<repository>>` must render as a cylinder (rect + ellipse caps) with a
/// light-tan (#fef3c7) header and «repository» guillemet label.
#[test]
fn stereotype_repository_renders_as_cylinder_with_tan_header() {
    let s = svg(r#"@startuml
class UserRepository <<repository>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}repository\u{bb}"),
        "<<repository>> must render «repository» guillemet label: {s}"
    );
    assert!(
        s.contains("#fef3c7"),
        "<<repository>> must carry light-tan header fill #fef3c7: {s}"
    );
    assert!(
        s.contains("uml-stereotype-repository"),
        "<<repository>> must carry the uml-stereotype-repository CSS class: {s}"
    );
    // Cylinder uses <ellipse> for the caps
    assert!(
        s.contains("<ellipse"),
        "<<repository>> must render <ellipse> elements (cylinder caps): {s}"
    );
}

// ── <<value>> — hexagon, lavender header ─────────────────────────────────────

/// `<<value>>` (DDD value object) must render as a flat-top hexagon with a
/// lavender (#e9d5ff) header and «value» guillemet label.
#[test]
fn stereotype_value_renders_as_hexagon_with_lavender_header() {
    let s = svg(r#"@startuml
class Money <<value>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}value\u{bb}"),
        "<<value>> must render «value» guillemet label: {s}"
    );
    assert!(
        s.contains("#e9d5ff"),
        "<<value>> must carry lavender header fill #e9d5ff: {s}"
    );
    assert!(
        s.contains("uml-stereotype-value"),
        "<<value>> must carry the uml-stereotype-value CSS class: {s}"
    );
    assert!(
        s.contains("<polygon"),
        "<<value>> must render as a <polygon> (hexagon): {s}"
    );
}

// ── <<aggregate>> — thick-border rounded rect, white header ──────────────────

/// `<<aggregate>>` must render as a thick-border rounded rect with a white
/// (#ffffff) header and «aggregate» guillemet label.
#[test]
fn stereotype_aggregate_renders_as_thick_border_rect_with_white_header() {
    let s = svg(r#"@startuml
class Order <<aggregate>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}aggregate\u{bb}"),
        "<<aggregate>> must render «aggregate» guillemet label: {s}"
    );
    assert!(
        s.contains("#ffffff"),
        "<<aggregate>> must carry white header fill #ffffff: {s}"
    );
    assert!(
        s.contains("uml-stereotype-aggregate"),
        "<<aggregate>> must carry the uml-stereotype-aggregate CSS class: {s}"
    );
}

// ── <<factory>> — rounded rect + salmon header (standard layout) ─────────────

/// `<<factory>>` uses the standard class-box layout but with a distinctive
/// salmon (#fed7aa) header colour and «factory» guillemet label.
#[test]
fn stereotype_factory_renders_with_salmon_header() {
    let s = svg(r#"@startuml
class WidgetFactory <<factory>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}factory\u{bb}"),
        "<<factory>> must render «factory» guillemet label: {s}"
    );
    assert!(
        s.contains("#fed7aa"),
        "<<factory>> must carry salmon header fill #fed7aa: {s}"
    );
    // Factory falls through to the standard rect renderer
    assert!(
        s.contains("<rect"),
        "<<factory>> must render as a <rect>: {s}"
    );
}

// ── <<datatype>> — double-border rectangle, white-gray header ────────────────

/// `<<datatype>>` must render as a double-border rectangle with a white-gray
/// (#f1f5f9) header and «datatype» guillemet label.
#[test]
fn stereotype_datatype_renders_as_double_border_rect_with_whitegray_header() {
    let s = svg(r#"@startuml
class Address <<datatype>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}datatype\u{bb}"),
        "<<datatype>> must render «datatype» guillemet label: {s}"
    );
    assert!(
        s.contains("#f1f5f9"),
        "<<datatype>> must carry white-gray header fill #f1f5f9: {s}"
    );
    assert!(
        s.contains("uml-stereotype-datatype"),
        "<<datatype>> must carry the uml-stereotype-datatype CSS class: {s}"
    );
    // Double-border uses the -inner CSS class
    assert!(
        s.contains("uml-stereotype-datatype-inner"),
        "<<datatype>> must contain a double-border inner rect element: {s}"
    );
}

// ── <<utility>> — rectangle + corner U mark, gray header ─────────────────────

/// `<<utility>>` must render as a rectangle with a corner U-mark path and a
/// gray (#cbd5e1) header.
#[test]
fn stereotype_utility_renders_with_corner_u_mark_and_gray_header() {
    let s = svg(r#"@startuml
class MathUtils <<utility>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}utility\u{bb}"),
        "<<utility>> must render «utility» guillemet label: {s}"
    );
    assert!(
        s.contains("#cbd5e1"),
        "<<utility>> must carry gray header fill #cbd5e1: {s}"
    );
    assert!(
        s.contains("uml-stereotype-utility"),
        "<<utility>> must carry the uml-stereotype-utility CSS class: {s}"
    );
    // Corner U mark is a <path>
    assert!(
        s.contains("uml-stereotype-utility-corner-u"),
        "<<utility>> must contain the corner-U path element: {s}"
    );
}

// ── Regression: wave-9-A <<entity>> mapping is preserved ─────────────────────

/// The wave-9-A <<entity>> mapping must not be regressed by the new entries.
#[test]
fn stereotype_entity_not_regressed_after_1285() {
    let s = svg(r#"@startuml
class User <<entity>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}entity\u{bb}"),
        "<<entity>> must still render «entity» guillemet label: {s}"
    );
    assert!(
        s.contains("#fde68a"),
        "<<entity>> must still carry the IE tan header fill #fde68a: {s}"
    );
}

// ── Regression: wave-13 <<enumeration>> and <<exception>> are preserved ───────

/// The wave-13 <<enumeration>> mapping must not be regressed.
#[test]
fn stereotype_enumeration_not_regressed_after_1285() {
    let s = svg(r#"@startuml
class Status <<enumeration>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}enumeration\u{bb}"),
        "<<enumeration>> must still render «enumeration» label: {s}"
    );
    assert!(
        s.contains("#ffffcc"),
        "<<enumeration>> must still carry lemon-yellow header #ffffcc: {s}"
    );
}

/// The wave-13 <<exception>> mapping must not be regressed.
#[test]
fn stereotype_exception_not_regressed_after_1285() {
    let s = svg(r#"@startuml
exception IOException
@enduml
"#);
    assert!(
        s.contains("\u{ab}exception\u{bb}"),
        "<<exception>> must still render «exception» label: {s}"
    );
    assert!(
        s.contains("#fecaca"),
        "<<exception>> must still carry reddish header #fecaca: {s}"
    );
}

// ── Unknown user stereotypes still get plain class box + guillemet ─────────────

/// A completely unknown stereotype not in the smart-default table must continue
/// to render as a plain class box (`<rect>`) with a guillemet label.
#[test]
fn stereotype_unknown_still_renders_as_plain_class_box() {
    let s = svg(r#"@startuml
class Widget <<myCustomThing>>
@enduml
"#);
    assert!(
        s.contains("\u{ab}myCustomThing\u{bb}"),
        "unknown stereotype must render as «myCustomThing» guillemet label: {s}"
    );
    assert!(
        s.contains("<rect"),
        "unknown stereotype must still render as a plain <rect> class box: {s}"
    );
    assert!(
        !s.contains("<<myCustomThing>>"),
        "raw <<myCustomThing>> must not appear in SVG output: {s}"
    );
}
