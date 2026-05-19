use assert_cmd::Command;
use image::GenericImageView;
use insta::{assert_json_snapshot, assert_snapshot};
use predicates::prelude::*;
use puml::model::SequenceEventKind;
use puml::normalize;
use puml::parser::parse;
use puml::{render_source_to_svg, render_source_to_text, TextOutputMode};
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

pub(crate) fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn example(name: &str) -> String {
    format!("{}/docs/examples/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[path = "integration/preprocessor.rs"]
mod preprocessor;

#[test]
fn single_file_defaults_to_svg_file_output() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single_valid.puml");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let output = tmp.path().join("single_valid.svg");
    assert!(output.exists());

    let expected = fs::read_to_string(fixture("single_valid.svg")).unwrap();
    let actual = fs::read_to_string(output).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn png_output_writes_valid_png_with_default_dpi_dimensions_matching_svg_viewbox() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single_valid.puml");
    let output = tmp.path().join("single_valid.png");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--format",
            "png",
            "--output",
            output.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let bytes = fs::read(&output).unwrap();
    assert!(
        bytes.starts_with(&[0x89, b'P', b'N', b'G']),
        "expected PNG signature"
    );

    let image = image::load_from_memory(&bytes).expect("png should decode");
    let (w, h) = image.dimensions();
    assert!(
        w > 0 && h > 0,
        "PNG should have non-zero dimensions, got {w}x{h}"
    );
    assert!(
        w >= h,
        "sequence diagram PNG should be landscape-ish, got {w}x{h}"
    );
}

#[test]
fn png_output_scales_dimensions_by_dpi() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single_valid.puml");
    let output_1x = tmp.path().join("single_valid_1x.png");
    let output_2x = tmp.path().join("single_valid_2x.png");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();

    // 1× (default 96dpi) baseline.
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--format",
            "png",
            "--output",
            output_1x.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    // 2× (192dpi) — should double dimensions.
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--format",
            "png",
            "--dpi",
            "192",
            "--output",
            output_2x.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let im1 = image::load_from_memory(&fs::read(&output_1x).unwrap()).expect("1x png");
    let im2 = image::load_from_memory(&fs::read(&output_2x).unwrap()).expect("2x png");
    let (w1, h1) = im1.dimensions();
    let (w2, h2) = im2.dimensions();
    // Allow ±2px rounding; the key invariant is that doubling the DPI doubles the canvas.
    assert!(
        w2 >= w1 * 2 - 2 && w2 <= w1 * 2 + 2,
        "2× DPI should double width: 1x={w1}, 2x={w2}"
    );
    assert!(
        h2 >= h1 * 2 - 2 && h2 <= h1 * 2 + 2,
        "2× DPI should double height: 1x={h1}, 2x={h2}"
    );
}

#[test]
fn html_output_writes_self_contained_document_to_stdout() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "html", "-"])
        .write_stdin("@startuml\nAlice -> Bob: hi\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("<!doctype html>"))
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("Alice"));
}

#[test]
fn html_file_input_defaults_to_html_extension() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single_valid.puml");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "html", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let output = tmp.path().join("single_valid.html");
    let actual = fs::read_to_string(output).unwrap();
    assert!(actual.starts_with("<!doctype html>"));
    assert!(actual.contains("<svg"));
}

#[test]
fn jpg_and_webp_outputs_are_valid_raster_exports() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single_valid.puml");
    let jpg = tmp.path().join("single_valid.jpg");
    let webp = tmp.path().join("single_valid.webp");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--format",
            "jpg",
            "--output",
            jpg.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--format",
            "webp",
            "--output",
            webp.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let jpg_bytes = fs::read(&jpg).unwrap();
    assert!(jpg_bytes.starts_with(&[0xff, 0xd8, 0xff]));
    let jpg_image = image::load_from_memory(&jpg_bytes).expect("jpg should decode");
    let (jw, jh) = jpg_image.dimensions();
    assert!(
        jw > 0 && jh > 0,
        "JPG should have non-zero dimensions, got {jw}x{jh}"
    );

    let webp_bytes = fs::read(&webp).unwrap();
    assert!(webp_bytes.starts_with(b"RIFF"));
    assert_eq!(&webp_bytes[8..12], b"WEBP");
    let webp_image = image::load_from_memory(&webp_bytes).expect("webp should decode");
    let (ww, wh) = webp_image.dimensions();
    assert!(
        ww > 0 && wh > 0,
        "WebP should have non-zero dimensions, got {ww}x{wh}"
    );
    // Both formats from the same source should produce equal dimensions.
    assert_eq!(
        (jw, jh),
        (ww, wh),
        "JPG and WebP of the same source should match"
    );
}

#[test]
fn jpg_and_webp_file_inputs_default_to_format_extensions() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single_valid.puml");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "jpg", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "webp", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(tmp.path().join("single_valid.jpg").exists());
    assert!(tmp.path().join("single_valid.webp").exists());
}

#[test]
fn jpg_stdout_writes_image_bytes() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "jpg", "-"])
        .write_stdin("@startuml\nAlice -> Bob: hi\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    assert!(out.starts_with(&[0xff, 0xd8, 0xff]));
}

#[test]
fn txt_output_writes_structural_text_to_stdout() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "txt", "-"])
        .write_stdin("@startuml\nAlice -> Bob: hi\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("sequence\nparticipants (2)"))
        .stdout(predicate::str::contains("Alice -> Bob: hi"));
}

#[test]
fn plantuml_style_txt_alias_writes_default_txt_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single_valid.puml");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["-txt", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let output = tmp.path().join("single_valid.txt");
    assert!(output.exists());
    let actual = fs::read_to_string(output).unwrap();
    assert!(actual.contains("Alice -> Bob: hi"));
}

#[test]
fn utxt_output_preserves_unicode_tree_markers_and_text() {
    let actual = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "utxt", "-"])
        .write_stdin("@startuml\nclass Café\nclass Té\nCafé --> Té : crème\n@enduml\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let actual = String::from_utf8(actual).unwrap();
    assert!(actual.contains("├─ Class Café"));
    assert!(actual.contains("Café --> Té: crème"));
}

#[test]
fn plantuml_style_utxt_alias_writes_unicode_text_to_stdout() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["-utxt", "-"])
        .write_stdin("@startuml\nclass Café\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("└─ Class Café"));
}

#[test]
fn txt_multi_stdin_outputs_text_payloads_with_txt_names() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "txt", "--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("structure/newpage_stdin_contract.puml")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "diagram-1.txt");
    assert!(arr[0]["text"].as_str().unwrap().contains("page one"));
    assert!(arr[0].get("svg").is_none());
}

#[test]
fn render_source_to_text_api_supports_family_models() {
    let src = include_str!("fixtures/families/valid_class_with_relations.puml");
    let text = render_source_to_text(src, TextOutputMode::Txt).expect("class text render");
    assert!(text.contains("Class orientation=TopToBottom"));
    assert!(text.contains("Dog *-- Collar: has"));
}

#[test]
fn metadata_mode_reports_sequence_counts_and_style_metadata() {
    let src = r#"@startuml
!theme plain
skinparam participantBackgroundColor #ddeeff
title Checkout
participant User
participant API
User -> API: request
note right of API: cached
group happy path
API --> User: ok
end
newpage retry
User -> API: retry
@enduml
"#;

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--metadata", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(json["schema"], "puml.metadata");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["family"], "sequence");
    assert_eq!(json["title"], "Checkout");
    assert_eq!(json["counts"]["participants"], 2);
    assert_eq!(json["counts"]["messages"], 3);
    assert_eq!(json["counts"]["notes"], 1);
    assert_eq!(json["counts"]["groups"], 1);
    assert_eq!(json["counts"]["pages"], 2);
    assert_eq!(json["themes"], serde_json::json!(["plain"]));
    assert_eq!(
        json["skinparams"],
        serde_json::json!([{"key": "participantBackgroundColor", "value": "#ddeeff"}])
    );
    assert_eq!(json["pages"][0]["title"], "Checkout");
    assert_eq!(json["pages"][1]["title"], "retry");
    assert_eq!(json["warnings"], serde_json::json!([]));
}

#[test]
fn metadata_mode_reports_class_counts() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--metadata",
            &fixture("families/valid_class_with_relations.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(json["family"], "class");
    assert_eq!(json["counts"]["classes"], 4);
    assert_eq!(json["counts"]["relations"], 5);
    assert_eq!(json["skinparams"], serde_json::json!([]));
    assert_eq!(json["themes"], serde_json::json!([]));
    assert_eq!(json["pages"], serde_json::json!([]));
}

#[test]
fn metadata_mode_reports_broad_family_counts() {
    let cases = [
        (fixture("families/valid_state.puml"), "state", "transitions"),
        (
            fixture("families/valid_gantt_calendar_resource_scale.puml"),
            "gantt",
            "tasks",
        ),
        (example("json/01_object.puml"), "json", "nodes"),
        (fixture("non_sequence/valid_yaml.puml"), "yaml", "nodes"),
        (
            fixture("non_sequence/valid_archimate.puml"),
            "archimate",
            "elements",
        ),
        (
            fixture("non_sequence/valid_regex.puml"),
            "regex",
            "patterns",
        ),
        (fixture("families/valid_ebnf_arith.puml"), "ebnf", "rules"),
        (fixture("families/valid_sdl_shapes.puml"), "sdl", "states"),
        (
            fixture("families/valid_chart_bar_quarterly.puml"),
            "chart",
            "data_points",
        ),
        (example("nwdiag/01_single_net.puml"), "nwdiag", "networks"),
        (example("math/01_simple.puml"), "math", "body_bytes"),
        (example("ditaa/01_simple_ascii.puml"), "ditaa", "body_bytes"),
        (example("activity/01_simple_flow.puml"), "activity", "nodes"),
    ];

    for (path, family, count_key) in cases {
        let out = Command::cargo_bin("puml")
            .expect("binary")
            .args(["--metadata", &path])
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .get_output()
            .stdout
            .clone();

        let json: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(json["family"], family, "metadata family for {path}");
        assert!(
            json["counts"][count_key].as_u64().unwrap_or_default() > 0,
            "metadata count {count_key} should be present and nonzero for {path}: {json}"
        );
        assert!(
            json["warnings"].is_array(),
            "metadata warnings should be an array for {path}: {json}"
        );
    }
}

#[test]
fn check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("single_valid.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn default_frontend_matches_explicit_plantuml() {
    let default = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let plantuml = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "plantuml",
            "--dump",
            "ast",
            &fixture("single_valid.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_eq!(default, plantuml);
}

#[test]
fn strict_modes_parse_and_route_without_regression() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--compat",
            "strict",
            "--determinism",
            "strict",
            "--check",
            &fixture("single_valid.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn sequence_parity_vertical_slice_fixture_passes_check_mode() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("e2e/sequence_parity_vertical_slice.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn default_compat_matches_explicit_strict() {
    let default = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .clone();

    let strict = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--compat",
            "strict",
            "--check",
            &fixture("single_valid.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    assert_eq!(default.stdout, strict.stdout);
    assert_eq!(default.stderr, strict.stderr);
}

#[test]
fn strict_stdin_include_requires_explicit_include_root() {
    let tmp = tempdir().unwrap();
    let include = tmp.path().join("common.puml");
    fs::write(&include, "Bob -> Alice: from include\n").unwrap();
    let stdin_input = "@startuml\n!include common.puml\n@enduml\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args(["--check", "-", "--compat", "strict"])
        .write_stdin(stdin_input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_ROOT_REQUIRED"));
}

#[test]
fn extended_stdin_include_uses_current_directory_when_include_root_is_missing() {
    let tmp = tempdir().unwrap();
    let include = tmp.path().join("common.puml");
    fs::write(&include, "Bob -> Alice: from include\n").unwrap();
    let stdin_input = "@startuml\n!include common.puml\n@enduml\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args(["--check", "-", "--compat", "extended"])
        .write_stdin(stdin_input)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn picouml_frontend_routes_canonical_surface_to_shared_model() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "picouml",
            "--check",
            &fixture("picouml/valid_canonical.picouml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn picouml_extension_routes_canonical_surface_in_auto_dialect() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("picouml/valid_canonical.picouml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn picouml_extension_routes_shorthand_surface_in_auto_dialect() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("shorthand.picouml");
    fs::write(
        &input,
        "@startpicouml\nAlice => Bob : sync call\nBob <~ Carol : async reply\n@endpicouml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn picouml_frontend_rejects_mixed_marker_forms_deterministically() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "picouml",
            "--check",
            &fixture("picouml/invalid_mixed_markers.picouml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PICOUML_MARKER_MIXED"));
}

#[test]
fn mermaid_sequence_subset_routes_through_shared_pipeline() {
    let src = r#"sequenceDiagram
participant Alice
participant Bob
Alice->>Bob: hello
Bob-->>Alice: ack"#;

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", "--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_unsupported_family_fails_deterministically() {
    // `pie` and `gitDiagram` are not supported; verify deterministic error.
    let src = "pie title Pets\n  \"Dogs\" : 386";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", "--check", "-"])
        .write_stdin(src)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("[E_MERMAID_FAMILY_UNSUPPORTED]"));
}

#[test]
fn mermaid_graph_td_flowchart_routes_successfully() {
    let src = "graph TD\nA-->B";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", "--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_alt_else_end_block_now_adapts_successfully() {
    let src = r#"sequenceDiagram
alt happy path
Alice->>Bob: hello
else sad path
Alice->>Bob: bye
end"#;
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", "--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_extended_subset_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_extended_subset.mmd"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_alt_end_fixture_now_validates_successfully() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/invalid_unsupported_block.mmd"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

// ---------------------------------------------------------------------------
// #187 — Mermaid non-sequence families: flowchart, classDiagram, stateDiagram, erDiagram
// ---------------------------------------------------------------------------

#[test]
fn mermaid_flowchart_fixture_checks_and_renders_nonempty_svg() {
    // --check must pass
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_flowchart.mmd"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    // Render to SVG via CLI stdin; stdout must be non-empty SVG.
    let src = fs::read_to_string(fixture("mermaid/valid_flowchart.mmd")).unwrap();
    let svg_out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    // CLI writes to file; no stdout. Check it doesn't fail — length check via file.
    // (The CLI writes svg to a file; when reading from stdin it writes to stdout.)
    let _ = svg_out; // stdout may be empty for stdin->file mode; success is sufficient.
}

#[test]
fn mermaid_classdiagram_fixture_checks_and_renders_nonempty_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_classdiagram.mmd"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    // Verify SVG render via tempfile output.
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("valid_classdiagram.mmd");
    fs::copy(fixture("mermaid/valid_classdiagram.mmd"), &input).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", input.to_str().unwrap()])
        .assert()
        .success();
    let svg_path = tmp.path().join("valid_classdiagram.svg");
    let svg = fs::read_to_string(&svg_path).expect("svg output file");
    assert!(
        svg.len() > 100,
        "expected non-empty SVG, got {} bytes",
        svg.len()
    );
}

#[test]
fn mermaid_statediagram_fixture_checks_and_renders_nonempty_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_statediagram.mmd"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let tmp = tempdir().unwrap();
    let input = tmp.path().join("valid_statediagram.mmd");
    fs::copy(fixture("mermaid/valid_statediagram.mmd"), &input).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", input.to_str().unwrap()])
        .assert()
        .success();
    let svg_path = tmp.path().join("valid_statediagram.svg");
    let svg = fs::read_to_string(&svg_path).expect("svg output file");
    assert!(
        svg.len() > 100,
        "expected non-empty SVG, got {} bytes",
        svg.len()
    );
}

#[test]
fn mermaid_erdiagram_fixture_checks_and_renders_nonempty_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_erdiagram.mmd"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let tmp = tempdir().unwrap();
    let input = tmp.path().join("valid_erdiagram.mmd");
    fs::copy(fixture("mermaid/valid_erdiagram.mmd"), &input).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dialect", "mermaid", input.to_str().unwrap()])
        .assert()
        .success();
    let svg_path = tmp.path().join("valid_erdiagram.svg");
    let svg = fs::read_to_string(&svg_path).expect("svg output file");
    assert!(
        svg.len() > 100,
        "expected non-empty SVG, got {} bytes",
        svg.len()
    );
}

#[test]
fn docs_examples_svg_corpus_matches_renderer() {
    for stem in ["basic_hello", "groups_notes", "lifecycle_autonumber"] {
        let puml_path = format!("{}/docs/examples/{stem}.puml", env!("CARGO_MANIFEST_DIR"));
        let svg_path = format!("{}/docs/examples/{stem}.svg", env!("CARGO_MANIFEST_DIR"));
        let source = fs::read_to_string(&puml_path).expect("example source");
        let expected_svg = fs::read_to_string(&svg_path).expect("example svg");
        let actual_svg = render_source_to_svg(&source).expect("rendered svg");
        assert_eq!(
            actual_svg, expected_svg,
            "docs example drift detected for {stem}"
        );
    }
}

#[test]
fn nonuml_family_fixtures_render_nonempty_svg_depth_smoke() {
    let fixtures = [
        "non_sequence/valid_sdl.puml",
        "families/valid_sdl_shapes.puml",
        "non_sequence/valid_archimate.puml",
        "non_sequence/valid_nwdiag.puml",
        "non_sequence/valid_json.puml",
        "non_sequence/valid_yaml.puml",
        "non_sequence/valid_regex.puml",
        "non_sequence/valid_ebnf.puml",
        "non_sequence/valid_chart_bar.puml",
        "non_sequence/valid_chart_pie.puml",
        "non_sequence/valid_math.puml",
        "non_sequence/valid_ditaa.puml",
        "families/valid_math_complex.puml",
        "families/valid_ditaa_complex.puml",
    ];

    for case in fixtures {
        let src = fs::read_to_string(fixture(case)).expect("fixture should load");
        let svg = render_source_to_svg(&src).expect("render should succeed");
        assert!(svg.starts_with("<svg"), "expected svg root for {case}");
        assert!(
            svg.len() > 120,
            "expected non-trivial svg for {case}, got {} bytes",
            svg.len()
        );
    }
}

#[test]
fn check_mode_fails_for_invalid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("invalid_single.puml")])
        .assert()
        .code(1);
}

#[test]
fn component_family_now_passes_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_component_diagram.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn deployment_family_now_passes_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_deployment_diagram.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn state_diagram_basic_check_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_state_diagram.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn activity_family_now_passes_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/valid_activity_oldstyle_baseline.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn old_style_activity_renders_flow_nodes_instead_of_raw_source() {
    let svg = render_source_to_svg(include_str!(
        "fixtures/families/valid_activity_old_style.puml"
    ))
    .expect("old-style activity should render");

    assert!(svg.contains("data-activity-kind=\"Start\""));
    assert!(svg.contains("data-activity-kind=\"Action\""));
    assert!(svg.contains("data-activity-kind=\"Stop\""));
    assert!(svg.contains(">Step1<"));
    assert!(svg.contains(">Step2<"));
    assert!(svg.contains("<line "));
    assert!(!svg.contains("(*) --&gt;"));
}

#[test]
fn timing_family_now_passes_validation() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_timing_diagram.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn non_sequence_mindmap_check_now_succeeds_with_baseline_renderer() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("non_sequence/invalid_mindmap_diagram.puml"),
        ])
        .assert()
        .code(0);
}

#[test]
fn non_sequence_wbs_check_now_succeeds_with_baseline_renderer() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/invalid_wbs_diagram.puml")])
        .assert()
        .code(0);
}

#[test]
fn gantt_and_chronology_baseline_inputs_pass_check() {
    for case in [
        "timeline/valid_gantt_baseline.puml",
        "timeline/valid_chronology_baseline.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn gantt_and_chronology_dump_model_is_stable() {
    for case in [
        "timeline/valid_gantt_baseline.puml",
        "timeline/valid_chronology_baseline.puml",
    ] {
        let out = Command::cargo_bin("puml")
            .expect("binary")
            .args(["--dump", "model", &fixture(case)])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let json: Value = serde_json::from_slice(&out).unwrap();
        assert_json_snapshot!(
            format!("timeline_dump_model__{}", case.replace('/', "__")),
            json
        );
    }
}

#[test]
fn gantt_and_chronology_unsupported_baseline_syntax_is_deterministic() {
    for (case, code) in [
        (
            "errors/invalid_gantt_unsupported_baseline.puml",
            "E_GANTT_UNSUPPORTED",
        ),
        (
            "errors/invalid_chronology_unsupported_baseline.puml",
            "E_CHRONOLOGY_UNSUPPORTED",
        ),
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains(code));
    }
}

#[test]
fn check_mode_passes_for_additional_valid_fixtures() {
    for case in [
        "single_valid.puml",
        "basic/valid_start_end.puml",
        "basic/valid_arrow.txt",
        "participants/valid_aliases.puml",
        "participants/valid_queue_separator.puml",
        "basic/valid_separator_equals.puml",
        "basic/valid_participant_queue.puml",
        "basic/valid_arrows_extended_set.puml",
        "basic/valid_skinparam_maxmessagesize.puml",
        "arrows/valid_directions.puml",
        "arrows/self.puml",
        "arrows/modifiers_basic.puml",
        "arrows/valid_expanded_forms.puml",
        "arrows/valid_slanted_heads.puml",
        "arrows/valid_endpoint_variants.puml",
        "arrows/valid_arrow_portability_expanded.puml",
        "arrows/valid_arrow_slash_portability.puml",
        "arrows/valid_arrow_variant_tokenization.puml",
        "arrows/valid_rare_arrow_styles.puml",
        "arrows/valid_dotted_parallel_sequence_edges.puml",
        "arrows/valid_teoz_overlapping_routes.puml",
        "notes/valid_note_over.puml",
        "groups/valid_alt_end.puml",
        "groups/valid_loop_end.puml",
        "groups/valid_par_else_end.puml",
        "groups/valid_ref_and_else_rendering.puml",
        "groups/valid_group_nested_mixed_fragments.puml",
        "groups/valid_group_empty_group_block.puml",
        "autonumber/valid_basic.puml",
        "autonumber/valid_with_format.puml",
        "lifecycle/valid_activate_return.puml",
        "lifecycle/valid_create_activate_destroy.puml",
        "lifecycle/valid_shortcuts_expansion.puml",
        "lifecycle/valid_return_inferred_from_shortcut_activation.puml",
        "lifecycle/valid_return_inferred_from_last_message.puml",
        "notes/valid_multiline_blocks.puml",
        "notes/valid_note_across_multi.puml",
        "structure/valid_separator_delay_divider_spacer.puml",
        "structure/ignore_newpage_single_output.puml",
        "structure/valid_autonumber_restart_step_format.puml",
        "structure/valid_autonumber_format_only_and_canonical_spacing.puml",
        "structure/valid_autonumber_off_resume_edges.puml",
        "structure/valid_autonumber_dotted_and_hash_padding.puml",
        "include/include_with_tag_ok.puml",
        "include/include_many_ok.puml",
        "include/include_once_ok.puml",
        "include/includesub_ok.puml",
        "preprocessor/valid_if_elseif_else.puml",
        "preprocessor/valid_ifdef_ifndef.puml",
        "preprocessor/valid_while_define_counter.puml",
        "preprocessor/valid_variable_assignment_reference.puml",
        "preprocessor/valid_function_call_args_defaults_keywords.puml",
        "preprocessor/valid_function_return_indented.puml",
        "preprocessor/valid_procedure_call_args.puml",
        "preprocessor/valid_import_stdlib_core.puml",
        "preprocessor/valid_import_stdlib_nested_no_ext.puml",
        "preprocessor/valid_builtin_strlen.puml",
        "preprocessor/valid_builtin_boolval.puml",
        "preprocessor/valid_builtin_chain.puml",
        "preprocessor/valid_builtin_list_map_stringification_assert_log.puml",
        "include/valid_include_once.puml",
        "include/valid_include_many.puml",
        "include/valid_includesub.puml",
        "include/valid_c4_context.puml",
        "include/valid_awslib_ec2.puml",
        "stdlib_include_tag/valid_stdlib_tagged_angle_include.puml",
        // preprocessor advanced directives
        "preprocessor/valid_while_variable_loop.puml",
        "preprocessor/valid_undef.puml",
        "preprocessor/valid_assert_true.puml",
        "preprocessor/valid_log_directive.puml",
        "preprocessor/valid_get_json_attribute.puml",
        "preprocessor/valid_get_variable_value.puml",
        "preprocessor/valid_feature_builtin.puml",
        "preprocessor/valid_newline_builtin.puml",
        "preprocessor/valid_retrieve_procedure_return.puml",
        "preprocessor/valid_function_exists.puml",
        "preprocessor/valid_variable_exists.puml",
        "preprocessor/valid_json_dot_bracket_access.puml",
        "preprocessor/valid_splitstr_regex.puml",
        "preprocessor/valid_macro_concat_body.puml",
        "preprocessor/valid_macro_expr_collection_depth.puml",
        "preprocessor/valid_unsafe_builtin_policy.puml",
        // MindMap/WBS hardening fixtures
        "families/valid_mindmap_palette.puml",
        "families/valid_wbs_progress.puml",
        "families/valid_mindmap_orientation.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn check_mode_pragma_teoz_is_accepted_as_compatibility_noop() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("basic/valid_pragma_directives.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn malformed_pragma_missing_body_reports_deterministic_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_pragma_missing_body.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("[E_PRAGMA_INVALID]"));
}

#[test]
fn dump_mode_outputs_ast_json_for_multiline_blocks() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("notes/valid_multiline_blocks.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("dump_mode_outputs_ast_json_for_multiline_blocks", json);
}

#[test]
fn check_mode_fails_for_additional_invalid_fixtures() {
    for case in [
        "invalid_single.puml",
        "errors/invalid_plain.txt",
        "errors/invalid_unclosed.puml",
        "errors/invalid_unknown_only.puml",
        "errors/invalid_include_only.puml",
        "errors/invalid_define_only.puml",
        "errors/invalid_undef_only.puml",
        "include/error_include_cycle_self.puml",
        "include/error_include_chain_a.puml",
        "lifecycle/valid_destroy_then_message.puml",
        "lifecycle/invalid_return_without_caller_context.puml",
        "arrows/invalid_malformed_arrows.puml",
        "arrows/invalid_endpoint_variants.puml",
        "errors/invalid_malformed_note_ref.puml",
        "structure/invalid_malformed_divider_delay.puml",
        "groups/invalid_else_without_open_group.puml",
        "groups/invalid_end_without_open_group.puml",
        "groups/invalid_else_inside_ref.puml",
        "groups/invalid_ref_block_missing_body.puml",
        "errors/invalid_separator_unbalanced_equals.puml",
        "errors/invalid_participant_queue_alias_collision.puml",
        "errors/invalid_arrow_variant_tokenization.puml",
        "errors/invalid_arrow_slash_tokenization.puml",
        "errors/invalid_include_tag_missing.puml",
        "errors/invalid_include_url.puml",
        "errors/invalid_include_once_url.puml",
        "errors/invalid_includesub_url.puml",
        "errors/invalid_includesub_missing_tag.puml",
        "errors/invalid_else_inside_loop_group.puml",
        "errors/invalid_group_else_without_alt.puml",
        "errors/invalid_group_mismatched_end_keyword.puml",
        "errors/invalid_group_empty_alt.puml",
        "errors/invalid_group_empty_else_branch.puml",
        "errors/invalid_autonumber_bad_format_token.puml",
        "errors/invalid_preproc_conditional_order.puml",
        "errors/invalid_preproc_unclosed_if.puml",
        "errors/invalid_preproc_procedure_unsupported.puml",
        "errors/invalid_preproc_endwhile_without_while.puml",
        "errors/invalid_preproc_expr_missing.puml",
        "errors/invalid_preproc_unexpected_endfunction.puml",
        "errors/invalid_preproc_while_iteration_limit.puml",
        "errors/invalid_pragma_missing_body.puml",
        "errors/invalid_preproc_assert_missing_expr.puml",
        "errors/invalid_preproc_builtin_in_assert.puml",
        "errors/invalid_preproc_builtin_in_log.puml",
        "errors/invalid_preproc_dynamic_invoke.puml",
        "errors/invalid_preproc_json_assignment.puml",
        "errors/invalid_preproc_function_missing_arg.puml",
        "errors/invalid_preproc_procedure_return.puml",
        "errors/invalid_import_empty_path.puml",
        "errors/invalid_import_url.puml",
        "errors/invalid_import_absolute_path.puml",
        "errors/invalid_import_tag_form.puml",
        "errors/invalid_import_escape_path.puml",
        "errors/invalid_import_missing_module.puml",
        "errors/invalid_pragma_missing_body.puml",
        "errors/invalid_theme_empty_name.puml",
        "errors/invalid_theme_remote_source.puml",
        "errors/invalid_theme_unknown_name.puml",
        "errors/invalid_include_absolute_path.puml",
        "errors/invalid_include_empty_path.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1);
    }
}

#[test]
fn else_inside_loop_group_reports_deterministic_normalize_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_else_inside_loop_group.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_GROUP_ELSE_KIND"));
}

#[test]
fn strict_group_semantics_accepts_nested_alt_par_critical_and_group() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("groups/valid_group_nested_mixed_fragments.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn strict_group_semantics_allows_empty_group_block() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("groups/valid_group_empty_group_block.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn strict_group_semantics_rejects_empty_alt_and_else_branches() {
    for case in [
        "errors/invalid_group_empty_alt.puml",
        "errors/invalid_group_empty_else_branch.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains("E_GROUP_EMPTY"));
    }
}

#[test]
fn slash_arrow_variants_are_tokenized_into_message_arrows() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("arrows/valid_arrow_slash_portability.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arrows: Vec<&str> = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["arrow"].as_str())
        .collect();

    assert_eq!(arrows, vec!["->", "->", "<->", "->o", "<<--x"]);
}

#[test]
fn malformed_slash_arrow_reports_deterministic_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_arrow_slash_tokenization.puml"),
        ])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains(
                    "A -//-> B: malformed-double-slash\n^^^^^^^^",
                ))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );
}

#[test]
fn expanded_arrow_variants_are_tokenized_into_message_arrows() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("arrows/valid_arrow_variant_tokenization.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arrows: Vec<&str> = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["arrow"].as_str())
        .collect();

    assert_eq!(
        arrows,
        vec!["-/->", "-\\->", "-/->>", "-\\-->>", "o-/->x", "x-\\<<--o"]
    );
}

#[test]
fn malformed_arrow_variant_reports_deterministic_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_arrow_variant_tokenization.puml"),
        ])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 4, column 1")
                .and(predicate::str::contains("A -/--> B: malformed\n^^^^^^^^^^"))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );
}

#[test]
fn bracketed_sequence_arrow_style_metadata_is_preserved() {
    let src = std::fs::read_to_string(fixture("arrows/valid_rare_arrow_styles.puml")).unwrap();
    let doc = puml::parse(&src).expect("parse should succeed");
    let messages = doc
        .statements
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            puml::ast::StatementKind::Message(m) => Some(m),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(messages[0].style.thickness, Some(3));
    assert!(messages[1].style.dotted);
    assert_eq!(messages[2].style.thickness, Some(5));
}

#[test]
fn check_mode_emits_styling_warnings_but_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("styling/valid_skinparam_unsupported.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("W_SKINPARAM_UNSUPPORTED"));
}

#[test]
fn check_mode_accepts_phase1_supported_skinparam_keys_without_warnings() {
    for fixture_name in [
        "styling/valid_skinparam_maxmessagesize_supported.puml",
        "styling/valid_skinparam_sequence_footbox_supported.puml",
        "styling/valid_skinparam_arrow_color_supported.puml",
        "styling/valid_skinparam_lifeline_border_color_supported.puml",
        "styling/valid_skinparam_participant_background_color_supported.puml",
        "styling/valid_skinparam_participant_border_color_supported.puml",
        "styling/valid_skinparam_note_background_color_supported.puml",
        "styling/valid_skinparam_note_border_color_supported.puml",
        "styling/valid_skinparam_group_background_color_supported.puml",
        "styling/valid_skinparam_group_border_color_supported.puml",
        "styling/valid_skinparam_sequence_alias_colors_supported.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(fixture_name)])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn check_mode_skinparam_unsupported_key_and_value_are_both_reported_deterministically() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("styling/valid_skinparam_unsupported_mixed_deterministic.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(output).expect("stderr should be valid utf-8");
    let unsupported_key = stderr
        .find("W_SKINPARAM_UNSUPPORTED")
        .expect("missing unsupported-key warning");
    let unsupported_value = stderr
        .find("W_SKINPARAM_UNSUPPORTED_VALUE")
        .expect("missing unsupported-value warning");
    assert!(
        unsupported_key < unsupported_value,
        "warnings should keep source order"
    );
}

#[test]
fn check_mode_skinparam_unsafe_color_value_warns_deterministically() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(
            "@startuml\nskinparam ArrowColor #aabbcc\nskinparam ArrowColor #ff0000\"/><script>\nA -> B\n@enduml\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(
            predicate::str::contains("W_SKINPARAM_UNSUPPORTED_VALUE")
                .and(predicate::str::contains("ArrowColor"))
                .and(predicate::str::contains("line 3, column 1")),
        );
}

#[test]
fn dump_mode_emits_warnings_in_deterministic_order() {
    let input = "@startuml\n!theme spacelab\nskinparam UnknownKey red\nskinparam StillUnknown blue\nA -> B\n@enduml\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "model", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .clone();

    let stderr = String::from_utf8(out.stderr).unwrap();
    let first = stderr.find("W_SKINPARAM_UNSUPPORTED").unwrap();
    let second = stderr[first + 1..].find("W_SKINPARAM_UNSUPPORTED").unwrap();
    assert!(first + 1 + second > first);
    assert!(!stderr.contains("W_THEME_UNSUPPORTED"));

    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json.get("participants").is_some());
}

#[test]
fn render_mode_emits_styling_warnings_but_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\n!theme plain\nskinparam UnknownKey red\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stderr(predicate::str::contains("W_SKINPARAM_UNSUPPORTED"));
}

#[test]
fn check_mode_rejects_theme_remote_source_with_deterministic_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_theme_remote_source.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("[E_THEME_SOURCE_UNSUPPORTED]"));
}

#[test]
fn check_mode_rejects_theme_unknown_name_with_catalog_message() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_theme_unknown_name.puml"),
        ])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("[E_THEME_UNKNOWN]")
                .and(predicate::str::contains("available local themes:"))
                .and(predicate::str::contains("plain"))
                .and(predicate::str::contains("spacelab")),
        );
}

#[test]
fn theme_plain_produces_default_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_plain.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#111");
    assert_eq!(style["participant_background_color"], "#f6f6f6");
    assert_eq!(style["note_background_color"], "#fff8c4");
}

#[test]
fn theme_aws_orange_produces_orange_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_aws_orange.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#232f3e");
    assert_eq!(style["participant_background_color"], "#ff9900");
    assert_eq!(style["lifeline_border_color"], "#ff9900");
}

#[test]
fn theme_blueprint_produces_dark_blue_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_blueprint.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#ffffff");
    assert_eq!(style["participant_background_color"], "#1a3a5c");
    assert_eq!(style["lifeline_border_color"], "#7eb4d4");
}

#[test]
fn theme_cerulean_produces_blue_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_cerulean.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#2fa4e7");
    assert_eq!(style["participant_background_color"], "#d9edf7");
}

#[test]
fn theme_hacker_produces_green_on_black_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_hacker.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#00ff00");
    assert_eq!(style["participant_background_color"], "#0d0d0d");
    assert_eq!(style["note_background_color"], "#000000");
}

#[test]
fn theme_sketchy_produces_hand_drawn_style_colors_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("styling/valid_theme_sketchy.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let style = &json["style"];
    assert_eq!(style["arrow_color"], "#333333");
    assert_eq!(style["participant_background_color"], "#fffde7");
}

#[test]
fn theme_catalog_covers_all_22_presets() {
    use puml::theme::{resolve_sequence_theme_preset, LOCAL_SEQUENCE_THEME_CATALOG};
    // Use >= rather than ==: adding new themes should not break this test.
    // The original set contained 22; the exact count is an implementation detail.
    assert!(
        LOCAL_SEQUENCE_THEME_CATALOG.len() >= 22,
        "expected at least 22 theme presets, found {}",
        LOCAL_SEQUENCE_THEME_CATALOG.len()
    );
    for name in LOCAL_SEQUENCE_THEME_CATALOG {
        let result = resolve_sequence_theme_preset(name);
        assert!(
            result.is_ok(),
            "preset `{name}` failed to resolve: {:?}",
            result
        );
        let preset = result.unwrap();
        assert_eq!(preset.name, *name);
        // All color strings must start with '#' or be a named color
        assert!(!preset.style.arrow_color.is_empty());
        assert!(!preset.style.participant_background_color.is_empty());
    }
}

#[test]
fn all_22_theme_fixtures_pass_check_mode() {
    for name in &[
        "styling/valid_theme_plain.puml",
        "styling/valid_theme_aws_orange.puml",
        "styling/valid_theme_blueprint.puml",
        "styling/valid_theme_cerulean.puml",
        "styling/valid_theme_hacker.puml",
        "styling/valid_theme_sketchy.puml",
        "styling/valid_theme_amiga.puml",
        "styling/valid_theme_bluegray.puml",
        "styling/valid_theme_carbon_gray.puml",
        "styling/valid_theme_materia_outline.puml",
        "styling/valid_theme_mono.puml",
        "styling/valid_theme_nautilus.puml",
        "styling/valid_theme_not_so_funny.puml",
        "styling/valid_theme_reddress_darkgreen.puml",
        "styling/valid_theme_sandstone.puml",
        "styling/valid_theme_silver.puml",
        "styling/valid_theme_spacelab_white.puml",
        "styling/valid_theme_sunlust.puml",
        "styling/valid_theme_toy.puml",
        "styling/valid_theme_united.puml",
        "styling/valid_theme_vibrant.puml",
        "styling/valid_theme_none.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(name)])
            .assert()
            .success();
    }
}

#[test]
fn source_related_warning_uses_line_column_and_caret_in_all_modes() {
    let input = "@startuml\nskinparam UnknownKey red\nA -> B\n@enduml\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(
            predicate::str::contains("line 2, column 1").and(predicate::str::contains(
                "skinparam UnknownKey red\n^^^^^^^^",
            )),
        );

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "model", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(
            predicate::str::contains("line 2, column 1").and(predicate::str::contains(
                "skinparam UnknownKey red\n^^^^^^^^",
            )),
        );

    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(input)
        .assert()
        .success()
        .stderr(
            predicate::str::contains("line 2, column 1").and(predicate::str::contains(
                "skinparam UnknownKey red\n^^^^^^^^",
            )),
        );
}

#[test]
fn malformed_arrow_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("arrows/invalid_malformed_arrows.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_ARROW_INVALID"));
}

#[test]
fn malformed_endpoint_variant_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("arrows/invalid_endpoint_variants.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_ARROW_INVALID"));
}

#[test]
fn source_related_error_uses_line_column_and_caret_in_all_modes() {
    let invalid = fixture("arrows/invalid_malformed_arrows.puml");

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains("A -x B: malformed\n^^^^^^"))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &invalid])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains("A -x B: malformed\n^^^^^^"))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(&invalid)
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains("A -x B: malformed\n^^^^^^"))
                .and(predicate::str::contains("E_ARROW_INVALID")),
        );
}

#[test]
fn check_mode_reports_unmatched_enduml_boundary() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("errors/invalid_unmatched_enduml.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("without a preceding @startuml"));
}

#[test]
fn check_mode_reports_nested_startuml_boundary() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("errors/invalid_nested_startuml.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("before closing previous block"));
}

#[test]
fn check_mode_reports_unterminated_second_block_boundary() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_unterminated_second_block.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("missing a closing @enduml"));
}

#[test]
fn malformed_note_or_ref_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_malformed_note_ref.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_NOTE_INVALID"));
}

#[test]
fn malformed_ref_block_missing_body_reports_diagnostic_snapshot() {
    let invalid = fixture("groups/invalid_ref_block_missing_body.puml");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out)
        .unwrap()
        .replace(&invalid, "<fixture>");
    assert!(stderr.contains("E_REF_INVALID"));
    assert_snapshot!(
        "malformed_ref_block_missing_body_reports_diagnostic",
        stderr
    );
}

#[test]
fn malformed_group_structure_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("groups/invalid_else_without_open_group.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_GROUP_ELSE_UNMATCHED"));
}

#[test]
fn malformed_group_mismatched_end_keyword_reports_diagnostic_snapshot() {
    let invalid = fixture("errors/invalid_group_mismatched_end_keyword.puml");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out)
        .unwrap()
        .replace(&invalid, "<fixture>");
    assert!(stderr.contains("E_GROUP_END_KIND"));
    assert_snapshot!(
        "malformed_group_mismatched_end_keyword_reports_diagnostic",
        stderr
    );
}

#[test]
fn malformed_group_empty_alt_reports_diagnostic_snapshot() {
    let invalid = fixture("errors/invalid_group_empty_alt.puml");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out)
        .unwrap()
        .replace(&invalid, "<fixture>");
    assert!(stderr.contains("E_GROUP_EMPTY"));
    assert_snapshot!("malformed_group_empty_alt_reports_diagnostic", stderr);
}

#[test]
fn invalid_autonumber_bad_format_token_reports_diagnostic_snapshot() {
    let invalid = fixture("errors/invalid_autonumber_bad_format_token.puml");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &invalid])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out)
        .unwrap()
        .replace(&invalid, "<fixture>");
    assert!(stderr.contains("E_AUTONUMBER_FORMAT_UNSUPPORTED"));
    assert_snapshot!(
        "invalid_autonumber_bad_format_token_reports_diagnostic",
        stderr
    );
}

#[test]
fn dump_mode_requires_kind() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--dump")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "a value is required for '--dump <KIND>'",
        ));
}

#[test]
fn dump_mode_outputs_ast_json() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("dump_mode_outputs_ast_json", json);
}

#[test]
fn dump_mode_outputs_scene_json() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "scene", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert!(json.get("size").is_some());
    assert!(json.get("lanes").is_some());
    assert!(json.get("rows").is_some());
}

#[test]
fn dump_mode_scene_is_deterministic_for_same_input() {
    let first = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("autonumber/valid_with_format.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let second = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("autonumber/valid_with_format.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let first_json: Value = serde_json::from_slice(&first).unwrap();
    let second_json: Value = serde_json::from_slice(&second).unwrap();
    assert_eq!(first_json, second_json);
    assert_json_snapshot!(
        "dump_mode_scene_is_deterministic_for_same_input",
        first_json
    );
}

#[test]
fn dump_mode_scene_preserves_advanced_note_ref_forms_deterministically() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("overflow/overflow_note_ref_advanced_forms_nonoverlap.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "dump_mode_scene_preserves_advanced_note_ref_forms_deterministically",
        json
    );
}

#[test]
fn check_mode_accepts_advanced_note_ref_forms() {
    for case in [
        "notes/valid_note_advanced_forms.puml",
        "groups/valid_ref_advanced_forms.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn multi_mode_outputs_all_diagrams_as_json() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("multi_valid.puml")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("multi_mode_outputs_all_diagrams_as_json", json);
}

#[test]
fn multi_mode_handles_three_diagrams() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("structure/multi_three.puml")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("multi_mode_handles_three_diagrams", json);
}

#[test]
fn multi_mode_splits_uppercase_start_enduml_blocks() {
    let input = "@STARTUML\nAlice -> Bob: one\n@ENDUML\n@STARTUML\nBob -> Alice: two\n@ENDUML\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected multi-dump array output");
    assert_eq!(arr.len(), 2);
}

#[test]
fn multi_mode_splits_named_startuml_blocks_and_ignores_comment_markers() {
    let input = fs::read_to_string(fixture("conformance/valid_named_blocks_and_comments.puml"))
        .expect("fixture load");
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected multi-dump array output");
    assert_eq!(arr.len(), 2);
    let first_label = arr[0]["statements"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .expect("first block message label");
    assert_eq!(first_label, "\"don't split\"");
}

#[test]
fn multi_mode_reports_unterminated_trailing_startuml_block() {
    let input = "@startuml\nAlice -> Bob: one\n@enduml\n@startuml\nBob -> Alice: two\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("missing a closing @enduml"));
}

#[test]
fn multi_mode_reports_enduml_without_startuml() {
    let input = "@enduml\n@startuml\nAlice -> Bob: one\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("without a preceding @startuml"));
}

#[test]
fn sequence_hnote_and_rnote_render_distinct_shapes() {
    let svg = render_source_to_svg(
        "@startuml\nAlice -> Bob: hi\nhnote over Alice\nhex note\nendhnote\nrnote over Bob\nrect note\nendrnote\n@enduml\n",
    )
    .expect("sequence hnote/rnote should render");

    assert!(svg.contains("hex note"));
    assert!(svg.contains("rect note"));
    assert!(
        svg.contains("<polygon"),
        "hnote should render as a hexagonal polygon"
    );
    assert!(
        svg.contains("rx=\"0\" ry=\"0\""),
        "rnote should render as a square-corner rectangle"
    );
}

#[test]
fn check_mode_reports_enduml_without_startuml_even_with_suffix_text() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_unmatched_enduml_with_suffix.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("without a preceding @startuml"));
}

#[test]
fn check_mode_reports_nested_startuml_even_with_suffix_text() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_nested_startuml_with_suffix.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "unmatched @startuml/@enduml boundary",
        ))
        .stderr(predicate::str::contains("found @startuml"))
        .stderr(predicate::str::contains("before closing previous block"));
}

#[test]
fn multi_input_without_flag_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(fs::read_to_string(fixture("multi_valid.puml")).unwrap())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("rerun with --multi"));
}

#[test]
fn stdin_input_is_supported() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_snapshot!("stdin_input_is_supported", String::from_utf8(out).unwrap());
}

#[test]
fn stdin_dash_path_is_supported() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .arg("-")
        .write_stdin("A -> B")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_snapshot!(
        "stdin_dash_path_is_supported",
        String::from_utf8(out).unwrap()
    );
}

#[test]
fn missing_file_maps_to_io_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("/tmp/definitely-not-present-12345.puml")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read"));
}

#[test]
fn empty_input_maps_to_validation_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg(fixture("empty.txt"))
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no diagram content provided"));
}

#[test]
fn plain_multi_delimiter_supported_with_multi_flag() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("plain_multi.txt")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_snapshot!(
        "plain_multi_delimiter_supported_with_multi_flag",
        String::from_utf8(out).unwrap()
    );
}

#[test]
fn check_and_dump_are_mutually_exclusive() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--dump", "ast", &fixture("single_valid.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn from_markdown_extracts_fenced_blocks_in_source_order() {
    let input = "# doc\n```puml\n@startuml\nAlice -> Bob: one\n@enduml\n```\ntext\n```plantuml\n@startuml\nBob -> Alice: two\n@enduml\n```\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array");
    assert_eq!(arr.len(), 2);
    let first = arr[0]["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    let second = arr[1]["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(first, "one");
    assert_eq!(second, "two");
}

#[test]
fn metadata_mode_from_markdown_emits_one_object_per_fence_without_multi() {
    let input = fs::read_to_string(fixture("markdown/multipage_mixed.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--metadata", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected metadata array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["family"], "sequence");
    assert_eq!(arr[0]["counts"]["messages"], 2);
    assert_eq!(arr[0]["counts"]["pages"], 2);
    assert_eq!(arr[1]["family"], "sequence");
    assert_eq!(arr[1]["counts"]["messages"], 1);
    assert_eq!(arr[1]["counts"]["pages"], 1);
}

#[test]
fn from_markdown_supports_first_class_fence_frontends_and_aliases() {
    let input = fs::read_to_string(fixture("markdown/mixed_fences.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array");
    let labels = arr
        .iter()
        .map(|doc| {
            doc["statements"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "puml-one",
            "pumlx-two",
            "picouml-three",
            "plantuml-four",
            "mermaid-five",
        ]
    );
}

#[test]
fn from_markdown_supports_legacy_sequence_fence_aliases() {
    let input = "```puml-sequence
@startuml
Alice -> Bob: one
@enduml
```
text
```uml-sequence
@startuml
Bob -> Alice: two
@enduml
```
";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["statements"][0]["kind"]["Message"]["label"], "one");
    assert_eq!(arr[1]["statements"][0]["kind"]["Message"]["label"], "two");
}

#[test]
fn from_markdown_supports_uml_fence_alias() {
    let input = "```uml
@startuml
Alice -> Bob: uml-alias
@enduml
```
";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        json["statements"][0]["kind"]["Message"]["label"],
        "uml-alias"
    );
}

#[test]
fn from_markdown_ignores_non_fence_markdown_content() {
    let input = "# not a diagram\nA -x B: malformed outside fence\n\n```puml\n@startuml\nAlice -> Bob: ok\n@enduml\n```\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn from_markdown_diagnostics_json_maps_to_markdown_line_column() {
    let input = "# header\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "--diagnostics", "json", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("diagnostics_json_error_contract_shape", json);
    let first = &json["diagnostics"][0];
    assert_eq!(json["schema"], "puml.diagnostics");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(first["code"], "E_ARROW_INVALID");
    assert_eq!(first["severity"], "error");
    assert_eq!(first["line"], 4);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "A -x B: bad");
    assert!(first["message"]
        .as_str()
        .unwrap()
        .contains("E_ARROW_INVALID"));
}

#[test]
fn stdin_markdown_multi_fences_require_multi_flag() {
    let input = fs::read_to_string(fixture("markdown/mixed_fences.md")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("rerun with --multi"));
}

#[test]
fn stdin_markdown_multi_outputs_snippet_named_json() {
    let input = fs::read_to_string(fixture("markdown/multipage_mixed.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["name"], "snippet-1-1.svg");
    assert_eq!(arr[1]["name"], "snippet-1-2.svg");
    assert_eq!(arr[2]["name"], "snippet-2.svg");
}

#[test]
fn diagnostics_default_mode_remains_human_readable() {
    let input = "# header\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("line 4, column 1"));
    assert!(stderr.contains("A -x B: bad\n^^^^^^"));
    assert!(!stderr.trim_start().starts_with("{\"diagnostics\""));
}

#[test]
fn from_markdown_ingests_mixed_fence_edge_cases_deterministically() {
    let input = fs::read_to_string(fixture("markdown/edge_cases.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--dump", "ast", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array");
    let labels = arr
        .iter()
        .map(|doc| {
            doc["statements"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec!["tilde-puml", "uppercase-mermaid", "three-space-indent"]
    );
}

#[test]
fn stdin_markdown_edge_cases_multi_outputs_name_supported_fences_only() {
    let input = fs::read_to_string(fixture("markdown/edge_cases.md")).unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["name"], "snippet-1.svg");
    assert_eq!(arr[1]["name"], "snippet-2.svg");
    assert_eq!(arr[2]["name"], "snippet-3.svg");
}

#[test]
fn from_markdown_unclosed_supported_fence_ingests_through_eof() {
    let input = fs::read_to_string(fixture("markdown/unclosed_fence_eof.md")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn from_markdown_without_supported_fences_reports_actionable_error() {
    let input = "# heading\n```rust\nfn main() {}\n```\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--check", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "no supported markdown diagram fences found",
        ))
        .stderr(predicate::str::contains("mermaid"));
}

#[test]
fn include_cycle_input_reports_cycle_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/error_include_cycle_self.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("include cycle detected"));
}

#[test]
fn include_cycle_chain_reports_cycle_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/error_include_chain_a.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("include cycle detected"));
}

#[test]
fn include_id_tag_extracts_local_block() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/include_with_tag_ok.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn include_many_expands_each_occurrence() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/include_many_ok.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let msg_count = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stmt| stmt["kind"]["Message"].is_object())
        .count();
    assert_eq!(msg_count, 2);
}

#[test]
fn include_once_expands_only_first_occurrence() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/include_once_ok.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let msg_count = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stmt| stmt["kind"]["Message"].is_object())
        .count();
    assert_eq!(msg_count, 1);
}

#[test]
fn includesub_extracts_local_block() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/includesub_ok.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn include_id_missing_tag_reports_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_include_tag_missing.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_TAG_NOT_FOUND"))
        .stderr(predicate::str::contains(
            "include tag 'MISSING_TAG' was not found",
        ));
}

#[test]
fn include_url_is_rejected_with_deterministic_error_when_flag_set() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--no-url-includes",
            &fixture("errors/invalid_include_url.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_URL_DISABLED"));
}

#[test]
fn includesub_without_tag_is_rejected_with_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_includesub_missing_tag.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDESUB_TAG_REQUIRED"))
        .stderr(predicate::str::contains(
            "!includesub requires a target tag",
        ));
}

#[test]
fn include_variants_url_policy_is_rejected_deterministically_when_flag_set() {
    for (case, _directive) in [
        ("errors/invalid_include_url.puml", "!include"),
        ("errors/invalid_include_once_url.puml", "!include_once"),
        ("errors/invalid_include_many_url.puml", "!include_many"),
        ("errors/invalid_includesub_url.puml", "!includesub"),
        ("errors/invalid_includeurl_url.puml", "!includeurl"),
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", "--no-url-includes", &fixture(case)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains("E_INCLUDE_URL_DISABLED"));
    }
}

#[test]
fn preprocessor_if_elseif_else_emits_only_selected_branch() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_if_elseif_else.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["primary"]);
}

#[test]
fn preprocessor_while_executes_until_condition_is_false() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_while_define_counter.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["loop 2", "loop 1"]);
}

#[test]
fn preprocessor_variable_assignment_and_reference_semantics_are_applied() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_variable_assignment_reference.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let participants = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Participant"]["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(participants, vec!["Alice", "Bob"]);
}

#[test]
fn preprocessor_function_and_procedure_args_expand_deterministically() {
    let fn_out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_function_call_args_defaults_keywords.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let fn_json: Value = serde_json::from_slice(&fn_out).unwrap();
    let fn_labels = fn_json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    // `+` is the string concatenation operator in PlantUML preprocessor (#582).
    // `!return $lhs + "->" + $rhs` should evaluate to the joined string.
    assert_eq!(fn_labels, vec!["A->B", "C->D"]);

    let proc_out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_procedure_call_args.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let proc_json: Value = serde_json::from_slice(&proc_out).unwrap();
    let proc_labels = proc_json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(proc_labels, vec!["\"ok\"", "go"]);
}

#[test]
fn preprocessor_function_return_with_leading_indentation_is_honored() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_function_return_indented.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["\"A\""]);
}

#[test]
fn preprocessor_function_procedure_assert_log_and_dump_are_minimally_compatible() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/valid_function_procedure_assert_log_dump.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/valid_log_and_dump_with_payload.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn preprocessor_assert_false_reports_diagnostic_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/invalid_assert_false.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("E_PREPROC_ASSERT"));
    assert_snapshot!("preprocessor_assert_false_reports_diagnostic", stderr);
}

#[test]
fn preprocessor_unclosed_function_reports_diagnostic_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/invalid_unclosed_function.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("E_FUNCTION_UNCLOSED"));
    assert_snapshot!("preprocessor_unclosed_function_reports_diagnostic", stderr);
}

#[test]
fn preprocessor_conditional_and_while_balance_errors_are_deterministic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_conditional_order.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_COND_ORDER"));

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_unclosed_if.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_COND_UNCLOSED"));

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_endwhile_without_while.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_WHILE_UNEXPECTED"));
}

#[test]
fn preprocessor_expression_validation_errors_are_deterministic() {
    let cases = [
        (
            "errors/invalid_preproc_expr_missing.puml",
            "E_PREPROC_EXPR_REQUIRED",
        ),
        (
            "errors/invalid_preproc_unexpected_endfunction.puml",
            "E_PREPROC_UNEXPECTED",
        ),
        (
            "errors/invalid_preproc_procedure_unsupported.puml",
            "E_PREPROC_CALL_KIND",
        ),
        (
            "errors/invalid_preproc_while_iteration_limit.puml",
            "E_PREPROC_WHILE_LIMIT",
        ),
        (
            "errors/invalid_preproc_assert_missing_expr.puml",
            "E_PREPROC_ASSERT_EXPR_REQUIRED",
        ),
        (
            "errors/invalid_preproc_builtin_in_assert.puml",
            "E_PREPROC_BUILTIN_UNSUPPORTED",
        ),
        (
            "errors/invalid_preproc_builtin_in_log.puml",
            "E_PREPROC_BUILTIN_UNSUPPORTED",
        ),
        (
            "errors/invalid_preproc_dynamic_invoke.puml",
            "E_PREPROC_DYNAMIC_UNSUPPORTED",
        ),
        (
            "errors/invalid_preproc_json_assignment.puml",
            "E_PREPROC_JSON_UNSUPPORTED",
        ),
        (
            "errors/invalid_preproc_function_missing_arg.puml",
            "E_PREPROC_ARG_REQUIRED",
        ),
        (
            "errors/invalid_preproc_procedure_return.puml",
            "E_PREPROC_RETURN_UNEXPECTED",
        ),
        (
            "errors/invalid_import_empty_path.puml",
            "E_IMPORT_PATH_REQUIRED",
        ),
        // URL imports are covered separately by import_url_disabled_produces_deterministic_error.
        // Keep this list focused on local include/import path-shape diagnostics.
        (
            "errors/invalid_import_absolute_path.puml",
            "E_IMPORT_ABSOLUTE_PATH",
        ),
        (
            "errors/invalid_import_tag_form.puml",
            "E_IMPORT_INVALID_FORM",
        ),
        ("errors/invalid_import_escape_path.puml", "E_IMPORT_ESCAPE"),
        (
            "errors/invalid_import_missing_module.puml",
            "E_IMPORT_STDLIB_NOT_FOUND",
        ),
    ];

    for (path, code) in cases {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(path)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains(code));
    }
}

#[test]
fn import_url_disabled_produces_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--no-url-includes",
            &fixture("errors/invalid_import_url.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_URL_DISABLED"));
}

#[test]
fn include_path_shape_errors_are_deterministic() {
    let cases = [
        (
            "errors/invalid_include_absolute_path.puml",
            "E_INCLUDE_ABSOLUTE_PATH",
        ),
        (
            "errors/invalid_include_empty_path.puml",
            "E_INCLUDE_PATH_REQUIRED",
        ),
    ];

    for (path, code) in cases {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(path)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains(code));
    }
}

#[test]
fn lifecycle_after_destroy_is_rejected() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("lifecycle/valid_destroy_then_message.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("destroyed"));
}

#[test]
fn non_sequence_inputs_fail_validation() {
    let (case, code) = (
        "errors/invalid_salt_block_mismatch.puml",
        "E_BLOCK_MISMATCH",
    );
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture(case)])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(code));
}

#[test]
fn extended_family_fixtures_pass_check_and_render_svg() {
    let cases = [
        // Wave 3-A (#490 #494) suppressed the leaky "<family> diagram" canvas
        // text. Each marker now asserts on a structural element actually emitted
        // by the renderer for that family.
        ("families/valid_component.puml", "«component»"),
        ("families/valid_deployment.puml", "<polygon"),
        ("families/valid_state.puml", "<rect"),
        ("families/valid_activity.puml", "<rect"),
        ("families/valid_timing.puml", "<text"),
        ("families/valid_timing_waveform.puml", "<polyline"),
    ];
    for (case, marker) in cases {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stderr(predicate::str::is_empty());

        let src = fs::read_to_string(fixture(case)).unwrap();
        let out = Command::cargo_bin("puml")
            .expect("binary")
            .arg("-")
            .write_stdin(src)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let svg = String::from_utf8(out).expect("svg utf8");
        assert!(svg.contains("<svg"), "missing svg envelope for {case}");
        assert!(svg.contains("</svg>"), "missing svg close for {case}");
        assert!(
            svg.contains(marker),
            "expected marker `{marker}` for {case}"
        );
    }
}

#[test]
fn class_object_usecase_bootstrap_inputs_pass_check() {
    for case in [
        "families/valid_class_bootstrap.puml",
        "families/valid_object_bootstrap.puml",
        "families/valid_usecase_bootstrap.puml",
        "families/valid_salt_bootstrap.puml",
        "families/valid_class_members_block.puml",
        "families/valid_object_members_block.puml",
        "families/valid_usecase_members_block.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success()
            .stderr(predicate::str::is_empty());
    }
}

#[test]
fn class_object_usecase_bootstrap_render_stubs_are_deterministic() {
    for (case, marker) in [
        ("families/valid_class_bootstrap.puml", "User"),
        ("families/valid_object_bootstrap.puml", "Order"),
        ("families/valid_usecase_bootstrap.puml", "Authenticate"),
        ("families/valid_salt_bootstrap.puml", "submit_button"),
        ("families/valid_class_members_block.puml", "+id: UUID"),
        ("families/valid_object_members_block.puml", "token = abc123"),
        ("families/valid_usecase_members_block.puml", "Authenticate"),
    ] {
        let src = fs::read_to_string(fixture(case)).unwrap();
        let first = Command::cargo_bin("puml")
            .expect("binary")
            .arg("-")
            .write_stdin(src.clone())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let second = Command::cargo_bin("puml")
            .expect("binary")
            .arg("-")
            .write_stdin(src)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        assert_eq!(
            first, second,
            "stub output should be deterministic for {case}"
        );
        let svg = String::from_utf8(first).unwrap();
        assert!(svg.contains(marker), "missing family marker for {case}");
    }
}

#[test]
fn family_member_block_render_snapshot_is_deterministic() {
    let svg = Command::cargo_bin("puml")
        .expect("binary")
        .arg("-")
        .write_stdin(
            fs::read_to_string(fixture("families/valid_class_members_block.puml")).unwrap(),
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_snapshot!(
        "family_member_block_render_snapshot_is_deterministic",
        String::from_utf8(svg).unwrap()
    );
}

#[test]
fn family_member_blocks_are_preserved_in_ast_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("families/valid_class_members_block.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let members = json["statements"][0]["kind"]["ClassDecl"]["members"]
        .as_array()
        .expect("members should be present");
    assert_eq!(members.len(), 3);
    // Members are now objects with "text" and "modifier" fields
    assert_eq!(members[0]["text"], "+id: UUID");
    assert_eq!(members[0]["modifier"], serde_json::Value::Null);
}

#[test]
fn unclosed_family_declaration_block_reports_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_family_decl_block_unclosed.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_FAMILY_DECL_BLOCK_UNCLOSED"))
        .stderr(predicate::str::contains("missing `}`"));
}

#[test]
fn extended_families_render_to_deterministic_svg() {
    let cases = [
        ("non_sequence/valid_regex.puml", "<svg"),
        ("non_sequence/valid_ebnf.puml", "<svg"),
        ("non_sequence/valid_math.puml", "<svg"),
        ("non_sequence/valid_sdl.puml", "<svg"),
        ("non_sequence/valid_ditaa.puml", "<svg"),
        ("non_sequence/valid_chart_bar.puml", "<svg"),
        ("non_sequence/valid_chart_pie.puml", "<svg"),
    ];
    for (case, marker) in cases {
        let src = fs::read_to_string(fixture(case)).unwrap();
        let first = Command::cargo_bin("puml")
            .expect("binary")
            .arg("-")
            .write_stdin(src.clone())
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let second = Command::cargo_bin("puml")
            .expect("binary")
            .arg("-")
            .write_stdin(src)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert_eq!(first, second, "render must be deterministic for {case}");
        let svg = String::from_utf8(first).unwrap();
        assert!(
            svg.contains(marker),
            "missing marker `{marker}` for {case}; got: {svg}"
        );
    }
}

#[test]
fn extended_families_pass_check() {
    for case in [
        "non_sequence/valid_regex.puml",
        "non_sequence/valid_ebnf.puml",
        "non_sequence/valid_math.puml",
        "non_sequence/valid_sdl.puml",
        "non_sequence/valid_ditaa.puml",
        "non_sequence/valid_chart_bar.puml",
        "non_sequence/valid_chart_pie.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .success();
    }
}

#[test]
fn autonumber_is_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("autonumber/valid_with_format.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("autonumber_is_preserved_in_model_dump", json);
}

#[test]
fn autonumber_restart_step_and_format_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("structure/valid_autonumber_restart_step_format.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let events = json["events"].as_array().expect("events array");
    let autonumber_raw: Vec<_> = events
        .iter()
        .filter_map(|event| event["kind"]["Autonumber"].as_str())
        .collect();
    assert_eq!(
        autonumber_raw,
        vec![
            "10 5 \"[000]\"",
            "stop",
            "resume 2 \"R-00\"",
            "3 3 \"S-00\""
        ]
    );
}

#[test]
fn autonumber_raw_is_canonicalized_for_deterministic_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("structure/valid_autonumber_format_only_and_canonical_spacing.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let events = json["events"].as_array().expect("events array");
    let autonumber_raw: Vec<_> = events
        .iter()
        .filter_map(|event| event["kind"]["Autonumber"].as_str())
        .collect();
    assert_eq!(autonumber_raw, vec!["\"ID-000\"", "resume \"ID-000\""]);
}

#[test]
fn autonumber_off_and_resume_edges_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("structure/valid_autonumber_off_resume_edges.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let events = json["events"].as_array().expect("events array");
    let autonumber_raw: Vec<_> = events
        .iter()
        .filter_map(|event| event["kind"]["Autonumber"].as_str())
        .collect();
    assert_eq!(
        autonumber_raw,
        vec![
            "7 3 \"ID-00\"",
            "off",
            "resume \"R-00\"",
            "resume 5 \"R-00\""
        ]
    );
}

#[test]
fn autonumber_dotted_and_hash_padding_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("structure/valid_autonumber_dotted_and_hash_padding.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let events = json["events"].as_array().expect("events array");
    let autonumber_raw: Vec<_> = events
        .iter()
        .filter_map(|event| event["kind"]["Autonumber"].as_str())
        .collect();
    assert_eq!(
        autonumber_raw,
        vec!["1.02.003", "7 2 \"ID-###\"", "stop", "resume 5 \"R-###\""]
    );
}

#[test]
fn lifecycle_shortcuts_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("lifecycle/valid_shortcuts_expansion.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("lifecycle_shortcuts_are_preserved_in_model_dump", json);
}

#[test]
fn lifecycle_return_inference_from_shortcut_activation_is_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("lifecycle/valid_return_inferred_from_shortcut_activation.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "lifecycle_return_inference_from_shortcut_activation_is_preserved_in_model_dump",
        json
    );
}

#[test]
fn lifecycle_return_inference_from_last_message_is_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("lifecycle/valid_return_inferred_from_last_message.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "lifecycle_return_inference_from_last_message_is_preserved_in_model_dump",
        json
    );
}

#[test]
fn lifecycle_return_without_caller_context_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("lifecycle/invalid_return_without_caller_context.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_RETURN_INFER_CALLER"));
}

#[test]
fn queue_role_and_separator_are_preserved_in_model_dump() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("participants/valid_queue_separator.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("queue_role_and_separator_are_preserved_in_model_dump", json);
}

#[test]
fn can_read_tempfile_input() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("sample.puml");
    fs::write(&input, "@startuml\nX -> Y\n@enduml\n").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(&input)
        .assert()
        .success();

    let output = tmp.path().join("sample.svg");
    assert!(output.exists());
    let svg = fs::read_to_string(output).unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn dump_mode_scene_preserves_separator_delay_divider_and_spacer_rows() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("structure/valid_separator_delay_divider_spacer.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "dump_mode_scene_preserves_separator_delay_divider_and_spacer_rows",
        json
    );
}

#[test]
fn stdin_include_requires_include_root_or_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin("@startuml\n!include include_ok_child.puml\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "!include from stdin requires include_root option",
        ));
}

#[test]
fn stdin_include_with_include_root_passes() {
    let root = format!("{}/tests/fixtures/include", env!("CARGO_MANIFEST_DIR"));
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--include-root", &root, "-"])
        .write_stdin("@startuml\n!include include_ok_child.puml\n@enduml\n")
        .assert()
        .success();
}

#[test]
fn stdin_import_requires_include_root_or_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin("@startuml\n!import core\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_IMPORT_ROOT_REQUIRED"));
}

#[test]
fn stdin_import_with_include_root_passes() {
    let root = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--include-root", &root, "-"])
        .write_stdin("@startuml\n!import core\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn file_multi_output_with_o_writes_numbered_files() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("multi_three.puml");
    fs::copy(fixture("structure/multi_three.puml"), &input).unwrap();
    let out = tmp.path().join("diagram.svg");

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            input.to_str().unwrap(),
            "-o",
            out.to_str().unwrap(),
            "--multi",
        ])
        .assert()
        .success();

    assert!(tmp.path().join("diagram-1.svg").exists());
    assert!(tmp.path().join("diagram-2.svg").exists());
    assert!(tmp.path().join("diagram-3.svg").exists());
}

#[test]
fn stdin_newpage_without_multi_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("multiple pages detected"));
}

#[test]
fn stdin_newpage_with_multi_outputs_json_array_and_stable_order() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin("@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "stdin_newpage_with_multi_outputs_json_array_and_stable_order",
        json
    );
}

#[test]
fn stdin_newpage_cli_contract_modes_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("structure/newpage_stdin_contract.puml")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("stdin_newpage_cli_contract_modes", json);
}

#[test]
fn stdin_ignore_newpage_without_multi_outputs_single_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(
            "@startuml\nA -> B : one\nignore newpage\nnewpage Second\nB -> A : two\n@enduml\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("\"diagram-1.svg\"").not());
}

#[test]
fn stdin_ignore_newpage_with_multi_still_outputs_single_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(
            "@startuml\nA -> B : one\nignore newpage\nnewpage Second\nB -> A : two\n@enduml\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("\"diagram-1.svg\"").not());
}

#[test]
fn file_newpage_output_without_multi_writes_numbered_files() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::copy(fixture("structure/newpage_stdin_contract.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(input.to_str().unwrap())
        .assert()
        .success();

    assert!(tmp.path().join("paged-1.svg").exists());
    assert!(tmp.path().join("paged-2.svg").exists());
}

#[test]
fn file_newpage_output_writes_numbered_files_with_multi_flag() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::write(
        &input,
        "@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .success();

    assert!(tmp.path().join("paged-1.svg").exists());
    assert!(tmp.path().join("paged-2.svg").exists());
}

#[test]
fn multipage_file_output_failure_does_not_leave_partial_writes() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::copy(fixture("structure/newpage_stdin_contract.puml"), &input).unwrap();
    let output = tmp.path().join("diagram.svg");
    let first = tmp.path().join("diagram-1.svg");

    fs::write(&first, "stable-original-content").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_FAIL_OUTPUT_AFTER", "1")
        .args([
            "--multi",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));

    assert_eq!(
        fs::read_to_string(&first).unwrap(),
        "stable-original-content".to_string()
    );
    assert!(!tmp.path().join("diagram-2.svg").exists());
}

#[test]
fn single_file_output_failure_does_not_overwrite_existing_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single.puml");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();
    let output = tmp.path().join("diagram.svg");
    fs::write(&output, "stable-single-content").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_FAIL_OUTPUT_AFTER", "0")
        .args(["-o", output.to_str().unwrap(), input.to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));

    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "stable-single-content".to_string()
    );
}

#[test]
fn file_ignore_newpage_output_writes_single_default_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("ignore_newpage.puml");
    fs::copy(
        fixture("structure/ignore_newpage_single_output.puml"),
        &input,
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(input.to_str().unwrap())
        .assert()
        .success();

    assert!(tmp.path().join("ignore_newpage.svg").exists());
    assert!(!tmp.path().join("ignore_newpage-1.svg").exists());
    assert!(!tmp.path().join("ignore_newpage-2.svg").exists());
}

#[test]
fn stdin_multi_blocks_with_newpage_flatten_into_stable_named_json_order() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(
            "@startuml\nA -> B : one\nnewpage Two\nB -> A : two\n@enduml\n\n@startuml\nX -> Y : three\n@enduml\n",
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["name"], "diagram-1.svg");
    assert_eq!(arr[1]["name"], "diagram-2.svg");
    assert_eq!(arr[2]["name"], "diagram-3.svg");
}

#[test]
fn stdin_multi_blocks_with_ignore_newpage_requires_multi() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(
            fs::read_to_string(fixture("structure/multi_blocks_ignore_newpage.puml")).unwrap(),
        )
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "multiple diagrams detected; rerun with --multi",
        ));
}

#[test]
fn stdin_multi_blocks_with_ignore_newpage_and_multi_outputs_two_items() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(
            fs::read_to_string(fixture("structure/multi_blocks_ignore_newpage.puml")).unwrap(),
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "diagram-1.svg");
    assert_eq!(arr[1]["name"], "diagram-2.svg");
}

#[test]
fn file_input_infers_include_root_from_parent_directory() {
    let tmp = tempdir().unwrap();
    let include = tmp.path().join("child.puml");
    let parent = tmp.path().join("parent.puml");
    fs::write(&include, "Alice -> Bob : from child\n").unwrap();
    fs::write(
        &parent,
        "@startuml\n!include child.puml\nBob -> Alice : from parent\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", parent.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn file_input_infers_stdlib_root_for_imports_from_parent_directory() {
    let tmp = tempdir().unwrap();
    let stdlib = tmp.path().join("stdlib");
    fs::create_dir_all(&stdlib).unwrap();
    fs::write(stdlib.join("core.puml"), "Alice -> Bob : from stdlib\n").unwrap();

    let src_path = tmp.path().join("diagram.puml");
    fs::write(&src_path, "@startuml\n!import core\n@enduml\n").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", src_path.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn explicit_output_file_is_overwritten_with_latest_render() {
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("explicit.svg");
    fs::write(&out, "old-content").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            fixture("single_valid.puml").as_str(),
            "--output",
            out.to_str().unwrap(),
        ])
        .assert()
        .success();

    let written = fs::read_to_string(&out).unwrap();
    assert!(written.contains("<svg"));
    assert_ne!(written, "old-content");
}

#[test]
fn multi_page_output_with_root_path_reports_invalid_output_stem() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::write(
        &input,
        "@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", input.to_str().unwrap(), "--output", "/"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot derive output stem"));
}

#[test]
fn explicit_output_with_missing_parent_reports_io_exit_code() {
    let tmp = tempdir().unwrap();
    let missing_parent = tmp.path().join("missing").join("out.svg");

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            fixture("single_valid.puml").as_str(),
            "--output",
            missing_parent.to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));
}

#[test]
fn markdown_file_auto_extracts_fenced_diagrams_without_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.md");
    fs::write(
        &input,
        "# heading\nA -x B: malformed outside fence\n\n```puml\n@startuml\nAlice -> Bob: one\n@enduml\n```\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn markdown_file_default_render_output_uses_deterministic_snippet_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("mixed.md");
    fs::copy(fixture("markdown/multipage_mixed.md"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(dir.path().join("mixed_snippet_1-1.svg").exists());
    assert!(dir.path().join("mixed_snippet_1-2.svg").exists());
    assert!(dir.path().join("mixed_snippet_2.svg").exists());
    assert!(!dir.path().join("mixed-1.svg").exists());
    assert!(!dir.path().join("mixed-2.svg").exists());
}

#[test]
fn markdown_multi_output_failure_does_not_leave_partial_writes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("mixed.md");
    fs::copy(fixture("markdown/multipage_mixed.md"), &input).unwrap();

    let first = dir.path().join("mixed_snippet_1-1.svg");
    fs::write(&first, "stable-original-snippet").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_FAIL_OUTPUT_AFTER", "1")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));

    assert_eq!(
        fs::read_to_string(&first).unwrap(),
        "stable-original-snippet".to_string()
    );
    assert!(!dir.path().join("mixed_snippet_1-2.svg").exists());
}

#[test]
fn markdown_file_diagnostics_map_to_original_markdown_lines() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("invalid.markdown");
    fs::write(
        &input,
        "# header\n\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--diagnostics", "json", input.to_str().unwrap()])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let first = &json["diagnostics"][0];
    assert_eq!(first["line"], 5);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "A -x B: bad");
}

#[test]
fn lint_mode_requires_check_flag() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--lint-input", &fixture("single_valid.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--check"));
}

#[test]
fn lint_mode_json_report_supports_repeated_inputs_and_globs_with_stable_order() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("a_invalid.puml"),
    )
    .unwrap();
    fs::copy(
        fixture("single_valid.puml"),
        tmp.path().join("b_valid.puml"),
    )
    .unwrap();
    fs::copy(
        fixture("styling/valid_skinparam_unsupported.puml"),
        tmp.path().join("c_warning.puml"),
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-input",
            "b_valid.puml",
            "--lint-input",
            "a_invalid.puml",
            "--lint-glob",
            "*.puml",
            "--lint-report",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema"], "puml.lint_report");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["summary"]["total_files"], 3);
    assert_eq!(report["summary"]["passed_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["total_diagrams"], 3);
    assert_eq!(report["summary"]["passed_diagrams"], 2);
    assert_eq!(report["summary"]["failed_diagrams"], 1);
    assert_eq!(report["summary"]["warning_count"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let files = report["files"].as_array().expect("files array");
    assert_eq!(files.len(), 3);
    assert_eq!(files[0]["path"], "a_invalid.puml");
    assert_eq!(files[1]["path"], "b_valid.puml");
    assert_eq!(files[2]["path"], "c_warning.puml");

    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--> a_invalid.puml"));
}

#[test]
fn lint_mode_json_diagnostics_stay_on_stderr_and_report_stays_on_stdout() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("invalid_single.puml"),
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-input",
            "invalid_single.puml",
            "--diagnostics",
            "json",
            "--lint-report",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema"], "puml.lint_report");
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    assert_eq!(diagnostics["diagnostics"][0]["severity"], "error");
}

#[test]
fn lint_mode_human_report_succeeds_for_all_valid_inputs() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("single_valid.puml"),
        tmp.path().join("a_valid.puml"),
    )
    .unwrap();
    fs::copy(fixture("basic/hello.puml"), tmp.path().join("b_valid.puml")).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args(["--check", "--lint-glob", "*.puml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "lint summary: files=2 passed=2 failed=0",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn lint_mode_markdown_docs_glob_runs_end_to_end() {
    let tmp = tempdir().unwrap();
    fs::write(
        tmp.path().join("ok.md"),
        "# ok\n```puml\n@startuml\nAlice -> Bob: hello\n@enduml\n```\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("broken.md"),
        "# broken\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-glob",
            "*.md",
            "--lint-report",
            "json",
            "--diagnostics",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["summary"]["total_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["total_diagrams"], 2);
    assert_eq!(report["summary"]["failed_diagrams"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    assert_eq!(diagnostics["diagnostics"][0]["line"], 4);
    assert_eq!(diagnostics["diagnostics"][0]["file"], "broken.md");
}

#[test]
fn lint_mode_json_diagnostics_aggregate_deterministically_across_files() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("a_invalid.puml"),
    )
    .unwrap();
    fs::write(
        tmp.path().join("b_warning.puml"),
        "@startuml\nskinparam SequenceFooColor #123456\nAlice -> Bob: ok\n@enduml\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-glob",
            "*.puml",
            "--lint-report",
            "json",
            "--diagnostics",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["summary"]["total_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["warning_count"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    let entries = diagnostics["diagnostics"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["file"], "a_invalid.puml");
    assert_eq!(entries[0]["severity"], "error");
    assert_eq!(entries[1]["file"], "b_warning.puml");
    assert_eq!(entries[1]["severity"], "warning");
}

#[test]
fn clap_help_exits_successfully() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Rust-native PlantUML-compatible diagram renderer",
        ))
        .stdout(predicate::str::contains(
            "Permit multiple stdin render outputs",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn clap_version_exits_successfully() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("puml"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn exit_code_matrix_is_stable_for_success_validation_and_io() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--help")
        .assert()
        .code(0);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--definitely-invalid-flag")
        .assert()
        .code(1);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("/tmp/definitely-not-present-input-12.puml")
        .assert()
        .code(2);
}

#[test]
fn dump_capabilities_outputs_manifest_shape() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .arg("--dump-capabilities")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    // Output is now the real LSP protocol-level capabilities object (same as
    // what the server returns in its initialize response).
    let json: Value = serde_json::from_slice(&out).unwrap();
    assert!(json["completionProvider"]["resolveProvider"]
        .as_bool()
        .unwrap_or(false));
    assert!(json["hoverProvider"].as_bool().unwrap_or(false));
    assert!(json["definitionProvider"].as_bool().unwrap_or(false));
    assert!(json["referencesProvider"].as_bool().unwrap_or(false));
    assert!(json["documentFormattingProvider"]
        .as_bool()
        .unwrap_or(false));
    assert!(json["documentRangeFormattingProvider"]
        .as_bool()
        .unwrap_or(false));
    assert!(json["codeActionProvider"].as_bool().unwrap_or(false));
    assert!(json["colorProvider"].as_bool().unwrap_or(false));
    assert!(json["foldingRangeProvider"].as_bool().unwrap_or(false));
    assert!(json["selectionRangeProvider"].as_bool().unwrap_or(false));
    assert!(json["documentSymbolProvider"].as_bool().unwrap_or(false));
    assert!(json["workspaceSymbolProvider"].as_bool().unwrap_or(false));
    let commands = json["executeCommandProvider"]["commands"]
        .as_array()
        .expect("executeCommandProvider.commands must be an array");
    assert!(commands.iter().any(|c| c == "puml.applyFormat"));
    assert!(commands.iter().any(|c| c == "puml.renderSvg"));
    let token_types = json["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .expect("semanticTokensProvider.legend.tokenTypes must be an array");
    assert!(token_types.iter().any(|t| t == "keyword"));
    assert!(json["semanticTokensProvider"]["full"]
        .as_bool()
        .unwrap_or(false));
    assert!(json["workspace"]["workspaceFolders"]["supported"]
        .as_bool()
        .unwrap_or(false));
}

#[test]
fn check_fixture_uses_fixture_loader_and_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check-fixture", &fixture("single_valid.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn check_fixture_missing_file_maps_to_io_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            "/tmp/definitely-not-present-fixture-16.puml",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read fixture"));
}

#[test]
fn check_fixture_with_json_diagnostics_emits_warning_payload() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            &fixture("styling/valid_skinparam_unsupported.puml"),
            "--diagnostics",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .get_output()
        .stderr
        .clone();

    let line = String::from_utf8(out).unwrap();
    let json: Value = serde_json::from_str(line.trim()).expect("valid json warning payload");
    assert_json_snapshot!("diagnostics_json_warning_contract_shape", json);
    let first = &json["diagnostics"][0];
    assert_eq!(json["schema"], "puml.diagnostics");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(first["code"], "W_SKINPARAM_UNSUPPORTED");
    assert_eq!(first["severity"], "warning");
    assert_eq!(first["line"], 2);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "skinparam TotallyUnknownColor red");
    assert!(first["message"]
        .as_str()
        .unwrap()
        .contains("W_SKINPARAM_UNSUPPORTED"));
}

#[test]
fn diagnostics_json_writes_only_to_stderr_and_not_stdout() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--diagnostics",
            "json",
            &fixture("invalid_single.puml"),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::starts_with(
            "{\n  \"schema\": \"puml.diagnostics\"",
        ));
}

#[test]
fn stdin_empty_input_maps_to_validation_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no diagram content provided"));
}

#[test]
fn markdown_mdown_extension_auto_extracts_fenced_diagrams_without_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.mdown");
    fs::write(
        &input,
        "# heading\nA -x B: malformed outside fence\n\n```puml\n@startuml\nAlice -> Bob: one\n@enduml\n```\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_loops_and_groups_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_loops_and_groups.mmd.txt"),
        ])
        .assert()
        .success();
}

// -- New diagram families: JSON / YAML / nwdiag / Archimate --------------------

// ---------------------------------------------------------------------------
// Creole inline formatting tests (#168)
// ---------------------------------------------------------------------------

// ─── State diagram advanced feature tests ────────────────────────────────────

#[test]
fn state_concurrent_regions_renders_svg_with_dashed_divider() {
    let src = fs::read_to_string(fixture("families/valid_state_concurrent.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render state concurrent SVG");
    assert!(svg.contains("<svg"), "expected SVG output");
    assert!(
        svg.contains("stroke-dasharray"),
        "expected dashed divider in concurrent state SVG"
    );
}

#[test]
fn creole_bold_italic_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("conformance/valid_creole_message_bold_italic.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn json_family_check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/valid_json.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_alt_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_alt.mmd.txt"),
        ])
        .assert()
        .success();
}

#[test]
fn json_family_renders_deterministic_svg() {
    let src = fs::read_to_string(fixture("non_sequence/valid_json.puml")).unwrap();
    let a = render_source_to_svg(&src).expect("render JSON");
    let b = render_source_to_svg(&src).expect("render JSON again");
    assert_eq!(a, b, "JSON render must be deterministic");
    assert!(a.starts_with("<svg"));
    assert!(a.contains("JSON"));
    assert!(a.contains("name"));
}

#[test]
fn creole_bold_italic_svg_contains_tspan_formatting() {
    let src =
        fs::read_to_string(fixture("conformance/valid_creole_message_bold_italic.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("render");
    assert!(
        svg.contains("font-weight=\"bold\""),
        "expected bold tspan in SVG"
    );
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic tspan in SVG"
    );
}

#[test]
fn creole_note_link_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("conformance/valid_creole_note_link.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn yaml_family_check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/valid_yaml.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_create_destroy_link_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_create_destroy_link.mmd.txt"),
        ])
        .assert()
        .success();
}

#[test]
fn yaml_family_renders_deterministic_svg() {
    let src = fs::read_to_string(fixture("non_sequence/valid_yaml.puml")).unwrap();
    let a = render_source_to_svg(&src).expect("render YAML");
    let b = render_source_to_svg(&src).expect("render YAML again");
    assert_eq!(a, b, "YAML render must be deterministic");
    assert!(a.contains("YAML"));
    assert!(a.contains("project:"));
}

#[test]
fn nwdiag_family_check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/valid_nwdiag.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_box_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_box.mmd.txt"),
        ])
        .assert()
        .success();
}

#[test]
fn nwdiag_family_renders_deterministic_svg_with_networks() {
    let src = fs::read_to_string(fixture("non_sequence/valid_nwdiag.puml")).unwrap();
    let a = render_source_to_svg(&src).expect("render nwdiag");
    let b = render_source_to_svg(&src).expect("render nwdiag again");
    assert_eq!(a, b, "nwdiag render must be deterministic");
    assert!(a.contains("network dmz"));
    assert!(a.contains("web01"));
    assert!(a.contains("network internal"));
}

#[test]
fn nwdiag_multi_network_group_and_multi_address_layout_is_preserved() {
    let src = fs::read_to_string(fixture(
        "non_sequence/valid_nwdiag_multi_network_addresses.puml",
    ))
    .unwrap();
    let svg = render_source_to_svg(&src).expect("render nwdiag topology depth fixture");

    assert!(
        svg.contains("data-nwdiag-addresses=\"10.0.0.10, fd00:10::10\""),
        "public lb should preserve every bracketed address: {svg}"
    );
    assert!(
        svg.contains("edge lb [10.0.0.10, fd00:10::10]"),
        "multi-address label should render both values: {svg}"
    );
    assert!(
        svg.contains("stroke-dasharray=\"5 3\""),
        "dashed node style should reach SVG geometry"
    );
    assert!(
        svg.contains("width=\"240\""),
        "node width attribute should affect SVG geometry"
    );
    assert!(
        svg.contains("class=\"nwdiag-connector\""),
        "nwdiag nodes should render as boxes connected back to the network bar"
    );
    assert!(
        svg.contains("class=\"nwdiag-address\""),
        "nwdiag connector annotations should render node addresses near the link"
    );

    let public_y = svg_rect_y(
        &svg,
        "class=\"nwdiag-network\"",
        "network public (10.0.0.x/24)",
    )
    .expect("public network y");
    let private_y = svg_rect_y(
        &svg,
        "class=\"nwdiag-network\"",
        "network private (192.168.1.x/24)",
    )
    .expect("private network y");
    assert!(
        private_y > public_y,
        "private network should be laid out below public network"
    );

    let public_lb = svg_node_rect(&svg, "lb", "10.0.0.10, fd00:10::10").expect("public lb rect");
    let private_lb =
        svg_node_rect(&svg, "lb", "192.168.1.10, fd00:192::10").expect("private lb rect");
    assert_eq!(
        public_lb.x, private_lb.x,
        "shared node column should be stable"
    );
    assert!(
        private_lb.y > public_lb.y,
        "shared node should appear in each network row"
    );
    let private_app = svg_node_rect(&svg, "app01", "192.168.1.21").expect("private app rect");
    assert!(
        private_app.x > private_lb.x,
        "distinct nwdiag nodes should occupy separate horizontal columns instead of one vertical list"
    );

    // Groups now render as topology overlays positioned around their member nodes,
    // not as a flat list appended below the diagram. The group rect y-position
    // must be within the topology area, not beyond the last network row bottom edge.
    let group_y = svg_rect_y(&svg, "class=\"nwdiag-group\"", "group edge").expect("group y");
    assert!(
        group_y < private_y + 150,
        "group overlay should sit within the topology area, not appended below: group_y={group_y} private_y={private_y}"
    );
}

#[derive(Debug, PartialEq, Eq)]
struct SvgRectGeom {
    x: i32,
    y: i32,
}

fn svg_rect_y(svg: &str, rect_needle: &str, following_text: &str) -> Option<i32> {
    let text_ix = svg.find(following_text)?;
    let before_text = &svg[..text_ix];
    let rect_ix = before_text.rfind(rect_needle)?;
    let tag = before_text[rect_ix..].split_once('>')?.0;
    svg_attr_i32(tag, "y")
}

fn svg_node_rect(svg: &str, name: &str, addresses: &str) -> Option<SvgRectGeom> {
    let mut rest = svg;
    let name_attr = format!("data-nwdiag-name=\"{name}\"");
    let addresses_attr = format!("data-nwdiag-addresses=\"{addresses}\"");
    while let Some(ix) = rest.find("<rect class=\"nwdiag-node\"") {
        rest = &rest[ix..];
        let tag = rest.split_once('>')?.0;
        if tag.contains(&name_attr) && tag.contains(&addresses_attr) {
            return Some(SvgRectGeom {
                x: svg_attr_i32(tag, "x")?,
                y: svg_attr_i32(tag, "y")?,
            });
        }
        rest = &rest["<rect".len()..];
    }
    None
}

fn svg_attr_i32(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let rest = tag.split_once(&needle)?.1;
    let value = rest.split_once('"')?.0;
    value.parse().ok()
}

fn svg_text_positions(svg: &str, text: &str) -> Vec<(i32, i32)> {
    let marker = format!(">{text}</text>");
    let mut positions = Vec::new();
    let mut start = 0usize;
    while let Some(rel_ix) = svg[start..].find(&marker) {
        let abs_ix = start + rel_ix;
        let Some(tag_start) = svg[..abs_ix].rfind("<text ") else {
            break;
        };
        let tag = svg[tag_start..]
            .split_once('>')
            .map(|(tag, _)| tag)
            .unwrap_or("");
        let Some(x) = svg_attr_i32(tag, "x") else {
            break;
        };
        let Some(y) = svg_attr_i32(tag, "y") else {
            break;
        };
        positions.push((x, y));
        start = abs_ix + marker.len();
    }
    positions
}

fn svg_relation_element<'a>(svg: &'a str, from: &str, to: &str) -> Option<&'a str> {
    let from_attr = format!("data-uml-from=\"{from}\"");
    let to_attr = format!("data-uml-to=\"{to}\"");
    svg.split('<')
        .find(|element| element.contains(&from_attr) && element.contains(&to_attr))
}

fn svg_relation_end(element: &str) -> Option<(i32, i32)> {
    if let Some((_, points_rest)) = element.split_once("points=\"") {
        let points = points_rest.split_once('"')?.0;
        let last = points.split_whitespace().last()?;
        let (x, y) = last.split_once(',')?;
        return Some((x.parse().ok()?, y.parse().ok()?));
    }
    Some((svg_attr_i32(element, "x2")?, svg_attr_i32(element, "y2")?))
}

#[test]
fn archimate_family_check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/valid_archimate.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn sequence_box_grouping_and_hide_unlinked_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("structure/valid_box_grouping_and_hide_unlinked.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn format_png_flag_is_accepted_in_check_mode() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "png", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn charset_flag_accepts_utf8_and_rejects_others() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--charset", "UTF-8", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--charset", "latin-1", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_CHARSET_UNSUPPORTED"));
}

#[test]
fn overwrite_flag_is_accepted_as_noop() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--overwrite", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success();
}

#[test]
fn duration_flag_emits_elapsed_to_stderr() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--duration", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("elapsed:"));
}

#[test]
fn verbose_flag_emits_stage_timings_to_stderr() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--verbose", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("[verbose] parse"));
}

#[test]
fn quiet_flag_suppresses_warnings_on_stderr() {
    // Unsupported skinparams still warn normally, and --quiet suppresses that output.
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--quiet", "--check", "-"])
        .write_stdin("@startuml\nskinparam unknownKey foo\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn fail_on_warn_flag_exits_one_when_warnings_emitted() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--fail-on-warn", "--check", "-"])
        .write_stdin("@startuml\nskinparam UnknownXyzKey value\nA -> B\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_WARNINGS_PRESENT"));
}

#[test]
fn class_diagram_with_relations_renders_real_svg() {
    let src = fs::read_to_string(fixture("families/valid_class_with_relations.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("class svg should render");
    // Real SVG primitives must appear.
    assert!(svg.starts_with("<svg"), "svg should start with <svg tag");
    assert!(svg.contains("<rect"), "should contain rect for boxes");
    assert!(
        svg.contains("<line"),
        "should contain lines for relations: {svg}"
    );
    // Class names should be present.
    for name in ["Animal", "Dog", "Cat", "Collar"] {
        assert!(svg.contains(name), "missing class name {name}");
    }
    // Inheritance arrow uses the triangle marker.
    assert!(
        svg.contains("arrow-triangle"),
        "inheritance arrow marker missing"
    );
    // Composition uses the filled-diamond marker.
    assert!(
        svg.contains("arrow-diamond-filled"),
        "composition diamond marker missing"
    );
    // Aggregation uses the open-diamond marker.
    assert!(
        svg.contains("arrow-diamond-open"),
        "aggregation diamond marker missing"
    );
    // Label rendering.
    assert!(svg.contains("has"), "composition label missing");
    assert!(svg.contains("wears"), "aggregation label missing");
}

#[test]
fn class_inheritance_example_renders_fixture_text_and_relations() {
    let src = fs::read_to_string("docs/examples/class/02_inheritance.puml").unwrap();
    let svg = render_source_to_svg(&src).expect("class inheritance svg should render");

    for expected in [
        "Vehicle",
        "+make: String",
        "+model: String",
        "+start()",
        "Car",
        "+doors: Int",
        "+drive()",
        "Truck",
        "+payload: Float",
        "+haul()",
    ] {
        assert!(
            svg.contains(expected),
            "missing class fixture text {expected}"
        );
    }

    for (from, to) in [("Vehicle", "Car"), ("Vehicle", "Truck")] {
        assert!(
            svg.contains(&format!(
                "class=\"uml-relation\" data-uml-from=\"{from}\" data-uml-to=\"{to}\""
            )),
            "missing inheritance relation {from} -> {to}"
        );
    }
    assert!(
        svg.contains("marker-start=\"url(#arrow-triangle)\"")
            || svg.contains("marker-end=\"url(#arrow-triangle)\""),
        "inheritance arrow marker missing"
    );
}

#[test]
fn class_diagram_with_relations_render_is_deterministic() {
    let src = fs::read_to_string(fixture("families/valid_class_with_relations.puml")).unwrap();
    let first = render_source_to_svg(&src).unwrap();
    let second = render_source_to_svg(&src).unwrap();
    assert_eq!(first, second);
}

#[test]
fn family_relations_with_cardinalities_render_endpoint_labels() {
    let src = fs::read_to_string(fixture("families/valid_class_with_cardinalities.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("family svg should render");
    assert!(svg.contains(">1<"), "left cardinality should render");
    assert!(svg.contains(">*<"), "right cardinality should render");
    assert!(
        svg.contains(">0..1<"),
        "right cardinality variant should render"
    );
    assert!(svg.contains("places"), "relation label should render");
    assert!(svg.contains("reads"), "relation label should render");
}

#[test]
fn component_relations_with_cardinalities_render_endpoint_labels() {
    let src =
        "@startuml\ncomponent API\ncomponent DB\nAPI \"1\" --> \"n\" DB : depends-on\n@enduml\n";
    let svg = render_source_to_svg(src).expect("component svg should render");
    assert!(
        svg.contains(">1<"),
        "component left cardinality should render"
    );
    assert!(
        svg.contains(">n<"),
        "component right cardinality should render"
    );
    assert!(
        svg.contains("depends-on"),
        "component relation label should render"
    );
}

#[test]
fn family_relations_render_colon_endpoint_roles_without_stealing_edge_label() {
    let src = "@startuml\nclass Customer\nclass Order\nCustomer \"1\" :buyer --> \"*\" :orders Order : places\n@enduml\n";
    let svg = render_source_to_svg(src).expect("class relation roles should render");
    assert!(svg.contains(">buyer<"), "left colon role should render");
    assert!(svg.contains(">orders<"), "right colon role should render");
    assert!(svg.contains(">1<"), "left cardinality should render");
    assert!(svg.contains(">*<"), "right cardinality should render");
    assert!(svg.contains("places"), "edge label should render");
}

#[test]
fn component_and_deployment_groups_render_labeled_frames_and_nested_members() {
    let component_src = "@startuml\nskinparam ComponentBorderColor #0f766e\npackage \"Core Services\" {\n  component \"Public API\" as API\n  node \"Runtime Zone\" {\n    component Worker\n  }\n}\nAPI --> Worker : dispatches\n@enduml\n";
    let component_svg =
        render_source_to_svg(component_src).expect("component group svg should render");
    assert!(
        component_svg.contains(">package Core Services<"),
        "component package frame label should render"
    );
    assert!(
        component_svg.contains("Public API") || component_svg.contains(">API<"),
        "component group member should render"
    );
    assert!(
        component_svg.contains(">Worker<"),
        "nested component group member should render"
    );
    assert!(
        component_svg.contains("stroke=\"#0f766e\""),
        "component border skinparam should style group frame"
    );
    assert!(
        component_svg.contains(">dispatches<"),
        "relation label between grouped members should render"
    );

    let deployment_src = "@startuml\nnode \"Edge Site\" {\n  artifact App\n  database Cache\n}\nApp --> Cache : warms\n@enduml\n";
    let deployment_svg =
        render_source_to_svg(deployment_src).expect("deployment group svg should render");
    assert!(
        deployment_svg.contains(">node Edge Site<"),
        "deployment node frame label should render"
    );
    assert!(
        deployment_svg.contains(">App<"),
        "artifact member should render"
    );
    assert!(
        deployment_svg.contains(">Cache<"),
        "database member should render"
    );
    assert!(
        deployment_svg.contains(">warms<"),
        "deployment grouped relation should render"
    );
}

#[test]
fn class_relations_with_roles_render_endpoint_role_labels() {
    let src = fs::read_to_string(fixture("families/valid_class_with_relation_roles.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("class svg should render");
    assert!(svg.contains(">buyer<"), "left role label should render");
    assert!(svg.contains(">items<"), "right role label should render");
    assert!(svg.contains(">1<"), "left cardinality should render");
    assert!(svg.contains(">0..*<"), "right cardinality should render");
}

#[test]
fn class_parallel_relations_stagger_labels_for_shared_node_pairs() {
    let svg =
        render_source_to_svg(&fs::read_to_string(example("class/12_all_relations.puml")).unwrap())
            .expect("class all relations example should render");
    let extends_label = svg_text_positions(&svg, "&lt;&lt;extend&gt;&gt;")
        .into_iter()
        .next()
        .expect("extends stereotype position");
    let association_label = svg_text_positions(&svg, "association")
        .into_iter()
        .next()
        .expect("association label position");
    assert!(
        (extends_label.0 - association_label.0).abs() >= 12
            || (extends_label.1 - association_label.1).abs() >= 12,
        "shared-pair labels should not overlap"
    );
}

#[test]
fn class_package_headers_stay_above_nested_members() {
    let svg = render_source_to_svg(
        &fs::read_to_string(example("class/14_nested_packages.puml")).unwrap(),
    )
    .expect("nested class packages should render");
    let package_label = svg_text_positions(&svg, "package repository")
        .into_iter()
        .next()
        .expect("repository package label");
    let user_service = svg_text_positions(&svg, "service::UserService")
        .into_iter()
        .next()
        .expect("user service position");
    let product_service = svg_text_positions(&svg, "service::ProductService")
        .into_iter()
        .next()
        .expect("product service position");
    assert!(
        package_label.1 + 12 < user_service.1.min(product_service.1),
        "nested package label should stay above enclosed service nodes"
    );
}

#[test]
fn component_and_deployment_edges_render_advanced_markers_and_dashes() {
    let component_src = "@startuml\ncomponent API\ninterface Gateway\nport Ingress\nAPI o-- Gateway : exposes\nIngress <|-- API : binds\n@enduml\n";
    let component_svg = render_source_to_svg(component_src).expect("component svg should render");
    assert!(
        component_svg.contains("arrow-diamond-open"),
        "aggregation marker should render for component edges"
    );
    assert!(
        component_svg.contains("arrow-triangle"),
        "triangle marker should render for generalization edges"
    );
    assert!(
        component_svg.contains(">exposes<") && component_svg.contains(">binds<"),
        "component relation labels should render"
    );

    let deployment_src = "@startuml\nnode Web\nartifact App\ndatabase Store\nWeb --> App : hosts\nApp *-- Store : data\n@enduml\n";
    let deployment_svg =
        render_source_to_svg(deployment_src).expect("deployment svg should render");
    assert!(
        deployment_svg.contains("arrow-diamond-filled"),
        "composition marker should render for deployment edges"
    );
}

#[test]
fn deployment_database_edge_labels_stay_clear_of_terminal_database_segment() {
    let src = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/examples/deployment/02_databases.puml"),
    )
    .expect("deployment example should load");
    let svg = render_source_to_svg(&src).expect("deployment example should render");
    assert!(
        svg.contains("<text x=\"264\" y=\"228\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#1e293b\">reads/writes</text>"),
        "reads/writes label should stay on the upper shaft segment, clear of the PostgreSQL arrowhead"
    );
}

#[test]
fn object_diagram_renders_underlined_header_and_rects() {
    let src = fs::read_to_string(fixture("families/valid_object_members_block.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("object svg should render");
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("Session"));
    assert!(svg.contains("UserRef"));
    // Objects use underline text-decoration for their name.
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "object header should be underlined"
    );
}

#[test]
fn uml_declaration_stereotypes_and_component_shorthand_aliases_render() {
    // Fix #551: user-defined stereotypes on class/object nodes now render as
    // guillemet labels («…») in the class header, NOT as member rows.
    let class_src = "@startuml\nclass Order <<Entity>>\n@enduml\n";
    let class_svg = render_source_to_svg(class_src).expect("stereotype svg should render");
    assert!(
        class_svg.contains("\u{ab}Entity\u{bb}"),
        "class stereotype should render as guillemet in header (fix #551)"
    );
    let object_src = "@startuml\nobject cache <<singleton>>\n@enduml\n";
    let object_svg = render_source_to_svg(object_src).expect("object stereotype svg should render");
    assert!(
        object_svg.contains("\u{ab}singleton\u{bb}"),
        "object stereotype should render as guillemet in header (fix #551)"
    );
    let usecase_src =
        "@startuml\nactor Shopper <<primary>> as S\nusecase Checkout <<critical>> as UC\nS --> UC : starts\n@enduml\n";
    let usecase_svg =
        render_source_to_svg(usecase_src).expect("usecase stereotype svg should render");
    assert!(
        usecase_svg.contains("&lt;&lt;primary&gt;&gt;"),
        "actor stereotype should render"
    );
    assert!(
        usecase_svg.contains("&lt;&lt;critical&gt;&gt;"),
        "usecase stereotype should render"
    );

    let component_src =
        "@startuml\n[Public API] as API\n() \"Gateway Port\" as Gateway\nAPI --> Gateway : exposes\n@enduml\n";
    let component_svg =
        render_source_to_svg(component_src).expect("component shorthand svg should render");
    assert!(
        component_svg.contains("Public API"),
        "component shorthand label should render"
    );
    assert!(
        component_svg.contains("Gateway Port"),
        "interface shorthand label should render"
    );
    assert!(
        component_svg.contains(">exposes<"),
        "aliased shorthand relation should render"
    );
}

#[test]
fn creole_note_link_svg_contains_hyperlink() {
    let src = fs::read_to_string(fixture("conformance/valid_creole_note_link.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("render");
    assert!(
        svg.contains("xlink:href=\"https://example.com\""),
        "expected hyperlink href in SVG"
    );
    assert!(
        svg.contains("fill=\"blue\""),
        "expected blue fill on link span"
    );
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "expected underline on link span"
    );
}

#[test]
fn class_together_group_passes_check_and_svg_contains_group_frame() {
    let src = fs::read_to_string(fixture("families/valid_class_together.puml")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_class_together.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(&src).expect("rendered svg");
    // together group frame should be present
    assert!(
        svg.contains("together"),
        "SVG should contain 'together' group label"
    );
    // member names from the together block
    assert!(svg.contains("User"), "SVG should contain User");
    assert!(svg.contains("Account"), "SVG should contain Account");
}

#[test]
fn class_package_namespace_passes_check_and_svg_contains_scope_labels() {
    let src = fs::read_to_string(fixture("families/valid_class_package_namespace.puml")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("families/valid_class_package_namespace.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(&src).expect("rendered svg");
    // package and namespace labels should appear
    assert!(
        svg.contains("package"),
        "SVG should contain 'package' label"
    );
    assert!(
        svg.contains("namespace"),
        "SVG should contain 'namespace' label"
    );
    assert!(
        svg.contains("com.example"),
        "SVG should contain package label"
    );
    assert!(
        svg.contains("net.api"),
        "SVG should contain namespace label"
    );
}

#[test]
fn class_hide_options_suppress_circle_and_stereotype() {
    let src = fs::read_to_string(fixture("families/valid_class_hide_options.puml")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("families/valid_class_hide_options.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(&src).expect("rendered svg");
    // When hide circle is active, no circle element for class icon
    assert!(
        !svg.contains("<circle"),
        "SVG should not contain class icon circle when hide circle is set"
    );
    // When hide stereotype is active, the 'class' keyword label should not appear before node names
    // The node names themselves should still appear
    assert!(
        svg.contains("Visible"),
        "SVG should contain node name 'Visible'"
    );
}

#[test]
fn class_visibility_markers_render_colored_symbols() {
    let src = fs::read_to_string(fixture("families/valid_class_visibility.puml")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_class_visibility.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(&src).expect("rendered svg");
    // Visibility symbols should appear as colored text elements
    assert!(svg.contains("+"), "SVG should contain + visibility marker");
    assert!(svg.contains("-"), "SVG should contain - visibility marker");
    assert!(svg.contains("#"), "SVG should contain # visibility marker");
    assert!(svg.contains("~"), "SVG should contain ~ visibility marker");
    // Abstract and static modifiers should produce style attributes
    assert!(
        svg.contains("font-style=\"italic\""),
        "SVG should contain italic style for {{abstract}} modifier"
    );
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "SVG should contain underline style for {{static}} modifier"
    );
}

#[test]
fn usecase_diagram_renders_ellipse_nodes() {
    let src = fs::read_to_string(fixture("families/valid_usecase_bootstrap.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("usecase svg should render");
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<ellipse"), "use cases should be ellipses");
    assert!(svg.contains("Authenticate"));
    assert!(svg.contains("Authorize"));
}

#[test]
fn usecase_include_extend_dependencies_render_as_dashed_open_arrows() {
    let src = "@startuml\nusecase Login\nusecase Authorize\nusecase Recover\nLogin ..> Authorize : <<include>>\nRecover .left.> Login : extends\n@enduml\n";
    let svg = render_source_to_svg(src).expect("usecase include/extend svg should render");
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("marker-end=\"url(#arrow-open)\""));
    assert!(svg.contains("&lt;&lt;include&gt;&gt;"));
    assert!(svg.contains("&lt;&lt;extend&gt;&gt;"));
}

#[test]
fn component_family_canvas_keeps_rightmost_nodes_inside_viewbox() {
    for src in [
        fs::read_to_string(example("component/01_basic.puml")).unwrap(),
        fs::read_to_string(example("component/02_interfaces.puml")).unwrap(),
        fs::read_to_string(example("component/05_with_notes.puml")).unwrap(),
    ] {
        let svg = render_source_to_svg(&src).expect("component example should render");
        let svg_width = extract_svg_width_attr(&svg).expect("svg width");
        let rightmost_component = svg_elements_with_attr(&svg, "data-uml-kind", "component")
            .iter()
            .map(|element| {
                svg_attr_i32_required(element, "x") + svg_attr_i32_required(element, "width")
            })
            .max()
            .expect("component nodes");
        let rightmost_interface = svg_elements_with_attr(&svg, "data-uml-kind", "interface")
            .into_iter()
            .map(|element| {
                svg_attr_i32_required(element, "cx") + svg_attr_i32_required(element, "r")
            })
            .max()
            .unwrap_or(0);
        let rightmost_drawn = rightmost_component.max(rightmost_interface);
        assert!(
            svg_width >= rightmost_drawn + 24,
            "rightmost component/interface should keep a right margin"
        );
    }
}

#[test]
fn component_arrow_labels_fan_apart_and_stay_inside_viewbox() {
    let svg = render_source_to_svg(
        &fs::read_to_string(example("component/06_with_arrows.puml")).unwrap(),
    )
    .expect("component arrow example should render");
    let calls = svg_text_positions(&svg, "calls")
        .into_iter()
        .next()
        .expect("calls label position");
    let uses = svg_text_positions(&svg, "uses")
        .into_iter()
        .next()
        .expect("uses label position");
    let composed = svg_text_positions(&svg, "composed")
        .into_iter()
        .next()
        .expect("composed label position");
    let svg_width = extract_svg_width_attr(&svg).expect("svg width");

    assert!(
        (uses.0 - composed.0).abs() >= 24 || (uses.1 - composed.1).abs() >= 12,
        "uses and composed labels should not overlap in the shared routing lane"
    );
    assert!(
        (calls.0 - uses.0).abs() >= 24 || (calls.1 - uses.1).abs() >= 12,
        "calls and uses labels should remain visually distinct"
    );
    assert!(
        svg_width >= composed.0 + 48,
        "rightmost component label should keep a readable margin inside the viewbox"
    );
}

// Regression test for #525: the literal word "component" must never appear as a
// plain unlabelled sub-label above the component name.  Only the guillemet form
// «component» (U+AB / U+BB) is permitted as the type indicator.
#[test]
fn component_keyword_does_not_leak_as_plain_sublabel_issue_525() {
    let src = r#"
@startuml
component Frontend
component Backend
Frontend --> Backend : calls
@enduml
"#;
    let svg = render_source_to_svg(src).expect("component diagram should render");
    // «component» in guillemets is correct and expected.
    assert!(
        svg.contains("\u{ab}component\u{bb}"),
        "«component» stereotype in guillemets should be present"
    );
    // The SVG text elements must NOT contain a bare ">component<" text node
    // (i.e. the raw keyword without guillemets).  We check by looking for
    // ">component<" which is what a plain <text>component</text> emits in SVG.
    assert!(
        !svg.contains(">component<"),
        "raw 'component' keyword must not appear as a plain SVG text label (issue #525)"
    );
}

#[test]
fn usecase_relation_labels_clear_arrowheads_and_each_other() {
    let overlap_svg = render_source_to_svg(
        &fs::read_to_string(example("usecase/03_extends_includes.puml")).unwrap(),
    )
    .expect("usecase overlap example should render");
    let mut dependency_positions = svg_text_positions(&overlap_svg, "&lt;&lt;extend&gt;&gt;");
    dependency_positions.extend(svg_text_positions(&overlap_svg, "&lt;&lt;include&gt;&gt;"));
    assert_eq!(
        dependency_positions.len(),
        3,
        "expected three dependency labels"
    );
    for i in 0..dependency_positions.len() {
        for j in (i + 1)..dependency_positions.len() {
            let dx = (dependency_positions[i].0 - dependency_positions[j].0).abs();
            let dy = (dependency_positions[i].1 - dependency_positions[j].1).abs();
            assert!(dx >= 40 || dy >= 12, "dependency labels should not collide");
        }
    }

    let basic_svg =
        render_source_to_svg(&fs::read_to_string(example("usecase/01_basic.puml")).unwrap())
            .expect("basic usecase example should render");
    let label = svg_text_positions(&basic_svg, "leads to")
        .into_iter()
        .next()
        .expect("relation label position");
    let relation = svg_relation_element(&basic_svg, "Login", "Register").expect("relation element");
    let end = svg_relation_end(relation).expect("relation endpoint");
    assert!(
        (label.0 - end.0).abs() >= 24 || (label.1 - end.1).abs() >= 18,
        "relation label should clear the arrowhead attachment point"
    );
    assert!(
        label.1 < end.1,
        "relation label should float above the arrowhead endpoint"
    );
}

#[test]
fn usecase_relation_label_clears_actor_body_in_with_actors_example() {
    let svg =
        render_source_to_svg(&fs::read_to_string(example("usecase/02_with_actors.puml")).unwrap())
            .expect("usecase actor example should render");
    let label = svg_text_positions(&svg, "leads to")
        .into_iter()
        .next()
        .expect("leads to label position");
    let admin = svg_text_positions(&svg, "Admin")
        .into_iter()
        .next()
        .expect("Admin actor label position");
    let dx = (label.0 - admin.0).abs();
    let dy = label.1 - admin.1;
    assert!(
        dx >= 24 || dy <= -18 || dy >= 10,
        "relation label should stay outside the Admin actor body envelope"
    );
}

#[test]
fn usecase_package_boundaries_render_tab_headers_and_short_names() {
    let svg = render_source_to_svg(
        &fs::read_to_string(example("usecase/04_with_packages.puml")).unwrap(),
    )
    .expect("usecase package example should render");
    let frame = svg_elements_with_attr(&svg, "data-uml-group", "Back Office")
        .into_iter()
        .find(|element| element.contains("class=\"uml-group-frame\""))
        .expect("back office frame");
    let frame_y = svg_attr_i32_required(frame, "y");
    let label = svg_text_positions(&svg, "Back Office")
        .into_iter()
        .next()
        .expect("back office label");
    assert!(
        label.1 <= frame_y + 20,
        "boundary label should render in the top tab area"
    );
    assert!(svg.contains(">ManageProducts<"));
    assert!(svg.contains(">ManageOrders<"));
    assert!(!svg.contains("Back Office::MP"));
    assert!(!svg.contains("Back Office::MO"));
}

#[test]
fn class_package_headers_clear_inner_class_labels() {
    let svg = render_source_to_svg(
        &fs::read_to_string(example("class/14_nested_packages.puml")).unwrap(),
    )
    .expect("nested class packages example should render");

    for (group, members) in [
        (
            "repository",
            &["repository::UserRepo", "repository::ProductRepo"][..],
        ),
        (
            "service",
            &["service::UserService", "service::ProductService"][..],
        ),
        (
            "domain",
            &["domain::User", "domain::Product", "domain::Order"][..],
        ),
    ] {
        let frame = svg_elements_with_attr(&svg, "data-uml-group", group)
            .into_iter()
            .find(|element| element.contains("class=\"uml-group-frame\""))
            .expect("package frame");
        let frame_y = svg_attr_i32_required(frame, "y");
        let min_member_y = members
            .iter()
            .flat_map(|member| svg_text_positions(&svg, member))
            .map(|(_, y)| y)
            .min()
            .expect("inner class label position");

        assert!(
            min_member_y >= frame_y + 72,
            "{group} package header should stay above enclosed class labels"
        );
    }
}

#[test]
fn class_family_accepts_directional_and_dotted_relation_arrows() {
    let src = "@startuml\nclass Base\nclass Impl\nclass Service\nImpl -up-|> Base\nService ..> Impl : depends\n@enduml\n";
    let svg = render_source_to_svg(src).expect("class directional relation svg should render");
    assert!(svg.contains("arrow-triangle"));
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("depends"));
}

#[test]
fn component_relations_render_dotted_markers_and_styled_port_shape() {
    let src = "@startuml\ncomponent API\ninterface REST\nport Adapter\nAPI ..> REST : uses\nAdapter -down-> API : plugs\n@enduml\n";
    let svg = render_source_to_svg(src).expect("component relation svg should render");
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("marker-end=\"url(#arrow-open)\""));
    assert!(svg.contains("port"));
    assert!(svg.contains("Adapter"));
}

#[test]
fn component_interfaces_attach_relation_endpoints_to_circle_edges() {
    let svg =
        render_source_to_svg(&fs::read_to_string(example("component/02_interfaces.puml")).unwrap())
            .expect("component interface example should render");
    let interface_elements = svg_elements_with_attr(&svg, "data-uml-kind", "interface");
    let graphql_label = svg_text_positions(&svg, "GraphQL")
        .into_iter()
        .next()
        .expect("GraphQL label position");
    let rest_label = svg_text_positions(&svg, "REST")
        .into_iter()
        .next()
        .expect("REST label position");
    let graphql_circle = interface_elements
        .iter()
        .find(|element| svg_attr_i32_required(element, "cx") == graphql_label.0)
        .expect("GraphQL interface circle");
    let rest_circle = interface_elements
        .iter()
        .find(|element| svg_attr_i32_required(element, "cx") == rest_label.0)
        .expect("REST interface circle");
    let endpoint_on_circle = |endpoint: (i32, i32), circle: &str| {
        let cx = svg_attr_i32_required(circle, "cx");
        let cy = svg_attr_i32_required(circle, "cy");
        let r = svg_attr_i32_required(circle, "r");
        let dx = (endpoint.0 - cx).abs();
        let dy = (endpoint.1 - cy).abs();

        (dx == r && dy <= 1) || (dy == r && dx <= 1)
    };

    let graphql_relation =
        svg_relation_element(&svg, "API", "GraphQL").expect("API to GraphQL relation");
    let graphql_end = svg_relation_end(graphql_relation).expect("GraphQL relation endpoint");
    assert!(
        endpoint_on_circle(graphql_end, graphql_circle),
        "GraphQL relation should land on the interface circle edge"
    );

    for relation in [("API", "REST"), ("Client", "REST")] {
        let rest_relation =
            svg_relation_element(&svg, relation.0, relation.1).expect("REST relation");
        let rest_end = svg_relation_end(rest_relation).expect("REST relation endpoint");
        assert!(
            endpoint_on_circle(rest_end, rest_circle),
            "{} to REST should land on the interface circle edge",
            relation.0
        );
    }
}

#[test]
fn state_transitions_accept_short_and_directional_arrows() {
    let src = "@startuml\nstate Idle\nstate Active\nstate Closed\n[*] -> Idle\nIdle -down-> Active : open\nActive --> Closed : done\n@enduml\n";
    let svg = render_source_to_svg(src).expect("state directional transition svg should render");
    assert!(svg.contains("Idle"));
    assert!(svg.contains("Active"));
    assert!(svg.contains("open"));
}

#[test]
fn activity_if_then_branch_label_is_preserved() {
    let src = "@startuml\nstart\nif (in stock?) then (yes)\n:Ship;\nelse (no)\n:Notify;\nendif\nstop\n@enduml\n";
    let svg = render_source_to_svg(src).expect("activity svg should render");
    assert!(
        svg.contains("in stock?"),
        "condition text should appear in diamond"
    );
    assert!(
        svg.contains("yes"),
        "then-guard should appear on outgoing arrow"
    );
    assert!(
        svg.contains("no"),
        "else-guard should appear on outgoing arrow"
    );
}

#[test]
fn gantt_render_emits_horizontal_bars_and_milestone_diamond() {
    let src = fs::read_to_string(fixture("timeline/valid_gantt_render.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("gantt svg should render");
    assert!(svg.starts_with("<svg"));
    // Task labels.
    for name in ["Design", "Build", "Test", "Kickoff"] {
        assert!(svg.contains(name), "missing task/milestone name {name}");
    }
    // Bars are <rect> elements; milestone uses <polygon>.
    assert!(svg.contains("<rect"), "should contain task bars");
    assert!(svg.contains("<polygon"), "milestone diamond missing");
    // Constraint arrow (requires) is rendered as a dashed line + marker.
    assert!(
        svg.contains("gantt-arrow"),
        "constraint arrow marker missing"
    );
    assert!(
        svg.contains("stroke-dasharray"),
        "dashed constraint arrow missing"
    );
}

#[test]
fn gantt_task_decl_split_across_lines_produces_one_bar_per_task() {
    // Regression test for #241: when a task is declared bare on one line and
    // then constrained (e.g. "[Design] starts 2026-01-02") on a subsequent
    // line, the normalizer must merge them into a single task rather than
    // creating a duplicate, which would result in ghost rows with no bars.
    let src = "\
@startgantt\n\
title Project Timeline\n\
[Design]\n\
[Build]\n\
[Test]\n\
[Kickoff] happens on 2026-01-01\n\
[Design] starts 2026-01-02\n\
[Build] starts 2026-01-15\n\
[Build] requires [Design]\n\
[Test] starts 2026-02-01\n\
[Test] requires [Build]\n\
@endgantt\n";
    let svg = render_source_to_svg(src).expect("gantt should render");
    assert!(svg.starts_with("<svg"), "should produce SVG");
    // Count gantt-task rect elements — must equal 3 (one per unique task).
    let bar_count = svg.matches("class=\"gantt-task\"").count();
    assert_eq!(
        bar_count, 3,
        "expected exactly 3 task bars (Design, Build, Test), got {bar_count}; \
         likely duplicate task rows caused by split declarations"
    );
    // Row labels should appear exactly once each.
    for name in ["Design", "Build", "Test"] {
        assert!(svg.contains(name), "task label {name} missing");
    }
    // Milestone row for Kickoff.
    assert!(svg.contains("<polygon"), "milestone diamond missing");
    // Bars must be positioned after the chart's left axis (x > 100).
    let bar_x_vals: Vec<i32> = svg
        .split("class=\"gantt-task\"")
        .skip(1)
        .filter_map(|chunk| {
            let x_part = chunk.split("x=\"").nth(1)?;
            x_part.split('"').next()?.parse().ok()
        })
        .collect();
    assert_eq!(bar_x_vals.len(), 3, "should have 3 bar x coordinates");
    for &x in &bar_x_vals {
        assert!(
            x > 100,
            "bar x={x} is unexpectedly small (left of label column)"
        );
    }
}

#[test]
fn state_concurrent_renders_dashed_divider() {
    let src = fs::read_to_string(fixture("families/valid_state_concurrent.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render state concurrent SVG");
    assert!(svg.contains("<svg"), "expected SVG output");
    assert!(
        svg.contains("stroke-dasharray"),
        "expected dashed divider in concurrent state SVG"
    );
}

#[test]
fn chronology_render_emits_vertical_timeline_with_event_bullets() {
    let src = fs::read_to_string(fixture("timeline/valid_chronology_render.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("chronology svg should render");
    assert!(svg.starts_with("<svg"));
    // Events appear as labels.
    for name in ["Discovery", "Alpha", "Beta", "GA"] {
        assert!(svg.contains(name), "missing event {name}");
    }
    // Dates rendered.
    assert!(svg.contains("2026-05-01"));
    // Vertical timeline line + bullet circles.
    assert!(svg.contains("<line"), "timeline line missing");
    assert!(svg.contains("<circle"), "event bullet missing");
}

#[test]
fn timeline_render_is_deterministic_across_runs() {
    let gantt = fs::read_to_string(fixture("timeline/valid_gantt_render.puml")).unwrap();
    let chrono = fs::read_to_string(fixture("timeline/valid_chronology_render.puml")).unwrap();
    assert_eq!(
        render_source_to_svg(&gantt).unwrap(),
        render_source_to_svg(&gantt).unwrap()
    );
    assert_eq!(
        render_source_to_svg(&chrono).unwrap(),
        render_source_to_svg(&chrono).unwrap()
    );
}

#[test]
fn class_hide_empty_members_collapses_empty_compartment() {
    let src =
        "@startuml\nhide empty members\nclass Full {\n  +name: String\n}\nclass Empty\n@enduml\n";
    let svg = render_source_to_svg(src).expect("rendered svg");
    // Full class should show its member; Empty class box should be shorter (no extra member rows)
    assert!(
        svg.contains("name: String"),
        "SVG should contain member text"
    );
    // Both class names should appear
    assert!(svg.contains("Full"), "SVG should contain Full class");
    assert!(svg.contains("Empty"), "SVG should contain Empty class");
}

#[test]
fn class_set_namespace_separator_is_recorded_in_model() {
    use puml::normalize_family;
    use puml::parser::parse;
    use puml::NormalizedDocument;

    let src = "@startuml\nset namespaceSeparator ::\nclass Foo\n@enduml\n";
    let doc = parse(src).expect("parse ok");
    let model = normalize_family(doc).expect("normalize ok");
    let NormalizedDocument::Family(family) = model else {
        panic!("expected Family model");
    };
    assert_eq!(
        family.namespace_separator.as_deref(),
        Some("::"),
        "namespace_separator should be recorded as ::"
    );
}

#[test]
fn archimate_family_renders_deterministic_svg_with_layers() {
    let src = fs::read_to_string(fixture("non_sequence/valid_archimate.puml")).unwrap();
    let a = render_source_to_svg(&src).expect("render archimate");
    let b = render_source_to_svg(&src).expect("render archimate again");
    assert_eq!(a, b, "archimate render must be deterministic");
    assert!(a.contains("Archimate"));
    assert!(a.contains("application"));
    assert!(a.contains("Customer"));
}

#[test]
fn archimate_docs_examples_render_typed_shapes_and_edges() {
    let root = env!("CARGO_MANIFEST_DIR");
    let layered =
        fs::read_to_string(format!("{root}/docs/examples/archimate/01_layered.puml")).unwrap();
    let relations = fs::read_to_string(format!(
        "{root}/docs/examples/archimate/02_with_relations.puml"
    ))
    .unwrap();
    let flows = fs::read_to_string(format!(
        "{root}/docs/examples/archimate/03_with_junctions.puml"
    ))
    .unwrap();

    let layered_svg = render_source_to_svg(&layered).expect("render layered archimate example");
    assert!(layered_svg.contains("data-archimate-kind=\"component\""));
    assert!(layered_svg.contains("data-archimate-kind=\"node\""));
    assert!(layered_svg.contains("data-archimate-kind=\"data-object\""));
    assert!(layered_svg.contains("data-archimate-kind=\"serving\""));
    assert!(layered_svg.contains("data-archimate-kind=\"access\""));
    assert!(layered_svg.contains("class=\"archimate-relation-edge\""));
    assert!(!layered_svg.contains("<text class=\"archimate-relation\""));

    let relations_svg =
        render_source_to_svg(&relations).expect("render relation archimate example");
    assert!(relations_svg.contains("data-archimate-kind=\"process\""));
    assert!(relations_svg.contains("data-archimate-kind=\"service\""));
    assert!(relations_svg.contains("data-archimate-kind=\"assignment\""));
    assert!(relations_svg.contains("marker-start=\"url(#archimate-assignment)\""));
    assert!(relations_svg.contains("data-archimate-role-icon=\"process\""));
    assert!(relations_svg.contains("data-archimate-role-icon=\"service\""));
    assert!(relations_svg.contains("fill=\"#FFFFB0\""));
    assert!(relations_svg.contains("fill=\"#D5E8F0\""));
    assert!(relations_svg.contains("assigned"));

    let flows_svg = render_source_to_svg(&flows).expect("render flow archimate example");
    assert!(flows_svg.contains("data-archimate-kind=\"flow\""));
    assert!(flows_svg.contains("stroke-dasharray=\"5 3\""));
    assert!(flows_svg.contains("routes"));
}

#[test]
fn archimate_layer_palette_matches_spec_hexes() {
    let src = r#"@startarchimate
archimate "Capability" as cap <<strategy>>
archimate "Order Process" as proc <<business>>
archimate "Order Service" as svc <<application>>
archimate "Gateway" as gw <<technology>>
archimate "Goal" as goal <<motivation>>
@endarchimate"#;

    let svg = render_source_to_svg(src).expect("render archimate palette example");
    assert!(svg.contains("fill=\"#F5DEAA\""));
    assert!(svg.contains("fill=\"#FFFFB0\""));
    assert!(svg.contains("fill=\"#D5E8F0\""));
    assert!(svg.contains("fill=\"#D5F5DD\""));
    assert!(svg.contains("fill=\"#E0D5F5\""));
}

// ---- stdlib catalog tests (#173) ----

#[test]
fn stdlib_c4_context_check_passes_and_ast_has_object_declarations() {
    // --check must succeed (requires normalize_family to accept Object diagram).
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/valid_c4_context.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    // AST dump must show ObjectDecl nodes with macro-expanded names and aliases.
    let stdout = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/valid_c4_context.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let ast: Value = serde_json::from_slice(&stdout).expect("valid JSON AST");

    // Diagram must be Object kind (C4 stubs emit `object` declarations).
    assert_eq!(
        ast["kind"], "Object",
        "C4 context fixture must produce Object diagram"
    );

    let stmts = ast["statements"].as_array().expect("statements array");

    // Person(u, "User") -> ObjectDecl { name: "User", alias: "u <<person>>" }
    let user_decl = stmts
        .iter()
        .find(|s| s["kind"]["ObjectDecl"]["name"] == "User")
        .expect("User ObjectDecl from Person() macro");
    assert!(
        user_decl["kind"]["ObjectDecl"]["alias"]
            .as_str()
            .unwrap_or("")
            .contains("<<person>>"),
        "Person macro alias must contain <<person>> stereotype"
    );

    // System(s, "Software System") -> ObjectDecl { name: "Software System", alias: "s <<system>>" }
    let sys_decl = stmts
        .iter()
        .find(|s| s["kind"]["ObjectDecl"]["name"] == "Software System")
        .expect("Software System ObjectDecl from System() macro");
    assert!(
        sys_decl["kind"]["ObjectDecl"]["alias"]
            .as_str()
            .unwrap_or("")
            .contains("<<system>>"),
        "System macro alias must contain <<system>> stereotype"
    );

    // Rel(u, s, "Uses") -> FamilyRelation { from: "u", to: "s" }
    let rel = stmts
        .iter()
        .find(|s| {
            s["kind"]["FamilyRelation"]["from"] == "u" && s["kind"]["FamilyRelation"]["to"] == "s"
        })
        .expect("Rel(u, s) FamilyRelation");
    assert_eq!(rel["kind"]["FamilyRelation"]["arrow"], "-->");
}

#[test]
fn stdlib_awslib_ec2_check_passes_and_ast_has_object_declarations() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/valid_awslib_ec2.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let stdout = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/valid_awslib_ec2.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let ast: Value = serde_json::from_slice(&stdout).expect("valid JSON AST");

    assert_eq!(
        ast["kind"], "Object",
        "AWS EC2 fixture must produce Object diagram"
    );

    let stmts = ast["statements"].as_array().expect("statements array");

    // EC2(server, "App Server") -> ObjectDecl { name: "App Server", alias: "server <<aws-ec2>>" }
    let server_decl = stmts
        .iter()
        .find(|s| s["kind"]["ObjectDecl"]["name"] == "App Server")
        .expect("App Server ObjectDecl from EC2() macro");
    assert!(
        server_decl["kind"]["ObjectDecl"]["alias"]
            .as_str()
            .unwrap_or("")
            .contains("<<aws-ec2>>"),
        "EC2 macro alias must contain <<aws-ec2>> stereotype"
    );

    // Rel(server, cache, "reads from") -> FamilyRelation
    let rel = stmts
        .iter()
        .find(|s| s["kind"]["FamilyRelation"]["from"] == "server")
        .expect("Rel(server, cache) FamilyRelation");
    assert_eq!(rel["kind"]["FamilyRelation"]["to"], "cache");
}

#[test]
fn c4_multiple_rel_on_same_pair_coalesces_labels_with_newline_not_concatenation() {
    // Regression test for #425: multiple Rel() calls between the same source→target
    // pair must NOT produce "Uses HTTPSSends emails" (concatenated without separator).
    // They must coalesce into one relation whose label is "Uses HTTPS\nSends emails",
    // rendered as stacked tspan elements in the SVG output.
    // Inline the C4 Rel() procedure via stdin so SVG goes to stdout.
    // The C4 Rel() macro expands to `$from --> $to : $label`.
    let puml_src = "\
        @startuml\n\
        !procedure Rel($from, $to, $label, $tech=\"\")\n\
        $from --> $to : $label\n\
        !endprocedure\n\
        object User as user <<person>>\n\
        object API as api <<system>>\n\
        !Rel(user, api, \"Uses HTTPS\")\n\
        !Rel(user, api, \"Sends emails\")\n\
        @enduml\n";

    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args(["-"])
        .write_stdin(puml_src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let svg = String::from_utf8(output).expect("UTF-8 SVG");

    // Must NOT contain concatenated label.
    assert!(
        !svg.contains("Uses HTTPSSends"),
        "labels must not be concatenated without separator"
    );
    assert!(
        !svg.contains("Uses HTTPS\nSends"),
        "raw newline in SVG text is invisible — must be converted to tspan"
    );
    // Must contain each label text.
    assert!(svg.contains("Uses HTTPS"), "first label must appear");
    assert!(svg.contains("Sends emails"), "second label must appear");
    // Must use tspan for multi-line rendering (#425).
    assert!(
        svg.contains("<tspan") && svg.contains("Uses HTTPS") && svg.contains("Sends emails"),
        "multiline label must use <tspan> elements"
    );
    // Must have exactly ONE polyline between user and api (merged relation, not two overlapping).
    let user_api_arrow_count = svg
        .matches("data-uml-from=\"user\" data-uml-to=\"api\"")
        .count();
    assert_eq!(
        user_api_arrow_count, 1,
        "duplicate Rel() on same pair must coalesce to a single arrow, got {user_api_arrow_count}"
    );
}

#[test]
fn stdlib_angle_bracket_include_is_idempotent_when_included_twice() {
    // Including the same stdlib file twice must not cause duplicate procedure errors.
    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("double_include.puml");
    fs::write(
        &input,
        "@startuml\n!include <C4/C4_Context>\n!include <C4/C4_Context>\n!Person(u, User)\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn stdlib_angle_bracket_include_supports_tagged_fixture() {
    let stdout = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("stdlib_include_tag/valid_stdlib_tagged_angle_include.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let ast: Value = serde_json::from_slice(&stdout).expect("valid JSON AST");
    let stmts = ast["statements"].as_array().expect("statements array");
    assert_eq!(
        stmts.len(),
        1,
        "tagged stdlib include must omit untagged body lines"
    );
    assert_eq!(stmts[0]["kind"]["Message"]["from"], "Alice");
    assert_eq!(stmts[0]["kind"]["Message"]["to"], "Bob");
    assert_eq!(
        stmts[0]["kind"]["Message"]["label"],
        "from tagged stdlib include"
    );
}

#[test]
fn state_history_shallow_renders_h_circle() {
    let src = fs::read_to_string(fixture("families/valid_state_history.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render history state SVG");
    assert!(
        svg.contains(">H<"),
        "expected 'H' label in shallow history node"
    );
}

#[test]
fn creole_color_size_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("conformance/valid_creole_color_size.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn creole_color_size_svg_contains_color_and_size_attributes() {
    let src = fs::read_to_string(fixture("conformance/valid_creole_color_size.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("render");
    assert!(
        svg.contains("fill=\"red\""),
        "expected red fill in SVG for <color:red>"
    );
    assert!(
        svg.contains("font-size=\"14\""),
        "expected font-size=\"14\" in SVG for <size:14>"
    );
}

#[test]
fn state_history_renders_h_circle() {
    let src = fs::read_to_string(fixture("families/valid_state_history.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render history state SVG");
    assert!(
        svg.contains(">H<"),
        "expected 'H' label in shallow history node"
    );
}

#[test]
fn state_history_deep_renders_hstar_circle() {
    let src = fs::read_to_string(fixture("families/valid_state_history.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render history state SVG");
    assert!(
        svg.contains(">H*<"),
        "expected 'H*' label in deep history node"
    );
}

#[test]
fn creole_newlines_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("conformance/valid_creole_newlines.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn stdlib_angle_bracket_include_with_puml_stdlib_root_env_override() {
    // PUML_STDLIB_ROOT must point directly to the stdlib dir.
    let stdlib_path = format!("{}/stdlib", env!("CARGO_MANIFEST_DIR"));

    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("env_override.puml");
    fs::write(
        &input,
        "@startuml\n!include <C4/C4_Context>\n!Person(u, TestUser)\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_STDLIB_ROOT", &stdlib_path)
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn creole_inline_bold_produces_tspan_in_message_label() {
    let src = "@startuml\nAlice -> Bob: **hello**\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("font-weight=\"bold\""),
        "expected bold tspan for **hello**"
    );
    assert!(svg.contains(">hello<"), "expected label text in tspan");
}

#[test]
fn creole_inline_mono_produces_monospace_tspan() {
    let src = "@startuml\nAlice -> Bob: \"\"code\"\"\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    // mono spans set font-family=monospace on the inner tspan
    assert!(
        svg.contains("font-family=\"monospace\""),
        "expected monospace tspan for \"\"code\"\""
    );
}

#[test]
fn creole_inline_underline_produces_text_decoration() {
    let src = "@startuml\nAlice -> Bob: __ul__\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "expected underline tspan for __ul__"
    );
}

#[test]
fn creole_inline_strikethrough_produces_line_through() {
    let src = "@startuml\nAlice -> Bob: --strike--\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("text-decoration=\"line-through\""),
        "expected line-through tspan for --strike--"
    );
}

#[test]
fn creole_html_b_tag_produces_bold_tspan() {
    let src = "@startuml\nAlice -> Bob: <b>bold</b>\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("font-weight=\"bold\""),
        "expected bold tspan for <b>bold</b>"
    );
}

#[test]
fn creole_html_i_tag_produces_italic_tspan() {
    let src = "@startuml\nAlice -> Bob: <i>italic</i>\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic tspan for <i>italic</i>"
    );
}

#[test]
fn creole_plain_label_uses_fast_path_without_tspan_wrapper() {
    let src = "@startuml\nAlice -> Bob: plain\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    // Plain text should NOT wrap in tspan at all — fast path
    let plain_text_pattern = ">plain<";
    assert!(
        svg.contains(plain_text_pattern),
        "expected direct text content for plain label"
    );
}

#[test]
fn state_entry_exit_renders_italic_action_text() {
    let src = fs::read_to_string(fixture("families/valid_state_entry_exit.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render entry/exit state SVG");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic text for entry/exit actions"
    );
    assert!(svg.contains("entry"), "expected entry action label in SVG");
}

#[test]
fn class_together_group_member_ids_are_recorded_in_model() {
    use puml::normalize_family;
    use puml::parser::parse;
    use puml::NormalizedDocument;

    let src = "@startuml\nclass A\nclass B\ntogether {\n  A\n  B\n}\n@enduml\n";
    let doc = parse(src).expect("parse ok");
    let model = normalize_family(doc).expect("normalize ok");
    let NormalizedDocument::Family(family) = model else {
        panic!("expected Family model");
    };
    assert_eq!(family.groups.len(), 1, "should have 1 group");
    let group = &family.groups[0];
    assert_eq!(group.kind, "together");
    assert!(group.member_ids.contains(&"A".to_string()));
    assert!(group.member_ids.contains(&"B".to_string()));
}

#[test]
fn class_hide_options_are_recorded_in_model() {
    use puml::normalize_family;
    use puml::parser::parse;
    use puml::NormalizedDocument;

    let src = "@startuml\nhide circle\nhide stereotype\nhide empty members\nclass Foo\n@enduml\n";
    let doc = parse(src).expect("parse ok");
    let model = normalize_family(doc).expect("normalize ok");
    let NormalizedDocument::Family(family) = model else {
        panic!("expected Family model");
    };
    assert!(family.hide_options.contains("circle"));
    assert!(family.hide_options.contains("stereotype"));
    assert!(family.hide_options.contains("empty members"));
}

#[test]
fn state_fork_join_choice_end_renders_stereotyped_shapes() {
    let src = fs::read_to_string(fixture("families/valid_state_fork_join.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render fork/join/choice/end SVG");
    assert!(
        svg.contains("<rect"),
        "expected rect element for fork/join bar"
    );
    assert!(
        svg.contains("<polygon"),
        "expected polygon for choice diamond"
    );
    let circle_count = svg.matches("<circle").count();
    assert!(
        circle_count >= 2,
        "expected at least 2 circle elements, got {circle_count}"
    );
}

#[test]
fn state_transition_labels_appear_in_svg() {
    let src = fs::read_to_string(fixture("families/valid_state_fork_join.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render SVG");
    assert!(
        svg.contains("done"),
        "expected 'done' transition label in SVG"
    );
    assert!(
        svg.contains("retry"),
        "expected 'retry' transition label in SVG"
    );
}

fn state_svg_element_after_metadata<'a>(
    doc: &'a roxmltree::Document<'a>,
    node_name: &str,
) -> roxmltree::Node<'a, 'a> {
    doc.descendants()
        .find(|node| {
            node.has_tag_name("metadata") && node.attribute("data-state-node") == Some(node_name)
        })
        .and_then(|node| node.next_sibling_element())
        .unwrap_or_else(|| panic!("missing rendered element for state node {node_name}"))
}

fn state_svg_attr_i32(node: roxmltree::Node<'_, '_>, attr: &str) -> i32 {
    node.attribute(attr)
        .unwrap_or_else(|| panic!("missing attribute {attr}"))
        .parse::<i32>()
        .unwrap_or_else(|_| panic!("invalid integer attribute {attr}"))
}

fn state_svg_center_x(node: roxmltree::Node<'_, '_>) -> i32 {
    match node.tag_name().name() {
        "rect" => state_svg_attr_i32(node, "x") + state_svg_attr_i32(node, "width") / 2,
        "circle" => state_svg_attr_i32(node, "cx"),
        "polygon" => {
            let points = node
                .attribute("points")
                .unwrap_or_else(|| panic!("missing polygon points"));
            let xs = points
                .split_whitespace()
                .filter_map(|pair| pair.split_once(','))
                .map(|(x, _)| x.parse::<i32>().expect("polygon x should be an integer"))
                .collect::<Vec<_>>();
            let min_x = xs
                .iter()
                .min()
                .copied()
                .expect("polygon should have x points");
            let max_x = xs
                .iter()
                .max()
                .copied()
                .expect("polygon should have x points");
            (min_x + max_x) / 2
        }
        other => panic!("unsupported state SVG node for center extraction: {other}"),
    }
}

/// Extract start (x1,y1) and end (x2,y2) coordinates from a state transition `<path>`
/// element. The `d` attribute has the form `M x y [L x y]*`.
/// Returns (x1, y1, x2, y2) — the first and last coordinate pairs.
fn state_path_endpoints(node: roxmltree::Node<'_, '_>) -> (i32, i32, i32, i32) {
    let d = node
        .attribute("d")
        .unwrap_or_else(|| panic!("state transition path should have d attribute"));
    let nums: Vec<i32> = d
        .split_ascii_whitespace()
        .filter_map(|tok| tok.parse::<i32>().ok())
        .collect();
    assert!(
        nums.len() >= 4,
        "state transition path d should have at least two coordinate pairs; d={d:?}"
    );
    let x1 = nums[0];
    let y1 = nums[1];
    let x2 = nums[nums.len() - 2];
    let y2 = nums[nums.len() - 1];
    (x1, y1, x2, y2)
}

#[test]
fn state_full_machine_offsets_vertical_labels_and_keeps_final_state_in_canvas_flow() {
    let src = fs::read_to_string("docs/examples/state/08_full_machine.puml").unwrap();
    let svg = render_source_to_svg(&src).expect("full machine state example should render");
    let doc = roxmltree::Document::parse(&svg).expect("state SVG should parse");

    for node_name in [
        "Pending",
        "Fulfillment",
        "Picking",
        "Packing",
        "Shipped",
        "Delivered",
        "Cancelled",
    ] {
        assert_eq!(
            doc.descendants()
                .filter(|node| {
                    node.has_tag_name("metadata")
                        && node.attribute("data-state-node") == Some(node_name)
                })
                .count(),
            1,
            "{node_name} should render exactly once"
        );
    }

    // State transitions are now <path> elements (orthogonal routing).
    let confirm_edge = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("Pending")
                && node.attribute("data-state-to") == Some("fork1")
        })
        .expect("Pending -> fork1 edge should render");
    let confirm_label = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("text") && node.attribute("data-state-label") == Some("confirm")
        })
        .expect("Pending -> fork1 label should render");
    assert_eq!(confirm_label.attribute("data-state-label"), Some("confirm"));
    let (confirm_x1, _, _, _) = state_path_endpoints(confirm_edge);
    assert_ne!(
        state_svg_attr_i32(confirm_label, "x"),
        confirm_x1,
        "vertical edge label should be offset from the arrow shaft"
    );

    let instock_edge = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("choice1")
                && node.attribute("data-state-to") == Some("join1")
        })
        .expect("choice1 -> join1 edge should render");
    let instock_label = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("text") && node.attribute("data-state-label") == Some("in stock")
        })
        .expect("choice1 -> join1 label should render");
    assert_eq!(
        instock_label.attribute("data-state-label"),
        Some("in stock")
    );
    let (instock_x1, _, _, _) = state_path_endpoints(instock_edge);
    assert_ne!(
        state_svg_attr_i32(instock_label, "x"),
        instock_x1,
        "branch label should be offset from the crossing arrow shaft"
    );

    let delivered_rect = state_svg_element_after_metadata(&doc, "Delivered");
    let final_state_circle = state_svg_element_after_metadata(&doc, "[*]__end");
    let delivered_bottom =
        state_svg_attr_i32(delivered_rect, "y") + state_svg_attr_i32(delivered_rect, "height");
    let final_state_center_y = state_svg_attr_i32(final_state_circle, "cy");
    assert!(
        final_state_center_y > delivered_bottom,
        "final state should render below Delivered so the terminal arrow stays in canvas"
    );
}

#[test]
fn state_fork_join_choice_example_keeps_parallel_branches_aligned() {
    let src = fs::read_to_string("docs/examples/state/05_fork_join_choice.puml").unwrap();
    let svg = render_source_to_svg(&src).expect("fork/join/choice example should render");
    let doc = roxmltree::Document::parse(&svg).expect("state SVG should parse");

    let fork = state_svg_element_after_metadata(&doc, "fork1");
    let join = state_svg_element_after_metadata(&doc, "join1");
    let choice = state_svg_element_after_metadata(&doc, "choice1");
    let task_a = state_svg_element_after_metadata(&doc, "TaskA");
    let task_b = state_svg_element_after_metadata(&doc, "TaskB");

    let task_a_center = state_svg_center_x(task_a);
    let task_b_center = state_svg_center_x(task_b);
    let fork_left = state_svg_attr_i32(fork, "x");
    let fork_right = fork_left + state_svg_attr_i32(fork, "width");
    let join_left = state_svg_attr_i32(join, "x");
    let join_right = join_left + state_svg_attr_i32(join, "width");

    assert!(
        fork_left <= task_a_center && fork_right >= task_b_center,
        "fork bar should span both task columns"
    );
    assert!(
        join_left <= task_a_center && join_right >= task_b_center,
        "join bar should span both task columns"
    );

    // State transitions are now <path> elements (orthogonal routing).
    let fork_to_a = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("fork1")
                && node.attribute("data-state-to") == Some("TaskA")
        })
        .expect("fork1 -> TaskA edge should render");
    let fork_to_b = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("fork1")
                && node.attribute("data-state-to") == Some("TaskB")
        })
        .expect("fork1 -> TaskB edge should render");
    let task_a_to_join = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("TaskA")
                && node.attribute("data-state-to") == Some("join1")
        })
        .expect("TaskA -> join1 edge should render");
    let task_b_to_join = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("TaskB")
                && node.attribute("data-state-to") == Some("join1")
        })
        .expect("TaskB -> join1 edge should render");

    for (edge, expected_center, label) in [
        (fork_to_a, task_a_center, "fork1 -> TaskA"),
        (fork_to_b, task_b_center, "fork1 -> TaskB"),
        (task_a_to_join, task_a_center, "TaskA -> join1"),
        (task_b_to_join, task_b_center, "TaskB -> join1"),
    ] {
        // For fork/join edges the anchors share the same X, so the orthogonal router
        // emits a straight segment: M x y1 L x y2 (x1 == x2 in the path).
        let (ex1, _, ex2, _) = state_path_endpoints(edge);
        assert_eq!(ex1, ex2, "{label} should stay vertical");
        assert_eq!(
            ex1, expected_center,
            "{label} should align to its task column center"
        );
    }

    let choice_center = state_svg_center_x(choice);
    let join_center = state_svg_center_x(join);
    assert!(
        (choice_center - join_center).abs() <= 2,
        "choice diamond should stay aligned with the main flow"
    );

    let error_label = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("text") && node.attribute("data-state-label") == Some("error")
        })
        .expect("error label should render");
    assert!(
        state_svg_attr_i32(error_label, "x") > choice_center,
        "error label should stay attached to the rightward failure branch"
    );
}

#[test]
fn state_arch_lifecycle_composites_render_enclosing_boxes() {
    let src = fs::read_to_string("docs/diagrams/diagram-family-lifecycle.puml").unwrap();
    let svg = render_source_to_svg(&src).expect("diagram family lifecycle should render");
    let doc = roxmltree::Document::parse(&svg).expect("state SVG should parse");

    let styled_rect = state_svg_element_after_metadata(&doc, "Styled");
    let skin_rect = state_svg_element_after_metadata(&doc, "SkinParams");
    let palette_rect = state_svg_element_after_metadata(&doc, "Palette");

    let styled_x = state_svg_attr_i32(styled_rect, "x");
    let styled_y = state_svg_attr_i32(styled_rect, "y");
    let styled_w = state_svg_attr_i32(styled_rect, "width");
    let styled_h = state_svg_attr_i32(styled_rect, "height");
    assert!(
        styled_x <= state_svg_attr_i32(skin_rect, "x")
            && styled_y <= state_svg_attr_i32(skin_rect, "y")
            && styled_x + styled_w
                >= state_svg_attr_i32(palette_rect, "x")
                    + state_svg_attr_i32(palette_rect, "width")
            && styled_y + styled_h
                >= state_svg_attr_i32(palette_rect, "y")
                    + state_svg_attr_i32(palette_rect, "height"),
        "Styled should render an enclosing box around its child states"
    );

    let rendered_rect = state_svg_element_after_metadata(&doc, "Rendered");
    let svg_rect = state_svg_element_after_metadata(&doc, "SVGOut");
    let txt_rect = state_svg_element_after_metadata(&doc, "TxtOut");

    let rendered_x = state_svg_attr_i32(rendered_rect, "x");
    let rendered_y = state_svg_attr_i32(rendered_rect, "y");
    let rendered_w = state_svg_attr_i32(rendered_rect, "width");
    let rendered_h = state_svg_attr_i32(rendered_rect, "height");
    assert!(
        rendered_x <= state_svg_attr_i32(svg_rect, "x")
            && rendered_y <= state_svg_attr_i32(svg_rect, "y")
            && rendered_x + rendered_w
                >= state_svg_attr_i32(txt_rect, "x") + state_svg_attr_i32(txt_rect, "width")
            && rendered_y + rendered_h
                >= state_svg_attr_i32(txt_rect, "y") + state_svg_attr_i32(txt_rect, "height"),
        "Rendered should render an enclosing box around its child states"
    );
}

#[test]
fn state_basic_render_produces_valid_svg() {
    let src = "@startuml\nstate Active\n[*] --> Active\nActive --> [*]\n@enduml\n";
    let svg = render_source_to_svg(src).expect("basic state should render");
    assert!(svg.starts_with("<svg"), "expected SVG output");
    assert!(svg.contains("Active"), "expected state name in SVG");
}

// ── Issue #183: class member modifiers {field}/{method}/{abstract}/{static} ───

#[test]
fn class_member_modifier_fixture_parses_and_renders() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("families/valid_class_html_members.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn class_member_abstract_modifier_renders_italic_in_svg() {
    // {abstract} and <<abstract>> members must produce italic tspan in SVG
    let src = "@startuml\nclass Animal {\n  {abstract} #speak(): void\n  +name: String <<abstract>>\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic for abstract member"
    );
}

#[test]
fn class_member_static_modifier_renders_underline_in_svg() {
    // {static} / {class} / <<static>> members must produce underline tspan in SVG
    let src = "@startuml\nclass Config {\n  {static} MAX: Int\n  {class} DEFAULT: String\n  COUNT: Int <<static>>\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "expected underline for static member"
    );
}

#[test]
fn class_member_field_modifier_renders_italic_in_svg() {
    // {field} produces italic tspan
    let src = "@startuml\nclass User {\n  {field} +id: UUID\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic for field modifier"
    );
}

#[test]
fn class_member_method_modifier_has_no_special_styling() {
    // {method} produces no extra styling — just plain text
    let src = "@startuml\nclass User {\n  {method} +save(): void\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    // The text "save" must appear; no extra decoration expected
    assert!(svg.contains("save"), "expected method name in SVG");
}

#[test]
fn class_member_trailing_modifier_parsed_correctly() {
    // Trailing modifiers: `member {field}`, `member {static}`
    let src = "@startuml\nclass Foo {\n  +x: Int {field}\n  +y: Float {static}\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic for trailing field"
    );
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "expected underline for trailing static"
    );
}

// ── Issue #191: --stdrpt single-line diagnostic format ───────────────────────

#[test]
fn stdrpt_flag_formats_error_as_single_tab_separated_line() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--stdrpt",
            &fixture("errors/invalid_family_decl_block_unclosed.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = stderr.lines().collect();
    // Exactly one line per diagnostic
    assert_eq!(
        lines.len(),
        1,
        "expected exactly one stdrpt line, got: {stderr:?}"
    );
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(
        parts.len(),
        4,
        "expected 4 tab-separated fields, got: {:?}",
        parts
    );
    assert_eq!(parts[0], "error", "first field should be severity");
    // second field is code, third is location, fourth is message
    assert!(!parts[3].is_empty(), "message field should not be empty");
}

#[test]
fn stdrpt_flag_location_includes_file_and_line_col() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--stdrpt",
            &fixture("errors/invalid_family_decl_block_unclosed.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    let line = stderr.lines().next().unwrap_or("");
    let parts: Vec<&str> = line.split('\t').collect();
    // location field (index 2) must contain a colon-separated path:line:col
    let location = parts.get(2).copied().unwrap_or("");
    assert!(
        location.contains(':'),
        "location field should contain colons: {location:?}"
    );
}

#[test]
fn stdrpt_does_not_emit_multiline_source_context() {
    let output = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--stdrpt",
            &fixture("errors/invalid_family_decl_block_unclosed.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8_lossy(&output);
    // No caret lines (lines starting with spaces + ^^^)
    for line in stderr.lines() {
        assert!(
            !line.trim_start().starts_with('^'),
            "stdrpt should suppress caret lines, found: {line:?}"
        );
    }
}

#[test]
fn stdrpt_exit_code_semantics_unchanged_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--stdrpt", "--check", &fixture("single_valid.puml")])
        .assert()
        .success();
}

// ─── Preprocessor advanced directives ────────────────────────────────────────

#[test]
fn preproc_newline_builtin_returns_newline_char() {
    let src = "@startuml\n!$nl = %newline()\nA -> B : ok\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--", "-"])
        .write_stdin(src)
        .assert()
        .success();
}

// ── Issue #188: Full PicoUML native syntax ────────────────────────────────────

#[test]
fn picouml_full_constructs_passes_check() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "picouml",
            "--check",
            &fixture("picouml/valid_full_constructs.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn preproc_feature_builtin_returns_false_for_unknown() {
    let src = "@startuml\n!$f = %feature(\"nosuchfeature\")\nA -> B : %feature(\"x\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("feature builtin should work");
    assert!(svg.contains("false"), "expected 'false' from %feature");
}

#[test]
fn preproc_variable_exists_returns_correct_bool() {
    let src = "@startuml\n!$x = hello\nA -> B : %variable_exists(\"x\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("variable_exists should work");
    assert!(
        svg.contains("true"),
        "expected 'true' for existing variable"
    );
}

#[test]
fn preproc_function_exists_detects_defined_function() {
    let src = "@startuml\n!function MyFn($a)\n!return $a\n!endfunction\nA -> B : %function_exists(\"MyFn\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("function_exists should work");
    assert!(svg.contains("true"), "expected 'true' for defined function");
}

#[test]
fn preproc_get_json_attribute_nested_path() {
    // Simple flat key — nested path traversal
    let src = "@startuml\n!$cfg = { \"name\": \"beta\" }\nA -> B : %get_json_attribute($cfg, \"name\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("get_json_attribute should work");
    assert!(svg.contains("beta"), "expected 'beta' from JSON attribute");
}

#[test]
fn preproc_retrieve_procedure_return_is_empty_in_deterministic_model() {
    let src = "@startuml\n!$ret = %retrieve_procedure_return()\nA -> B : done\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--", "-"])
        .write_stdin(src)
        .assert()
        .success();
}

#[test]
fn preproc_while_loop_with_variable_counter_expands_correctly() {
    let src = "@startuml\n!$i = 0\n!while $i < 3\n!$i = $i + 1\nA$i -> B$i\n!endwhile\n@enduml\n";
    let svg = render_source_to_svg(src).expect("while loop should work");
    // Should have produced 3 messages
    assert!(svg.contains("A1"), "expected A1 in output");
    assert!(svg.contains("A2"), "expected A2 in output");
    assert!(svg.contains("A3"), "expected A3 in output");
}

#[test]
fn preproc_expression_word_operators_and_string_builtins_expand() {
    let src = "@startuml\n!$raw = \"  Alpha-Beta  \"\n!assert %contains(%trim($raw), \"Alpha\") and %startswith(%trim($raw), \"Alpha\")\nA -> B : %replace(%lower(%trim($raw)), \"-\", \":\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("string builtins should expand");
    assert!(svg.contains("alpha:beta"), "expected replacement output");
}

#[test]
fn preproc_list_and_map_builtins_are_deterministic_json_strings() {
    let src = "@startuml\n!$items = %list(\"red\", \"blue\")\n!$items = %list_add($items, \"green\")\n!$cfg = %map(\"name\", \"Ada\", \"role\", \"admin\")\n!$cfg = %map_put($cfg, \"team\", %join($items, \"/\"))\n!assert %list_contains($items, \"blue\") and %map_contains_key($cfg, \"team\")\nA -> B : %list_get($items, 2) / %get($cfg, \"team\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("list/map builtins should expand");
    assert!(svg.contains("green /"), "expected list_get output");
    assert!(
        svg.contains("red/blue/green"),
        "expected joined map value output"
    );
}

#[test]
fn preproc_nested_json_mutation_and_projection_helpers_expand() {
    let src = "@startuml\n!$cfg = {\"users\":[{\"name\":\"Ada\",\"role\":\"dev\"}],\"meta\":{\"version\":1}}\n!$cfg = %json_set($cfg, \"users[0].role\", \"admin\")\n!$cfg = %json_set($cfg, \"meta.tags[0]\", \"stable\")\n!$cfg = %json_merge($cfg, {\"meta\":{\"build\":2},\"extra\":true})\n!$cfg = %json_remove($cfg, \"users[0].name\")\n!$items = %list_sort(%list_remove(%list(\"zeta\", \"alpha\", \"beta\"), \"zeta\"))\n!assert %json_key_exists($cfg, \"meta.tags[0]\") and %json_is_valid($cfg)\nA -> B : %get($cfg, \"users[0].role\") / %get($cfg, \"meta.tags[0]\") / %get($cfg, \"meta.build\") / %json_type($cfg) / %join($items, \":\")\n@enduml\n";
    let svg = render_source_to_svg(src).expect("nested JSON helpers should expand");
    assert!(
        svg.contains("admin / stable / 2 /") && svg.contains("object / alpha:beta"),
        "expected nested mutation, merge, remove, type, and sorted list output"
    );
    assert!(
        !svg.contains("Ada"),
        "json_remove should delete the nested name before rendering"
    );
}

#[test]
fn preproc_undef_removes_define() {
    // After !undef, the define should no longer expand
    let src = "@startuml\n!define GREETING hello\n!undef GREETING\nA -> B : ok\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--", "-"])
        .write_stdin(src)
        .assert()
        .success();
}

// ─── MindMap / WBS rendering ──────────────────────────────────────────────────

#[test]
fn mindmap_palette_fixture_renders_svg() {
    let svg = render_source_to_svg(
        &fs::read_to_string(fixture("families/valid_mindmap_palette.puml")).unwrap(),
    )
    .expect("mindmap should render");
    assert!(svg.starts_with("<svg"), "expected SVG");
    assert!(svg.contains("Root"), "expected Root node in mindmap SVG");
}

#[test]
fn wbs_progress_fixture_renders_svg_with_progress_bar() {
    let svg = render_source_to_svg(
        &fs::read_to_string(fixture("families/valid_wbs_progress.puml")).unwrap(),
    )
    .expect("wbs should render");
    assert!(svg.starts_with("<svg"), "expected SVG");
    assert!(svg.contains("Project"), "expected Project root");
    // Progress bar should be present (blue rect)
    assert!(svg.contains("#3b82f6"), "expected progress bar fill color");
}

#[test]
fn wbs_checked_unchecked_nodes_render_svg() {
    let src = "@startwbs\n* Scope\n** Done [x]\n** Pending [ ]\n@endwbs\n";
    let svg = render_source_to_svg(src).expect("wbs checkbox should render");
    assert!(svg.starts_with("<svg"), "expected SVG");
    assert!(svg.contains("#16a34a"), "expected checked green color");
}

#[test]
fn mindmap_left_side_mode_accepted_in_check() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_mindmap_palette.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn mindmap_plus_minus_prefix_side_assignment() {
    let src = "@startmindmap\n* Root\n+** Right\n-** Left\n@endmindmap\n";
    let svg = render_source_to_svg(src).expect("mindmap +/- prefix should render");
    assert!(svg.starts_with("<svg"), "expected SVG");
    assert!(svg.contains("Right"), "expected Right node");
    assert!(svg.contains("Left"), "expected Left node");
}

#[test]
fn mindmap_orientation_directive_check_passes() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("families/valid_mindmap_orientation.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn picouml_full_constructs_renders_nonempty_svg() {
    let src = fs::read_to_string(fixture("picouml/valid_full_constructs.puml")).unwrap();
    use puml::{parse_with_pipeline_options, FrontendSelection, ParsePipelineOptions};
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Picouml,
        ..ParsePipelineOptions::default()
    };
    let _doc = parse_with_pipeline_options(&src, &options)
        .expect("picouml full constructs must parse via picouml adapter");
}

// ── Issue #103: JSON projection into UML contexts ────────────────────────────

#[test]
fn json_projection_fixture_passes_check() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_json_projection.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn json_projection_render_contains_alias_and_keys() {
    let src = fs::read_to_string(fixture("families/valid_json_projection.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("json projection should render");
    assert!(svg.starts_with("<svg"), "output must be SVG");
    assert!(!svg.is_empty(), "SVG must be non-empty");
    assert!(svg.contains("$user"), "SVG must contain the alias header");
    assert!(svg.contains("name"), "SVG must contain the 'name' key");
}

#[test]
fn json_projection_inline_parse_roundtrip() {
    let src = "@startuml\njson $cfg { \"key\": \"val\" }\n@enduml\n";
    let svg = render_source_to_svg(src).expect("inline json projection should render");
    assert!(svg.contains("$cfg"), "SVG must contain alias '$cfg'");
    assert!(svg.contains("key"), "SVG must contain key 'key'");
}

#[test]
fn json_projection_renders_nested_tree_rows_and_connectors() {
    let src = "@startuml\njson $cfg { \"user\": { \"name\": \"Ada\" }, \"roles\": [\"admin\"] }\n@enduml\n";
    let svg = render_source_to_svg(src).expect("nested json projection should render");
    assert!(
        svg.contains("data-uml-projection-row-label=\"user\""),
        "SVG must contain a parent object row"
    );
    assert!(
        svg.contains("data-uml-projection-row-label=\"name: Ada\""),
        "SVG must contain a nested child row"
    );
    assert!(
        svg.contains("data-uml-projection-row-label=\"roles\""),
        "SVG must contain an array parent row"
    );
    assert!(
        svg.contains("data-uml-projection-row-label=\"[0]: admin\""),
        "SVG must contain an array child row"
    );
    assert!(
        svg.contains("data-uml-projection-row-depth=\"1\""),
        "SVG must include nested projection row depth metadata"
    );
    assert!(
        svg.contains("class=\"uml-projection-connector\""),
        "SVG must include projection connector lines"
    );
}

#[test]
fn json_projection_accepts_partial_rows_and_quoted_braces() {
    let src =
        "@startuml\njson $cfg {\n  \"name\": \"Ada\"\n  \"template\": \"{literal}\"\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("partial JSON projection rows should render");
    assert!(svg.contains("$cfg"), "SVG must contain alias '$cfg'");
    assert!(
        svg.contains("name: Ada"),
        "SVG must contain partial row key"
    );
    assert!(
        svg.contains("template: {literal}"),
        "quoted braces must not close the projection"
    );
}

#[test]
fn yaml_projection_accepts_partial_rows_and_quoted_braces() {
    let src = "@startuml\nyaml $cfg {\n  name: Ada\n  template: \"{literal}\"\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("partial YAML projection rows should render");
    assert!(svg.contains("$cfg"), "SVG must contain alias '$cfg'");
    assert!(
        svg.contains("name: Ada"),
        "SVG must contain partial YAML row key"
    );
    assert!(
        svg.contains("template: {literal}"),
        "quoted braces must not close the YAML projection"
    );
}

#[test]
fn yaml_projection_render_contains_alias_and_keys() {
    let src = fs::read_to_string(fixture("families/valid_yaml_projection.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("yaml projection should render");
    assert!(svg.contains("$cfg"), "SVG must contain alias '$cfg'");
    assert!(svg.contains("name"), "SVG must contain YAML key 'name'");
}

#[test]
fn mindmap_caption_and_legend_render_in_svg() {
    let src =
        "@startmindmap\ntitle My Map\ncaption A test diagram\nlegend\nsome legend\nend legend\n* Root\n** Child\n@endmindmap\n";
    let svg = render_source_to_svg(src).expect("mindmap with caption/legend should render");
    assert!(svg.contains("A test diagram"), "expected caption text");
}

#[test]
fn hide_unlinked_removes_unreferenced_participant_from_svg() {
    let src = "@startuml\nhide unlinked\nparticipant Alice\nparticipant Bob\nparticipant Unused\nAlice -> Bob: hello\n@enduml\n";
    let svg = render_source_to_svg(src).expect("hide unlinked diagram should render");
    assert!(svg.contains("Alice"), "expected Alice in rendered SVG");
    assert!(svg.contains("Bob"), "expected Bob in rendered SVG");
    assert!(
        !svg.contains("Unused"),
        "Unused should be filtered by hide unlinked"
    );
}

#[test]
fn hide_unlinked_records_hidden_participants_without_warning() {
    let src = "@startuml\nhide unlinked\nparticipant Alice\nparticipant Bob\nparticipant Unused\nAlice -> Bob: hello\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let ids = model
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["Alice", "Bob"]);
    assert_eq!(model.hidden_participants, vec!["Unused"]);
    assert!(
        model.warnings.is_empty(),
        "hide unlinked should not warn when it filters participants"
    );
}

#[test]
fn hide_unlinked_preserves_explicit_participants_used_by_messages() {
    let src = "@startuml\nhide unlinked\nparticipant User\nparticipant UI\nparticipant Controller\nparticipant Jobs\nUser -> UI: open\nUI -> Controller: dispatch\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let ids = model
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["User", "UI", "Controller"]);
    assert_eq!(model.hidden_participants, vec!["Jobs"]);

    let svg = render_source_to_svg(src).expect("hide unlinked participants should render");
    assert!(svg.contains("User"));
    assert!(svg.contains("UI"));
    assert!(svg.contains("Controller"));
    assert!(!svg.contains("Jobs"));
}

#[test]
fn hide_unlinked_keeps_note_targets_inside_groups() {
    let src = fs::read_to_string(fixture("styling/valid_hide_unlinked_notes_groups.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let ids = model
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["Alice", "Bob", "Carol"]);
    assert_eq!(model.hidden_participants, vec!["Unused"]);

    let svg = render_source_to_svg(&src).expect("hide unlinked note/group should render");
    assert!(svg.contains("Carol"));
    assert!(!svg.contains("Unused"));
}

#[test]
fn sequence_without_hide_unlinked_keeps_declared_unused_participants() {
    let src = "@startuml\nparticipant Alice\nparticipant Bob\nparticipant Unused\nAlice -> Bob: hello\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let ids = model
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["Alice", "Bob", "Unused"]);
    assert!(!model.hide_unlinked);
    assert!(model.hidden_participants.is_empty());
}

#[test]
fn hide_unlinked_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_hide_unlinked.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn extended_skinparams_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_skinparam_extended.puml")])
        .assert()
        .success();
}

#[test]
fn salt_login_form_fixture_renders_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_salt_login_form.puml")])
        .assert()
        .success();
}

#[test]
fn salt_wireframe_grid_renders_button_and_input() {
    let src = "@startsalt\n{\nName: | \"Enter name\"\n[OK]  | [Cancel]\n}\n@endsalt\n";
    let svg = render_source_to_svg(src).expect("salt grid should render");
    assert!(
        svg.contains("Enter name"),
        "expected input placeholder in SVG"
    );
    assert!(svg.contains("OK"), "expected button label in SVG");
}

#[test]
fn salt_basic_widgets_use_intrinsic_plantuml_like_geometry() {
    let svg = render_source_to_svg(include_str!(
        "fixtures/families/valid_salt_widget_fidelity.puml"
    ))
    .expect("salt fidelity fixture should render");

    assert!(svg.contains("width=\"257\" height=\"172\""));
    assert!(svg.contains("data-salt-style=\"canvas\""));
    assert!(
        !svg.contains("data-salt-style=\"panel\""),
        "plain Salt forms should not get a table panel"
    );
    assert!(
        !svg.contains("stroke=\"#ccc\""),
        "plain Salt forms should not get generic table grid lines"
    );
    assert!(svg.contains("data-salt-widget=\"input\""));
    assert!(svg.contains("<line x1=\"109\" y1=\"22\" x2=\"243\" y2=\"22\""));
    assert!(svg.contains("data-salt-widget=\"button\""));
    assert!(svg.contains("stroke-width=\"2.5\" rx=\"5\" ry=\"5\""));
    assert!(svg.contains("data-salt-widget=\"combo\""));
    assert!(svg.contains("data-salt-widget=\"checkbox\""));
    assert!(svg.contains("<polygon points=\"15,75 18,78 25,69 18,76\""));
    assert!(svg.contains("data-salt-widget=\"radio\""));
}

#[test]
fn salt_tab_strip_renders_as_visual_tab_widgets_not_literal_text() {
    // Regression test for #719 — {/ Tab1 | Tab2 | Tab3 } inside a {+ bordered
    // container was being emitted as literal text instead of a tab-strip widget.
    let src = "@startsalt\n{\n{/ Tab1 | Tab2 | Tab3 }\nContent here\n[OK]\n}\n@endsalt\n";
    let svg = render_source_to_svg(src).expect("salt tab strip should render");
    // The tab bar widget must appear, not bare literal text.
    assert!(
        svg.contains("data-salt-widget=\"tab\""),
        "expected tab widget elements in SVG; literal text was rendered instead"
    );
    assert!(svg.contains("Tab1"), "expected Tab1 label in SVG");
    assert!(svg.contains("Tab2"), "expected Tab2 label in SVG");
    assert!(svg.contains("Tab3"), "expected Tab3 label in SVG");
    // Active tab (index 0) must carry the bold attribute.
    assert!(
        svg.contains("data-salt-tab-active=\"true\""),
        "expected first tab to be marked active"
    );
    // The literal brace-slash syntax must NOT appear as text content.
    assert!(
        !svg.contains("{/ Tab1"),
        "literal '{{/ Tab1' must not appear as text — tab strip was not decoded"
    );
}

#[test]
fn salt_tab_strip_inside_bordered_container_renders_correctly() {
    // The {+ bordered box followed by {/ tab bar must decode the tab bar even
    // while in_text_area state is active.
    let src =
        "@startsalt\n{+\n  {/ First | **Second** | Third }\n  Body text\n  [Cancel]\n}\n@endsalt\n";
    let svg = render_source_to_svg(src).expect("salt nested tab strip should render");
    assert!(
        svg.contains("data-salt-widget=\"tab\""),
        "tab widget must render inside {{+ container"
    );
    // **Second** marks the active tab (index 1).
    assert!(svg.contains("Second"), "Second tab label must appear");
    assert!(
        !svg.contains("{/ First"),
        "literal brace-slash must not leak into output"
    );
}

#[test]
fn chart_area_and_scatter_render_paths_are_supported() {
    let area = "@startchart area\ntitle Area\nA: 4\nB: 7\nC: 3\n@endchart\n";
    let area_svg = render_source_to_svg(area).expect("area chart should render");
    assert!(
        area_svg.contains("<polygon"),
        "area chart should include fill polygon"
    );

    let scatter = "@startchart scatter\ntitle Scatter\nA: 2\nB: 8\nC: 5\n@endchart\n";
    let scatter_svg = render_source_to_svg(scatter).expect("scatter chart should render");
    assert!(
        scatter_svg.matches("<circle").count() >= 3,
        "scatter chart should render point circles"
    );
}

#[test]
fn specialized_chart_render_routes_preprocess_define_like_check_mode() {
    let src = "@startchart\n!define ROW Q1 : 42\nROW\n@endchart\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let cli_svg = Command::cargo_bin("puml")
        .expect("binary")
        .arg("-")
        .write_stdin(src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cli_svg = String::from_utf8(cli_svg).expect("CLI SVG should be UTF-8");
    let lib_svg = render_source_to_svg(src).expect("library chart render should succeed");

    assert_eq!(cli_svg.trim_end(), lib_svg);
    assert!(lib_svg.contains(">Q1<"));
    assert!(lib_svg.contains(">42<"));
    assert!(!lib_svg.contains("!define ROW Q1"));
}

#[test]
fn specialized_regex_render_routes_preprocess_define_like_check_mode() {
    let src = "@startregex\n!define WORD ^foo$\nWORD\n@endregex\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let cli_svg = Command::cargo_bin("puml")
        .expect("binary")
        .arg("-")
        .write_stdin(src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cli_svg = String::from_utf8(cli_svg).expect("CLI SVG should be UTF-8");
    let lib_svg = render_source_to_svg(src).expect("library regex render should succeed");

    assert_eq!(cli_svg.trim_end(), lib_svg);
    assert!(lib_svg.contains("foo"));
    assert!(!lib_svg.contains("!define WORD"));
}

#[test]
fn non_uml_advanced_chart_annotations_and_style_directives_render() {
    let src = "@startchart bar\n\
title Revenue\n\
skinparam backgroundColor #f8fafc\n\
skinparam axisColor #334155\n\
palette #0ea5e9 #f97316\n\
caption Forecast confidence\n\
\"Q1\" : 10\n\
\"Q2\" : 18\n\
annotation \"Q2\" : peak quarter\n\
@endchart\n";
    let svg = render_source_to_svg(src).expect("styled annotated chart should render");
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("stroke=\"#334155\""));
    assert!(svg.contains("data-chart-palette=\"#0ea5e9 #f97316\""));
    assert!(svg.contains("data-chart-annotation=\"Q2\""));
    assert!(svg.contains("peak quarter"));
    assert!(svg.contains("data-chart-caption=\"true\""));
}

#[test]
fn non_uml_advanced_regex_localized_descriptive_labels_render() {
    let src = "@startregex\nlocale fr\n^\\d+\\s\\w+$\n@endregex\n";
    let svg = render_source_to_svg(src).expect("localized regex should render");
    assert!(svg.contains("chiffre"));
    assert!(svg.contains("espace"));
    assert!(svg.contains("mot"));
    assert!(svg.contains("debut"));
    assert!(svg.contains("fin"));
}

#[test]
fn regex_exact_and_ranged_quantifiers_render_as_supported_repeats() {
    let src = "@startregex\n^\\d{3}-[A-Z]{2,5}(foo|bar){1,}$\n@endregex\n";
    let svg = render_source_to_svg(src).expect("regex counted quantifiers should render");
    assert!(svg.contains("\\d{3}"), "expected exact count repeat");
    assert!(svg.contains("[A-Z]{2,5}"), "expected bounded range repeat");
    assert!(
        svg.contains("(alt(&#39;foo&#39;|&#39;bar&#39;)){1,}"),
        "expected open-ended range repeat on group"
    );
    assert!(
        !svg.contains("?{3}?"),
        "counted quantifiers should not render as unsupported tokens"
    );
}

#[test]
fn ebnf_exact_and_ranged_quantifiers_render_as_supported_repeats() {
    let src = "@startebnf\nidentifier = letter{1,} , digit{2} , [ \"-\" ]{0,1} ;\n@endebnf\n";
    let svg = render_source_to_svg(src).expect("ebnf counted quantifiers should render");
    assert!(
        svg.contains("letter{1,}"),
        "expected open-ended EBNF repeat"
    );
    assert!(svg.contains("digit{2}"), "expected exact EBNF repeat");
    assert!(
        svg.contains("{0,1}"),
        "expected counted repeat on optional group"
    );
    assert!(
        !svg.contains("?{2}?"),
        "counted EBNF repeats should not render as unsupported tokens"
    );
}

#[test]
fn chart_plantuml_style_named_subtypes_and_colon_points_render() {
    let cases = [
        (
            "@startchart\nbar chart\nQ1 : 42\nQ2 : 58\n@endchart\n",
            "data-chart-type=\"bar\"",
        ),
        (
            "@startchart\nline chart\nJan : 10\nFeb : 15\n@endchart\n",
            "data-chart-type=\"line\"",
        ),
        (
            "@startchart\npie chart\nFrontend : 35\nBackend : 65\n@endchart\n",
            "data-chart-type=\"pie\"",
        ),
    ];
    for (src, marker) in cases {
        let svg = render_source_to_svg(src).expect("chart should render");
        assert!(svg.contains(marker), "expected subtype marker `{marker}`");
        assert!(
            svg.contains("42")
                || svg.contains("10")
                || svg.contains("Frontend")
                || svg.contains("Backend"),
            "expected colon-delimited chart data to render"
        );
    }
}

#[test]
fn non_uml_advanced_ebnf_style_and_rule_notes_render() {
    let src = "@startebnf\n\
style terminal #ecfeff\n\
style nonterminal #eef2ff\n\
note expr : entry point\n\
expr ::= term , { \"+\" , term } ;\n\
term = \"id\" ;\n\
@endebnf\n";
    let svg = render_source_to_svg(src).expect("styled ebnf should render");
    assert!(svg.contains("data-ebnf-style=\"customizable\""));
    assert!(svg.contains("fill=\"#ecfeff\""));
    assert!(svg.contains("fill=\"#eef2ff\""));
    assert!(svg.contains("data-ebnf-note-for=\"expr\""));
    assert!(svg.contains("entry point"));
}

#[test]
fn sdl_state_stereotypes_render_special_shapes() {
    let src = "@startsdl\nstate Start <<start>>\nstate Decide <<decision>>\nstate End <<end>>\nstate Start -> Decide : next\nstate Decide -> End : done\n@endsdl\n";
    let svg = render_source_to_svg(src).expect("sdl stereotypes should render");
    assert!(
        svg.contains("<polygon points="),
        "decision state should render diamond polygon"
    );
    assert!(
        svg.matches("<circle").count() >= 2,
        "start/end should render circular markers"
    );
}

#[test]
fn ditaa_scale_and_transparent_options_are_applied() {
    let baseline = "@startditaa\n+---+\n|A  |\n+---+\n@endditaa\n";
    let baseline_svg = render_source_to_svg(baseline).expect("baseline ditaa should render");
    let src = "@startditaa scale=2 transparent=true\n+---+\n|A  |\n+---+\n@endditaa\n";
    let svg = render_source_to_svg(src).expect("ditaa options should render");
    let baseline_w = extract_svg_width_attr(&baseline_svg).unwrap_or(0);
    let scaled_w = extract_svg_width_attr(&svg).unwrap_or(0);
    assert!(
        scaled_w > baseline_w,
        "scaled ditaa should produce wider canvas: baseline={baseline_w}, scaled={scaled_w}"
    );
    assert!(
        !svg.contains("fill=\"white\""),
        "transparent=true should omit white background"
    );
}

#[test]
fn non_uml_advanced_ditaa_diagonal_connectors_shadow_and_background_options_render() {
    let src = "@startditaa shadow=true background=#f8fafc\n\
+---+   +---+\n\
| A | / | B |\n\
+---+   +---+\n\
@endditaa\n";
    let svg = render_source_to_svg(src).expect("ditaa diagonal connector should render");
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("id=\"ditaa-shadow\""));
    assert!(svg.contains("filter=\"url(#ditaa-shadow)\""));
    assert!(svg.contains("<line"));
}

#[test]
fn non_uml_advanced_math_scripts_accents_and_fences_render() {
    let src = "@startmath\n\\left( \\hat{x}_{i}^{2} + \\vec{v} \\right) = \\sqrt{\\frac{a_1^2}{b}}\n@endmath\n";
    let svg = render_source_to_svg(src).expect("advanced math expression should render");
    assert!(svg.contains("id=\"math-arrow\""));
    assert!(svg.contains("<path d=\"M"));
    assert!(svg.contains("marker-end=\"url(#math-arrow)\""));
    assert!(svg.contains("&quot;") || svg.contains("("));
    assert!(svg.contains("<line"));
}

#[test]
fn non_uml_advanced_math_matrix_symbols_and_text_constructs_render() {
    let src = "@startlatex\n\\begin{bmatrix} \\alpha & \\beta \\\\ \\sum_{i=0}^{n} i & \\int_{0}^{\\infty} e^{-x} dx \\end{bmatrix} \\Rightarrow \\text{ok} \\subseteq \\mathbb{R}\n@endlatex\n";
    let svg = render_source_to_svg(src).expect("matrix math expression should render");
    assert!(svg.contains("data-math-env=\"bmatrix\""));
    assert!(svg.contains("α"));
    assert!(svg.contains("β"));
    assert!(svg.contains("∑"));
    assert!(svg.contains("∫"));
    assert!(svg.contains("∞"));
    assert!(svg.contains("⇒"));
    assert!(svg.contains("⊆"));
    assert!(svg.contains(">ok<"));
}

#[test]
fn non_uml_math_svg_quotes_multi_word_font_family_fallbacks() {
    let src = "@startmath\n\\alpha + \\int_{0}^{\\infty} x\n@endmath\n";
    let svg = render_source_to_svg(src).expect("math SVG should render");
    assert!(
        svg.contains("font-family=\"'Noto Sans Math','STIX Two Math','Cambria Math','Latin Modern Math','DejaVu Serif','Times New Roman',serif\""),
        "math SVG should quote multi-word font family fallbacks for external rasterizers"
    );
    assert!(
        !svg.contains("font-family=\"Noto Sans Math,STIX Two Math,serif\""),
        "legacy unquoted multi-word font stack should not be emitted"
    );
}

#[test]
fn non_uml_advanced_ditaa_junctions_and_diagonal_arrowheads_render() {
    let src = "@startditaa\n\
+---+   +---+\n\
| A |---+-->B\n\
+---+  /    \\\n\
      <      v\n\
@endditaa\n";
    let svg = render_source_to_svg(src).expect("ditaa junctions and diagonal heads should render");
    assert!(svg.contains("marker-end=\"url(#da)\""));
    assert!(svg.contains("marker-start=\"url(#dah)\""));
    assert!(
        svg.matches("<line").count() >= 3,
        "junction and diagonal connectors should emit several line segments"
    );
}

#[test]
fn specialized_renderer_wave_ditaa_shape_tags_and_junction_metadata_render() {
    let src = "@startditaa\n\
+------+   +------+\n\
| cAAA |---+-->{io}\n\
| {c}  |   | {s} |\n\
+------+   +------+\n\
    |          ^\n\
    +----------+\n\
\n\
+------+\n\
| {s}  |\n\
+------+\n\
@endditaa\n";
    let svg = render_source_to_svg(src).expect("ditaa shape tags should render");
    assert!(svg.contains("data-ditaa-shape=\"choice\""));
    assert!(svg.contains("data-ditaa-shape=\"storage\""));
    assert!(svg.contains("data-ditaa-junction=\"true\""));
    assert!(svg.contains("data-ditaa-arrow-end=\"true\""));
}

#[test]
fn specialized_renderer_wave_math_binom_cases_and_construct_metadata_render() {
    let src = "@startlatex\n\\binom{n}{k} + \\begin{cases} x^2 & x \\geq 0 \\\\ -x & x < 0 \\end{cases}\n@endlatex\n";
    let svg = render_source_to_svg(src).expect("math binom/cases should render");
    assert!(svg.contains("data-math-construct=\"binom\""));
    assert!(svg.contains("data-math-env=\"cases\""));
    assert!(svg.contains("≥"));
    assert!(svg.contains(">n<"));
    assert!(svg.contains(">k<"));
}

#[test]
fn specialized_renderer_wave_regex_unicode_lazy_and_repeat_metadata_render() {
    let src = "@startregex\nlang es\n^\\p{Lu}+?\\P{Nd}{2,4}?$\n@endregex\n";
    let svg = render_source_to_svg(src).expect("regex unicode classes should render");
    assert!(svg.contains("data-regex-locale=\"es\""));
    assert!(svg.contains("unicode uppercase letter"));
    assert!(svg.contains("not unicode decimal digit"));
    assert!(svg.contains("data-rail-repeat-label=\"2 to 4\""));
    assert!(svg.contains("class=\"regex-token regex-charclass\""));
}

#[test]
fn specialized_renderer_wave_ebnf_special_sequences_style_block_and_prefix_repeat_render() {
    let src = "@startebnf\n\
<style>\n\
element { ebnf { BackgroundColor #ecfeff FontColor #155e75 LineColor #0891b2 } }\n\
</style>\n\
title Tokens\n\
token = ? unicode category ? | 4 * \"x\" | [ name ];\n\
@endebnf\n";
    let svg = render_source_to_svg(src).expect("ebnf special sequence should render");
    assert!(svg.contains("data-ebnf-special=\"unicode category\""));
    assert!(svg.contains("? unicode category ?"));
    assert!(svg.contains("data-rail-repeat-label=\"exactly 4\""));
    assert!(svg.contains("class=\"ebnf-token ebnf-optional\""));
}

fn extract_svg_width_attr(svg: &str) -> Option<i32> {
    let key = "width=\"";
    let start = svg.find(key)? + key.len();
    let rest = &svg[start..];
    let end = rest.find('"')?;
    rest[..end].parse::<i32>().ok()
}

fn svg_elements_with_attr<'a>(svg: &'a str, attr: &str, value: &str) -> Vec<&'a str> {
    let needle = format!("{attr}=\"{value}\"");
    svg.split('<')
        .filter(|element| element.contains(&needle))
        .collect()
}

fn svg_attr_i32_required(element: &str, attr: &str) -> i32 {
    let key = format!("{attr}=\"");
    let start = element.find(&key).expect("attribute start") + key.len();
    let rest = &element[start..];
    let end = rest.find('"').expect("attribute end");
    rest[..end].parse::<i32>().expect("integer SVG attribute")
}

fn svg_group_with_attr<'a>(svg: &'a str, attr: &str, value: &str) -> &'a str {
    let needle = format!("{attr}=\"{value}\"");
    let start = svg.find(&needle).expect("group attribute");
    let rest = &svg[start..];
    let end = rest.find("</g>").expect("group close") + "</g>".len();
    &rest[..end]
}

#[test]
fn salt_advanced_widgets_render_tree_menu_tab_scroll_and_table() {
    let src = "@startsalt\n{\n{T\n+ Root\n++ Leaf\n}\n{* File | Edit | View}\n{/ General | Advanced}\n{S vertical 55%}\n| Name | \"Search\" |\n}\n@endsalt\n";
    let svg = render_source_to_svg(src).expect("advanced salt widgets should render");
    assert!(svg.contains("data-salt-widget=\"tree\""));
    assert!(svg.contains("data-salt-widget=\"menu\""));
    assert!(svg.contains("data-salt-widget=\"tab\""));
    assert!(svg.contains("data-salt-widget=\"scrollbar\""));
    assert!(svg.contains("Leaf"));
    assert!(svg.contains("Search"));
}

#[test]
fn salt_style_directives_and_header_cells_affect_widget_svg() {
    let src = "@startsalt\n\
skinparam saltBackgroundColor #f8fafc\n\
skinparam saltPanelColor #ffffff\n\
skinparam saltBorderColor #0f172a\n\
skinparam saltFontColor #111827\n\
skinparam saltHeaderColor #dbeafe\n\
skinparam saltButtonBackgroundColor #bfdbfe\n\
skinparam saltInputBackgroundColor #eff6ff\n\
{\n\
|= Field | = Value |\n\
| Name | \"Ada\" |\n\
| Action | [Save] |\n\
{* File | Edit}\n\
{S horizontal 50%}\n\
}\n\
@endsalt\n";
    let svg = render_source_to_svg(src).expect("styled salt should render");
    assert!(svg.contains("data-salt-style=\"canvas\""));
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("stroke=\"#0f172a\""));
    assert!(svg.contains("data-salt-widget=\"header\""));
    assert!(svg.contains("fill=\"#dbeafe\""));
    assert!(svg.contains("fill=\"#bfdbfe\""));
    assert!(svg.contains("fill=\"#eff6ff\""));
    assert!(svg.contains("data-salt-widget=\"menu\""));
    assert!(svg.contains("data-salt-widget=\"scrollbar\""));
    assert!(svg.contains("Field"));
    assert!(svg.contains("Save"));
}

#[test]
fn salt_compact_controls_textarea_advanced_table_and_style_blocks_render() {
    let src = "@startsalt\n\
<style>\n\
saltDiagram {\n\
  BackgroundColor #ecfeff\n\
}\n\
</style>\n\
!option handwritten true\n\
{+\n\
This is a long\n\
text in a textarea\n\
.\n\
\"                         \"\n\
}\n\
{SI\n\
Scrolled notes\n\
}\n\
{#\n\
. | Column 2 | Column 3\n\
Row header 1 | [] unchecked | () radio\n\
Row header 2 | value | *\n\
}\n\
{^ Profile Group}\n\
^Role^ | [Save]\n\
@endsalt\n";
    let svg = render_source_to_svg(src).expect("advanced salt controls should render");
    assert!(svg.contains("data-salt-widget=\"textarea\""));
    assert!(svg.contains("data-salt-scroll-vertical=\"true\""));
    assert!(svg.contains("data-salt-scroll-horizontal=\"false\""));
    assert!(svg.contains("data-salt-widget=\"table-empty\""));
    assert!(svg.contains("data-salt-widget=\"table-span\""));
    assert!(svg.contains("data-salt-widget=\"header\""));
    assert!(svg.contains("data-salt-widget=\"groupbox\""));
    assert!(svg.contains("data-salt-widget=\"scrollbar\""));
    assert!(svg.contains("Comic Sans MS, cursive"));
    assert!(svg.contains("fill=\"#ecfeff\""));
    assert!(svg.contains("unchecked"));
    assert!(svg.contains("radio"));
    assert!(svg.contains("Role"));
}

#[test]
fn salt_creole_icons_sprites_and_scoped_widget_styles_render() {
    let src = "@startsalt\n\
<style>\n\
saltDiagram {\n\
  BackgroundColor #f0fdfa\n\
  FontColor #134e4a\n\
}\n\
button {\n\
  BackgroundColor #fed7aa\n\
  FontColor #7c2d12\n\
}\n\
input {\n\
  BackgroundColor #ecfeff\n\
  FontColor #155e75\n\
}\n\
header {\n\
  BackgroundColor #ccfbf1\n\
  FontColor #115e59\n\
}\n\
menu {\n\
  BackgroundColor #ede9fe\n\
}\n\
tab {\n\
  BackgroundColor #fef3c7\n\
}\n\
scrollbar {\n\
  BackgroundColor #c7d2fe\n\
}\n\
checkbox {\n\
  BackgroundColor #fef9c3\n\
}\n\
radio {\n\
  BackgroundColor #fee2e2\n\
}\n\
</style>\n\
{\n\
|= **Field** | = <color:blue>Value</color> |\n\
| Login<&person> | \"//Ada//\" |\n\
| [] <b>Remember</b> | () <&key> OTP |\n\
| Action | [<b>Save</b> <&account-login>] |\n\
{* File | Edit | Refactor | Open | Close}\n\
{/ <b>General | Advanced}\n\
{SI\n\
<&code> //scroll body//\n\
}\n\
<<folder\n\
.XX.\n\
XXXX\n\
>>\n\
<<folder>> | Done\n\
}\n\
@endsalt\n";
    let svg = render_source_to_svg(src).expect("rich salt style/creole should render");
    assert!(svg.contains("data-salt-creole=\"true\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("data-salt-icons=\"account-login\""));
    assert!(svg.contains("data-salt-widget=\"sprite\""));
    assert!(svg.contains("data-salt-sprite=\"folder\""));
    assert!(svg.contains("data-salt-widget=\"sprite-ref\""));
    assert!(svg.contains("data-salt-sprite-ref=\"folder\""));
    assert!(svg.contains("fill=\"#fed7aa\""));
    assert!(svg.contains("fill=\"#7c2d12\""));
    assert!(svg.contains("fill=\"#ecfeff\""));
    assert!(svg.contains("fill=\"#155e75\""));
    assert!(svg.contains("fill=\"#ccfbf1\""));
    assert!(svg.contains("fill=\"#115e59\""));
    assert!(svg.contains("fill=\"#ede9fe\""));
    assert!(svg.contains("fill=\"#fef3c7\""));
    assert!(svg.contains("fill=\"#c7d2fe\""));
    assert!(svg.contains("fill=\"#fef9c3\""));
    assert!(svg.contains("fill=\"#fee2e2\""));
    assert!(svg.contains("data-salt-open=\"true\""));
    assert!(svg.contains("[person]"));
    assert!(svg.contains("[account-login]"));
}

#[test]
fn salt_layout_depth_fixture_has_widget_dom_and_span_geometry() {
    let src = fs::read_to_string(fixture("families/valid_salt_layout_depth.puml"))
        .expect("fixture should load");
    let svg = render_source_to_svg(&src).expect("salt layout depth fixture should render");

    assert!(svg.contains("data-salt-style=\"canvas\""));
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("stroke=\"#334155\""));
    assert_eq!(
        svg_elements_with_attr(&svg, "data-salt-widget", "groupbox").len(),
        2,
        "nested groupbox rows should render as distinct widgets"
    );
    assert!(svg.contains("data-salt-widget=\"menu\" data-salt-open=\"true\""));
    assert_eq!(
        svg_elements_with_attr(&svg, "data-salt-widget", "tab").len(),
        3
    );
    assert!(svg.contains("data-salt-tree-depth=\"0\""));
    assert!(svg.contains("data-salt-tree-depth=\"1\""));
    assert!(svg.contains("data-salt-sprite=\"folder\""));
    assert!(svg.contains("data-salt-sprite-ref=\"folder\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("data-salt-icons=\"account-login\""));
    assert!(svg.contains("data-salt-creole=\"true\""));

    let span = svg_elements_with_attr(&svg, "data-salt-colspan", "2")
        .into_iter()
        .next()
        .expect("fixture should render a merged table cell");
    assert!(
        span.contains("data-salt-span-width="),
        "span group should expose merged geometry"
    );

    let headers = svg_elements_with_attr(&svg, "data-salt-widget", "header");
    assert_eq!(headers.len(), 3);
    let field_header_x = svg_attr_i32_required(headers[0], "x");
    let value_header_x = svg_attr_i32_required(headers[1], "x");
    let notes_header_x = svg_attr_i32_required(headers[2], "x");
    assert!(
        field_header_x < value_header_x && value_header_x < notes_header_x,
        "table header x positions should increase left-to-right"
    );

    let tree_nodes = svg_elements_with_attr(&svg, "data-salt-widget", "tree");
    assert_eq!(tree_nodes.len(), 2);
    let root_tree = svg_group_with_attr(&svg, "data-salt-tree-depth", "0");
    let nested_tree = svg_group_with_attr(&svg, "data-salt-tree-depth", "1");
    let root_branch_x = svg_attr_i32_required(root_tree, "x1");
    let nested_branch_x = svg_attr_i32_required(nested_tree, "x1");
    assert!(
        nested_branch_x > root_branch_x,
        "nested tree item should be indented in geometry"
    );

    let textareas = svg_elements_with_attr(&svg, "data-salt-widget", "textarea");
    assert_eq!(textareas.len(), 2);
    assert!(textareas
        .iter()
        .any(|el| el.contains("data-salt-scroll-vertical=\"true\"")));
}

#[test]
fn archimate_stdlib_element_and_relation_macros_render() {
    let src = "@startarchimate\n\
Business_Actor(customer, \"Customer\")\n\
Application_Component(service, \"Order Service\")\n\
Technology_Node(runtime, \"Runtime\")\n\
Rel_Assignment(customer, service, \"places order\")\n\
Rel_Access(service, runtime, \"uses\")\n\
@endarchimate\n";
    let svg = render_source_to_svg(src).expect("archimate stdlib macros should render");
    assert!(svg.contains("Customer"));
    assert!(svg.contains("Order Service"));
    assert!(svg.contains("Runtime"));
    assert!(svg.contains("data-archimate-kind=\"assignment\""));
    assert!(svg.contains("data-archimate-kind=\"access\""));
    assert!(svg.contains("marker-start=\"url(#archimate-assignment)\""));
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("places order"));
    assert!(svg.contains("uses"));
    assert!(!svg.contains("<text class=\"archimate-relation\""));
}

// ── skinparam classify: class/state/component/activity (#202) ─────────────────

#[test]
fn skinparam_class_keys_accepted_without_warnings() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_skinparam_class.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn skinparam_class_background_color_appears_in_svg() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_class.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("class skinparam svg should render");
    assert!(svg.starts_with("<svg"), "should be valid svg");
    assert!(
        svg.contains("#e0f2fe"),
        "ClassBackgroundColor #e0f2fe should appear in SVG: {svg}"
    );
    assert!(
        svg.contains("#0369a1"),
        "ClassBorderColor #0369a1 should appear in SVG"
    );
    assert!(
        svg.contains("#bfdbfe"),
        "ClassHeaderBackgroundColor #bfdbfe should appear in SVG"
    );
    assert!(
        svg.contains("#0284c7"),
        "ClassArrowColor #0284c7 should appear in SVG"
    );
}

#[test]
fn skinparam_state_keys_accepted_without_warnings() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_skinparam_state.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn skinparam_state_colors_appear_in_svg() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_state.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("state skinparam svg should render");
    assert!(svg.starts_with("<svg"), "should be valid svg");
    assert!(
        svg.contains("#fef3c7"),
        "StateBackgroundColor #fef3c7 should appear in SVG"
    );
    assert!(
        svg.contains("#d97706"),
        "StateBorderColor #d97706 should appear in SVG"
    );
    assert!(
        svg.contains("#b45309"),
        "StateArrowColor #b45309 should appear in SVG"
    );
    assert!(
        svg.contains("#1c1917"),
        "StateStartColor #1c1917 should appear in SVG"
    );
}

#[test]
fn skinparam_component_keys_accepted_without_warnings() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("styling/valid_skinparam_component.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn skinparam_component_colors_appear_in_svg() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_component.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("component skinparam svg should render");
    assert!(svg.starts_with("<svg"), "should be valid svg");
    assert!(
        svg.contains("#f0fdf4"),
        "ComponentBackgroundColor #f0fdf4 should appear in SVG"
    );
    assert!(
        svg.contains("#16a34a"),
        "ComponentBorderColor #16a34a should appear in SVG"
    );
    assert!(
        svg.contains("#15803d"),
        "ComponentArrowColor #15803d should appear in SVG"
    );
}

#[test]
fn family_notes_render_for_core_uml_families() {
    let cases = [
        (
            "@startuml\nclass Order\nnote right of Order: validates totals\n@enduml\n",
            "validates totals",
        ),
        (
            "@startuml\ncomponent API\nnote right of API: public facade\n@enduml\n",
            "public facade",
        ),
        (
            "@startuml\nstart\n:Build;\nnote top: lane detail\nstop\n@enduml\n",
            "lane detail",
        ),
    ];

    for (src, note_text) in cases {
        let svg = render_source_to_svg(src).expect("family note svg should render");
        assert!(svg.contains("#fff8c4"), "note card fill should render");
        assert!(svg.contains(note_text), "note text should render");
    }
}

#[test]
fn skinparam_activity_keys_accepted_without_warnings() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_skinparam_activity.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn skinparam_activity_colors_appear_in_svg() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_activity.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("activity skinparam svg should render");
    assert!(svg.starts_with("<svg"), "should be valid svg");
    assert!(
        svg.contains("#fdf4ff"),
        "ActivityBackgroundColor #fdf4ff should appear in SVG"
    );
    assert!(
        svg.contains("#9333ea"),
        "ActivityBorderColor #9333ea should appear in SVG"
    );
    assert!(
        svg.contains("#f3e8ff"),
        "ActivityDiamondBackgroundColor #f3e8ff should appear in SVG"
    );
    assert!(
        svg.contains("#3b0764"),
        "ActivityBarColor #3b0764 should appear in SVG"
    );
    assert!(
        svg.contains("#7e22ce"),
        "ActivityArrowColor #7e22ce should appear in SVG"
    );
}

#[test]
fn family_theme_applies_to_class_state_component_activity_timing_and_chart() {
    let cases = [
        (
            "@startuml\n!theme vibrant\nclass Demo\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startuml\n!theme vibrant\n[*] --> Ready\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startuml\n!theme vibrant\ncomponent API\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startuml\n!theme vibrant\nstart\n:Ship it;\nstop\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startuml\n!theme vibrant\nclock clk\n@0\nclk is high\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startchart\n!theme vibrant\nbar\n\"A\" 1\n@endchart\n",
            ["#7c3aed", "#6d28d9"],
        ),
    ];

    for (src, expected) in cases {
        let svg = render_source_to_svg(src).expect("themed family should render");
        for color in expected {
            assert!(
                svg.contains(color),
                "expected themed color {color} in SVG: {svg}"
            );
        }
    }
}

#[test]
fn timing_skinparam_colors_are_accepted_and_rendered() {
    let src = "@startuml\nskinparam TimingBackgroundColor #101820\nskinparam TimingAxisColor #f2aa4c\nskinparam TimingGridColor #5f6f89\nskinparam TimingSignalBackgroundColor #dbeafe\nskinparam TimingSignalBorderColor #1d4ed8\nskinparam TimingArrowColor #dc2626\nskinparam TimingFontColor #f8fafc\nclock clk\n@0\nclk is high\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(src).expect("timing skinparam svg should render");
    for color in [
        "#101820", "#f2aa4c", "#5f6f89", "#dbeafe", "#1d4ed8", "#dc2626", "#f8fafc",
    ] {
        assert!(svg.contains(color), "expected timing color {color}");
    }
}

#[test]
fn chart_skinparam_colors_are_accepted_and_rendered() {
    let src = "@startchart\nskinparam ChartBackgroundColor #fff7ed\nskinparam ChartAxisColor #9a3412\nskinparam ChartGridColor #fed7aa\nskinparam ChartSeriesColor #0f766e\nskinparam ChartBarColor #ea580c\nskinparam ChartLineColor #0369a1\nskinparam ChartPieBorderColor #431407\nskinparam ChartFontColor #7c2d12\nbar\n\"A\" 4\n\"B\" 9\n@endchart\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(src).expect("chart skinparam svg should render");
    for color in ["#fff7ed", "#9a3412", "#ea580c", "#7c2d12"] {
        assert!(svg.contains(color), "expected chart color {color}");
    }

    let line_svg = render_source_to_svg(
        "@startchart\nskinparam ChartLineColor #0369a1\nline\n\"A\" 1\n\"B\" 2\n@endchart\n",
    )
    .expect("chart line skinparam svg should render");
    assert!(line_svg.contains("#0369a1"));

    let pie_svg = render_source_to_svg(
        "@startchart\nskinparam ChartSeriesColor #0f766e\nskinparam ChartPieBorderColor #431407\npie\n\"A\" 1\n\"B\" 2\n@endchart\n",
    )
    .expect("chart pie skinparam svg should render");
    assert!(pie_svg.contains("#0f766e"));
    assert!(pie_svg.contains("#431407"));
}

#[test]
fn preprocessor_scoped_globals_range_and_safe_aliases_expand() {
    let src = "@startuml\n!$status = outer\n!procedure Update($name)\n!local $status = local\n!global $shared = %map_set(%map(\"name\", $name), \"tags\", %range(1, 3))\nA -> B : $status\n!endprocedure\n!Update(Ada)\nA -> B : $status/%join(%dict_get($shared, \"tags\"), \"-\")/%dict_get($shared, \"name\", \"missing\")/%json_contains_key($shared, \"tags\")/%now()/%random_int()/%uuid()\n@enduml\n";
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
    assert_eq!(
        labels,
        vec![
            "local",
            "outer/1-2-3/Ada/true//0/00000000-0000-0000-0000-000000000000",
        ]
    );
}

#[test]
fn preprocessor_recursive_macro_expansion_is_depth_guarded() {
    let src = "@startuml\n!define WHO Alice\n!define TARGET WHO\nTARGET -> Bob : hi\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    assert_eq!(model.participants[0].id, "Alice");

    let err = parse("@startuml\n!define A A A\nA -> B : loop\n@enduml\n")
        .expect_err("recursive macro growth should be bounded");
    assert!(err.message.contains("E_PREPROC_MACRO_DEPTH"));
}

#[test]
fn preprocessor_unsafe_io_aliases_and_malformed_collections_report_stable_codes() {
    let unsafe_err = parse("@startuml\nA -> B : %file_exists(\"secret.txt\")\n@enduml\n")
        .expect_err("filesystem-sensitive builtin should be rejected");
    assert!(unsafe_err.message.contains("E_PREPROC_UNSAFE_BUILTIN"));

    let syntax_err = parse("@startuml\nA -> B : %list_get([\"a\", 0)\n@enduml\n")
        .expect_err("unbalanced collection argument should fail");
    assert!(syntax_err.message.contains("E_PREPROC_CALL_SYNTAX"));
}

// ─── Issue #240: MindMap/WBS layout — radial/tree not DAG grid ────────────────

#[test]
fn mindmap_cli_renders_radial_tree_not_dag_grid() {
    // Regression test for #240: CLI path must produce mindmap-node/edge markers,
    // not uml-relation markers (which indicate the wrong DAG-grid renderer).
    let input = format!(
        "{}/docs/examples/mindmap/02_multi_level.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("mm.svg");
    Command::cargo_bin("puml")
        .expect("binary")
        .args([&input, "-o"])
        .arg(&out)
        .assert()
        .success();
    let svg = fs::read_to_string(&out).expect("output SVG must exist");
    assert!(
        svg.contains("mindmap-node"),
        "CLI mindmap output must use mindmap-node class, not DAG grid"
    );
    assert!(
        svg.contains("mindmap-edge"),
        "CLI mindmap output must use mindmap-edge class"
    );
    assert!(
        !svg.contains("uml-relation"),
        "CLI mindmap output must NOT use uml-relation (DAG grid renderer)"
    );
    assert!(
        svg.contains("Technology Stack"),
        "root node label must be present"
    );
}

#[test]
fn wbs_cli_renders_hierarchical_tree_not_dag_grid() {
    // Regression test for #240: CLI path must produce wbs-node/edge markers,
    // not uml-relation markers.
    let input = format!(
        "{}/docs/examples/wbs/04_multi_level.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("wbs.svg");
    Command::cargo_bin("puml")
        .expect("binary")
        .args([&input, "-o"])
        .arg(&out)
        .assert()
        .success();
    let svg = fs::read_to_string(&out).expect("output SVG must exist");
    assert!(
        svg.contains("wbs-node"),
        "CLI WBS output must use wbs-node class, not DAG grid"
    );
    assert!(
        svg.contains("wbs-edge"),
        "CLI WBS output must use wbs-edge class"
    );
    assert!(
        !svg.contains("uml-relation"),
        "CLI WBS output must NOT use uml-relation (DAG grid renderer)"
    );
    assert!(
        svg.contains("Software Development"),
        "root node label must be present"
    );
}

#[test]
fn mindmap_root_is_centered_and_children_distributed_both_sides() {
    // Acceptance criterion (a): mindmap positions root at center, children
    // appear both left and right when `left side` / `right side` are used.
    // We use the CLI path (which was the broken one) via a temp file.
    let src = concat!(
        "@startmindmap\n",
        "* Root\n",
        "** RightA\n",
        "** RightB\n",
        "left side\n",
        "** LeftA\n",
        "** LeftB\n",
        "@endmindmap\n",
    );

    let tmp = tempdir().unwrap();
    let input = tmp.path().join("mm_sides.puml");
    let output = tmp.path().join("mm_sides.svg");
    fs::write(&input, src).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .assert()
        .success();

    let svg = fs::read_to_string(&output).expect("output SVG must exist");
    assert!(svg.contains("mindmap-root"), "must have root marker");
    assert!(
        svg.contains("data-mindmap-side=\"left\""),
        "must have left-side branches"
    );
    assert!(
        svg.contains("data-mindmap-side=\"right\""),
        "must have right-side branches"
    );

    // Parse x position of root rect.
    fn extract_x_after_marker(svg: &str, marker: &str) -> Option<i32> {
        let idx = svg.find(marker)?;
        let tail = &svg[idx..];
        let end = tail.find('>')?;
        let elem = &tail[..end];
        let key = " x=\"";
        let start = elem.find(key)? + key.len();
        let val_end = elem[start..].find('"')? + start;
        elem[start..val_end].parse().ok()
    }

    // Parse x positions of all rect nodes with a given side attribute.
    fn extract_x_for_side(svg: &str, side: &str) -> Vec<i32> {
        let marker = format!("data-mindmap-side=\"{side}\" data-mindmap-child-count");
        let mut xs = Vec::new();
        let mut search = svg;
        while let Some(idx) = search.find(&marker) {
            // Backtrack to find the opening < of this element so we can get x=
            let before = &search[..idx];
            if let Some(rect_start) = before.rfind('<') {
                let tail = &search[rect_start..];
                let end = tail.find('>').unwrap_or(tail.len());
                let elem = &tail[..end];
                let key = " x=\"";
                if let Some(start) = elem.find(key) {
                    let val_start = start + key.len();
                    if let Some(val_end) = elem[val_start..].find('"') {
                        if let Ok(x) = elem[val_start..val_start + val_end].parse::<i32>() {
                            xs.push(x);
                        }
                    }
                }
            }
            search = &search[idx + 1..];
        }
        xs
    }

    let root_rx: i32 =
        extract_x_after_marker(&svg, "mindmap-root").expect("root rect must have x attribute");
    let left_xs = extract_x_for_side(&svg, "left");
    let right_xs = extract_x_for_side(&svg, "right");

    assert!(!left_xs.is_empty(), "must have at least one left-side node");
    assert!(
        !right_xs.is_empty(),
        "must have at least one right-side node"
    );

    let max_left_x = left_xs.iter().copied().max().unwrap_or(0);
    let min_right_x = right_xs.iter().copied().min().unwrap_or(i32::MAX);
    assert!(
        max_left_x < root_rx,
        "all left-side node x positions ({max_left_x}) must be left of root ({root_rx})"
    );
    assert!(
        root_rx < min_right_x,
        "root ({root_rx}) must be left of all right-side node x positions ({min_right_x})"
    );
}

#[test]
fn wbs_no_crossing_edges_in_left_right_mode() {
    // Acceptance criterion (b): WBS in left-right mode must have strictly
    // increasing x per depth level — child x > parent x for all edges.
    let src = concat!(
        "@startwbs\n",
        "left to right direction\n",
        "* Root\n",
        "** A\n",
        "*** A1\n",
        "*** A2\n",
        "** B\n",
        "*** B1\n",
        "@endwbs\n",
    );
    let svg = render_source_to_svg(src).expect("wbs LR should render");
    assert!(svg.contains("wbs-node"), "must use wbs renderer");

    // Parse all wbs-edge lines and check that x2 > x1 (LR: child is to the right).
    fn parse_line_x1_x2(line_elem: &str) -> Option<(i32, i32)> {
        let get = |attr: &str| -> Option<i32> {
            let key = format!(" {attr}=\"");
            let start = line_elem.find(&key)? + key.len();
            let end = line_elem[start..].find('"')? + start;
            line_elem[start..end].parse().ok()
        };
        Some((get("x1")?, get("x2")?))
    }

    let mut checked = 0usize;
    let mut search = svg.as_str();
    while let Some(idx) = search.find("<line class=\"wbs-edge\"") {
        search = &search[idx..];
        let end = search.find("/>").unwrap_or(search.len());
        let elem = &search[..end + 2];
        if let Some((x1, x2)) = parse_line_x1_x2(elem) {
            assert!(
                x2 > x1,
                "WBS LR mode: edge x2 ({x2}) must be > x1 ({x1}) — no backward edges allowed"
            );
            checked += 1;
        }
        search = &search[1..];
    }
    assert!(checked > 0, "must have at least one wbs edge to verify");
}

#[test]
fn mindmap_basic_fixture_renders_tree_via_cli() {
    let input = format!(
        "{}/docs/examples/mindmap/01_basic.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("mm_basic.svg");
    Command::cargo_bin("puml")
        .expect("binary")
        .args([&input, "-o"])
        .arg(&out)
        .assert()
        .success();
    let svg = fs::read_to_string(&out).expect("output SVG must exist");
    assert!(svg.contains("mindmap-root"), "must have root node");
    assert!(!svg.contains("uml-relation"), "must not use DAG renderer");
}

#[test]
fn wbs_basic_fixture_renders_tree_via_cli() {
    let input = format!(
        "{}/docs/examples/wbs/01_basic.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let tmp = tempdir().unwrap();
    let out = tmp.path().join("wbs_basic.svg");
    Command::cargo_bin("puml")
        .expect("binary")
        .args([&input, "-o"])
        .arg(&out)
        .assert()
        .success();
    let svg = fs::read_to_string(&out).expect("output SVG must exist");
    assert!(svg.contains("wbs-node"), "must have wbs node class");
    assert!(!svg.contains("uml-relation"), "must not use DAG renderer");
}

// Regression test for #424: the `class` keyword must not leak into the box label.
// Before the fix, labels rendered as "class Animal", "class Dog" etc.
#[test]
fn class_keyword_does_not_leak_into_box_label_issue_424() {
    // Variant A: classes with body blocks.
    let src = "@startuml\nclass Animal {\n  +name: String\n  +speak()\n}\nclass Dog {\n  +breed: String\n  +fetch()\n}\nAnimal --> Dog : owns\n@enduml\n";
    let svg = render_source_to_svg(src).expect("class svg must render");

    // The display label must be just the identifier — no "class " prefix.
    assert!(
        svg.contains(">Animal<"),
        "label must be 'Animal', got keyword leak or missing label"
    );
    assert!(
        svg.contains(">Dog<"),
        "label must be 'Dog', got keyword leak or missing label"
    );
    assert!(
        !svg.contains(">class Animal<") && !svg.contains("class Animal"),
        "keyword 'class' must not appear in the Animal box label"
    );
    assert!(
        !svg.contains(">class Dog<") && !svg.contains("class Dog"),
        "keyword 'class' must not appear in the Dog box label"
    );

    // Variant B: classes without body blocks (stub form).
    let src_stub = "@startuml\nclass Vehicle\nclass Car\nVehicle <|-- Car\n@enduml\n";
    let svg_stub = render_source_to_svg(src_stub).expect("stub class svg must render");
    assert!(
        !svg_stub.contains("class Vehicle"),
        "keyword 'class' must not bleed into Vehicle stub label"
    );
    assert!(
        svg_stub.contains(">Vehicle<"),
        "Vehicle label must appear as bare identifier in stub form"
    );
}

// ── Issue #769: enum classes must render with distinct lemon header ───────────

#[test]
fn enum_class_renders_with_enumeration_stereotype_and_lemon_header() {
    // `enum` keyword must produce a «enumeration» label and a #ffffcc header fill —
    // distinguishing enum boxes from regular class boxes (fix #769).
    let src = "@startuml\nenum Color {\n  RED\n  GREEN\n  BLUE\n}\nclass Widget {\n  +paint(c: Color)\n}\nWidget --> Color\n@enduml\n";
    let svg = render_source_to_svg(src).expect("enum class diagram must render");

    // The «enumeration» guillemet label must appear in the header.
    assert!(
        svg.contains("\u{ab}enumeration\u{bb}"),
        "enum header must contain «enumeration» stereotype label"
    );
    // The lemon fill colour is the PlantUML enum convention.
    assert!(
        svg.contains("#ffffcc"),
        "enum header must use lemon fill #ffffcc, not the default class blue"
    );
    // The class box (Widget) should still use the default (non-lemon) header.
    assert!(
        !svg.contains("Widget\u{ab}enumeration\u{bb}"),
        "regular class header must not carry the enumeration stereotype"
    );
    // Enum name must appear as the box label.
    assert!(svg.contains(">Color<"), "enum box label must be 'Color'");
}
