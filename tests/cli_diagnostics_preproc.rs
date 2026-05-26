use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::fs;
use tempfile::tempdir;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn preproc_stdin_uses_include_root_defines_and_duration_without_rendering() {
    let tmp = tempdir().unwrap();
    fs::write(
        tmp.path().join("snippet.puml"),
        "Alice -> Bob : $GREETING\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--preproc",
            "--duration",
            "--include-root",
            tmp.path().to_str().unwrap(),
            "-D",
            "GREETING=hello from define",
            "-",
        ])
        .write_stdin("@startuml\n!include snippet.puml\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice -> Bob : hello from define"))
        .stdout(predicate::str::contains("!include snippet.puml").not())
        .stderr(predicate::str::contains("elapsed:"));
}

#[test]
fn preproc_markdown_multi_dumps_each_fence_with_stable_blank_separator() {
    let input = "# diagrams\n\n```puml\n@startuml\nAlice -> Bob : $MSG\n@enduml\n```\n\n```plantuml\n@startuml\nCarol -> Dave : $MSG\n@enduml\n```\n";

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--multi", "--preproc", "-DMSG=hi", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).unwrap();
    assert!(stdout.contains("Alice -> Bob : hi"));
    assert!(stdout.contains("Carol -> Dave : hi"));
    assert!(
        stdout.contains("@enduml\n\n@startuml"),
        "multi preproc output should separate diagrams with one blank line: {stdout:?}"
    );
}

#[test]
fn preproc_json_diagnostics_maps_markdown_span_to_original_document() {
    let input = "# header\n\n```puml\n@startuml\n!assert %nosuchbuiltin() : bad\n@enduml\n```\n";

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--preproc", "--diagnostics", "json", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let first = &json["diagnostics"][0];
    assert_eq!(json["schema"], "puml.diagnostics");
    assert_eq!(first["severity"], "error");
    assert_eq!(first["code"], "E_PREPROC_BUILTIN_UNSUPPORTED");
    assert_eq!(first["line"], Value::Null);
    assert_eq!(first["snippet"], Value::Null);
    assert!(first["message"].as_str().unwrap().contains("nosuchbuiltin"));
}

#[test]
fn preproc_stdrpt_diagnostic_is_single_line_and_uses_mapped_location() {
    let input = "# header\n\n```puml\n@startuml\n!assert %nosuchbuiltin() : bad\n@enduml\n```\n";

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--from-markdown", "--preproc", "--stdrpt", "-"])
        .write_stdin(input)
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out).unwrap();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "stdrpt should emit one diagnostic line");
    let parts: Vec<&str> = lines[0].split('\t').collect();
    assert_eq!(parts.len(), 4);
    assert_eq!(parts[0], "error");
    assert_eq!(parts[1], "E_PREPROC_BUILTIN_UNSUPPORTED");
    assert_eq!(parts[2], "-");
    assert!(parts[3].contains("nosuchbuiltin"));
}

#[test]
fn cli_json_diagnostic_inside_include_reports_included_origin() {
    let tmp = tempdir().unwrap();
    let child = tmp.path().join("broken.puml");
    fs::write(&child, "A -x B\n").unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--diagnostics",
            "json",
            "--include-root",
            tmp.path().to_str().unwrap(),
            "-",
        ])
        .write_stdin("@startuml\n!include broken.puml\n@enduml\n")
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let first = &json["diagnostics"][0];
    assert_eq!(first["code"], "E_ARROW_INVALID");
    assert!(first["file"].as_str().unwrap().ends_with("broken.puml"));
    assert_eq!(first["line"], 1);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "A -x B");
    assert!(first["origin"]["file"]
        .as_str()
        .unwrap()
        .ends_with("broken.puml"));
    assert!(first["include_stack"][0]
        .as_str()
        .unwrap()
        .ends_with("broken.puml"));
}

#[test]
fn cli_json_missing_include_stays_on_authored_include_statement() {
    let tmp = tempdir().unwrap();
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--diagnostics",
            "json",
            "--include-root",
            tmp.path().to_str().unwrap(),
            "-",
        ])
        .write_stdin("@startuml\n!include missing.puml\n@enduml\n")
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let first = &json["diagnostics"][0];
    assert_eq!(first["code"], "E_INCLUDE_READ");
    assert_eq!(first["file"], Value::Null);
    assert_eq!(first["line"], 2);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "!include missing.puml");
    assert!(first["origin"].is_null());
}

#[test]
fn check_fixture_stdrpt_warning_routes_through_warning_emitter() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            &fixture("styling/valid_skinparam_unsupported.puml"),
            "--stdrpt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out).unwrap();
    let line = stderr.lines().next().expect("warning line");
    let parts: Vec<&str> = line.split('\t').collect();
    assert_eq!(parts[0], "warning");
    assert_eq!(parts[1], "W_SKINPARAM_UNSUPPORTED");
    assert_eq!(parts[2], "-:2:1");
    assert!(parts[3].contains("unsupported skinparam"));
    assert_eq!(stderr.lines().count(), 1);
}

#[test]
fn render_human_warning_path_remains_multiline_with_caret_context() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            &fixture("styling/valid_skinparam_unsupported.puml"),
            "--color",
            "never",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "warning[W_SKINPARAM_UNSUPPORTED]: unsupported skinparam",
        ))
        .stderr(predicate::str::contains(
            "skinparam TotallyUnknownColor red",
        ))
        .stderr(predicate::str::contains("^^^^^^^^^"));
}
