mod svg_test_helpers;

use puml::render_source_to_svg;
use svg_test_helpers::{bounds, Bounds, SvgDoc};

fn assert_clear(a: Bounds, b: Bounds, label: &str) {
    assert!(
        !bounds_overlap(a, b, 6.0),
        "{label} should not overlap: {a:?} vs {b:?}"
    );
}

fn bounds_overlap(a: Bounds, b: Bounds, clearance: f64) -> bool {
    a.x < b.right() + clearance
        && a.right() + clearance > b.x
        && a.y < b.bottom() + clearance
        && a.bottom() + clearance > b.y
}

#[test]
fn chen_employee_schema_keeps_dense_center_clear() {
    let svg = render_source_to_svg(include_str!("../docs/examples/chen/02_keys.puml")).unwrap();
    let doc = SvgDoc::parse(&svg);

    let works_in = bounds(doc.first_with_attr("polygon", "data-chen-relationship", "WorksIn"));
    let has_dependent =
        bounds(doc.first_with_attr("polygon", "data-chen-relationship", "HasDependent"));
    let age =
        bounds(doc.first_with_attr("ellipse", "data-chen-attribute-id", "entity:Employee:Age"));
    let dept_name = bounds(doc.first_with_attr(
        "ellipse",
        "data-chen-attribute-id",
        "entity:Department:DeptName",
    ));
    let dependent_relationship = bounds(doc.first_with_attr(
        "ellipse",
        "data-chen-attribute-id",
        "entity:Dependent:Relationship",
    ));

    assert_clear(age, works_in, "derived Age attribute");
    assert_clear(dept_name, works_in, "Department.DeptName attribute");
    assert_clear(
        dependent_relationship,
        has_dependent,
        "Dependent.Relationship attribute",
    );
    assert!(
        has_dependent.width >= 132.0,
        "long identifying relationship labels should expand the diamond"
    );
}

#[test]
fn chen_library_schema_places_relationship_attributes_in_clear_lanes() {
    let svg = render_source_to_svg(include_str!(
        "../docs/examples/chen/03_multi_relationship.puml"
    ))
    .unwrap();
    let doc = SvgDoc::parse(&svg);

    let borrows = bounds(doc.first_with_attr("polygon", "data-chen-relationship", "Borrows"));
    let manages = bounds(doc.first_with_attr("polygon", "data-chen-relationship", "Manages"));
    let age_rating =
        bounds(doc.first_with_attr("ellipse", "data-chen-attribute-id", "entity:Book:AgeRating"));
    let borrow_date = bounds(doc.first_with_attr(
        "ellipse",
        "data-chen-attribute-id",
        "relationship:Borrows:BorrowDate",
    ));
    let due_date = bounds(doc.first_with_attr(
        "ellipse",
        "data-chen-attribute-id",
        "relationship:Borrows:DueDate",
    ));
    let librarian_name =
        bounds(doc.first_with_attr("ellipse", "data-chen-attribute-id", "entity:Librarian:Name"));

    assert_clear(age_rating, borrows, "Book.AgeRating attribute");
    assert_clear(borrow_date, borrows, "Borrows.BorrowDate attribute");
    assert_clear(due_date, borrows, "Borrows.DueDate attribute");
    assert_clear(borrow_date, due_date, "Borrows relationship attributes");
    assert_clear(librarian_name, manages, "Librarian.Name attribute");
    assert!(
        borrow_date.y > borrows.bottom() && due_date.y > borrows.bottom(),
        "relationship attributes should prefer the lane below their diamond"
    );
}

#[test]
fn chen_layout_is_deterministic_and_emits_geometry_hooks() {
    let src = r#"
@startchen
title Long Relationship Labels

entity Employee {
  key EmployeeID
  Name
}

entity Dependent weak {
  key DependentName
}

relationship SupervisesDependentEnrollment identifying {
  Employee -> Dependent [1:N]
  VerificationTimestamp
}
@endchen
"#;

    let first = render_source_to_svg(src).expect("chen diagram should render");
    let second = render_source_to_svg(src).expect("chen diagram should render deterministically");
    assert_eq!(first, second);

    let doc = SvgDoc::parse(&first);
    let hook_nodes = doc.hook_nodes();
    let hook_edges = doc.hook_edges();
    assert_eq!(
        hook_nodes.len(),
        7,
        "Chen fixture should expose entity, relationship, and attribute puml-node hooks"
    );
    assert_eq!(
        hook_edges.len(),
        6,
        "Chen fixture should expose relationship and attribute puml-edge hooks"
    );
    assert!(
        hook_nodes.iter().all(|node| !node.id.is_empty()),
        "puml-node hooks should have stable ids"
    );
    assert!(
        hook_edges
            .iter()
            .all(|edge| !edge.from.is_empty() && !edge.to.is_empty() && !edge.segments.is_empty()),
        "puml-edge hooks should have endpoint ids and geometry"
    );

    let rel = bounds(doc.first_with_attr(
        "polygon",
        "data-chen-relationship",
        "SupervisesDependentEnrollment",
    ));
    let rel_attr = bounds(doc.first_with_attr(
        "ellipse",
        "data-chen-attribute-id",
        "relationship:SupervisesDependentEnrollment:VerificationTimestamp",
    ));

    assert!(
        rel.width >= 220.0,
        "long relationship labels should drive content-aware diamond width"
    );
    assert_clear(rel_attr, rel, "long relationship attribute");
    assert!(
        !doc.elements_with_class("text", "chen-cardinality")
            .is_empty(),
        "cardinality labels should have stable SVG hooks"
    );
}
