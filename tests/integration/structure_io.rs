use super::*;

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
fn stdin_import_requires_include_root_or_fails() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin("@startuml\n!import core\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_IMPORT_ROOT_REQUIRED"));
}

#[test]
fn stdin_import_with_include_root_passes() {
    let root = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--include-root", &root, "-"])
        .write_stdin("@startuml\n!import core\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
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
fn stdin_newpage_cli_contract_modes_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(fs::read_to_string(fixture("structure/newpage_stdin_contract.puml")).unwrap())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!("stdin_newpage_cli_contract_modes", json);
}

#[test]
fn stdin_ignore_newpage_without_multi_outputs_single_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(
            "@startuml\nA -> B : one\nignore newpage\nnewpage Second\nB -> A : two\n@enduml\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("\"diagram-1.svg\"").not());
}

#[test]
fn stdin_ignore_newpage_with_multi_still_outputs_single_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(
            "@startuml\nA -> B : one\nignore newpage\nnewpage Second\nB -> A : two\n@enduml\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("\"diagram-1.svg\"").not());
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
fn file_newpage_output_writes_numbered_files_with_multi_flag() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::write(
        &input,
        "@startuml\nA -> B : one\nnewpage Second\nB -> A : two\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .success();

    assert!(tmp.path().join("paged-1.svg").exists());
    assert!(tmp.path().join("paged-2.svg").exists());
}

#[test]
fn multipage_file_output_failure_does_not_leave_partial_writes() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("paged.puml");
    fs::copy(fixture("structure/newpage_stdin_contract.puml"), &input).unwrap();
    let output = tmp.path().join("diagram.svg");
    let first = tmp.path().join("diagram-1.svg");

    fs::write(&first, "stable-original-content").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_FAIL_OUTPUT_AFTER", "1")
        .args([
            "--multi",
            input.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));

    assert_eq!(
        fs::read_to_string(&first).unwrap(),
        "stable-original-content".to_string()
    );
    assert!(!tmp.path().join("diagram-2.svg").exists());
}

#[test]
fn single_file_output_failure_does_not_overwrite_existing_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("single.puml");
    fs::copy(fixture("single_valid.puml"), &input).unwrap();
    let output = tmp.path().join("diagram.svg");
    fs::write(&output, "stable-single-content").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_FAIL_OUTPUT_AFTER", "0")
        .args(["-o", output.to_str().unwrap(), input.to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));

    assert_eq!(
        fs::read_to_string(&output).unwrap(),
        "stable-single-content".to_string()
    );
}

#[test]
fn file_ignore_newpage_output_writes_single_default_file() {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("ignore_newpage.puml");
    fs::copy(
        fixture("structure/ignore_newpage_single_output.puml"),
        &input,
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .arg(input.to_str().unwrap())
        .assert()
        .success();

    assert!(tmp.path().join("ignore_newpage.svg").exists());
    assert!(!tmp.path().join("ignore_newpage-1.svg").exists());
    assert!(!tmp.path().join("ignore_newpage-2.svg").exists());
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
fn stdin_multi_blocks_with_ignore_newpage_requires_multi() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(
            fs::read_to_string(fixture("structure/multi_blocks_ignore_newpage.puml")).unwrap(),
        )
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "multiple diagrams detected; rerun with --multi",
        ));
}

#[test]
fn stdin_multi_blocks_with_ignore_newpage_and_multi_outputs_two_items() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", "-"])
        .write_stdin(
            fs::read_to_string(fixture("structure/multi_blocks_ignore_newpage.puml")).unwrap(),
        )
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let arr = json.as_array().expect("expected array output");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["name"], "diagram-1.svg");
    assert_eq!(arr[1]["name"], "diagram-2.svg");
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
fn file_input_infers_stdlib_root_for_imports_from_parent_directory() {
    let tmp = tempdir().unwrap();
    let stdlib = tmp.path().join("stdlib");
    fs::create_dir_all(&stdlib).unwrap();
    fs::write(stdlib.join("core.puml"), "Alice -> Bob : from stdlib\n").unwrap();

    let src_path = tmp.path().join("diagram.puml");
    fs::write(&src_path, "@startuml\n!import core\n@enduml\n").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", src_path.to_str().unwrap()])
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
        .args(["--multi", input.to_str().unwrap(), "--output", "/"])
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

#[test]
fn markdown_file_auto_extracts_fenced_diagrams_without_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.md");
    fs::write(
        &input,
        "# heading\nA -x B: malformed outside fence\n\n```puml\n@startuml\nAlice -> Bob: one\n@enduml\n```\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn markdown_file_default_render_output_uses_deterministic_snippet_names() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("mixed.md");
    fs::copy(fixture("markdown/multipage_mixed.md"), &input).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(dir.path().join("mixed_snippet_1-1.svg").exists());
    assert!(dir.path().join("mixed_snippet_1-2.svg").exists());
    assert!(dir.path().join("mixed_snippet_2.svg").exists());
    assert!(!dir.path().join("mixed-1.svg").exists());
    assert!(!dir.path().join("mixed-2.svg").exists());
}

#[test]
fn markdown_multi_output_failure_does_not_leave_partial_writes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("mixed.md");
    fs::copy(fixture("markdown/multipage_mixed.md"), &input).unwrap();

    let first = dir.path().join("mixed_snippet_1-1.svg");
    fs::write(&first, "stable-original-snippet").unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_FAIL_OUTPUT_AFTER", "1")
        .args(["--multi", input.to_str().unwrap()])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to write"));

    assert_eq!(
        fs::read_to_string(&first).unwrap(),
        "stable-original-snippet".to_string()
    );
    assert!(!dir.path().join("mixed_snippet_1-2.svg").exists());
}

#[test]
fn markdown_file_diagnostics_map_to_original_markdown_lines() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("invalid.markdown");
    fs::write(
        &input,
        "# header\n\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "--diagnostics", "json", input.to_str().unwrap()])
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let first = &json["diagnostics"][0];
    assert_eq!(first["line"], 5);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "A -x B: bad");
}
