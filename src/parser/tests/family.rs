use super::*;

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
fn parses_scoped_association_class_tuple_as_class_family() {
    let doc = parse_with_options(
        "package domain {\n  class User\n  class Group\n  class Membership\n  (User, Group) .. Membership\n}\n",
        &ParseOptions::default(),
    )
    .unwrap();

    assert_eq!(doc.kind, DiagramKind::Class);
    match &doc.statements[0].kind {
        StatementKind::ClassGroup { relations, .. } => {
            assert_eq!(relations.len(), 3);
            assert!(relations
                .iter()
                .any(|rel| rel.from == "domain::Membership" && rel.to == "domain::User"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_object_and_usecase_bootstrap_kinds() {
    let object_doc =
        parse_with_options("object Order\nobject Customer\n", &ParseOptions::default())
            .unwrap();
    assert_eq!(object_doc.kind, DiagramKind::Object);

    let usecase_doc = parse_with_options(
        "usecase Authenticate\nusecase Authorize\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(usecase_doc.kind, DiagramKind::UseCase);

    let single_usecase_doc =
        parse_with_options("usecase Checkout\n", &ParseOptions::default()).unwrap();
    assert_eq!(single_usecase_doc.kind, DiagramKind::UseCase);
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
fn parses_ie_entity_blocks_as_class_family_with_crows_feet() {
    let doc = parse_with_options(
        "skinparam linetype ortho\nentity CUSTOMER {\n  *customer_id : number <<generated>>\n  --\n  *name : text\n}\nentity ORDER {\n  *order_id : number <<generated>>\n}\nCUSTOMER ||--o{ ORDER : places\nORDER }|..|| CUSTOMER : owned by\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::SkinParam { .. }
    ));
    match &doc.statements[1].kind {
        StatementKind::ClassDecl(decl) => {
            assert_eq!(decl.name, "CUSTOMER");
            assert_eq!(decl.members[0].text, "*customer_id : number <<generated>>");
            assert_eq!(decl.members[1].text, "--");
            assert_eq!(decl.members[2].text, "*name : text");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[3].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "CUSTOMER");
            assert_eq!(rel.to, "ORDER");
            assert_eq!(rel.arrow, "||--o{");
            assert_eq!(rel.label.as_deref(), Some("places"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[4].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "ORDER");
            assert_eq!(rel.to, "CUSTOMER");
            assert_eq!(rel.arrow, "}|..||");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_ie_endpoint_pairs_and_dotted_variants() {
    let doc = parse_with_options(
        "entity A {\n}\nentity B {\n}\nA |o--|| B\nA ||..|| B\nA }o--|| B\nA }|..|| B\nB ||--o{ A\nB ||..|{ A\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    let arrows: Vec<&str> = doc
        .statements
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            StatementKind::FamilyRelation(rel) => Some(rel.arrow.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        arrows,
        vec!["|o--||", "||..||", "}o--||", "}|..||", "||--o{", "||..|{"]
    );
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
fn parses_family_declaration_blocks_with_members() {
    let doc = parse_with_options(
        "class User {\n  +id: UUID\n  +name: String\n}\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    match &doc.statements[0].kind {
        StatementKind::ClassDecl(decl) => {
            assert_eq!(decl.name, "User");
            assert_eq!(decl.members.len(), 2);
            assert_eq!(decl.members[0].text, "+id: UUID");
            assert_eq!(decl.members[0].modifier, None);
            assert_eq!(decl.members[1].text, "+name: String");
            assert_eq!(decl.members[1].modifier, None);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn unclosed_family_declaration_block_reports_deterministic_error() {
    let err = parse_with_options(
        "object Config {\nkey = \"value\"\n",
        &ParseOptions::default(),
    )
    .unwrap_err();
    assert!(err.message.contains("E_FAMILY_DECL_BLOCK_UNCLOSED"));
}


#[test]
fn parses_usecase_relations_with_alias_and_label() {
    let doc = parse_with_options(
        "usecase Authenticate as Auth\nusecase User\nAuth --> User : validates\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::UseCase);
    match &doc.statements[2].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Auth");
            assert_eq!(rel.to, "User");
            assert_eq!(rel.arrow, "-->");
            assert_eq!(rel.label.as_deref(), Some("validates"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn malformed_family_relation_is_preserved_as_unsupported_statement() {
    let doc = parse_with_options("class User\nUser -->\n", &ParseOptions::default()).unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::UnsupportedSyntax(_)
    ));
}

#[test]
fn state_keyword_is_parsed_as_state_decl() {
    let doc = parse_with_options("state Running\n", &ParseOptions::default()).unwrap();
    assert_eq!(doc.kind, DiagramKind::State);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::StateDecl(_)
    ));
}

#[test]
fn family_newpage_parses_as_page_break() {
    let doc = parse_with_options("class A\nnewpage Page Two\nclass B\n", &ParseOptions::default())
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    assert!(matches!(doc.statements[1].kind, StatementKind::NewPage(_)));
}

#[test]
fn start_enduml_markers_accept_optional_block_suffixes() {
    let doc = parse_with_options(
        "@startuml \"Primary\"\nA -> B: one\n@enduml anything\n@startuml Second\nB -> A: two\n@enduml\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Sequence);
    let labels = doc
        .statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.as_deref(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["one", "two"]);
}


#[test]
fn mismatched_start_end_family_markers_report_deterministic_error() {
    let err = parse_with_options("@startmindmap\n* Root\n@endwbs\n", &ParseOptions::default())
        .unwrap_err();
    assert!(err.message.contains("E_BLOCK_MISMATCH"));
}
