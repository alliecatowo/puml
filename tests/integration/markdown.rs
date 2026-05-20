use super::*;

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
