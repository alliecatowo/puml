use assert_cmd::Command;
use predicates::prelude::*;
use puml::layout;
use puml::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
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
    assert!(err.message.contains("E_FAMILY_UNKNOWN"));
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
    assert!(err
        .message
        .contains("malformed quoted autonumber format"));
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
fn normalize_reports_group_empty_branch_before_else() {
    let src = "@startuml\nalt one\nelse two\nA -> B\nend\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected empty-branch-before-else error");
    assert!(err.message.contains("E_GROUP_EMPTY_BRANCH"));
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
        ("@startuml\ndestroy A\nactivate A\n@enduml\n", "E_LIFECYCLE_ACTIVATE_DESTROYED"),
        ("@startuml\ndestroy A\ndeactivate A\n@enduml\n", "E_LIFECYCLE_DEACTIVATE_DESTROYED"),
        ("@startuml\ndestroy A\ndestroy A\n@enduml\n", "E_LIFECYCLE_DESTROY_TWICE"),
        ("@startuml\ncreate A\ncreate A\n@enduml\n", "E_LIFECYCLE_CREATE_EXISTING"),
    ] {
        let doc = parse(src).expect("parse should succeed");
        let err = normalize::normalize(doc).expect_err("expected lifecycle diagnostic");
        assert!(err.message.contains(code), "expected {code}, got {}", err.message);
    }
}

#[test]
fn normalize_reports_destroyed_sender_message_error() {
    let src = "@startuml\ndestroy A\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected destroyed sender error");
    assert!(err.message.contains("E_LIFECYCLE_DESTROYED_SENDER"));
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
                }),
            },
        ],
    };
    let out = normalize_family(doc).expect("family normalize should succeed");
    let family = match out {
        NormalizedDocument::Family(v) => v,
        NormalizedDocument::Sequence(_) => panic!("expected family document"),
    };
    assert_eq!(family.header.as_deref(), Some("Top"));
    assert_eq!(family.footer.as_deref(), Some("Bottom"));
    assert_eq!(family.caption.as_deref(), Some("Cap"));
    assert_eq!(family.legend.as_deref(), Some("Leg"));
}
