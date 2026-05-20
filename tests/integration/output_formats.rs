use super::*;

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
    let src = include_str!("../fixtures/families/valid_class_with_relations.puml");
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
