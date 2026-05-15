use assert_cmd::Command;
use predicates::prelude::*;
use puml::layout;
use puml::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
};
use puml::normalize;
use puml::parser::{parse_with_options, ParseOptions};
use puml::scene::LayoutOptions;
use puml::{parse, render};
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
fn normalize_reports_destroy_active_for_shortcut() {
    let src = "@startuml\nA -> B++\nA -> B!!\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let err = normalize::normalize(doc).expect_err("expected lifecycle error");
    assert!(err.message.contains("E_LIFECYCLE_DESTROY_ACTIVE"));
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
        footbox_visible: true,
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
            },
        }],
        title: Some("T<&>\"'".to_string()),
        header: None,
        footer: None,
        caption: None,
        legend: None,
        skinparams: vec![],
        footbox_visible: true,
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
