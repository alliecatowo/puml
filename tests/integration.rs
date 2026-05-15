use assert_cmd::Command;
use insta::{assert_json_snapshot, assert_snapshot};
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::NamedTempFile;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn single_file_defaults_to_text_output() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .arg(fixture("single_valid.puml"))
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    assert_snapshot!(
        "single_file_defaults_to_text_output",
        String::from_utf8(out).unwrap()
    );
}

#[test]
fn check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("single_valid.puml")])
        .assert()
        .success()
        .stdout(predicate::str::contains("passed validation"));
}

#[test]
fn check_mode_fails_for_invalid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("invalid_single.puml")])
        .assert()
        .code(5)
        .stderr(predicate::str::contains("validation failed"));
}

#[test]
fn dump_mode_outputs_json_array() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("dump_mode_outputs_json_array", json);
}

#[test]
fn multi_mode_outputs_all_diagrams_as_json() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", &fixture("multi_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("multi_mode_outputs_all_diagrams_as_json", json);
}

#[test]
fn multi_input_without_flag_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg(fixture("multi_valid.puml"))
        .assert()
        .code(4)
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
        .code(3)
        .stderr(predicate::str::contains("failed to read"));
}

#[test]
fn empty_input_maps_to_input_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg(fixture("empty.txt"))
        .assert()
        .code(4)
        .stderr(predicate::str::contains("no diagram content provided"));
}

#[test]
fn plain_multi_delimiter_supported_with_multi_flag() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", &fixture("plain_multi.txt")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("plain_multi_delimiter_supported_with_multi_flag", json);
}

#[test]
fn json_format_outputs_single_record() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "json", &fixture("single_valid.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("json_format_outputs_single_record", json);
}

#[test]
fn check_and_dump_are_mutually_exclusive() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--dump", &fixture("single_valid.puml")])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn can_read_tempfile_input() {
    let tmp = NamedTempFile::new().unwrap();
    fs::write(tmp.path(), "@startuml\nX -> Y\n@enduml\n").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("X -> Y"));
}
