//! Regression tests for #1349 + #1350 (Wave-15 parity bundle).
//!
//! #1349 — Class member visibility: UML 2.x geometric glyphs replace ASCII +/-/#/~
//!   - `+` (public)    → hollow circle (field) / filled circle (method)
//!   - `-` (private)   → filled diamond
//!   - `#` (protected) → hollow diamond
//!   - `~` (package)   → filled triangle
//!
//! #1350 — Class-type header badge: small coloured ○ with letter C/A/I/E/O
//!   emitted at the header-left of every class and object node.

fn svg_for(src: &str) -> String {
    puml::render_source_to_svg(src).expect("svg should render")
}

// ── #1349: visibility glyph shapes ───────────────────────────────────────────

const VISIBILITY_SRC: &str = r#"@startuml
class BankAccount {
  +accountNumber: String
  #balance: Decimal
  -pin: String
  ~branch: String
  +deposit(amount: Decimal)
  -validatePin(pin: String): Boolean
}
@enduml"#;

/// Public field → hollow green circle (fill="none")
#[test]
fn public_field_renders_hollow_circle() {
    let svg = svg_for(VISIBILITY_SRC);
    // The glyph must be a <circle> with fill="none" and the public-green stroke.
    assert!(
        svg.contains("class=\"uml-vis-glyph\""),
        "SVG must contain uml-vis-glyph elements"
    );
    // hollow circle: fill="none"
    assert!(
        svg.contains("r=\"4\" fill=\"none\" stroke=\"#16a34a\""),
        "public field must render as hollow green circle; SVG:\n{svg}"
    );
}

/// Public method (+deposit) → filled green circle
#[test]
fn public_method_renders_filled_circle() {
    let svg = svg_for(VISIBILITY_SRC);
    // filled circle: fill equals the green color
    assert!(
        svg.contains("r=\"4\" fill=\"#16a34a\" stroke=\"#16a34a\""),
        "public method must render as filled green circle; SVG:\n{svg}"
    );
}

/// Private field (-pin) → filled red diamond polygon
#[test]
fn private_member_renders_filled_diamond() {
    let svg = svg_for(VISIBILITY_SRC);
    // private = red (#dc2626) filled polygon
    assert!(
        svg.contains("fill=\"#dc2626\" stroke=\"#dc2626\""),
        "private member must render as filled red diamond; SVG:\n{svg}"
    );
}

/// Protected field (#balance) → hollow orange diamond polygon
#[test]
fn protected_member_renders_hollow_diamond() {
    let svg = svg_for(VISIBILITY_SRC);
    // protected = orange (#d97706) hollow polygon (fill="none")
    assert!(
        svg.contains("fill=\"none\" stroke=\"#d97706\""),
        "protected member must render as hollow orange diamond; SVG:\n{svg}"
    );
}

/// Package field (~branch) → purple triangle polygon
#[test]
fn package_member_renders_triangle() {
    let svg = svg_for(VISIBILITY_SRC);
    // package = purple (#7c3aed) filled polygon
    assert!(
        svg.contains("fill=\"#7c3aed\" stroke=\"#7c3aed\""),
        "package member must render as filled purple triangle; SVG:\n{svg}"
    );
}

/// ASCII `+/-/#/~` prefixes must NOT appear as bare text in member rows
/// when attribute_icons is on (default).
#[test]
fn ascii_prefixes_not_in_member_text() {
    let svg = svg_for(VISIBILITY_SRC);
    // The member <text> elements should not start with the literal prefix characters.
    // We check that none of the data-uml-visibility member texts keep their prefix.
    assert!(
        !svg.contains(">+accountNumber"),
        "'+' prefix must be stripped from member text when icons are enabled"
    );
    assert!(
        !svg.contains(">#balance"),
        "'#' prefix must be stripped from member text when icons are enabled"
    );
    assert!(
        !svg.contains(">-pin"),
        "'-' prefix must be stripped from member text when icons are enabled"
    );
    assert!(
        !svg.contains(">~branch"),
        "'~' prefix must be stripped from member text when icons are enabled"
    );
}

/// Members with no visibility prefix must not get a glyph.
#[test]
fn no_prefix_member_has_no_glyph() {
    let src = "@startuml\nclass Foo {\n  bar: int\n}\n@enduml\n";
    let svg = svg_for(src);
    // No glyph elements should appear for a member without a visibility marker.
    assert!(
        !svg.contains("uml-vis-glyph"),
        "member without visibility prefix must not render a glyph; SVG:\n{svg}"
    );
}

// ── #1350: class-type header badge ───────────────────────────────────────────

/// Plain class → green C badge circle in header.
#[test]
fn plain_class_has_c_badge() {
    let src = "@startuml\nclass Foo {}\n@enduml\n";
    let svg = svg_for(src);
    assert!(
        svg.contains("class=\"uml-class-badge\""),
        "plain class must have a uml-class-badge circle; SVG:\n{svg}"
    );
    // Badge letter 'C' must be emitted as a <text> inside the header.
    assert!(
        svg.contains(">C<"),
        "plain class badge must contain letter 'C'; SVG:\n{svg}"
    );
    // Badge fill is the green palette.
    assert!(
        svg.contains("#A2D5A2"),
        "plain class badge fill must be #A2D5A2 (PlantUML green); SVG:\n{svg}"
    );
}

/// Abstract class → green A badge.
#[test]
fn abstract_class_has_a_badge() {
    let src = "@startuml\nabstract class Shape {}\n@enduml\n";
    let svg = svg_for(src);
    assert!(
        svg.contains(">A<"),
        "abstract class badge must contain letter 'A'; SVG:\n{svg}"
    );
}

/// Class-box marked with <<interface>> stereotype → blue I badge.
/// Note: the bare `interface` keyword renders as a lollipop circle node,
/// not a class box; the badge applies to class-box nodes with the <<interface>>
/// builtin-type marker.
#[test]
fn interface_stereotype_class_has_i_badge() {
    let src = "@startuml\nclass Drawable <<interface>> {}\n@enduml\n";
    let svg = svg_for(src);
    assert!(
        svg.contains(">I<"),
        "<<interface>> class badge must contain letter 'I'; SVG:\n{svg}"
    );
    assert!(
        svg.contains("#90CAF9"),
        "<<interface>> class badge fill must be #90CAF9 (blue); SVG:\n{svg}"
    );
}

/// Object node → amber O badge.
#[test]
fn object_has_o_badge() {
    let src = "@startuml\nobject Order {\n  id = 1\n}\n@enduml\n";
    let svg = svg_for(src);
    assert!(
        svg.contains(">O<"),
        "object badge must contain letter 'O'; SVG:\n{svg}"
    );
}

/// Enum node → yellow E badge.
#[test]
fn enum_has_e_badge() {
    let src = "@startuml\nenum Color {\n  RED\n  GREEN\n  BLUE\n}\n@enduml\n";
    let svg = svg_for(src);
    assert!(
        svg.contains(">E<"),
        "enum badge must contain letter 'E'; SVG:\n{svg}"
    );
}
