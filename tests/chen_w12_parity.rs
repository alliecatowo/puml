//! Wave-12 batch E — Chen ER advanced features parity tests.
//!
//! Covers: weak entities, multivalued attributes, derived attributes,
//! composite attributes, identifying relationships, and relationship
//! attributes (oval attached to a diamond).
//!
//! Refs #88

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

/// `weak-entity X { }` keyword must produce a double-rectangle glyph.
/// The SVG must contain `chen-weak-entity` (inner border element).
#[test]
fn chen_weak_entity_renders_double_rectangle() {
    let src = r#"
@startchen
weak-entity Dependent {
  DepName : string <<pk-partial>>
  BirthDate : date
}
entity Employee {
  EmpID : number <<key>>
}
relationship Has {
}
Employee 1 -- N Has
Has 1 -- 1 Dependent
@endchen
"#;
    let svg = render_svg(src);
    assert!(!svg.is_empty(), "SVG should be non-empty");
    assert!(
        svg.contains("chen-entity"),
        "SVG should contain entity rectangles: {svg}"
    );
    assert!(
        svg.contains("chen-weak-entity"),
        "SVG should contain chen-weak-entity inner border for weak entity: {svg}"
    );
    assert!(
        svg.contains("Dependent"),
        "SVG should contain the weak-entity label: {svg}"
    );
}

/// `[Phone]` bracket syntax must produce a double-oval (multivalued) glyph.
/// The SVG must contain `chen-multivalued`.
#[test]
fn chen_multivalued_attribute_renders_double_oval() {
    let src = r#"
@startchen
entity Employee {
  EmpID : number <<key>>
  [Phone]
}
@endchen
"#;
    let svg = render_svg(src);
    assert!(!svg.is_empty(), "SVG should be non-empty");
    assert!(
        svg.contains("chen-multivalued"),
        "SVG should contain chen-multivalued double-oval for [Phone]: {svg}"
    );
    assert!(
        svg.contains("Phone"),
        "SVG should contain the attribute label: {svg}"
    );
}

/// `Salary /` trailing-slash syntax must produce a dashed-oval glyph.
/// The SVG must use `stroke-dasharray` on the attribute ellipse.
#[test]
fn chen_derived_attribute_renders_dashed_oval() {
    let src = r#"
@startchen
entity Employee {
  EmpID : number <<key>>
  Salary / : number
}
@endchen
"#;
    let svg = render_svg(src);
    assert!(!svg.is_empty(), "SVG should be non-empty");
    assert!(
        svg.contains("stroke-dasharray"),
        "SVG should contain stroke-dasharray for derived attribute dashed oval: {svg}"
    );
    assert!(
        svg.contains("Salary"),
        "SVG should contain the derived attribute label: {svg}"
    );
}

/// `(FirstName, LastName)` parens syntax must produce composite sub-ovals.
/// The SVG must contain child attribute ovals for FirstName and LastName.
#[test]
fn chen_composite_attribute_renders_sub_ovals() {
    let src = r#"
@startchen
entity Employee {
  EmpID : number <<key>>
  (FirstName, LastName)
}
@endchen
"#;
    let svg = render_svg(src);
    assert!(!svg.is_empty(), "SVG should be non-empty");
    // Both child attribute labels must appear in the SVG
    assert!(
        svg.contains("FirstName"),
        "SVG should contain composite child FirstName: {svg}"
    );
    assert!(
        svg.contains("LastName"),
        "SVG should contain composite child LastName: {svg}"
    );
    // Must have at least two attribute ellipses for the children
    let ellipse_count = svg.matches("chen-attribute").count();
    assert!(
        ellipse_count >= 2,
        "SVG should have at least 2 chen-attribute ellipses for composite children, got {ellipse_count}: {svg}"
    );
}

/// Relationship-to-weak-entity connection triggers `chen-identifying` double-diamond.
/// The SVG must contain `chen-identifying`.
#[test]
fn chen_identifying_relationship_renders_double_diamond() {
    let src = r#"
@startchen
entity Employee {
  EmpID : number <<key>>
}
weak-entity Dependent {
  DepName : string <<pk-partial>>
}
relationship Has <<identifying>> {
}
Employee 1 -- N Has
Has 1 -- 1 Dependent
@endchen
"#;
    let svg = render_svg(src);
    assert!(!svg.is_empty(), "SVG should be non-empty");
    assert!(
        svg.contains("chen-identifying"),
        "SVG should contain chen-identifying double-diamond for identifying relationship: {svg}"
    );
}

/// Relationship with its own attribute block must render the attribute oval
/// attached to the diamond.
#[test]
fn chen_relationship_with_attribute_renders_attached_oval() {
    let src = r#"
@startchen
entity Employee {
  EmpID : number <<key>>
}
entity Project {
  ProjID : number <<key>>
}
relationship WorksOn {
  HoursWorked : number
}
Employee N -- 1 WorksOn
WorksOn N -- 1 Project
@endchen
"#;
    let svg = render_svg(src);
    assert!(!svg.is_empty(), "SVG should be non-empty");
    assert!(
        svg.contains("HoursWorked"),
        "SVG should contain the relationship attribute label HoursWorked: {svg}"
    );
    assert!(
        svg.contains("chen-attribute"),
        "SVG should contain an attribute oval (chen-attribute) for the relationship attribute: {svg}"
    );
}

/// Full acceptance test from the wave-12 spec: all features together.
#[test]
fn chen_w12_full_spec_diagram_renders() {
    let src = r#"
@startchen
entity Employee {
  *EmpID : number <<pk>>
  Name : string
  (FirstName, LastName)
  [Phone]
  Salary / : number
}

weak-entity Dependent {
  *DepName : string <<pk-partial>>
  BirthDate : date
}

relationship Has {
}
Employee 1 -- N Has
Has 1 -- 1 Dependent

relationship Project {
}
relationship WorksOn {
  HoursWorked : number
}
Employee N -- 1 WorksOn
WorksOn N -- 1 Project
@endchen
"#;
    let svg = render_svg(src);
    assert!(!svg.is_empty(), "Full spec SVG should be non-empty");
    // Weak entity
    assert!(
        svg.contains("chen-weak-entity"),
        "Full spec: chen-weak-entity missing: {svg}"
    );
    // Multivalued
    assert!(
        svg.contains("chen-multivalued"),
        "Full spec: chen-multivalued missing: {svg}"
    );
    // Derived
    assert!(
        svg.contains("stroke-dasharray"),
        "Full spec: derived attribute dashed oval missing: {svg}"
    );
    // Composite children
    assert!(
        svg.contains("FirstName") && svg.contains("LastName"),
        "Full spec: composite children missing: {svg}"
    );
    // Relationship attribute
    assert!(
        svg.contains("HoursWorked"),
        "Full spec: relationship attribute HoursWorked missing: {svg}"
    );
}
