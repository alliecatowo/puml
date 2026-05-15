use assert_cmd::Command;
use insta::{assert_json_snapshot, assert_snapshot};
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

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
fn check_mode_fails_for_invalid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("invalid_single.puml")])
        .assert()
        .code(1);
}

#[test]
fn check_mode_passes_for_additional_valid_fixtures() {
    for case in [
        "single_valid.puml",
        "basic/valid_start_end.puml",
        "basic/valid_arrow.txt",
        "participants/valid_aliases.puml",
        "participants/valid_queue_separator.puml",
        "arrows/valid_directions.puml",
        "arrows/self.puml",
        "arrows/modifiers_basic.puml",
        "arrows/valid_expanded_forms.puml",
        "notes/valid_note_over.puml",
        "groups/valid_alt_end.puml",
        "groups/valid_loop_end.puml",
        "groups/valid_par_else_end.puml",
        "groups/valid_ref_and_else_rendering.puml",
        "autonumber/valid_basic.puml",
        "autonumber/valid_with_format.puml",
        "lifecycle/valid_activate_return.puml",
        "lifecycle/valid_create_activate_destroy.puml",
        "lifecycle/valid_shortcuts_expansion.puml",
        "lifecycle/valid_return_inferred_from_shortcut_activation.puml",
        "notes/valid_multiline_blocks.puml",
        "notes/valid_note_across_multi.puml",
        "structure/valid_separator_delay_divider_spacer.puml",
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
        "styling/valid_skinparam_unsupported.puml",
        "non_sequence/invalid_class_diagram.puml",
        "non_sequence/invalid_state_diagram.puml",
        "include/error_include_cycle_self.puml",
        "include/error_include_chain_a.puml",
        "lifecycle/valid_destroy_then_message.puml",
        "lifecycle/invalid_return_without_activation.puml",
        "lifecycle/invalid_return_without_caller_context.puml",
        "arrows/invalid_malformed_arrows.puml",
        "errors/invalid_malformed_note_ref.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1);
    }
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
    for case in [
        "non_sequence/invalid_class_diagram.puml",
        "non_sequence/invalid_state_diagram.puml",
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(case)])
            .assert()
            .code(1)
            .stderr(
                predicate::str::contains("puml currently renders sequence diagrams only").or(
                    predicate::str::contains("[E_ARROW_INVALID] malformed sequence arrow syntax"),
                ),
            );
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
fn lifecycle_return_without_activation_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("lifecycle/invalid_return_without_activation.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_RETURN_INFER_EMPTY"));
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
fn file_newpage_output_writes_numbered_files_without_multi_flag() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::write(
        &input,
        "@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(input.to_str().unwrap())
        .assert()
        .success();

    assert!(tmp.path().join("paged-1.svg").exists());
    assert!(tmp.path().join("paged-2.svg").exists());
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
        .args([input.to_str().unwrap(), "--output", "/"])
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
