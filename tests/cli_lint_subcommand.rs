/// Integration tests for `puml lint <file>`.
///
/// Covers:
///   - Exit 0 on a known-good fixture (human and JSON formats)
///   - Exit 1 on a known-bad fixture (human and JSON formats)
///   - Exit 2 on a missing file
///   - --quiet suppresses non-error output in human mode
///   - JSON schema shape (file, diagnostics, summary fields present)
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

/// Path to a fixture that is known-good (parses and normalizes without errors).
const GOOD_FIXTURE: &str = "tests/fixtures/single_valid.puml";

/// Path to a fixture that is syntactically invalid and should produce an error.
const BAD_FIXTURE: &str = "tests/fixtures/errors/invalid_plain.txt";

// ── Known-good fixture ────────────────────────────────────────────────────────

#[test]
fn lint_good_file_exits_zero_human() {
    Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["lint", GOOD_FIXTURE])
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::is_empty());
}

#[test]
fn lint_good_file_exits_zero_json() {
    let output = Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["lint", GOOD_FIXTURE, "--format", "json"])
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("stdout should be valid JSON");

    // Schema shape assertions.
    assert!(value.get("file").is_some(), "JSON must have 'file' field");
    assert!(
        value.get("diagnostics").is_some(),
        "JSON must have 'diagnostics' field"
    );
    let summary = value
        .get("summary")
        .expect("JSON must have 'summary' field");
    assert_eq!(
        summary.get("errors").and_then(|v| v.as_u64()),
        Some(0),
        "good fixture must have 0 errors"
    );
    let diags = value["diagnostics"]
        .as_array()
        .expect("diagnostics must be array");
    assert!(diags.is_empty(), "good fixture must have no diagnostics");
}

// ── Known-bad fixture ─────────────────────────────────────────────────────────

#[test]
fn lint_bad_file_exits_one_human() {
    Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["lint", BAD_FIXTURE])
        .assert()
        .failure()
        .code(1)
        // At least one error line should appear on stderr.
        .stderr(predicate::str::contains("error"));
}

#[test]
fn lint_bad_file_exits_one_json() {
    let output = Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["lint", BAD_FIXTURE, "--format", "json"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).expect("stdout should be UTF-8");
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("stdout should be valid JSON even on error exit");

    // Schema shape assertions.
    let summary = value
        .get("summary")
        .expect("JSON must have 'summary' field");
    let errors = summary
        .get("errors")
        .and_then(|v| v.as_u64())
        .expect("summary.errors must be a number");
    assert!(errors > 0, "bad fixture must report at least one error");

    let diags = value["diagnostics"]
        .as_array()
        .expect("diagnostics must be array");
    assert!(
        !diags.is_empty(),
        "bad fixture must have at least one diagnostic"
    );

    // Each diagnostic entry must have severity, message, and span fields.
    for diag in diags {
        assert!(
            diag.get("severity").is_some(),
            "diagnostic must have severity"
        );
        assert!(
            diag.get("message").is_some(),
            "diagnostic must have message"
        );
        assert!(diag.get("span").is_some(), "diagnostic must have span");
        let span = &diag["span"];
        assert!(
            span.get("start_line").is_some(),
            "span must have start_line"
        );
        assert!(span.get("start_col").is_some(), "span must have start_col");
        assert!(span.get("end_line").is_some(), "span must have end_line");
        assert!(span.get("end_col").is_some(), "span must have end_col");
    }
}

// ── Missing file ──────────────────────────────────────────────────────────────

#[test]
fn lint_missing_file_exits_two() {
    Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["lint", "this_file_does_not_exist_ever.puml"])
        .assert()
        .failure()
        .code(2);
}

// ── --quiet flag ──────────────────────────────────────────────────────────────

#[test]
fn lint_quiet_good_file_suppresses_summary() {
    Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["lint", GOOD_FIXTURE, "--quiet"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn lint_quiet_bad_file_still_emits_errors() {
    Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["lint", BAD_FIXTURE, "--quiet"])
        .assert()
        .failure()
        .code(1)
        // Errors must still appear on stderr even with --quiet.
        .stderr(predicate::str::contains("error"));
}

#[test]
fn lint_splits_multi_block_files() {
    Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["lint", "tests/fixtures/structure/multi_three.puml"])
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::is_empty());
}

#[test]
fn lint_stdin_honors_global_include_root() {
    let root = tempdir().expect("tempdir");
    fs::write(root.path().join("child.puml"), "Alice -> Bob : included\n").unwrap();

    Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["--include-root", root.path().to_str().unwrap(), "lint", "-"])
        .write_stdin("@startuml\n!include child.puml\n@enduml\n")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::is_empty());
}

#[test]
fn lint_honors_global_define_variables() {
    Command::cargo_bin("puml")
        .expect("puml binary must exist")
        .args(["-D", "SHOW=yes", "lint", "-"])
        .write_stdin("@startuml\n!if $SHOW == \"yes\"\nAlice -> Bob : shown\n!endif\n@enduml\n")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::is_empty());
}
