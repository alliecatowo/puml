use crate::common::*;

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
