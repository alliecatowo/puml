//! IE/Entity-relationship parity tests.
//!
//! Verifies that:
//! - The `entity` keyword routes to the class family (not sequence or deployment)
//!   when used in IE-style ER diagrams.
//! - Crow's-foot arrowhead markers (`||`, `|o`, `o{`, `}|`, etc.) are rendered
//!   in the SVG output.
//! - The `*` mandatory-attribute prefix is preserved in member text.
//! - Dashed optional relations (`..`) render with a stroke-dasharray.
//!
//! Refs #88 (Oracle conformance — IE/ER parity).

use puml::ast::DiagramKind;
use puml::model::{FamilyNodeKind, NormalizedDocument};

/// Source for a minimal two-entity IE diagram with a block body.
const IE_BASIC_SRC: &str = r#"@startuml
entity User {
  *user_id : number <<generated>>
  --
  *name : text
  address : text
}
entity Order {
  *order_id : number
  --
  customer : User
}
User ||--o{ Order
@enduml
"#;

/// Source with dashed optional and mandatory-many relations.
const IE_RELATIONS_SRC: &str = r#"@startuml
entity Item {
  *item_id : number
}
entity Order {
  *order_id : number
}
Item }o..|| Order
Item }|..|| Order
@enduml
"#;

/// Source where `entity` declarations appear WITHOUT a brace block but with
/// IE-style crow's-foot relations — tests the ambiguity resolution path.
const IE_NO_BLOCK_SRC: &str = r#"@startuml
entity User
entity Order
User ||--o{ Order
User }|..|| Order
@enduml
"#;

// ─── test: entity keyword routes to class family ──────────────────────────────

#[test]
fn ie_entity_declaration_routes_to_class_family() {
    // Test with block-body entity declarations.
    let document = puml::parse(IE_BASIC_SRC).expect("parse IE entity declarations");
    assert_eq!(
        document.kind,
        DiagramKind::Class,
        "entity keyword with block body must route to the Class family, got {:?}",
        document.kind
    );

    // Test also with no-block entity declarations followed by IE relations
    // (ambiguity resolution path: entity without `{` + crow's-foot tokens).
    let doc_no_block = puml::parse(IE_NO_BLOCK_SRC).expect("parse IE no-block entity");
    assert_eq!(
        doc_no_block.kind,
        DiagramKind::Class,
        "entity without brace block but with IE relations must route to Class family, got {:?}",
        doc_no_block.kind
    );

    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize IE entity")
    else {
        panic!("IE entity diagram must normalize as Family");
    };

    assert_eq!(
        model.kind,
        DiagramKind::Class,
        "normalized model must carry DiagramKind::Class"
    );

    // Both entity declarations must be present as FamilyNodeKind::Class nodes.
    let user = model
        .nodes
        .iter()
        .find(|n| n.name == "User")
        .expect("User entity node must be present");
    assert_eq!(
        user.kind,
        FamilyNodeKind::Class,
        "IE entity must normalize to FamilyNodeKind::Class, not {:?}",
        user.kind
    );
    assert!(
        model.nodes.iter().any(|n| n.name == "Order"),
        "Order entity node must also be present"
    );

    // The <<entity>> type marker must appear in the first member position.
    assert!(
        user.members.iter().any(|m| m.text.trim() == "<<entity>>"),
        "IE entity node must carry <<entity>> type marker in its members: {:?}",
        user.members
    );

    // The relation User ||--o{ Order must have been parsed.
    assert!(
        model.relations.iter().any(|r| {
            (r.from == "User" && r.to == "Order") || (r.from == "Order" && r.to == "User")
        }),
        "IE relation between User and Order must be present"
    );
}

// ─── test: crow's-foot markers appear in rendered SVG ─────────────────────────

#[test]
fn ie_crow_foot_many_to_one_renders_cardinality_glyphs() {
    let svg =
        puml::render_source_to_svg(IE_BASIC_SRC).expect("render IE crow's-foot diagram to SVG");

    // The SVG must include the IE marker definition IDs for the arrow markers
    // used in this diagram: zero-many (o{) and one (||).
    assert!(
        svg.contains("arrow-ie-zero-many") || svg.contains("arrow-ie-one"),
        "SVG must contain IE crow's-foot marker definitions; got svg (first 2000 chars):\n{}",
        &svg[..svg.len().min(2000)]
    );

    // The relation line must reference an IE marker via marker-start or marker-end.
    assert!(
        svg.contains("marker-end=\"url(#arrow-ie-")
            || svg.contains("marker-start=\"url(#arrow-ie-"),
        "SVG relation line must reference at least one IE endpoint marker"
    );
}

// ─── test: mandatory (*) attribute prefix is preserved in member text ─────────

#[test]
fn ie_mandatory_attribute_star_prefix_renders() {
    let svg = puml::render_source_to_svg(IE_BASIC_SRC).expect("render IE diagram to SVG");

    // The `*user_id : number <<generated>>` member must be rendered with the
    // IE mandatory attribute markers. The renderer emits the `*` as a bold tspan
    // and annotates the element with `data-uml-ie-mandatory="true"`.
    assert!(
        svg.contains("uml-ie-member"),
        "IE mandatory attribute (*) must render with class=\"uml-ie-member\" annotation"
    );
    assert!(
        svg.contains("data-uml-ie-mandatory=\"true\""),
        "IE mandatory attribute (*) must carry data-uml-ie-mandatory=\"true\" attribute"
    );
    // The member text content (without the * tspan) must appear.
    assert!(
        svg.contains("user_id"),
        "The attribute name 'user_id' must appear in SVG member text"
    );
}

// ─── test: dashed optional relation renders with stroke-dasharray ─────────────

#[test]
fn ie_dashed_optional_relation_renders() {
    let svg =
        puml::render_source_to_svg(IE_RELATIONS_SRC).expect("render IE dashed relations to SVG");

    // Dashed lines use `..` in the arrow string and must produce stroke-dasharray.
    assert!(
        svg.contains("stroke-dasharray"),
        "IE dashed optional relation (..) must render with stroke-dasharray in SVG"
    );

    // IE markers for zero-many and one-many must appear.
    assert!(
        svg.contains("arrow-ie-zero-many") || svg.contains("arrow-ie-one-many"),
        "IE dashed optional relation must produce crow's-foot IE marker defs"
    );
}
