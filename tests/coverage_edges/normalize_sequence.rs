use crate::common::*;

#[test]
fn normalize_reports_destroy_active_for_shortcut() {
    let src = "@startuml\nA -> B++\nA -> B!!\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected lifecycle error");
    assert!(err.message.contains("E_LIFECYCLE_DESTROY_ACTIVE"));
}

#[test]
fn normalize_supports_sequence_footbox_skinparam_without_warning() {
    let src = fs::read_to_string(fixture(
        "styling/valid_skinparam_sequence_footbox_supported.puml",
    ))
    .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    assert!(!model.footbox_visible);
    assert!(model.warnings.is_empty());
}

#[test]
fn normalize_skinparam_unsupported_key_and_value_are_deterministic() {
    let unsupported_key_src =
        fs::read_to_string(fixture("styling/valid_skinparam_unsupported.puml"))
            .expect("fixture should load");
    let unsupported_key_doc = parse(&unsupported_key_src).expect("parse should succeed");
    let unsupported_key_model =
        normalize::normalize(unsupported_key_doc).expect("normalize should succeed");
    assert_eq!(unsupported_key_model.warnings.len(), 1);
    assert!(unsupported_key_model.warnings[0]
        .message
        .contains("W_SKINPARAM_UNSUPPORTED"));

    let unsupported_value_src =
        fs::read_to_string(fixture("styling/valid_skinparam_unsupported_value.puml"))
            .expect("fixture should load");
    let unsupported_value_doc = parse(&unsupported_value_src).expect("parse should succeed");
    let unsupported_value_model =
        normalize::normalize(unsupported_value_doc).expect("normalize should succeed");
    assert_eq!(unsupported_value_model.warnings.len(), 1);
    assert!(unsupported_value_model.warnings[0]
        .message
        .contains("W_SKINPARAM_UNSUPPORTED_VALUE"));
}

#[test]
fn normalize_supports_max_message_size_skinparam_without_warning() {
    let src = fs::read_to_string(fixture("basic/valid_skinparam_maxmessagesize.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    assert!(model.warnings.is_empty());
}

#[test]
fn normalize_applies_known_theme_and_rejects_unsupported_variants_deterministically() {
    let ok_src = "@startuml\n!theme spacelab\nA -> B\n@enduml\n";
    let ok_doc = parse(ok_src).expect("parse should succeed");
    let ok_model = normalize::normalize(ok_doc).expect("normalize should succeed");
    assert!(ok_model.warnings.is_empty());
    assert_eq!(ok_model.style.arrow_color, "#2f4f6f");

    let missing_name_src = "@startuml\n!theme\nA -> B\n@enduml\n";
    let missing_name_doc = parse(missing_name_src).expect("parse should succeed");
    let missing_name_err =
        normalize::normalize(missing_name_doc).expect_err("expected missing-name error");
    assert!(missing_name_err.message.contains("E_THEME_INVALID"));

    let remote_src = "@startuml\n!theme plain from https://example.com/themes\nA -> B\n@enduml\n";
    let remote_doc = parse(remote_src).expect("parse should succeed");
    let remote_err = normalize::normalize(remote_doc).expect_err("expected source-policy error");
    assert!(remote_err.message.contains("E_THEME_SOURCE_UNSUPPORTED"));

    let unknown_src = "@startuml\n!theme coffee\nA -> B\n@enduml\n";
    let unknown_doc = parse(unknown_src).expect("parse should succeed");
    let unknown_err = normalize::normalize(unknown_doc).expect_err("expected unknown-theme error");
    assert!(unknown_err.message.contains("E_THEME_UNKNOWN"));
}

#[test]
fn normalize_emits_deterministic_pragma_warnings() {
    let teoz_src = "@startuml\n!pragma teoz true\nA -> B: hi\n@enduml\n";
    let teoz_doc = parse(teoz_src).expect("parse should succeed");
    let teoz_model = normalize::normalize(teoz_doc).expect("normalize should succeed");
    assert_eq!(teoz_model.warnings.len(), 0);
    assert!(teoz_model.teoz);

    let generic_src = "@startuml\n!pragma foo bar\nA -> B: hi\n@enduml\n";
    let generic_doc = parse(generic_src).expect("parse should succeed");
    let generic_model = normalize::normalize(generic_doc).expect("normalize should succeed");
    assert_eq!(generic_model.warnings.len(), 1);
    assert!(generic_model.warnings[0]
        .message
        .contains("W_PRAGMA_UNSUPPORTED"));
}

#[test]
fn normalize_reports_invalid_arrow_when_ast_contains_malformed_arrow() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Sequence,
        statements: vec![puml::ast::Statement {
            span: Span { start: 0, end: 0 },
            kind: puml::ast::StatementKind::Message(puml::ast::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                arrow: "bogus".to_string(),
                label: None,
                style: Default::default(),
                from_virtual: None,
                to_virtual: None,
            }),
        }],
    };

    let err = normalize::normalize(doc).expect_err("expected malformed arrow error");
    assert!(err.message.contains("E_ARROW_INVALID"));
}

#[test]
fn normalize_reports_else_inside_loop_group_as_invalid_kind() {
    let src = fs::read_to_string(fixture("errors/invalid_else_inside_loop_group.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected group kind error");

    assert!(err.message.contains("E_GROUP_ELSE_KIND"));
}

#[test]
fn normalize_reports_group_end_keyword_mismatch() {
    let src = fs::read_to_string(fixture("errors/invalid_group_mismatched_end_keyword.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected end-kind mismatch");

    assert!(err.message.contains("E_GROUP_END_KIND"));
}

#[test]
fn normalize_reports_empty_alt_group() {
    let src =
        fs::read_to_string(fixture("errors/invalid_group_empty_alt.puml")).expect("fixture load");
    let doc = parse(&src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected empty group error");

    assert!(err.message.contains("E_GROUP_EMPTY"));
}

#[test]
fn normalize_rejects_deactivate_without_active_activation() {
    let src = "@startuml\ndeactivate A\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected deactivate-empty lifecycle error");
    assert!(err.message.contains("E_LIFECYCLE_DEACTIVATE_EMPTY"));
}

#[test]
fn normalize_rejects_deactivate_order_mismatch() {
    let src = "@startuml\nA -> B++\nB -> C++\ndeactivate B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected deactivate-order lifecycle error");
    assert!(err.message.contains("E_LIFECYCLE_DEACTIVATE_ORDER"));
}

#[test]
fn normalize_rejects_lifecycle_shortcuts_on_virtual_endpoints() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Sequence,
        statements: vec![puml::ast::Statement {
            span: Span { start: 0, end: 0 },
            kind: puml::ast::StatementKind::Message(puml::ast::Message {
                from: "[".to_string(),
                to: "A".to_string(),
                arrow: "->@L++".to_string(),
                label: None,
                style: Default::default(),
                from_virtual: None,
                to_virtual: None,
            }),
        }],
    };

    let err = normalize::normalize(doc).expect_err("expected virtual-endpoint lifecycle error");
    assert!(err.message.contains("E_LIFECYCLE_SHORTCUT_VIRTUAL"));
}

#[test]
fn normalize_ignores_horizontal_rule_unknown_syntax_passthrough() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Sequence,
        statements: vec![
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Unknown("---".to_string()),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::Message(puml::ast::Message {
                    from: "A".to_string(),
                    to: "B".to_string(),
                    arrow: "->".to_string(),
                    label: Some("ok".to_string()),
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: None,
                }),
            },
        ],
    };

    let model = normalize::normalize(doc).expect("normalize should succeed");
    assert_eq!(model.events.len(), 1);
}

#[test]
fn normalize_emits_single_bidirectional_message_event() {
    // Wave 3-B (#531): bidirectional arrows stay as single message events,
    // preserving solid and dashed variants for render parity coverage.
    let src = "@startuml\nA <-> B : ping\nA <--> B : pong\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let bidirectional_arrows = model
        .events
        .iter()
        .filter_map(|event| match &event.kind {
            SequenceEventKind::Message { arrow, .. } => Some(arrow.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let messages = model
        .events
        .iter()
        .filter(|event| matches!(event.kind, SequenceEventKind::Message { .. }))
        .count();
    assert_eq!(bidirectional_arrows, vec!["<->", "<-->"]);
    assert_eq!(messages, 2);
}

#[test]
fn paginate_newpage_blank_title_falls_back_to_document_title() {
    let src = "@startuml\ntitle Primary\nA -> B\nnewpage   \nB -> A\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let pages = normalize::paginate(&model);

    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].title.as_deref(), Some("Primary"));
    assert_eq!(pages[1].title.as_deref(), Some("Primary"));
}

#[test]
fn normalize_rejects_autonumber_with_embedded_quote_in_format() {
    let src = "@startuml\nautonumber 1 1 bad\"fmt\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected autonumber format diagnostic");
    assert!(err.message.contains("E_AUTONUMBER_FORMAT_UNSUPPORTED"));
}

#[test]
fn normalize_rejects_autonumber_with_nontrailing_quoted_format_token() {
    let src = "@startuml\nautonumber \"<b>\" 1\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected malformed quoted-format diagnostic");
    assert!(err.message.contains("malformed quoted autonumber format"));
}

#[test]
fn normalize_sequence_ignores_preprocessor_statements_in_ast() {
    let doc = puml::ast::Document {
        kind: puml::ast::DiagramKind::Sequence,
        statements: vec![
            puml::ast::Statement {
                span: Span::new(0, 1),
                kind: puml::ast::StatementKind::Include("x.puml".to_string()),
            },
            puml::ast::Statement {
                span: Span::new(1, 2),
                kind: puml::ast::StatementKind::Define {
                    name: "K".to_string(),
                    value: Some("1".to_string()),
                },
            },
            puml::ast::Statement {
                span: Span::new(2, 3),
                kind: puml::ast::StatementKind::Undef("K".to_string()),
            },
            puml::ast::Statement {
                span: Span::new(3, 4),
                kind: puml::ast::StatementKind::Message(puml::ast::Message {
                    from: "A".to_string(),
                    to: "B".to_string(),
                    arrow: "->".to_string(),
                    label: None,
                    style: puml::ast::MessageStyle::default(),
                    from_virtual: None,
                    to_virtual: None,
                }),
            },
        ],
    };
    let model = normalize::normalize(doc).expect("normalize should ignore preprocessor statements");
    assert_eq!(model.events.len(), 1);
}

#[test]
fn normalize_reports_group_unclosed_error() {
    let src = "@startuml\npar lane\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected unclosed group error");
    assert!(err.message.contains("E_GROUP_UNCLOSED"));
}

#[test]
fn normalize_reports_return_infer_empty_without_context() {
    let src = "@startuml\nreturn done\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected return inference error");
    assert!(err.message.contains("E_RETURN_INFER_EMPTY"));
}

#[test]
fn normalize_reports_lifecycle_explicit_destroyed_and_duplicate_errors() {
    for (src, code) in [
        (
            "@startuml\ndestroy A\nactivate A\n@enduml\n",
            "E_LIFECYCLE_ACTIVATE_DESTROYED",
        ),
        (
            "@startuml\ndestroy A\ndeactivate A\n@enduml\n",
            "E_LIFECYCLE_DEACTIVATE_DESTROYED",
        ),
        (
            "@startuml\ndestroy A\ndestroy A\n@enduml\n",
            "E_LIFECYCLE_DESTROY_TWICE",
        ),
        (
            "@startuml\ncreate A\ncreate A\n@enduml\n",
            "E_LIFECYCLE_CREATE_EXISTING",
        ),
    ] {
        let doc = parse(src).expect("parse should succeed");
        let err = normalize::normalize(doc).expect_err("expected lifecycle diagnostic");
        assert!(
            err.message.contains(code),
            "expected {code}, got {}",
            err.message
        );
    }
}

#[test]
fn normalize_reports_destroyed_sender_message_error() {
    let src = "@startuml\ndestroy A\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected destroyed sender error");
    assert!(err.message.contains("E_LIFECYCLE_DESTROYED_SENDER"));
}
