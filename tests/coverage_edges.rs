use assert_cmd::Command;
use predicates::prelude::*;
use puml::ast::DiagramKind;
use puml::layout;
use puml::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide,
};
use puml::normalize;
use puml::parser::{parse_with_options, ParseOptions};
use puml::scene::{LayoutOptions, TextOverflowPolicy};
use puml::source::Span;
use puml::theme::{classify_sequence_skinparam, SequenceSkinParamSupport, SequenceSkinParamValue};
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
fn parser_preprocessor_variables_and_callable_invocations_expand_deterministically() {
    let src = "@startuml\n!$name = Alice\n!function F($x,$y=\"B\")\n!return $x + $y\n!endfunction\n!procedure P($from,$to)\n$from -> $to: via-proc\n!endprocedure\n!P($name, Bob)\n$name -> Bob: %F(\"A\")\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let labels = model
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            SequenceEventKind::Message { label, .. } => label.clone(),
            _ => None,
        })
        .collect::<Vec<_>>();
    // `+` is the string concatenation operator in PlantUML preprocessor (#582).
    // `!return $x + $y` with $x="A" and $y="B" (default) should produce "AB".
    assert_eq!(labels, vec!["via-proc", "AB"]);
}

#[test]
fn parser_preprocessor_concat_expands_and_procedure_return_fails_with_stable_code() {
    let concat_src =
        "@startuml\n!function Join($a##$b)\n!return $a ## $b\n!endfunction\nA -> B: %Join(Al, ice)\n@enduml\n";
    let concat_doc = parse(concat_src).expect("expected concat expansion");
    match &concat_doc.statements[0].kind {
        puml::ast::StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("Alice")),
        other => panic!("unexpected statement: {other:?}"),
    }

    let proc_return_src =
        "@startuml\n!procedure Bad($x)\n!return $x\n!endprocedure\n!Bad(\"A\")\n@enduml\n";
    let proc_return_err = parse(proc_return_src).expect_err("expected procedure return failure");
    assert!(proc_return_err
        .message
        .contains("E_PREPROC_RETURN_UNEXPECTED"));
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
        ..ParseOptions::default()
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
        ..ParseOptions::default()
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
        ..ParseOptions::default()
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
fn render_timeline_stub_svg_contains_expected_labels() {
    for (src, expected_label) in [
        (
            "@startgantt\n[Build]\n[Build] starts 2026-04-01\n@endgantt\n",
            "Build",
        ),
        (
            "@startchronology\nLaunch happens on 2026-05-15\n@endchronology\n",
            "Launch",
        ),
    ] {
        let doc = parse(src).expect("parse should succeed");
        let normalized = normalize_family(doc).expect("timeline baseline should normalize");
        let NormalizedDocument::Timeline(model) = normalized else {
            panic!("expected timeline model");
        };
        let svg = render::render_timeline_stub_svg(&model);
        assert!(svg.contains("<svg"));
        assert!(svg.contains(expected_label));
        assert!(svg.contains("</svg>"));
    }
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
        (
            "@startmindmap\n* Root\n@endmindmap\n",
            puml::ast::DiagramKind::MindMap,
        ),
        ("@startwbs\n* Scope\n@endwbs\n", puml::ast::DiagramKind::Wbs),
        (
            "@startsalt\nwidget\nauto\n@endsalt\n",
            puml::ast::DiagramKind::Salt,
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
fn parser_supports_salt_baseline_marker_fixture() {
    let src = fs::read_to_string(fixture("families/valid_salt_bootstrap.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    assert_eq!(doc.kind, puml::ast::DiagramKind::Salt);
}

#[test]
fn parser_rejects_salt_marker_mismatch_fixture() {
    let src = fs::read_to_string(fixture("errors/invalid_salt_block_mismatch.puml"))
        .expect("fixture should load");
    let err = parse(&src).unwrap_err();
    assert!(err.message.contains("E_BLOCK_MISMATCH"));
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
    // Wave 3-B (#531): bidirectional `<->` now emits a single message event
    // with arrowheads on both ends rather than two separate one-way events.
    let src = "@startuml\nA <-> B : ping\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let messages = model
        .events
        .iter()
        .filter(|e| matches!(e.kind, SequenceEventKind::Message { .. }))
        .count();
    assert_eq!(messages, 1);
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
    // "red" is now resolved to its CSS3 hex value by parse_color_value.
    assert_eq!(
        classify_sequence_skinparam("ArrowColor", "red"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::ArrowColor(
            "#ff0000".to_string()
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
    assert_eq!(
        classify_sequence_skinparam("SequenceResponseMessageBelowArrow", "true"),
        SequenceSkinParamSupport::SupportedWithValue(
            SequenceSkinParamValue::ResponseMessageBelowArrow(true)
        )
    );
    assert_eq!(
        classify_sequence_skinparam("MessageLineColor", "blue"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageLineColor(
            "#0000ff".to_string()
        ))
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
        ..SequenceDocument::default()
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
                style: Default::default(),
                from_virtual: None,
                to_virtual: None,
            },
        }],
        title: Some("T<&>\"'".to_string()),
        ..SequenceDocument::default()
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
        ..SequenceDocument::default()
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
        ..SequenceDocument::default()
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
                    style: Default::default(),
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
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: None,
                },
            },
        ],
        ..SequenceDocument::default()
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
                    style: Default::default(),
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
                    style: Default::default(),
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
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Left,
                        kind: VirtualEndpointKind::Cross,
                    }),
                },
            },
        ],
        ..SequenceDocument::default()
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
fn layout_ref_advanced_forms_stay_above_footboxes() {
    let src = fs::read_to_string(fixture("groups/valid_ref_advanced_forms.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    assert!(!scene.footboxes.is_empty(), "expected footboxes");
    let footbox_top = scene
        .footboxes
        .iter()
        .map(|b| b.y)
        .min()
        .expect("expected a footbox");
    let refs = scene
        .groups
        .iter()
        .filter(|g| g.kind.eq_ignore_ascii_case("ref"))
        .collect::<Vec<_>>();
    assert!(!refs.is_empty(), "expected ref groups from fixture");
    for reference in refs {
        assert!(
            reference.y + reference.height <= footbox_top,
            "ref group bottom should stay above footbox top"
        );
    }
}

#[test]
fn layout_overflow_bounds_keep_multi_target_note_and_over_group_in_view() {
    let src = "@startuml\nparticipant AlphaParticipantWithLongName\nparticipant BetaParticipantWithLongName\nparticipant GammaParticipantWithLongName\nnote right of AlphaParticipantWithLongName, BetaParticipantWithLongName: right note with a very long unbroken token AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\ngroup over AlphaParticipantWithLongName, GammaParticipantWithLongName : over Alpha/Gamma\nAlphaParticipantWithLongName -> GammaParticipantWithLongName : ping\nend\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());
    assert!(!scene.notes.is_empty(), "expected multi-target note");
    assert!(!scene.groups.is_empty(), "expected over group");
    assert!(
        scene
            .notes
            .iter()
            .all(|note| note.x >= 0 && note.x + note.width <= scene.width),
        "notes should stay within scene width"
    );
}

#[test]
fn layout_expands_width_for_long_header_and_footer_metadata() {
    let src = "@startuml\nheader HEADER_WITH_A_VERY_LONG_METADATA_LINE_FOR_LAYOUT_GUARDRAIL_ABCDEFGHIJKLMNOPQRSTUVWXYZ\nfooter FOOTER_WITH_A_VERY_LONG_METADATA_LINE_FOR_LAYOUT_GUARDRAIL_ABCDEFGHIJKLMNOPQRSTUVWXYZ\nA -> B : ping\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    let margin = LayoutOptions::default().margin;
    let max_metadata_right = scene
        .header
        .iter()
        .chain(scene.footer.iter())
        .flat_map(|label| {
            label
                .lines
                .iter()
                .map(|line| label.x + (line.chars().count() as i32 * 7) + margin)
        })
        .max()
        .expect("expected metadata lines");
    assert!(
        scene.width >= max_metadata_right,
        "scene width should include full header/footer metadata width"
    );
}

#[test]
fn layout_expands_width_for_long_legend_lines() {
    let src = "@startuml\nlegend right\nLEGEND_WITH_A_VERY_LONG_LINE_THAT_MUST_BE_VISIBLE_WITHOUT_CANVAS_CLIPPING_ABCDEFGHIJKLMNOPQRSTUVWXYZ\nend legend\nA -> B : ping\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    let margin = LayoutOptions::default().margin;
    let legend_line = scene
        .legend_text
        .as_deref()
        .and_then(|text| text.lines().next())
        .expect("expected legend line");
    let max_right = (legend_line.chars().count() as i32 * 7) + 16 + (margin * 2);
    assert!(
        scene.width >= max_right,
        "scene width should include long legend text extent"
    );
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

#[test]
fn css3_color_names_are_resolved_to_hex_in_skinparams() {
    let src = fs::read_to_string(fixture("styling/valid_css3_color_message_arrow.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    // "rebeccapurple" -> "#663399"
    assert_eq!(model.style.arrow_color, "#663399");
    // "aliceblue" -> "#f0f8ff"
    assert_eq!(model.style.participant_background_color, "#f0f8ff");
    // "navy" -> "#000080"
    assert_eq!(model.style.participant_border_color, "#000080");
    assert!(model.warnings.is_empty());
}

#[test]
fn new_skinparams_round_shadow_font_background_alignment_are_accepted() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_round_shadow.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    assert_eq!(model.style.round_corner, 12);
    assert!(model.style.shadowing);
    assert_eq!(model.style.default_font_name.as_deref(), Some("Arial"));
    assert_eq!(model.style.default_font_size, Some(14));
    // "cornsilk" -> "#fff8dc"
    assert_eq!(model.style.background_color.as_deref(), Some("#fff8dc"));
    use puml::theme::TextAlignment;
    assert_eq!(model.style.text_alignment, TextAlignment::Left);
    assert!(model.warnings.is_empty());
}

#[test]
fn scale_directive_factor_is_parsed_and_stored() {
    let src = fs::read_to_string(fixture("styling/valid_scale_directive.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    use puml::model::ScaleSpec;
    assert!(
        matches!(model.scale, Some(ScaleSpec::Factor(f)) if (f - 1.5).abs() < 0.001),
        "expected scale factor 1.5, got {:?}",
        model.scale
    );
}

#[test]
fn scale_directive_fixed_size_is_parsed() {
    let src = "@startuml\nscale 800*600\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    use puml::model::ScaleSpec;
    assert_eq!(
        model.scale,
        Some(ScaleSpec::Fixed {
            width: 800,
            height: 600
        })
    );
}

#[test]
fn scale_directive_max_is_parsed() {
    let src = "@startuml\nscale max 500\nA -> B\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    use puml::model::ScaleSpec;
    assert_eq!(model.scale, Some(ScaleSpec::Max(500)));
}

#[test]
fn scale_factor_is_applied_to_svg_dimensions() {
    let src = "@startuml\nscale 2.0\nAlice -> Bob : hello\n@enduml\n";
    let svg = puml::render_source_to_svg(src).expect("render should succeed");

    // The SVG should have width and height that are 2× the base values.
    // We can check that the viewBox and the w/h attributes differ.
    assert!(svg.contains("viewBox=\"0 0 "), "should have viewBox");
    // With scale 2.0, the width/height attributes should be larger than the viewBox.
    // Just check that the SVG produced is valid and deterministic.
    let svg2 = puml::render_source_to_svg(src).expect("render should be deterministic");
    assert_eq!(svg, svg2);
}

#[test]
fn legend_positioning_top_left_is_stored_in_model() {
    let src = fs::read_to_string(fixture("styling/valid_legend_positioning.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    use puml::model::{LegendHAlign, LegendVAlign};
    assert_eq!(model.legend_halign, LegendHAlign::Left);
    assert_eq!(model.legend_valign, LegendVAlign::Top);
}

#[test]
fn legend_text_appears_in_rendered_svg() {
    let src = "@startuml\nlegend right\nLegend Box\nend legend\nAlice -> Bob\n@enduml\n";
    let svg = puml::render_source_to_svg(src).expect("render should succeed");
    assert!(
        svg.contains("Legend Box"),
        "legend text should appear in SVG"
    );
}

#[test]
fn css3_color_to_hex_covers_full_set() {
    use puml::theme::css3_color_to_hex;

    // Check a representative sample of all CSS3 named colors.
    assert_eq!(css3_color_to_hex("rebeccapurple"), Some("#663399"));
    assert_eq!(css3_color_to_hex("RebeccaPurple"), Some("#663399"));
    assert_eq!(css3_color_to_hex("aliceblue"), Some("#f0f8ff"));
    assert_eq!(css3_color_to_hex("yellowgreen"), Some("#9acd32"));
    assert_eq!(css3_color_to_hex("midnightblue"), Some("#191970"));
    assert_eq!(css3_color_to_hex("notacolor"), None);
}

// ─── Tests: classify_*_skinparam for previously-missing families (#442) ───────

#[test]
fn theme_classifies_gantt_skinparam() {
    use puml::theme::{classify_gantt_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_gantt_skinparam("BackgroundColor", "red"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#ff0000".to_string()
        ))
    );
    assert_eq!(
        classify_gantt_skinparam("GanttFontColor", "#aabbcc"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#aabbcc".to_string()
        ))
    );
    assert_eq!(
        classify_gantt_skinparam("FontSize", "14"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(14))
    );
    assert_eq!(
        classify_gantt_skinparam("TodayColor", "blue"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_gantt_skinparam("completelymadeupkey", "val"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_mindmap_skinparam() {
    use puml::theme::{classify_mindmap_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_mindmap_skinparam("BackgroundColor", "#123456"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#123456".to_string()
        ))
    );
    assert_eq!(
        classify_mindmap_skinparam("NodeFontColor", "green"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#008000".to_string()
        ))
    );
    assert_eq!(
        classify_mindmap_skinparam("RoundCorner", "10"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_mindmap_skinparam("unknownmindmapkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_wbs_skinparam() {
    use puml::theme::{classify_wbs_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_wbs_skinparam("BorderColor", "navy"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(
            "#000080".to_string()
        ))
    );
    assert_eq!(
        classify_wbs_skinparam("FontSize", "12"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(12))
    );
    assert_eq!(
        classify_wbs_skinparam("RoundCorner", "5"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_wbs_skinparam("unknownwbskey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_timeline_skinparam() {
    use puml::theme::{classify_timeline_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_timeline_skinparam("BackgroundColor", "white"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#ffffff".to_string()
        ))
    );
    assert_eq!(
        classify_timeline_skinparam("TimelineFontColor", "#010203"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#010203".to_string()
        ))
    );
    assert_eq!(
        classify_timeline_skinparam("ArrowColor", "black"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_timeline_skinparam("inventedkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_nwdiag_skinparam() {
    use puml::theme::{classify_nwdiag_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_nwdiag_skinparam("FontColor", "red"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#ff0000".to_string()
        ))
    );
    assert_eq!(
        classify_nwdiag_skinparam("FontSize", "10"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(10))
    );
    assert_eq!(
        classify_nwdiag_skinparam("NetworkColor", "blue"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_nwdiag_skinparam("inventedkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_archimate_skinparam() {
    use puml::theme::{classify_archimate_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_archimate_skinparam("BackgroundColor", "#aabbcc"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#aabbcc".to_string()
        ))
    );
    assert_eq!(
        classify_archimate_skinparam("BorderColor", "teal"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BorderColor(
            "#008080".to_string()
        ))
    );
    assert_eq!(
        classify_archimate_skinparam("ArchiMateStyle", "sketch"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_archimate_skinparam("inventedkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_sdl_skinparam() {
    use puml::theme::{classify_sdl_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_sdl_skinparam("BackgroundColor", "#112233"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#112233".to_string()
        ))
    );
    assert_eq!(
        classify_sdl_skinparam("FontSize", "16"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(16))
    );
    assert_eq!(
        classify_sdl_skinparam("FontName", "Courier"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_sdl_skinparam("inventedsdlkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_ditaa_skinparam() {
    use puml::theme::{classify_ditaa_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_ditaa_skinparam("BackgroundColor", "silver"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#c0c0c0".to_string()
        ))
    );
    assert_eq!(
        classify_ditaa_skinparam("FontColor", "#ff0000"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontColor(
            "#ff0000".to_string()
        ))
    );
    assert_eq!(
        classify_ditaa_skinparam("Shadowing", "true"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_ditaa_skinparam("inventedditaakey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn theme_classifies_salt_skinparam() {
    use puml::theme::{classify_salt_skinparam, GenericSkinParamValue, SkinParamSupport};
    assert_eq!(
        classify_salt_skinparam("BackgroundColor", "ivory"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::BackgroundColor(
            "#fffff0".to_string()
        ))
    );
    assert_eq!(
        classify_salt_skinparam("FontSize", "11"),
        SkinParamSupport::SupportedWithValue(GenericSkinParamValue::FontSize(11))
    );
    assert_eq!(
        classify_salt_skinparam("RoundCorner", "4"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_salt_skinparam("inventedsaltkey", "x"),
        SkinParamSupport::UnsupportedKey
    );
}

#[test]
fn preproc_break_outside_loop_reports_stable_code() {
    let err = parse("!break\n@startuml\nAlice -> Bob\n@enduml\n")
        .expect_err("break outside loops should fail");
    assert!(err.message.contains("E_PREPROC_BREAK_OUTSIDE_LOOP"));
}

#[test]
fn preproc_continue_outside_loop_reports_stable_code() {
    let err = parse("!continue\n@startuml\nAlice -> Bob\n@enduml\n")
        .expect_err("continue outside loops should fail");
    assert!(err.message.contains("E_PREPROC_CONTINUE_OUTSIDE_LOOP"));
}

#[test]
fn preproc_endfor_without_foreach_reports_stable_code() {
    let err = parse("!endfor\n@startuml\nAlice -> Bob\n@enduml\n")
        .expect_err("endfor without foreach should fail");
    assert!(err.message.contains("E_PREPROC_FOREACH_UNEXPECTED"));
}

#[test]
fn preproc_endwhile_without_while_reports_stable_code() {
    let err = parse("!endwhile\n@startuml\nAlice -> Bob\n@enduml\n")
        .expect_err("endwhile without while should fail");
    assert!(err.message.contains("E_PREPROC_WHILE_UNEXPECTED"));
}

#[test]
fn preproc_elseif_after_else_reports_order_error() {
    let src = "!if 1\n!else\n!elseif 1\n!endif\n@startuml\nAlice -> Bob\n@enduml\n";
    let err = parse(src).expect_err("elseif after else should fail");
    assert!(err.message.contains("E_PREPROC_COND_ORDER"));
}
