//! Layout invariance test for `--style puml|plantuml` (issue #1375).
//!
//! The acceptance gate for this feature: when switching from PUML-mode (default)
//! to PlantUML-mode, ALL layout coordinates must be byte-identical. Only paint
//! attributes (fill, stroke colour, badge presence) may differ.
//!
//! Strategy:
//! 1. Render each fixture in both modes → two SVG strings.
//! 2. Extract every `<rect ...>` and `<line ...>` element's geometry attrs.
//! 3. Assert the geometry sets are identical across modes.
//! 4. Assert that SOME fill/stroke attribute changed (smoke-check that the mode
//!    actually does something visual).

use puml::model::FamilyStyle;
use puml::theme::{ClassStyle, StyleMode};
use puml::{normalize_family, render_family_document_artifact, NormalizedDocument};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Render the source in the requested style mode and return the SVG string.
fn render_with_mode(source: &str, mode: StyleMode) -> String {
    let document = puml::parser::parse(source).expect("parse should succeed");
    let mut normalized = normalize_family(document).expect("normalize should succeed");
    // Apply style mode to the Family document, mirroring cli_run/render.rs.
    if let NormalizedDocument::Family(ref mut doc) = normalized {
        if let Some(FamilyStyle::Class(ref mut cs)) = doc.family_style {
            cs.style_mode = mode;
        } else if doc.family_style.is_none() {
            let mut cs = ClassStyle::default();
            cs.style_mode = mode;
            doc.family_style = Some(FamilyStyle::Class(cs));
        }
    }
    let NormalizedDocument::Family(family_doc) = normalized else {
        panic!("expected a Family document");
    };
    render_family_document_artifact(&family_doc).svg
}

/// Extract `x=`, `y=`, `width=`, `height=` attribute values from every `<rect`
/// element, and `x1=`, `y1=`, `x2=`, `y2=` from every `<line` element.
/// Returns a sorted list of geometry tokens for stable comparison.
///
/// Deliberately excludes `<text>` element coordinates: text anchor positions
/// are paint-adjacent (they move when labels change or ASCII prefixes are added),
/// not structural layout. The invariant only concerns box and edge geometry.
fn extract_geometry(svg: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();

    // Process rect elements (node boxes, header bands, background)
    for chunk in svg.split("<rect") {
        let attrs: Vec<&str> = chunk
            .split(|c: char| c == ' ' || c == '/' || c == '>')
            .take_while(|s| !s.contains('<'))
            .collect();
        for attr in &attrs {
            for prefix in &["x=", "y=", "width=", "height="] {
                if attr.starts_with(prefix) {
                    tokens.push(format!("rect:{attr}"));
                }
            }
        }
    }

    // Process line elements (dividers, edges)
    for chunk in svg.split("<line") {
        let attrs: Vec<&str> = chunk
            .split(|c: char| c == ' ' || c == '/' || c == '>')
            .take_while(|s| !s.contains('<'))
            .collect();
        for attr in &attrs {
            for prefix in &["x1=", "y1=", "x2=", "y2="] {
                if attr.starts_with(prefix) {
                    tokens.push(format!("line:{attr}"));
                }
            }
        }
    }

    tokens.sort();
    tokens
}

// ── fixtures ─────────────────────────────────────────────────────────────────

const OBJECT_SRC: &str = r#"@startuml
object Order {
  id = 1001
  status = pending
  total = 99.99
}
object Customer {
  id = 42
  name = Alice Smith
  email = alice@example.com
}
Order --> Customer : placedBy
@enduml"#;

const CLASS_SRC: &str = r#"@startuml
class BankAccount {
  +accountNumber: String
  #balance: Decimal
  -pin: String
  ~branch: String
  +deposit(amount: Decimal)
  +withdraw(amount: Decimal)
  #checkBalance(): Decimal
  -validatePin(pin: String): Boolean
}
@enduml"#;

const CLASS_WITH_INTERFACE_SRC: &str = r#"@startuml
interface Drawable {
  +draw(): void
  +getArea(): Double
}
class Circle implements Drawable {
  -radius: Double
  +draw(): void
  +getArea(): Double
}
@enduml"#;

// ── tests ─────────────────────────────────────────────────────────────────────

/// Object diagram: layout coords must be byte-identical across modes.
#[test]
fn object_diagram_layout_invariant_across_style_modes() {
    let puml_svg = render_with_mode(OBJECT_SRC, StyleMode::Puml);
    let plantuml_svg = render_with_mode(OBJECT_SRC, StyleMode::Plantuml);

    let puml_geom = extract_geometry(&puml_svg);
    let plantuml_geom = extract_geometry(&plantuml_svg);

    assert_eq!(
        puml_geom, plantuml_geom,
        "object diagram layout coordinates differ between puml and plantuml modes — \
         only paint should differ, not positions"
    );
}

/// Object diagram: PlantUML mode must use gray header, not yellow.
#[test]
fn object_diagram_plantuml_mode_drops_yellow_header() {
    let puml_svg = render_with_mode(OBJECT_SRC, StyleMode::Puml);
    let plantuml_svg = render_with_mode(OBJECT_SRC, StyleMode::Plantuml);

    // PUML mode: yellow/amber header
    assert!(
        puml_svg.contains("#fef3c7"),
        "PUML mode should use yellow amber header fill (#fef3c7)"
    );
    // PlantUML mode: neutral gray, no yellow
    assert!(
        !plantuml_svg.contains("#fef3c7"),
        "PlantUML mode should not contain yellow amber fill (#fef3c7)"
    );
    assert!(
        plantuml_svg.contains("#e2e8f0"),
        "PlantUML mode should use neutral gray header fill (#e2e8f0)"
    );
}

/// Object diagram: PlantUML mode must suppress the O type badge.
#[test]
fn object_diagram_plantuml_mode_drops_o_badge() {
    let puml_svg = render_with_mode(OBJECT_SRC, StyleMode::Puml);
    let plantuml_svg = render_with_mode(OBJECT_SRC, StyleMode::Plantuml);

    assert!(
        puml_svg.contains("uml-class-badge"),
        "PUML mode should emit class/object type badge"
    );
    assert!(
        !plantuml_svg.contains("uml-class-badge"),
        "PlantUML mode should not emit class/object type badge"
    );
}

/// Class diagram: layout coords must be byte-identical across modes.
#[test]
fn class_diagram_layout_invariant_across_style_modes() {
    let puml_svg = render_with_mode(CLASS_SRC, StyleMode::Puml);
    let plantuml_svg = render_with_mode(CLASS_SRC, StyleMode::Plantuml);

    let puml_geom = extract_geometry(&puml_svg);
    let plantuml_geom = extract_geometry(&plantuml_svg);

    assert_eq!(
        puml_geom, plantuml_geom,
        "class diagram layout coordinates differ between puml and plantuml modes — \
         only paint should differ, not positions"
    );
}

/// Class diagram: PlantUML mode must suppress the C badge and UML 2.x glyphs.
#[test]
fn class_diagram_plantuml_mode_drops_badge_and_glyphs() {
    let puml_svg = render_with_mode(CLASS_SRC, StyleMode::Puml);
    let plantuml_svg = render_with_mode(CLASS_SRC, StyleMode::Plantuml);

    // PUML mode has the C badge
    assert!(
        puml_svg.contains("uml-class-badge"),
        "PUML mode should emit the C badge"
    );
    // PlantUML mode drops it
    assert!(
        !plantuml_svg.contains("uml-class-badge"),
        "PlantUML mode should not emit the C badge"
    );
    // PUML mode has UML 2.x glyph SVG shapes
    assert!(
        puml_svg.contains("uml-vis-glyph"),
        "PUML mode should emit UML 2.x visibility glyphs"
    );
    // PlantUML mode drops them and keeps ASCII visibility prefixes in the text
    assert!(
        !plantuml_svg.contains("uml-vis-glyph"),
        "PlantUML mode should not emit UML 2.x visibility glyphs"
    );
}

/// Class diagram: PlantUML mode must include ASCII visibility prefixes in member text.
#[test]
fn class_diagram_plantuml_mode_uses_ascii_visibility() {
    let plantuml_svg = render_with_mode(CLASS_SRC, StyleMode::Plantuml);
    // The ASCII visibility prefix (+, #, -, ~) should appear in member text.
    // We check for the '+' prefix used by accountNumber (public method).
    assert!(
        plantuml_svg.contains("+accountNumber"),
        "PlantUML mode should use ASCII visibility prefix in member text"
    );
    // Private member should show dash prefix.
    assert!(
        plantuml_svg.contains("-pin"),
        "PlantUML mode should use ASCII '-' prefix for private members"
    );
}

/// Multi-class diagram: layout invariant across modes.
#[test]
fn multi_class_layout_invariant_across_style_modes() {
    let puml_svg = render_with_mode(CLASS_WITH_INTERFACE_SRC, StyleMode::Puml);
    let plantuml_svg = render_with_mode(CLASS_WITH_INTERFACE_SRC, StyleMode::Plantuml);

    let puml_geom = extract_geometry(&puml_svg);
    let plantuml_geom = extract_geometry(&plantuml_svg);

    assert_eq!(
        puml_geom, plantuml_geom,
        "multi-class diagram layout coordinates differ between modes — \
         only paint should differ, not positions"
    );
}
