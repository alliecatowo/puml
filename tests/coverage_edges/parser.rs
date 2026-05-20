use crate::common::*;

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
