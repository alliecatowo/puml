use super::*;

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
