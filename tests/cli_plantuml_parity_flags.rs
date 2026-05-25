use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

const SIMPLE: &str = "@startuml\nAlice -> Bob: hi\n@enduml\n";

#[test]
fn single_dash_thtml_alias_selects_html_output() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["-thtml", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::starts_with("<!doctype html>"))
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("hi"));
}

#[test]
fn output_format_alias_selects_supported_text_output() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--output-format", "txt", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("Alice -> Bob: hi"));
}

#[test]
fn plantuml_text_format_aliases_remain_supported() {
    for alias in ["-ttxt", "-tutxt", "-txt", "-utxt"] {
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

#[test]
fn threads_and_failfast2_are_deterministic_noops() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--threads", "4", "--failfast2", "--check", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn unsupported_latex_alias_reports_supported_formats_with_exit_2() {
    for args in [
        vec!["-tlatex", "-"],
        vec!["-tlatex:nopreamble", "-"],
        vec!["--format", "latex", "-"],
        vec!["--output-format", "latex:nopreamble", "-"],
    ] {
        Command::cargo_bin("puml")
            .expect("puml binary")
            .args(args)
            .write_stdin(SIMPLE)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("E_OUTPUT_FORMAT_UNSUPPORTED"))
            .stderr(predicate::str::contains("supported formats: svg, html"));
    }
}

#[test]
fn help_documents_parity_flags_and_output_format_alias() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--threads <N>"))
        .stdout(predicate::str::contains("--failfast2"))
        .stdout(predicate::str::contains("--extract"))
        .stdout(predicate::str::contains("--pattern <REGEX>"))
        .stdout(predicate::str::contains("--output-format"));
}

#[test]
fn extract_splits_file_inputs_into_deterministic_source_files() {
    let tmp = tempdir().expect("tempdir");
    let input = tmp.path().join("batch.puml");
    fs::write(
        &input,
        "@startuml\nAlice -> Bob : one\n@enduml\n\n@startuml\nCarol -> Dave : two\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--extract", "--verbose", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "[verbose] extracted 2 diagram source file(s)",
        ));

    let first = fs::read_to_string(tmp.path().join("batch-extracted-1.puml")).unwrap();
    let second = fs::read_to_string(tmp.path().join("batch-extracted-2.puml")).unwrap();
    assert_eq!(first, "@startuml\nAlice -> Bob : one\n@enduml\n");
    assert_eq!(second, "@startuml\nCarol -> Dave : two\n@enduml\n");
}

#[test]
fn extract_stdin_prints_split_sources_without_rendering() {
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
    assert!(stdout.contains("Alice -> Bob : one"));
    assert!(stdout.contains("Carol -> Dave : two"));
    assert!(
        stdout.contains("@enduml\n\n@startuml"),
        "extracted stdin sources should use one blank separator: {stdout:?}"
    );
}

#[test]
fn pattern_filters_lint_file_selection_with_stable_json_report() {
    let tmp = tempdir().expect("tempdir");
    let keep = tmp.path().join("keep_me.puml");
    let skip = tmp.path().join("skip_me.puml");
    fs::write(&keep, SIMPLE).unwrap();
    fs::write(&skip, SIMPLE).unwrap();
    let glob = format!("{}/*.puml", tmp.path().display());

    let output = Command::cargo_bin("puml")
        .expect("puml binary")
        .args([
            "--check",
            "--lint-glob",
            &glob,
            "--pattern",
            "keep_me",
            "--lint-report",
            "json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let report: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(report["summary"]["total_files"], 1);
    let path = report["files"][0]["path"].as_str().unwrap();
    assert!(path.ends_with("keep_me.puml"), "unexpected path: {path}");
}

#[test]
fn invalid_pattern_reports_validation_error_before_linting() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args([
            "--check",
            "--lint-input",
            "tests/fixtures/single_valid.puml",
            "--pattern",
            "[unterminated",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("invalid --pattern regex"));
}

#[test]
fn single_input_pattern_mismatch_reports_no_files_selected() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args([
            "--pattern",
            "does-not-match",
            "--check",
            "tests/fixtures/single_valid.puml",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("did not match --pattern"));
}

#[test]
fn verbose_render_reports_progress_without_changing_payload() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--verbose", "--threads", "2", "--format", "txt", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice -> Bob: hi"))
        .stderr(predicate::str::contains(
            "[verbose] rendering 1 diagram source(s) as .txt with 2 thread hints",
        ))
        .stderr(predicate::str::contains(
            "[verbose] rendered 1 output artifact(s)",
        ));
}
