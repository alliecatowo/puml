use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
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
fn file_multi_blocks_without_multi_writes_numbered_files() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("multi_three.puml");
    fs::copy(fixture("structure/multi_three.puml"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(input.to_str().unwrap())
        .assert()
        .success();

    assert!(tmp.path().join("multi_three-1.svg").exists());
    assert!(tmp.path().join("multi_three-2.svg").exists());
    assert!(tmp.path().join("multi_three-3.svg").exists());
}

#[test]
fn explicit_startuml_marker_detection_ignores_inline_text() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("inline_token.puml");
    fs::write(&input, "Alice -> Bob : mentions @startuml in message\n").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(input.to_str().unwrap())
        .assert()
        .success();

    assert!(tmp.path().join("inline_token.svg").exists());
}
