use crate::common::*;

#[test]
fn normalize_family_routes_bootstrap_families_to_stub_model() {
    for case in [
        "families/valid_class_bootstrap.puml",
        "families/valid_object_bootstrap.puml",
        "families/valid_usecase_bootstrap.puml",
        "families/valid_salt_bootstrap.puml",
    ] {
        let src = fs::read_to_string(fixture(case)).expect("fixture should load");
        let doc = parse(&src).expect("parse should succeed");
        let normalized = normalize_family(doc).expect("family normalize should succeed");
        match normalized {
            NormalizedDocument::Family(model) => {
                assert!(!model.nodes.is_empty(), "expected nodes for {case}");
                assert!(!model.relations.is_empty(), "expected relations for {case}");
            }
            NormalizedDocument::Sequence(_)
            | NormalizedDocument::Timeline(_)
            | NormalizedDocument::State(_) => {
                panic!("expected family stub model for {case}");
            }
            _ => panic!("expected family stub model for {case}"),
        }
    }
}

#[test]
fn normalize_family_succeeds_for_basic_state_diagram() {
    let src = fs::read_to_string(fixture("non_sequence/invalid_state_diagram.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let normalized = normalize_family(doc).expect("state family should now be supported");
    assert!(matches!(normalized, NormalizedDocument::State(_)));
}

#[test]
fn normalize_family_rejects_unknown_family_with_deterministic_code() {
    let src = "@startuml\nthis is not supported\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize_family(doc).expect_err("unknown family should be rejected");
    assert!(err.message.contains("E_FAMILY_UNKNOWN"));
}

#[test]
fn normalize_family_accepts_gantt_and_chronology_baseline_models() {
    for src in [
        "@startgantt\n[Build]\n[M1] happens on 2026-05-01\n[Build] starts 2026-04-01\n@endgantt\n",
        "@startchronology\nLaunch happens on 2026-05-15\n@endchronology\n",
    ] {
        let doc = parse(src).expect("parse should succeed");
        let normalized = normalize_family(doc).expect("timeline baseline should normalize");
        match normalized {
            NormalizedDocument::Timeline(model) => {
                assert!(!model.tasks.is_empty() || !model.chronology_events.is_empty())
            }
            other => panic!("expected timeline model, got {other:?}"),
        }
    }
}

#[test]
fn normalize_family_routes_salt_unknown_lines_to_widget_nodes() {
    // Plain (non-pipe) lines in a @startsalt block are encoded as SALT_ROW label nodes.
    let src = "@startsalt\nwidget title\n@endsalt\n";
    let doc = parse(src).expect("parse should succeed");
    let normalized = normalize_family(doc).expect("salt normalize should succeed");
    match normalized {
        NormalizedDocument::Family(model) => {
            assert_eq!(model.nodes.len(), 1, "expected one widget node");
            // Salt lines are now encoded as "SALT_ROW\x1fL:<text>" for the wireframe renderer.
            assert!(
                model.nodes[0].name.starts_with("SALT_ROW\x1f"),
                "expected salt row encoding, got: {}",
                model.nodes[0].name
            );
            assert!(
                model.nodes[0].name.contains("widget title"),
                "expected label text, got: {}",
                model.nodes[0].name
            );
            assert_eq!(model.relations.len(), 0, "no relations expected");
        }
        NormalizedDocument::Sequence(_) => panic!("expected salt family model"),
        NormalizedDocument::Timeline(_) => panic!("expected salt family model"),
        _ => panic!("expected salt family model"),
    }
}

#[test]
fn normalize_family_accepts_wave1_implemented_families() {
    let cases = [
        (
            "@startuml\ncomponent API\n@enduml\n",
            DiagramKind::Component,
        ),
        ("@startuml\nnode web\n@enduml\n", DiagramKind::Deployment),
        (
            "@startuml\nstart\n:step;\nstop\n@enduml\n",
            DiagramKind::Activity,
        ),
        ("@startuml\nclock clk\n@enduml\n", DiagramKind::Timing),
    ];
    for (src, expected_kind) in cases {
        let doc = parse(src).expect("parse should succeed");
        assert_eq!(doc.kind, expected_kind);
        let normalized = normalize_family(doc).expect("family should normalize");
        match normalized {
            NormalizedDocument::Family(model) => assert_eq!(model.kind, expected_kind),
            other => panic!("expected family model, got {other:?}"),
        }
    }
}

#[test]
fn normalize_family_accepts_gantt_and_chronology_timelines() {
    let cases = [
        (
            "non_sequence/valid_gantt_diagram.puml",
            puml::ast::DiagramKind::Gantt,
            3,
        ),
        (
            "non_sequence/valid_chronology_diagram.puml",
            puml::ast::DiagramKind::Chronology,
            3,
        ),
    ];

    for (path, expected_kind, expected_entries) in cases {
        let src = fs::read_to_string(fixture(path)).expect("fixture should load");
        let doc = parse(&src).expect("parse should succeed");
        assert_eq!(doc.kind, expected_kind);
        let normalized = normalize_family(doc).expect("timeline normalize should succeed");
        match normalized {
            NormalizedDocument::Timeline(model) => {
                assert!(
                    model.tasks.len()
                        + model.milestones.len()
                        + model.constraints.len()
                        + model.chronology_events.len()
                        >= expected_entries
                );
                assert_eq!(model.title.as_deref(), Some("Timeline Overview"));
            }
            other => panic!("expected timeline model, got {:?}", other),
        }
    }
}

#[test]
fn normalize_family_rejects_sequence_only_syntax_in_timeline_slice() {
    let src = "@startgantt\nparticipant A\nA -> B\n@endgantt\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize_family(doc).expect_err("mixed timeline/sequence syntax should fail");
    assert!(err.message.contains("E_GANTT_UNSUPPORTED"));
}

#[test]
fn normalize_family_rejects_mixed_bootstrap_declaration_kinds() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Class,
        statements: vec![puml::ast::Statement {
            span: Span { start: 0, end: 0 },
            kind: puml::ast::StatementKind::ObjectDecl(puml::ast::ObjectDecl {
                name: "Obj".to_string(),
                alias: None,
                members: Vec::new(),
            }),
        }],
    };

    let err = normalize_family(doc).expect_err("mixed family declarations should fail");
    assert!(err.message.contains("E_FAMILY_MIXED"));
}

#[test]
fn normalize_family_accepts_metadata_and_preprocessor_directives_in_stub_slice() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Class,
        statements: vec![
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Title("Family Title".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Header("Family Header".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Footer("Family Footer".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Caption("Family Caption".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Legend("Family Legend".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::SkinParam {
                    key: "ArrowColor".to_string(),
                    value: "red".to_string(),
                },
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Theme("plain".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Include("shared.puml".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Define {
                    name: "X".to_string(),
                    value: Some("1".to_string()),
                },
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Undef("X".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::ClassDecl(puml::ast::ClassDecl {
                    name: "User".to_string(),
                    alias: Some("U".to_string()),
                    members: vec![
                        puml::ast::ClassMember {
                            text: "+id: UUID".to_string(),
                            modifier: None,
                        },
                        puml::ast::ClassMember {
                            text: "+name: String".to_string(),
                            modifier: None,
                        },
                    ],
                }),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::FamilyRelation(puml::ast::FamilyRelation {
                    from: "User".to_string(),
                    to: "U".to_string(),
                    arrow: "-->".to_string(),
                    label: Some("owns".to_string()),
                    stereotype: None,
                    left_cardinality: None,
                    right_cardinality: None,
                    left_role: None,
                    right_role: None,
                    line_color: None,
                    dashed: false,
                    hidden: false,
                    thickness: None,
                    direction: None,
                    left_lollipop: false,
                    right_lollipop: false,
                }),
            },
        ],
    };

    let normalized = normalize_family(doc).expect("stub family normalize should succeed");
    match normalized {
        NormalizedDocument::Family(model) => {
            assert_eq!(model.title.as_deref(), Some("Family Title"));
            assert_eq!(model.header.as_deref(), Some("Family Header"));
            assert_eq!(model.footer.as_deref(), Some("Family Footer"));
            assert_eq!(model.caption.as_deref(), Some("Family Caption"));
            assert_eq!(model.legend.as_deref(), Some("Family Legend"));
            assert_eq!(model.nodes.len(), 1);
            assert_eq!(model.nodes[0].members.len(), 2);
            assert_eq!(model.relations.len(), 1);
            assert!(model.warnings.is_empty());
        }
        NormalizedDocument::Sequence(_)
        | NormalizedDocument::Timeline(_)
        | NormalizedDocument::State(_) => panic!("expected family model"),
        _ => panic!("expected family model"),
    }
}

#[test]
fn normalize_family_reports_parse_unknown_inside_stub_slice() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Class,
        statements: vec![puml::ast::Statement {
            span: Span { start: 3, end: 9 },
            kind: puml::ast::StatementKind::Unknown("class ???".to_string()),
        }],
    };
    let err = normalize_family(doc).expect_err("unknown stub syntax should fail");
    assert!(err.message.contains("E_PARSE_UNKNOWN"));
}

#[test]
fn normalize_family_rejects_sequence_only_events_in_bootstrap_slice() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::UseCase,
        statements: vec![puml::ast::Statement {
            span: Span { start: 0, end: 0 },
            kind: puml::ast::StatementKind::Participant(puml::ast::ParticipantDecl {
                role: puml::ast::ParticipantRole::Participant,
                name: "User".to_string(),
                alias: None,
                display: None,
                order: None,
            }),
        }],
    };

    let err = normalize_family(doc).expect_err("sequence-only syntax should fail in stub family");
    assert!(err.message.contains("E_FAMILY_STUB_UNSUPPORTED"));
}

#[test]
fn normalize_rejects_family_document_with_unknown_syntax_statement() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Class,
        statements: vec![puml::ast::Statement {
            span: Span::new(3, 8),
            kind: puml::ast::StatementKind::Unknown("???".to_string()),
        }],
    };
    let err = normalize_family(doc).expect_err("expected parse unknown error for family");
    assert!(err.message.contains("E_PARSE_UNKNOWN"));
}

#[test]
fn normalize_family_rejects_mixed_usecase_in_object_stub() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Object,
        statements: vec![puml::ast::Statement {
            span: Span::new(0, 2),
            kind: puml::ast::StatementKind::UseCaseDecl(puml::ast::UseCaseDecl {
                name: "UC".to_string(),
                alias: None,
                members: Vec::new(),
            }),
        }],
    };
    let err = normalize_family(doc).expect_err("expected family mixed error");
    assert!(err.message.contains("E_FAMILY_MIXED"));
}

#[test]
fn normalize_family_preserves_metadata_and_ignores_preprocessor_placeholders() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Class,
        statements: vec![
            puml::ast::Statement {
                span: Span::new(0, 1),
                kind: puml::ast::StatementKind::Header("Top".to_string()),
            },
            puml::ast::Statement {
                span: Span::new(1, 2),
                kind: puml::ast::StatementKind::Footer("Bottom".to_string()),
            },
            puml::ast::Statement {
                span: Span::new(2, 3),
                kind: puml::ast::StatementKind::Caption("Cap".to_string()),
            },
            puml::ast::Statement {
                span: Span::new(3, 4),
                kind: puml::ast::StatementKind::Legend("Leg".to_string()),
            },
            puml::ast::Statement {
                span: Span::new(4, 5),
                kind: puml::ast::StatementKind::Include("x".to_string()),
            },
            puml::ast::Statement {
                span: Span::new(5, 6),
                kind: puml::ast::StatementKind::Define {
                    name: "K".to_string(),
                    value: Some("1".to_string()),
                },
            },
            puml::ast::Statement {
                span: Span::new(6, 7),
                kind: puml::ast::StatementKind::Undef("K".to_string()),
            },
            puml::ast::Statement {
                span: Span::new(7, 8),
                kind: puml::ast::StatementKind::ClassDecl(puml::ast::ClassDecl {
                    name: "A".to_string(),
                    alias: None,
                    members: Vec::new(),
                }),
            },
        ],
    };
    let out = normalize_family(doc).expect("family normalize should succeed");
    let family = match out {
        NormalizedDocument::Family(v) => v,
        _ => panic!("expected family document"),
    };
    assert_eq!(family.header.as_deref(), Some("Top"));
    assert_eq!(family.footer.as_deref(), Some("Bottom"));
    assert_eq!(family.caption.as_deref(), Some("Cap"));
    assert_eq!(family.legend.as_deref(), Some("Leg"));
}
