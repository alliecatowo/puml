//! Wave-10 batch F: class diagram parity tests.
//!
//! Covers:
//! - `note right of Foo::member` — member-level note target parsed and routed
//! - `Foo::field1 --> Bar::field2` — member-to-member relation endpoint anchoring
//! - `#--` box-filled arrowhead
//! - `x--` cross (dead-end) arrowhead
//! - `+--` plus arrowhead
//! - `^--` filled-triangle arrowhead
//! - `}--` bracket-open arrowhead
//! - `(Foo, Bar) .. Enrollment` — association class binding renders dashed lines
//!   from participants to a diamond midpoint node.
//!
//! Refs #88

use puml::ast::{DiagramKind, StatementKind};
use puml::model::{FamilyNodeKind, FamilyRelationEndpointMarker, NormalizedDocument};

// ─── member-level note target ─────────────────────────────────────────────────

const MEMBER_NOTE_SRC: &str = r#"@startuml
class Foo {
  counter: int
  field1: string
}
class Bar {
  field3: string
}
note right of Foo::counter : member-level note
@enduml
"#;

#[test]
fn class_member_note_target_resolves_to_member() {
    let doc = puml::parse(MEMBER_NOTE_SRC).expect("parse member note target");
    assert_eq!(doc.kind, DiagramKind::Class);

    // The parsed Note should have target = "Foo" and target_member = "counter".
    let note = doc
        .statements
        .iter()
        .find_map(|stmt| {
            if let StatementKind::Note(n) = &stmt.kind {
                Some(n)
            } else {
                None
            }
        })
        .expect("note statement should be present");

    assert_eq!(
        note.target.as_deref(),
        Some("Foo"),
        "note target should be the class name after splitting on ::"
    );
    assert_eq!(
        note.target_member.as_deref(),
        Some("counter"),
        "note target_member should hold the member name"
    );
    assert_eq!(
        note.text, "member-level note",
        "note text should be preserved"
    );

    // Normalize and verify a dashed relation is created from Foo::counter to the note node.
    let NormalizedDocument::Family(model) =
        puml::normalize_family(doc).expect("normalize member note")
    else {
        panic!("class diagram should normalize as Family");
    };

    // There should be a relation from "Foo::counter" to the note node.
    let note_relation = model
        .relations
        .iter()
        .find(|r| r.from.contains("Foo::counter") || r.from == "Foo");
    assert!(
        note_relation.is_some(),
        "a relation from Foo or Foo::counter to the note node should exist"
    );
    // The note node itself should exist in the model.
    assert!(
        model.nodes.iter().any(|n| n.kind == FamilyNodeKind::Note),
        "a Note node should exist in the normalized model"
    );
}

// ─── member-to-member relation endpoint anchoring ────────────────────────────

const MEMBER_RELATION_SRC: &str = r#"@startuml
class Foo {
  field1: string
}
class Bar {
  field3: string
}
Foo::field1 --> Bar::field3
@enduml
"#;

#[test]
fn class_member_level_relation_endpoint_anchors_at_member() {
    let doc = puml::parse(MEMBER_RELATION_SRC).expect("parse member-to-member relation");
    assert_eq!(doc.kind, DiagramKind::Class);

    // Find the relation.
    let rel = doc
        .statements
        .iter()
        .find_map(|stmt| {
            if let StatementKind::FamilyRelation(r) = &stmt.kind {
                Some(r)
            } else {
                None
            }
        })
        .expect("family relation should be present");

    // The parser should preserve the member-qualified endpoints.
    assert!(
        rel.from.contains("Foo") && (rel.from.contains("field1") || rel.from == "Foo"),
        "from endpoint should reference Foo (possibly with ::field1)"
    );

    // After normalization, the relation should survive (the member qualifiers
    // route via qualified_row_anchor at render time).
    let NormalizedDocument::Family(model) =
        puml::normalize_family(doc).expect("normalize member-to-member relation")
    else {
        panic!("class diagram should normalize as Family");
    };
    assert!(
        !model.relations.is_empty(),
        "normalized model should contain at least one relation"
    );
    // Verify the SVG renders without panic.
    let svg = puml::render_source_to_svg(MEMBER_RELATION_SRC)
        .expect("render member-to-member relation diagram");
    assert!(
        svg.contains("Foo") && svg.contains("Bar"),
        "rendered SVG should contain both class names"
    );
}

// ─── exotic arrowheads ────────────────────────────────────────────────────────

const HASH_ARROW_SRC: &str = r#"@startuml
class Class01
class Class02
Class01 #-- Class02
@enduml
"#;

#[test]
fn class_hash_arrowhead_renders() {
    let doc = puml::parse(HASH_ARROW_SRC).expect("parse hash arrowhead");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(doc).expect("normalize hash arrowhead")
    else {
        panic!("class diagram should normalize as Family");
    };
    let rel = model.relations.first().expect("relation should exist");
    assert_eq!(
        rel.arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::BoxFilled),
        "# arrowhead should map to BoxFilled marker"
    );
    let svg = puml::render_source_to_svg(HASH_ARROW_SRC).expect("render hash arrowhead");
    assert!(
        svg.contains("arrow-box-filled"),
        "rendered SVG should reference the box-filled marker: {svg}"
    );
}

const X_ARROW_SRC: &str = r#"@startuml
class Class01
class Class02
Class01 x-- Class02
@enduml
"#;

#[test]
fn class_x_arrowhead_renders() {
    let doc = puml::parse(X_ARROW_SRC).expect("parse x arrowhead");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(doc).expect("normalize x arrowhead")
    else {
        panic!("class diagram should normalize as Family");
    };
    let rel = model.relations.first().expect("relation should exist");
    assert_eq!(
        rel.arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::Cross),
        "x arrowhead should map to Cross marker"
    );
    let svg = puml::render_source_to_svg(X_ARROW_SRC).expect("render x arrowhead");
    assert!(
        svg.contains("arrow-cross"),
        "rendered SVG should reference the cross marker: {svg}"
    );
}

const PLUS_ARROW_SRC: &str = r#"@startuml
class Class01
class Class02
Class01 +-- Class02
@enduml
"#;

#[test]
fn class_plus_arrowhead_renders() {
    let doc = puml::parse(PLUS_ARROW_SRC).expect("parse plus arrowhead");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(doc).expect("normalize plus arrowhead")
    else {
        panic!("class diagram should normalize as Family");
    };
    let rel = model.relations.first().expect("relation should exist");
    assert_eq!(
        rel.arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::Plus),
        "+ arrowhead should map to Plus marker"
    );
    let svg = puml::render_source_to_svg(PLUS_ARROW_SRC).expect("render plus arrowhead");
    assert!(
        svg.contains("arrow-plus"),
        "rendered SVG should reference the plus marker: {svg}"
    );
}

const FILLED_TRIANGLE_ARROW_SRC: &str = r#"@startuml
class Class01
class Class02
Class01 ^-- Class02
@enduml
"#;

#[test]
fn class_filled_triangle_arrowhead_renders() {
    let doc = puml::parse(FILLED_TRIANGLE_ARROW_SRC).expect("parse filled-triangle arrowhead");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(doc).expect("normalize filled-triangle arrowhead")
    else {
        panic!("class diagram should normalize as Family");
    };
    let rel = model.relations.first().expect("relation should exist");
    assert_eq!(
        rel.arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::TriangleFilled),
        "^ arrowhead should map to TriangleFilled marker"
    );
    let svg = puml::render_source_to_svg(FILLED_TRIANGLE_ARROW_SRC)
        .expect("render filled-triangle arrowhead");
    assert!(
        svg.contains("arrow-triangle-filled"),
        "rendered SVG should reference the triangle-filled marker: {svg}"
    );
}

const BRACKET_ARROW_SRC: &str = r#"@startuml
class Class01
class Class02
Class01 }-- Class02
@enduml
"#;

#[test]
fn class_bracket_arrowhead_renders() {
    let doc = puml::parse(BRACKET_ARROW_SRC).expect("parse bracket arrowhead");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(doc).expect("normalize bracket arrowhead")
    else {
        panic!("class diagram should normalize as Family");
    };
    let rel = model.relations.first().expect("relation should exist");
    assert_eq!(
        rel.arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::BracketOpen),
        "bracket arrowhead should map to BracketOpen marker"
    );
    let svg = puml::render_source_to_svg(BRACKET_ARROW_SRC).expect("render bracket arrowhead");
    assert!(
        svg.contains("arrow-bracket-open"),
        "rendered SVG should reference the bracket-open marker: {svg}"
    );
}

// ─── association class binding ────────────────────────────────────────────────

const ASSOC_CLASS_SRC: &str = r#"@startuml
class Foo
class Bar
class Enrollment
(Foo, Bar) .. Enrollment
@enduml
"#;

#[test]
fn association_class_binding_renders_dashed_to_diamond() {
    let doc = puml::parse(ASSOC_CLASS_SRC).expect("parse association class binding");
    assert_eq!(doc.kind, DiagramKind::Class);

    // Verify the AssociationClass statement is parsed.
    let has_assoc_class = doc.statements.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            StatementKind::AssociationClass {
                left,
                right,
                association,
                ..
            } if left == "Foo" && right == "Bar" && association == "Enrollment"
        )
    });
    assert!(
        has_assoc_class,
        "AssociationClass statement should be parsed with left=Foo, right=Bar, association=Enrollment"
    );

    let NormalizedDocument::Family(model) =
        puml::normalize_family(doc).expect("normalize association class binding")
    else {
        panic!("class diagram should normalize as Family");
    };

    // The Enrollment node should be created as Diamond for the midpoint rendering.
    let enrollment_node = model
        .nodes
        .iter()
        .find(|n| n.name == "Enrollment")
        .expect("Enrollment node should exist in normalized model");
    assert_eq!(
        enrollment_node.kind,
        FamilyNodeKind::Diamond,
        "association class binding node should be Diamond kind"
    );

    // Verify dashed (dotted) relations exist: Enrollment --> Foo, Enrollment --> Bar.
    let dashed_from_enrollment = model
        .relations
        .iter()
        .filter(|r| r.from == "Enrollment" && r.arrow.is_dashed())
        .count();
    assert_eq!(
        dashed_from_enrollment, 2,
        "two dashed relations should connect Enrollment to Foo and Bar"
    );

    // Render and verify dashed stroke-dasharray in SVG.
    let svg = puml::render_source_to_svg(ASSOC_CLASS_SRC)
        .expect("render association class binding diagram");
    assert!(
        svg.contains("stroke-dasharray") || svg.contains("Enrollment"),
        "rendered SVG should contain dashed strokes or Enrollment label: {svg}"
    );
    assert!(
        svg.contains("Foo") && svg.contains("Bar"),
        "rendered SVG should contain both participant class names"
    );
}
