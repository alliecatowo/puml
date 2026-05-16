use assert_cmd::Command;
use predicates::prelude::*;
use puml::layout;
use puml::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide,
};
use puml::normalize;
use puml::parser::{parse_with_options, ParseOptions};
use puml::scene::{LayoutOptions, TextOverflowPolicy};
use puml::source::Span;
use puml::theme::{
    classify_sequence_skinparam, SequenceSkinParamSupport, SequenceSkinParamValue, SequenceStyle,
};
use puml::{normalize_family, parse, render, NormalizedDocument};
use std::fs;
use tempfile::tempdir;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn parser_define_substitution_skips_quoted_tokens() {
    let src = "@startuml\n!define A Alice\nparticipant A\nA -> A : \"A\" and A\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let msg = model
        .events
        .iter()
        .find_map(|e| match &e.kind {
            SequenceEventKind::Message { label, .. } => label.clone(),
            _ => None,
        })
        .expect("expected message");

    assert_eq!(msg, "\"A\" and Alice");
}

#[test]
fn parser_include_from_stdin_requires_include_root() {
    let src = "@startuml\n!include child.puml\n@enduml\n";
    let err =
        parse_with_options(src, &ParseOptions::default()).expect_err("expected include_root error");
    assert!(err.message.contains("include_root option"));
}

#[test]
fn parser_reports_include_cycle_chain() {
    let src = fs::read_to_string(fixture("include/error_include_cycle_self.puml")).unwrap();
    let options = ParseOptions {
        include_root: Some(std::path::PathBuf::from(fixture("include"))),
    };
    let err = parse_with_options(&src, &options).expect_err("expected include cycle");
    assert!(err.message.contains("include cycle detected"));
}

#[test]
fn parser_blocks_include_parent_escape() {
    let tmp = tempdir().unwrap();
    let root = tmp.path().join("root");
    let outside = tmp.path().join("outside.puml");
    fs::create_dir_all(&root).unwrap();
    fs::write(&outside, "A -> B\n").unwrap();

    let src = "@startuml\n!include ../outside.puml\n@enduml\n";
    let options = ParseOptions {
        include_root: Some(root),
    };
    let err = parse_with_options(src, &options).expect_err("expected include escape");
    assert!(err.message.contains("E_INCLUDE_ESCAPE"));
}

#[cfg(unix)]
#[test]
fn parser_blocks_symlink_include_escape() {
    use std::os::unix::fs::symlink;

    let tmp = tempdir().unwrap();
    let root = tmp.path().join("root");
    let outside = tmp.path().join("outside.puml");
    fs::create_dir_all(&root).unwrap();
    fs::write(&outside, "A -> B\n").unwrap();
    symlink(&outside, root.join("linked.puml")).unwrap();

    let src = "@startuml\n!include linked.puml\n@enduml\n";
    let options = ParseOptions {
        include_root: Some(root),
    };
    let err = parse_with_options(src, &options).expect_err("expected symlink include escape");
    assert!(err.message.contains("E_INCLUDE_ESCAPE"));
}

#[test]
fn normalize_reports_destroy_active_for_shortcut() {
    let src = "@startuml\nA -> B++\nA -> B!!\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected lifecycle error");
    assert!(err.message.contains("E_LIFECYCLE_DESTROY_ACTIVE"));
}

#[test]
fn normalize_family_routes_bootstrap_families_to_stub_model() {
    for case in [
        "families/valid_class_bootstrap.puml",
        "families/valid_object_bootstrap.puml",
        "families/valid_usecase_bootstrap.puml",
    ] {
        let src = fs::read_to_string(fixture(case)).expect("fixture should load");
        let doc = parse(&src).expect("parse should succeed");
        let normalized = normalize_family(doc).expect("family normalize should succeed");
        match normalized {
            NormalizedDocument::Family(model) => {
                assert!(!model.nodes.is_empty(), "expected nodes for {case}");
                assert!(!model.relations.is_empty(), "expected relations for {case}");
            }
            NormalizedDocument::Sequence(_) => {
                panic!("expected family stub model for {case}");
            }
        }
    }
}

#[test]
fn normalize_family_rejects_unsupported_state_family() {
    let src = fs::read_to_string(fixture("non_sequence/invalid_state_diagram.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let err = normalize_family(doc).expect_err("state family should be unsupported");
    assert!(err.message.contains("E_FAMILY_STATE_UNSUPPORTED"));
}

#[test]
fn normalize_family_rejects_unknown_family_with_deterministic_code() {
    let src = "@startuml\nthis is not supported\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize_family(doc).expect_err("unknown family should be rejected");
    assert!(err.message.contains("E_FAMILY_UNKNOWN"));
}

#[test]
fn parser_tags_all_wave1_non_sequence_families_deterministically() {
    let cases = [
        (
            "@startuml\ncomponent API\n@enduml\n",
            puml::ast::DiagramKind::Component,
        ),
        (
            "@startuml\ninterface Gateway\n@enduml\n",
            puml::ast::DiagramKind::Component,
        ),
        (
            "@startuml\nport Ingress\n@enduml\n",
            puml::ast::DiagramKind::Component,
        ),
        (
            "@startuml\nnode web\n@enduml\n",
            puml::ast::DiagramKind::Deployment,
        ),
        (
            "@startuml\nartifact app\n@enduml\n",
            puml::ast::DiagramKind::Deployment,
        ),
        (
            "@startuml\ncloud edge\n@enduml\n",
            puml::ast::DiagramKind::Deployment,
        ),
        (
            "@startuml\nframe rack\n@enduml\n",
            puml::ast::DiagramKind::Deployment,
        ),
        (
            "@startuml\nstorage db\n@enduml\n",
            puml::ast::DiagramKind::Deployment,
        ),
        (
            "@startuml\nstate Running\n@enduml\n",
            puml::ast::DiagramKind::State,
        ),
        ("@startuml\n[H]\n@enduml\n", puml::ast::DiagramKind::State),
        (
            "@startuml\nstart\n@enduml\n",
            puml::ast::DiagramKind::Activity,
        ),
        (
            "@startuml\npartition lane\n@enduml\n",
            puml::ast::DiagramKind::Activity,
        ),
        (
            "@startuml\nfork\n@enduml\n",
            puml::ast::DiagramKind::Activity,
        ),
        (
            "@startuml\nclock clk\n@enduml\n",
            puml::ast::DiagramKind::Timing,
        ),
        (
            "@startuml\nbinary sig\n@enduml\n",
            puml::ast::DiagramKind::Timing,
        ),
        (
            "@startuml\nscale 1 as 1\n@enduml\n",
            puml::ast::DiagramKind::Timing,
        ),
    ];

    for (src, expected_kind) in cases {
        let doc = parse(src).expect("parse should succeed");
        assert_eq!(doc.kind, expected_kind);
    }
}

#[test]
fn parser_tags_additional_wave1_family_alias_tokens() {
    let cases = [
        (
            "@startuml\nportin ingress\n@enduml\n",
            puml::ast::DiagramKind::Component,
        ),
        (
            "@startuml\nportout egress\n@enduml\n",
            puml::ast::DiagramKind::Component,
        ),
        (
            "@startuml\nswimlane laneA\n@enduml\n",
            puml::ast::DiagramKind::Activity,
        ),
        (
            "@startuml\nconcise t\n@enduml\n",
            puml::ast::DiagramKind::Timing,
        ),
        (
            "@startuml\nrobust t\n@enduml\n",
            puml::ast::DiagramKind::Timing,
        ),
        ("@startuml\n@1\n@enduml\n", puml::ast::DiagramKind::Timing),
    ];

    for (src, expected_kind) in cases {
        let doc = parse(src).expect("parse should succeed");
        assert_eq!(doc.kind, expected_kind);
    }
}

#[test]
fn normalize_family_rejects_all_wave1_non_sequence_families_with_specific_codes() {
    let cases = [
        (
            "@startuml\ncomponent API\n@enduml\n",
            "E_FAMILY_COMPONENT_UNSUPPORTED",
        ),
        (
            "@startuml\nnode web\n@enduml\n",
            "E_FAMILY_DEPLOYMENT_UNSUPPORTED",
        ),
        (
            "@startuml\nstate Running\n@enduml\n",
            "E_FAMILY_STATE_UNSUPPORTED",
        ),
        (
            "@startuml\nstart\n:step;\nstop\n@enduml\n",
            "E_FAMILY_ACTIVITY_UNSUPPORTED",
        ),
        (
            "@startuml\nclock clk\n@enduml\n",
            "E_FAMILY_TIMING_UNSUPPORTED",
        ),
    ];

    for (src, code) in cases {
        let doc = parse(src).expect("parse should succeed");
        let err = normalize_family(doc).expect_err("family should be unsupported in this slice");
        assert!(err.message.contains(code), "missing code {code}");
    }
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
                }),
            },
            puml::ast::Statement {
                span: Span { start: 0, end: 0 },
                kind: puml::ast::StatementKind::FamilyRelation(puml::ast::FamilyRelation {
                    from: "User".to_string(),
                    to: "U".to_string(),
                    arrow: "-->".to_string(),
                    label: Some("owns".to_string()),
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
            assert_eq!(model.relations.len(), 1);
            assert!(model.warnings.is_empty());
        }
        NormalizedDocument::Sequence(_) => panic!("expected family model"),
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
            }),
        }],
    };

    let err = normalize_family(doc).expect_err("sequence-only syntax should fail in stub family");
    assert!(err.message.contains("E_FAMILY_STUB_UNSUPPORTED"));
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
fn normalize_emits_theme_warning_without_name_suffix_when_theme_name_is_empty() {
    let src = "@startuml\n!theme\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    assert_eq!(model.warnings.len(), 1);
    assert_eq!(
        model.warnings[0].message,
        "[W_THEME_UNSUPPORTED] !theme is not supported yet"
    );
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
fn normalize_expands_bidirectional_message_into_two_events() {
    let src = "@startuml\nA <-> B : ping\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let messages = model
        .events
        .iter()
        .filter(|e| matches!(e.kind, SequenceEventKind::Message { .. }))
        .count();
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
fn theme_classifies_sequence_skinparam_subset() {
    assert_eq!(
        classify_sequence_skinparam("maxMessageSize", "120"),
        SequenceSkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceFootbox", "hide"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::FootboxVisible(false))
    );
    assert_eq!(
        classify_sequence_skinparam("footbox", "show"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::FootboxVisible(true))
    );
    assert_eq!(
        classify_sequence_skinparam("sequenceFootbox", "bogus"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("ArrowColor", "red"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
            "red".to_string()
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("ArrowColor", "#AaBbCC"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
            "#aabbcc".to_string()
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("ArrowColor", "\"/><script"),
        SequenceSkinParamSupport::UnsupportedValue
    );
}

#[test]
fn layout_applies_autonumber_stop_and_restart() {
    let src = "@startuml\nautonumber\nA -> B : one\nautonumber stop\nB -> A : two\nautonumber 5\nA -> B : three\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    let labels = scene
        .messages
        .iter()
        .map(|m| m.label.clone().unwrap_or_default())
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["1 one", "two", "5 three"]);
}

#[test]
fn layout_handles_return_without_caller() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        events: vec![SequenceEvent {
            span: puml::source::Span { start: 0, end: 0 },
            kind: SequenceEventKind::Return {
                label: Some("noop".to_string()),
                from: None,
                to: None,
            },
        }],
        title: None,
        header: None,
        footer: None,
        caption: None,
        legend: None,
        skinparams: vec![],
        style: SequenceStyle::default(),
        footbox_visible: true,
        warnings: vec![],
    };

    let scene = layout::layout(&doc, LayoutOptions::default());
    assert!(scene.messages.is_empty());
}

#[test]
fn render_escapes_text_in_labels_and_titles() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A<&>\"'".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        events: vec![SequenceEvent {
            span: puml::source::Span { start: 0, end: 0 },
            kind: SequenceEventKind::Message {
                from: "A".to_string(),
                to: "A".to_string(),
                arrow: "->".to_string(),
                label: Some("<&>\"'".to_string()),
                from_virtual: None,
                to_virtual: None,
            },
        }],
        title: Some("T<&>\"'".to_string()),
        header: None,
        footer: None,
        caption: None,
        legend: None,
        skinparams: vec![],
        style: SequenceStyle::default(),
        footbox_visible: true,
        warnings: vec![],
    };
    let scene = layout::layout(&doc, LayoutOptions::default());
    let svg = render::render_svg(&scene);

    assert!(svg.contains("&lt;&amp;&gt;&quot;&#39;"));
}

#[test]
fn cli_output_directory_maps_to_io_exit_code() {
    let tmp = tempdir().unwrap();
    let out_dir = tmp.path().join("out_dir");
    fs::create_dir_all(&out_dir).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            fixture("single_valid.puml"),
            "--output".to_string(),
            out_dir.display().to_string(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));
}

#[test]
fn cli_include_root_allows_include_from_stdin() {
    let raw = fs::read_to_string(fixture("include/include_ok_child.puml")).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-", "--include-root", &fixture("include")])
        .write_stdin(raw)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn parser_reports_enduml_without_startuml() {
    let src = "@enduml\nA -> B : hi\n";
    let err = parse(src).expect_err("expected unmatched enduml");
    assert!(err.message.contains("unmatched @startuml/@enduml boundary"));
    assert!(err.message.contains("without a preceding @startuml"));
}

#[test]
fn parser_reports_unterminated_startuml() {
    let src = "@startuml\nA -> B : hi\n";
    let err = parse(src).expect_err("expected unterminated startuml");
    assert!(err.message.contains("unmatched @startuml/@enduml boundary"));
    assert!(err.message.contains("missing a closing @enduml"));
}

#[test]
fn layout_wraps_participant_labels_and_grows_boxes_by_default() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A very long participant label that should wrap".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        events: vec![],
        title: None,
        header: None,
        footer: None,
        caption: None,
        legend: None,
        skinparams: vec![],
        style: SequenceStyle::default(),
        footbox_visible: true,
        warnings: vec![],
    };
    let scene = layout::layout(&doc, LayoutOptions::default());

    let participant = scene
        .participants
        .iter()
        .find(|p| p.id == "A")
        .expect("expected participant A");
    assert!(participant.display_lines.len() > 1);
    assert!(participant.height > LayoutOptions::default().participant_height);

    let svg = render::render_svg(&scene);
    assert!(svg.contains(">A very long<"));
    assert!(svg.contains(">participant<"));
}

#[test]
fn layout_uses_ellipsis_for_single_line_overflow_policy() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A very long participant label that should truncate".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        events: vec![],
        title: None,
        header: None,
        footer: None,
        caption: None,
        legend: None,
        skinparams: vec![],
        style: SequenceStyle::default(),
        footbox_visible: true,
        warnings: vec![],
    };
    let options = LayoutOptions {
        text_overflow_policy: TextOverflowPolicy::EllipsisSingleLine,
        ..LayoutOptions::default()
    };
    let scene = layout::layout(&doc, options);

    let participant = scene
        .participants
        .iter()
        .find(|p| p.id == "A")
        .expect("expected participant A");
    assert_eq!(participant.display_lines.len(), 1);
    assert!(participant.display_lines[0].contains('…'));
    assert_eq!(
        participant.height,
        LayoutOptions::default().participant_height
    );
}

#[test]
fn layout_expands_rows_for_wrapped_labels_and_open_group_tail() {
    let model = SequenceDocument {
        participants: vec![
            Participant {
                id: "A".to_string(),
                display: "A".to_string(),
                role: ParticipantRole::Participant,
                explicit: true,
            },
            Participant {
                id: "B".to_string(),
                display: "B".to_string(),
                role: ParticipantRole::Participant,
                explicit: true,
            },
        ],
        events: vec![
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "B".to_string(),
                    arrow: "->".to_string(),
                    label: Some(
                        "one two three four five six seven eight nine ten eleven twelve"
                            .to_string(),
                    ),
                    from_virtual: None,
                    to_virtual: None,
                },
            },
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::GroupStart {
                    kind: "group".to_string(),
                    label: Some("over   ".to_string()),
                },
            },
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "B".to_string(),
                    to: "A".to_string(),
                    arrow: "->".to_string(),
                    label: Some("short".to_string()),
                    from_virtual: None,
                    to_virtual: None,
                },
            },
        ],
        title: None,
        header: None,
        footer: None,
        caption: None,
        legend: None,
        skinparams: vec![],
        style: SequenceStyle::default(),
        footbox_visible: true,
        warnings: vec![],
    };
    let options = LayoutOptions::default();
    let scene = layout::layout(&model, options);

    assert_eq!(scene.messages.len(), 2);
    let first = &scene.messages[0];
    let second = &scene.messages[1];
    assert!(
        first.label_lines.len() > 1,
        "expected wrapped first label to consume multiple rows"
    );
    assert!(
        second.y - first.y > options.message_row_height,
        "wrapped row should push following message downward"
    );
    assert_eq!(
        scene.groups.len(),
        1,
        "open group should be finalized at end"
    );
    assert!(
        scene.groups[0].height >= options.message_row_height,
        "finalized open group should have non-zero height"
    );
}

#[test]
fn layout_offsets_virtual_endpoints_for_overlap_cases() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        events: vec![
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "ghost-left".to_string(),
                    to: "ghost-right".to_string(),
                    arrow: "->".to_string(),
                    label: Some("both virtual".to_string()),
                    from_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Left,
                        kind: VirtualEndpointKind::Filled,
                    }),
                    to_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Right,
                        kind: VirtualEndpointKind::Filled,
                    }),
                },
            },
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "ghost-right".to_string(),
                    to: "A".to_string(),
                    arrow: "->".to_string(),
                    label: Some("from virtual".to_string()),
                    from_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Right,
                        kind: VirtualEndpointKind::Circle,
                    }),
                    to_virtual: None,
                },
            },
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "ghost-left".to_string(),
                    arrow: "->".to_string(),
                    label: Some("to virtual".to_string()),
                    from_virtual: None,
                    to_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Left,
                        kind: VirtualEndpointKind::Cross,
                    }),
                },
            },
        ],
        title: None,
        header: None,
        footer: None,
        caption: None,
        legend: None,
        skinparams: vec![],
        style: SequenceStyle::default(),
        footbox_visible: true,
        warnings: vec![],
    };
    let scene = layout::layout(&doc, LayoutOptions::default());
    let center = scene.participants[0].x + (scene.participants[0].width / 2);

    assert_eq!(scene.messages[0].x1, center - 56);
    assert_eq!(scene.messages[0].x2, center + 56);
    assert_eq!(scene.messages[1].x1, center + 56);
    assert_eq!(scene.messages[1].x2, center);
    assert_eq!(scene.messages[2].x1, center);
    assert_eq!(scene.messages[2].x2, center - 56);
}

#[test]
fn layout_pages_preserve_page_titles_and_footer_reserve_without_footboxes() {
    let src = "@startuml\ntitle Base Title\nfooter Shared Footer\nhide footbox\nA -> B : one\nnewpage Page Two\nB -> A : two\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let options = LayoutOptions::default();
    let pages = layout::layout_pages(&model, options);

    assert_eq!(pages.len(), 2);
    assert_eq!(
        pages[0]
            .title
            .as_ref()
            .map(|t| t.lines.join(" "))
            .as_deref(),
        Some("Base Title")
    );
    assert_eq!(
        pages[1]
            .title
            .as_ref()
            .map(|t| t.lines.join(" "))
            .as_deref(),
        Some("Page Two")
    );
    for scene in &pages {
        assert!(
            scene.footboxes.is_empty(),
            "hide footbox should remove footboxes"
        );
        for lifeline in &scene.lifelines {
            assert!(
                scene.height - lifeline.y2 >= options.footer_height + options.margin,
                "footer reserve should still remain when footboxes are hidden"
            );
        }
    }
}

#[test]
fn check_fixture_supports_json_diagnostics_for_errors() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            &fixture("arrows/invalid_malformed_arrows.puml"),
            "--diagnostics",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&out).expect("valid json diagnostics");
    assert_eq!(json["diagnostics"][0]["severity"], "error");
    assert_eq!(json["diagnostics"][0]["line"], 2);
    assert_eq!(json["diagnostics"][0]["column"], 1);
    assert_eq!(json["diagnostics"][0]["snippet"], "A -x B: malformed");
}
#[test]
fn check_fixture_supports_json_diagnostics_for_warnings() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            &fixture("styling/valid_skinparam_unsupported_value.puml"),
            "--diagnostics",
            "json",
        ])
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&out).expect("valid json diagnostics");
    assert_eq!(json["diagnostics"][0]["severity"], "warning");
    assert_eq!(json["diagnostics"][0]["line"], 2);
    assert_eq!(json["diagnostics"][0]["column"], 1);
    assert_eq!(
        json["diagnostics"][0]["snippet"],
        "skinparam sequenceFootbox maybe"
    );
    assert!(json["diagnostics"][0]["message"]
        .as_str()
        .unwrap()
        .contains("W_SKINPARAM_UNSUPPORTED_VALUE"));
}

#[test]
fn layout_row_sizing_advances_y_for_wrapped_message_labels_and_tall_notes() {
    let src = "@startuml\nA -> B : this is a deliberately long message label that wraps into several rows\nnote over A: note line one\nnote line two\nnote line three\nend note\nB -> A : done\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let options = LayoutOptions {
        participant_spacing: 90,
        ..LayoutOptions::default()
    };
    let scene = layout::layout(&model, options);

    assert!(scene.messages.len() >= 2, "expected two messages");
    assert!(
        scene.messages[0].label_lines.len() > 1,
        "first message should wrap"
    );

    assert!(!scene.notes.is_empty(), "expected at least one note");
    let note = &scene.notes[0];
    let second_message = &scene.messages[1];
    assert!(
        second_message.y >= note.y + options.message_row_height,
        "second message should be pushed below note row height expansion"
    );
}

#[test]
fn layout_group_else_separator_and_ref_min_height_are_deterministic() {
    let src = fs::read_to_string(fixture("groups/valid_ref_and_else_rendering.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let options = LayoutOptions::default();
    let scene = layout::layout(&model, options);

    let alt = scene
        .groups
        .iter()
        .find(|g| g.kind.eq_ignore_ascii_case("alt"))
        .expect("expected alt group");
    assert!(
        !alt.separators.is_empty(),
        "alt group should capture else separator rows"
    );
    assert!(alt.height >= options.message_row_height);

    let reference = scene
        .groups
        .iter()
        .find(|g| g.kind.eq_ignore_ascii_case("ref"))
        .expect("expected ref group");
    assert!(
        reference.height > options.message_row_height,
        "ref group should use computed min content height"
    );
}

#[test]
fn layout_overflow_bounds_keep_multi_target_note_and_over_group_in_view() {
    let src = "@startuml\nparticipant AlphaParticipantWithLongName\nparticipant BetaParticipantWithLongName\nparticipant GammaParticipantWithLongName\nnote right of AlphaParticipantWithLongName, BetaParticipantWithLongName: right note with a very long unbroken token AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\ngroup over AlphaParticipantWithLongName, GammaParticipantWithLongName : over Alpha/Gamma\nAlphaParticipantWithLongName -> GammaParticipantWithLongName : ping\nend\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    assert!(!scene.notes.is_empty(), "expected note");
    assert!(!scene.groups.is_empty(), "expected group");
    for note in &scene.notes {
        assert!(note.x >= LayoutOptions::default().margin);
        assert!(note.x + note.width <= scene.width);
    }
    for group in &scene.groups {
        assert!(group.x >= LayoutOptions::default().margin);
        assert!(group.x + group.width <= scene.width);
    }
}

#[test]
fn layout_pages_newpage_keeps_page_local_geometry_and_content() {
    let src = "@startuml\ntitle Base\nA -> B : one\nnewpage Second\nnote over A: page two note\nA -> B : two\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let pages = layout::layout_pages(&model, LayoutOptions::default());

    assert_eq!(pages.len(), 2, "newpage should split into two scene pages");
    assert_eq!(
        pages[0]
            .title
            .as_ref()
            .expect("page one title")
            .lines
            .first()
            .expect("line"),
        "Base"
    );
    assert_eq!(
        pages[1]
            .title
            .as_ref()
            .expect("page two title")
            .lines
            .first()
            .expect("line"),
        "Second"
    );
    assert!(
        pages[0].notes.is_empty(),
        "page one should not include page two note"
    );
    assert_eq!(pages[1].notes.len(), 1, "page two should contain its note");
}

#[test]
fn unknown_family_render_route_reports_deterministic_error_code() {
    use puml::DiagramFamily;

    let src = "@startuml\nfoo bar\n@enduml\n";
    let err = puml::render_source_to_svg_for_family(src, DiagramFamily::Unknown)
        .expect_err("expected unsupported family diagnostic");
    assert!(err.message.contains("E_RENDER_FAMILY_UNSUPPORTED"));
    assert!(!err.message.trim().is_empty());
}
