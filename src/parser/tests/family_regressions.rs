use super::*;

#[test]
fn parses_case_insensitive_parenthesized_c4_declarations() {
    let doc = parse_with_options(
        "Person_Ext(user, \"User\")\nSYSTEM_EXT(system, \"External System\")\nContainerDb(db, \"Database\")\nBoundary(gate, \"Security Zone\")\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.statements.len(), 4);
    assert_eq!(doc.kind, DiagramKind::Component);

    match &doc.statements[0].kind {
        StatementKind::ObjectDecl(decl) => {
            assert!(decl.alias.as_ref().expect("alias").contains("<<external-person>>"));
            assert_eq!(decl.name, "User");
            assert!(decl.alias.as_ref().expect("alias").contains("user"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::ObjectDecl(decl) => {
            assert!(decl.alias.as_ref().expect("alias").contains("<<external-system>>"));
            assert_eq!(decl.name, "External System");
            assert!(decl.alias.as_ref().expect("alias").contains("system"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[2].kind {
        StatementKind::ObjectDecl(decl) => {
            assert!(decl.alias.as_ref().expect("alias").contains("<<container-db>>"));
            assert_eq!(decl.name, "Database");
            assert!(decl.alias.as_ref().expect("alias").contains("db"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[3].kind {
        StatementKind::ObjectDecl(decl) => {
            assert!(decl.alias.as_ref().expect("alias").contains("<<boundary>>"));
            assert_eq!(decl.name, "Security Zone");
            assert!(decl.alias.as_ref().expect("alias").contains("gate"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_parenthesized_c4_macros_when_object_family_is_already_detected() {
    let doc = parse_with_options(
        "object User\nobject Service\nSystemDb(db, \"Database\")\nContainerQueue(queue, \"Queue\")\nRel_Back(Service, db, \"reads from\")\nBiRel(Service, User, \"syncs\")\nRel_Dynamic(Service, User, \"calls\", \"HTTP\")\n",
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(doc.kind, DiagramKind::Object);
    assert_eq!(doc.statements.len(), 8);
    assert!(matches!(doc.statements[0].kind, StatementKind::ObjectDecl(_)));
    assert!(matches!(doc.statements[1].kind, StatementKind::ObjectDecl(_)));
    assert!(matches!(doc.statements[2].kind, StatementKind::ObjectDecl(_)));
    assert!(matches!(doc.statements[3].kind, StatementKind::ObjectDecl(_)));
    assert!(matches!(doc.statements[4].kind, StatementKind::FamilyRelation(_)));
    assert!(matches!(doc.statements[5].kind, StatementKind::FamilyRelation(_)));
    assert!(matches!(doc.statements[6].kind, StatementKind::FamilyRelation(_)));

    let back = &doc.statements[4];
    match &back.kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "db");
            assert_eq!(rel.to, "Service");
            assert_eq!(rel.label.as_deref(), Some("reads from"));
            assert_eq!(rel.arrow, "->");
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    let birel = &doc.statements[5];
    match &birel.kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Service");
            assert_eq!(rel.to, "User");
            assert_eq!(rel.arrow, "->");
            assert_eq!(rel.label.as_deref(), Some("[C4 BiRel] syncs"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    let birel_reverse = &doc.statements[6];
    match &birel_reverse.kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "User");
            assert_eq!(rel.to, "Service");
            assert_eq!(rel.label.as_deref(), Some("[C4 BiRel] syncs"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    let dynamic = &doc.statements[7];
    match &dynamic.kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Service");
            assert_eq!(rel.to, "User");
            assert_eq!(rel.label.as_deref(), Some("[C4 Rel_Dynamic()] calls"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_c4_relations_before_object_declarations() {
    let doc = parse_with_options(
        "Rel_Dynamic(Service, User, \"calls\", \"HTTP\")\nSystemDb(db, \"Database\")\nRel_Back(Service, db, \"reads from\")\n",
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(doc.kind, DiagramKind::Component);
    assert_eq!(doc.statements.len(), 3);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::FamilyRelation(_)
    ));
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::ObjectDecl(_)
    ));
    assert!(matches!(
        doc.statements[2].kind,
        StatementKind::FamilyRelation(_)
    ));

    match &doc.statements[0].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Service");
            assert_eq!(rel.to, "User");
            assert_eq!(rel.label.as_deref(), Some("[C4 Rel_Dynamic()] calls"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    match &doc.statements[1].kind {
        StatementKind::ObjectDecl(decl) => {
            assert_eq!(decl.name, "Database");
            assert!(decl.alias.as_ref().expect("alias").contains("<<system-db>>"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    match &doc.statements[2].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "db");
            assert_eq!(rel.to, "Service");
            assert_eq!(rel.label.as_deref(), Some("reads from"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_c4_relations_after_unsupported_line_without_losing_component_family() {
    let doc = parse_with_options(
        "unsupported thing\nRel_Dynamic(Service, User, \"calls\", \"HTTP\")\nSystemDb(db, \"Database\")\nRel_Back(Service, db, \"reads\")\n",
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(doc.kind, DiagramKind::Component);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::UnsupportedSyntax(_)
    ));
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::FamilyRelation(_)
    ));
    assert!(matches!(
        doc.statements[2].kind,
        StatementKind::ObjectDecl(_)
    ));
    assert!(matches!(
        doc.statements[3].kind,
        StatementKind::FamilyRelation(_)
    ));

    match &doc.statements[1].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Service");
            assert_eq!(rel.to, "User");
            assert_eq!(rel.label.as_deref(), Some("[C4 Rel_Dynamic()] calls"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    match &doc.statements[2].kind {
        StatementKind::ObjectDecl(decl) => {
            assert_eq!(decl.alias.as_ref().expect("alias"), "db <<system-db>>");
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    match &doc.statements[3].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "db");
            assert_eq!(rel.to, "Service");
            assert_eq!(rel.label.as_deref(), Some("reads"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_leading_bang_parenthesized_c4_declarations_and_relations() {
    let doc = parse_preprocessed("!Person(person, \"Person\")\n!Container(container, \"Container\")\n!Rel_Back(Person, container, \"depends on\")\n").unwrap();

    assert_eq!(doc.statements.len(), 3);
    assert_eq!(doc.kind, DiagramKind::Component);

    match &doc.statements[0].kind {
        StatementKind::ObjectDecl(decl) => {
            assert!(decl.alias.as_ref().expect("alias").contains("<<person>>"));
            assert_eq!(decl.name, "Person");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::ObjectDecl(decl) => {
            assert!(decl.alias.as_ref().expect("alias").contains("<<container>>"));
            assert_eq!(decl.name, "Container");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[2].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "container");
            assert_eq!(rel.to, "Person");
            assert_eq!(rel.label.as_deref(), Some("depends on"));
            assert_eq!(rel.stereotype.as_deref(), Some("c4-rel-back"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_leading_bang_c4_relation_before_c4_declarations() {
    let doc = parse_preprocessed(
        "!Rel_Dynamic(Service, User, \"calls\", \"HTTP\")\n!SystemDb(db, \"Database\")\n!Rel_Back(Service, db, \"reads from\")\n",
    )
    .unwrap();

    assert_eq!(doc.statements.len(), 3);
    assert_eq!(doc.kind, DiagramKind::Component);

    match &doc.statements[0].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Service");
            assert_eq!(rel.to, "User");
            assert_eq!(rel.label.as_deref(), Some("[C4 Rel_Dynamic()] calls"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::ObjectDecl(decl) => {
            assert!(decl.alias.as_ref().expect("alias").contains("<<system-db>>"));
            assert_eq!(decl.name, "Database");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[2].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "db");
            assert_eq!(rel.to, "Service");
            assert_eq!(rel.label.as_deref(), Some("reads from"));
            assert_eq!(rel.arrow, "->");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_dynamic_relay_macro_with_index() {
    let doc = parse_with_options(
        "object Service\nobject User\nRel_Dynamic(Service, User, \"calls\", \"HTTP\", \"#00aa00\", 7)",
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(doc.statements.len(), 3);
    match &doc.statements[2].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Service");
            assert_eq!(rel.to, "User");
            assert_eq!(rel.label.as_deref(), Some("[C4 Rel_Dynamic(7)] calls"));
            assert_eq!(rel.line_color.as_deref(), Some("#00aa00"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}
