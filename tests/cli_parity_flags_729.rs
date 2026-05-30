//! Integration tests for PlantUML CLI parity flags added in issue #729.
//!
//! Covers: --failfast2 (exit code 2 on parse error), --metadata-output <file>
//! (write JSON metadata to a file), -tlatex/-thtml/-tutxt/-ttxt stubs,
//! --extract, and --pattern. All flags are tested end-to-end via the
//! compiled binary.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

const SIMPLE: &str = "@startuml\nAlice -> Bob: hi\n@enduml\n";
const INVALID: &str = "@startuml\n!!!not-valid-syntax!!!\n@enduml\n";

// ---------------------------------------------------------------------------
// --failfast2: exit code remapping
// ---------------------------------------------------------------------------

#[test]
fn failfast2_exits_2_on_parse_error() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--failfast2", "-"])
        .write_stdin(INVALID)
        .assert()
        .code(2)
        .stderr(predicate::str::contains("E_PREPROC_UNSUPPORTED").or(
            predicate::str::contains("E_FAMILY_UNKNOWN").or(predicate::str::contains("error")),
        ));
}

#[test]
fn failfast2_still_exits_0_on_valid_input() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--failfast2", "--check", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn without_failfast2_parse_error_exits_1() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["-"])
        .write_stdin(INVALID)
        .assert()
        .code(1);
}

// ---------------------------------------------------------------------------
// --metadata (bool): emit JSON to stdout
// --metadata-output <file>: write JSON metadata to a file
// ---------------------------------------------------------------------------

#[test]
fn metadata_flag_alone_writes_json_to_stdout() {
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--metadata", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).expect("stdout should be valid JSON");
    assert_eq!(json["schema"], "puml.metadata");
    assert_eq!(json["family"], "sequence");
}

#[test]
fn metadata_output_flag_writes_json_to_file_not_stdout() {
    let tmp = tempdir().expect("tempdir");
    let meta_path = tmp.path().join("meta.json");

    Command::cargo_bin("puml")
        .expect("puml binary")
        .args([
            "--metadata",
            "--metadata-output",
            meta_path.to_str().unwrap(),
            "-",
        ])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let contents = fs::read_to_string(&meta_path).expect("metadata output file should be written");
    let json: Value =
        serde_json::from_str(&contents).expect("metadata output file should be valid JSON");
    assert_eq!(json["schema"], "puml.metadata");
    assert_eq!(json["family"], "sequence");
    assert!(
        json["counts"]["participants"].as_u64().unwrap_or(0) >= 2,
        "should have at least 2 participants"
    );
}

#[test]
fn metadata_output_flag_with_verbose_reports_written_path() {
    let tmp = tempdir().expect("tempdir");
    let meta_path = tmp.path().join("out.json");

    Command::cargo_bin("puml")
        .expect("puml binary")
        .args([
            "--metadata",
            "--metadata-output",
            meta_path.to_str().unwrap(),
            "--verbose",
            "-",
        ])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("[verbose] metadata written to"));
}

#[test]
fn metadata_output_requires_metadata_flag() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--metadata-output", "/tmp/should-not-be-created.json", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("metadata-output").or(predicate::str::contains("requires")),
        );
}

// ---------------------------------------------------------------------------
// -tlatex / -tlatex:nopreamble: unsupported format stubs exit 2
// ---------------------------------------------------------------------------

#[test]
fn tlatex_alias_reports_unsupported_and_exits_2() {
    for args in [vec!["-tlatex", "-"], vec!["-tlatex:nopreamble", "-"]] {
        Command::cargo_bin("puml")
            .expect("puml binary")
            .args(args)
            .write_stdin(SIMPLE)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("E_OUTPUT_FORMAT_UNSUPPORTED"))
            .stderr(predicate::str::contains("svg"));
    }
}

// ---------------------------------------------------------------------------
// -thtml: supported format alias
// ---------------------------------------------------------------------------

#[test]
fn thtml_alias_produces_html_output() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["-thtml", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::starts_with("<!doctype html>"))
        .stdout(predicate::str::contains("hi"));
}

// ---------------------------------------------------------------------------
// -tutxt / -ttxt: ASCII art aliases
// ---------------------------------------------------------------------------

#[test]
fn text_format_aliases_produce_ascii_art_output() {
    for alias in ["-tutxt", "-ttxt", "-utxt", "-txt"] {
        Command::cargo_bin("puml")
            .expect("puml binary")
            .args([alias, "-"])
            .write_stdin(SIMPLE)
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .stdout(predicate::str::contains("Alice"));
    }
}

// ---------------------------------------------------------------------------
// --extract: split multi-diagram .puml into one file per diagram
// ---------------------------------------------------------------------------

#[test]
fn extract_splits_multi_diagram_file_into_numbered_puml_files() {
    let tmp = tempdir().expect("tempdir");
    let input = tmp.path().join("multi.puml");
    fs::write(
        &input,
        "@startuml\nAlice -> Bob : a\n@enduml\n\n@startuml\nCarol -> Dave : b\n@enduml\n",
    )
    .expect("write input");

    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--extract", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let first = fs::read_to_string(tmp.path().join("multi-extracted-1.puml")).unwrap();
    let second = fs::read_to_string(tmp.path().join("multi-extracted-2.puml")).unwrap();
    assert!(
        first.contains("Alice"),
        "first diagram should contain Alice"
    );
    assert!(
        second.contains("Carol"),
        "second diagram should contain Carol"
    );
}

#[test]
fn extract_stdin_prints_blocks_separated_by_blank_line() {
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--extract", "-"])
        .write_stdin(
            "@startuml\nAlice -> Bob : one\n@enduml\n\n@startuml\nCarol -> Dave : two\n@enduml\n",
        )
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Alice"), "first block should be present");
    assert!(stdout.contains("Carol"), "second block should be present");
}

// ---------------------------------------------------------------------------
// --pattern: regex filter on file selection
// ---------------------------------------------------------------------------

#[test]
fn pattern_filters_files_by_regex_in_lint_mode() {
    let tmp = tempdir().expect("tempdir");
    let keep = tmp.path().join("keep_this.puml");
    let skip = tmp.path().join("skip_this.puml");
    fs::write(&keep, SIMPLE).unwrap();
    fs::write(&skip, SIMPLE).unwrap();
    let glob = format!("{}/*.puml", tmp.path().display());

    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args([
            "--check",
            "--lint-glob",
            &glob,
            "--pattern",
            "keep_this",
            "--lint-report",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let report: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(report["summary"]["total_files"], 1);
    let path = report["files"][0]["path"].as_str().unwrap();
    assert!(
        path.ends_with("keep_this.puml"),
        "unexpected path in report: {path}"
    );
}

#[test]
fn pattern_with_invalid_regex_reports_validation_error_exit_1() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args([
            "--check",
            "--lint-input",
            "tests/fixtures/single_valid.puml",
            "--pattern",
            "[bad-regex",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("invalid --pattern regex"));
}
