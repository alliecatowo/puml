#[test]
fn parses_class_bootstrap_declarations_and_relations() {
    let doc = parse_with_options(
        "class User\nclass Account as Acct\nUser --> Acct : owns\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::ClassDecl(_)
    ));
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::ClassDecl(_)
    ));
    match &doc.statements[2].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "User");
            assert_eq!(rel.to, "Acct");
            assert_eq!(rel.arrow, "-->");
            assert_eq!(rel.label.as_deref(), Some("owns"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_object_and_usecase_bootstrap_kinds() {
    let object_doc =
        parse_with_options("object Order\nobject Customer\n", &ParseOptions::default()).unwrap();
    assert_eq!(object_doc.kind, DiagramKind::Object);

    let usecase_doc = parse_with_options(
        "usecase Authenticate\nusecase Authorize\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(usecase_doc.kind, DiagramKind::UseCase);
}

#[test]
fn parses_core_uml_broad_partial_declaration_forms() {
    let class_doc = parse_with_options(
            "interface Gateway\nabstract class Shape\nannotation Trace\nstruct Payload\nGateway -[#blue,dashed]-> Shape : adapts\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(class_doc.kind, DiagramKind::Class);
    match &class_doc.statements[0].kind {
        StatementKind::ClassDecl(decl) => {
            assert_eq!(decl.name, "Gateway");
            assert_eq!(decl.members[0].text, "<<interface>>");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &class_doc.statements[1].kind {
        StatementKind::ClassDecl(decl) => {
            assert_eq!(decl.name, "Shape");
            assert_eq!(decl.members[0].text, "<<abstract class>>");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    assert!(matches!(
        class_doc.statements[4].kind,
        StatementKind::FamilyRelation(_)
    ));
    match &class_doc.statements[4].kind {
        StatementKind::FamilyRelation(rel) => assert_eq!(rel.arrow, "-->"),
        other => panic!("unexpected statement: {other:?}"),
    }

    let object_doc = parse_with_options(
        "map Settings {\n  theme => light\n}\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(object_doc.kind, DiagramKind::Object);
    match &object_doc.statements[0].kind {
        StatementKind::ObjectDecl(decl) => {
            assert_eq!(decl.name, "Settings");
            assert_eq!(decl.members[0].text, "<<map>>");
            assert_eq!(decl.members[1].text, "theme => light");
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    let usecase_doc = parse_with_options(
        "actor Customer as C\nusecase (Login) as UC1\nC ..> UC1 : <<include>>\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(usecase_doc.kind, DiagramKind::UseCase);
    match &usecase_doc.statements[0].kind {
        StatementKind::UseCaseDecl(decl) => {
            assert_eq!(decl.name, "Customer");
            assert_eq!(decl.alias.as_deref(), Some("C"));
            assert_eq!(decl.members[0].text, "<<actor>>");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &usecase_doc.statements[1].kind {
        StatementKind::UseCaseDecl(decl) => {
            assert_eq!(decl.name, "Login");
            assert_eq!(decl.alias.as_deref(), Some("UC1"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &usecase_doc.statements[2].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.arrow, "..>");
            assert_eq!(rel.label.as_deref(), None);
            assert_eq!(rel.stereotype.as_deref(), Some("include"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_family_relations_with_tight_labels_quotes_and_cardinality() {
    let doc = parse_with_options(
            "class \"Order-Service\"\nclass \"Line-Item\"\nclass \"Price-List\"\n\"Order-Service\" \"1\" --> \"0..*\" \"Line-Item\": contains\nLine-Item --> \"Price-List\": priced by\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    match &doc.statements[3].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Order-Service");
            assert_eq!(rel.to, "Line-Item");
            assert_eq!(rel.label.as_deref(), Some("contains"));
            assert_eq!(rel.left_cardinality.as_deref(), Some("1"));
            assert_eq!(rel.right_cardinality.as_deref(), Some("0..*"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[4].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Line-Item");
            assert_eq!(rel.to, "Price-List");
            assert_eq!(rel.label.as_deref(), Some("priced by"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_component_namespace_groups_and_lollipop_endpoint_cleanup() {
    let doc = parse_with_options(
            "@startuml\nnamespace Edge {\n  component API\n  interface \"Orders\" as Orders\n}\nAPI --() Orders: provides\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Component);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::ClassGroup { .. }
    ));
    match &doc.statements[1].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "API");
            assert_eq!(rel.to, "Orders");
            assert_eq!(rel.label.as_deref(), Some("provides"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_scoped_core_uml_relations_and_lollipop_endpoints() {
    let doc = parse_with_options(
            "@startuml\npackage Domain {\n  namespace Core {\n    class Api\n    class Repo\n    Api \"1\" -[#green,dashed]-> \"0..*\" Repo : owns\n  }\n}\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
    match &doc.statements[0].kind {
        StatementKind::ClassGroup {
            members, relations, ..
        } => {
            assert!(members.iter().any(|m| m == "Domain::Core::Api"));
            assert_eq!(relations.len(), 1);
            assert_eq!(relations[0].from, "Domain::Core::Api");
            assert_eq!(relations[0].to, "Domain::Core::Repo");
            assert_eq!(relations[0].left_cardinality.as_deref(), Some("1"));
            assert_eq!(relations[0].right_cardinality.as_deref(), Some("0..*"));
            assert_eq!(relations[0].line_color.as_deref(), Some("#008000"));
            assert!(relations[0].dashed);
        }
        other => panic!("unexpected statement: {other:?}"),
    }

    let component_doc = parse_with_options(
            "@startuml\nnamespace Edge {\n  component API\n  interface Orders\n  API --() Orders : provides\n}\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
    match &component_doc.statements[0].kind {
        StatementKind::ClassGroup { relations, .. } => {
            assert_eq!(relations.len(), 1);
            assert_eq!(relations[0].from, "Edge::API");
            assert_eq!(relations[0].to, "Edge::Orders");
            assert!(relations[0].right_lollipop);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_sequence_decorated_arrow_styles_as_portable_arrow_core() {
    let doc = parse_with_options(
            "participant A\nparticipant B\nA -[#red,dashed]> B : styled\nB ->[#blue,dashed]> A : open styled\nA -[hidden]-> B : hidden\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Sequence);
    match &doc.statements[2].kind {
        StatementKind::Message(m) => {
            assert_eq!(m.arrow, "->");
            assert_eq!(m.style.color.as_deref(), Some("red"));
            assert!(m.style.dashed);
            assert!(!m.style.hidden);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[3].kind {
        StatementKind::Message(m) => {
            assert_eq!(m.arrow, "->>");
            assert_eq!(m.style.color.as_deref(), Some("blue"));
            assert!(m.style.dashed);
            assert!(!m.style.hidden);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[4].kind {
        StatementKind::Message(m) => {
            assert_eq!(m.arrow, "-->");
            assert!(m.style.hidden);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_sequence_participants_in_theme_fixture_context() {
    let fixture = fs::read_to_string(format!(
        "{}/docs/examples/themes/07_no_theme_default.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("theme fixture");

    let doc = parse_with_options(&fixture, &ParseOptions::default()).unwrap();

    assert_eq!(doc.kind, DiagramKind::Sequence);
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::Participant(_)
    ));
    assert!(matches!(
        doc.statements[2].kind,
        StatementKind::Participant(_)
    ));
    assert!(matches!(doc.statements[3].kind, StatementKind::Message(_)));
    assert!(matches!(doc.statements[4].kind, StatementKind::Message(_)));
}
