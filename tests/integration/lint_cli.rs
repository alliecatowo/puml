use super::*;

#[test]
fn lint_mode_requires_check_flag() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--lint-input", &fixture("single_valid.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("--check"));
}

#[test]
fn lint_mode_json_report_supports_repeated_inputs_and_globs_with_stable_order() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("a_invalid.puml"),
    )
    .unwrap();
    fs::copy(
        fixture("single_valid.puml"),
        tmp.path().join("b_valid.puml"),
    )
    .unwrap();
    fs::copy(
        fixture("styling/valid_skinparam_unsupported.puml"),
        tmp.path().join("c_warning.puml"),
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-input",
            "b_valid.puml",
            "--lint-input",
            "a_invalid.puml",
            "--lint-glob",
            "*.puml",
            "--lint-report",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema"], "puml.lint_report");
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["summary"]["total_files"], 3);
    assert_eq!(report["summary"]["passed_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["total_diagrams"], 3);
    assert_eq!(report["summary"]["passed_diagrams"], 2);
    assert_eq!(report["summary"]["failed_diagrams"], 1);
    assert_eq!(report["summary"]["warning_count"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let files = report["files"].as_array().expect("files array");
    assert_eq!(files.len(), 3);
    assert_eq!(files[0]["path"], "a_invalid.puml");
    assert_eq!(files[1]["path"], "b_valid.puml");
    assert_eq!(files[2]["path"], "c_warning.puml");

    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--> a_invalid.puml"));
}

#[test]
fn lint_mode_json_diagnostics_stay_on_stderr_and_report_stays_on_stdout() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("invalid_single.puml"),
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-input",
            "invalid_single.puml",
            "--diagnostics",
            "json",
            "--lint-report",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["schema"], "puml.lint_report");
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    assert_eq!(diagnostics["diagnostics"][0]["severity"], "error");
}

#[test]
fn lint_mode_human_report_succeeds_for_all_valid_inputs() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("single_valid.puml"),
        tmp.path().join("a_valid.puml"),
    )
    .unwrap();
    fs::copy(fixture("basic/hello.puml"), tmp.path().join("b_valid.puml")).unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args(["--check", "--lint-glob", "*.puml"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "lint summary: files=2 passed=2 failed=0",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn lint_mode_markdown_docs_glob_runs_end_to_end() {
    let tmp = tempdir().unwrap();
    fs::write(
        tmp.path().join("ok.md"),
        "# ok\n```puml\n@startuml\nAlice -> Bob: hello\n@enduml\n```\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("broken.md"),
        "# broken\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-glob",
            "*.md",
            "--lint-report",
            "json",
            "--diagnostics",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["summary"]["total_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["total_diagrams"], 2);
    assert_eq!(report["summary"]["failed_diagrams"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    assert_eq!(diagnostics["diagnostics"][0]["line"], 4);
    assert_eq!(diagnostics["diagnostics"][0]["file"], "broken.md");
}

#[test]
fn lint_mode_json_diagnostics_aggregate_deterministically_across_files() {
    let tmp = tempdir().unwrap();
    fs::copy(
        fixture("invalid_single.puml"),
        tmp.path().join("a_invalid.puml"),
    )
    .unwrap();
    fs::write(
        tmp.path().join("b_warning.puml"),
        "@startuml\nskinparam SequenceFooColor #123456\nAlice -> Bob: ok\n@enduml\n",
    )
    .unwrap();

    let out = Command::cargo_bin("puml")
        .expect("binary")
        .current_dir(tmp.path())
        .args([
            "--check",
            "--lint-glob",
            "*.puml",
            "--lint-report",
            "json",
            "--diagnostics",
            "json",
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();

    let report: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["summary"]["total_files"], 2);
    assert_eq!(report["summary"]["failed_files"], 1);
    assert_eq!(report["summary"]["warning_count"], 1);
    assert_eq!(report["summary"]["error_count"], 1);

    let diagnostics: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(diagnostics["schema"], "puml.diagnostics");
    let entries = diagnostics["diagnostics"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["file"], "a_invalid.puml");
    assert_eq!(entries[0]["severity"], "error");
    assert_eq!(entries[1]["file"], "b_warning.puml");
    assert_eq!(entries[1]["severity"], "warning");
}

#[test]
fn clap_help_exits_successfully() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Rust-native PlantUML-compatible diagram renderer",
        ))
        .stdout(predicate::str::contains(
            "Permit multiple stdin render outputs",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn clap_version_exits_successfully() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("puml"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn exit_code_matrix_is_stable_for_success_validation_and_io() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--help")
        .assert()
        .code(0);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--definitely-invalid-flag")
        .assert()
        .code(1);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("/tmp/definitely-not-present-input-12.puml")
        .assert()
        .code(2);
}

#[test]
fn dump_capabilities_outputs_manifest_shape() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .arg("--dump-capabilities")
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .get_output()
        .stdout
        .clone();

    // Output is now the real LSP protocol-level capabilities object (same as
    // what the server returns in its initialize response).
    let json: Value = serde_json::from_slice(&out).unwrap();
    assert!(json["completionProvider"]["resolveProvider"]
        .as_bool()
        .unwrap_or(false));
    assert!(json["hoverProvider"].as_bool().unwrap_or(false));
    assert!(json["definitionProvider"].as_bool().unwrap_or(false));
    assert!(json["referencesProvider"].as_bool().unwrap_or(false));
    assert!(json["documentFormattingProvider"]
        .as_bool()
        .unwrap_or(false));
    assert!(json["documentRangeFormattingProvider"]
        .as_bool()
        .unwrap_or(false));
    assert!(json["codeActionProvider"].as_bool().unwrap_or(false));
    assert!(json["colorProvider"].as_bool().unwrap_or(false));
    assert!(json["foldingRangeProvider"].as_bool().unwrap_or(false));
    assert!(json["selectionRangeProvider"].as_bool().unwrap_or(false));
    assert!(json["documentSymbolProvider"].as_bool().unwrap_or(false));
    assert!(json["workspaceSymbolProvider"].as_bool().unwrap_or(false));
    let commands = json["executeCommandProvider"]["commands"]
        .as_array()
        .expect("executeCommandProvider.commands must be an array");
    assert!(commands.iter().any(|c| c == "puml.applyFormat"));
    assert!(commands.iter().any(|c| c == "puml.renderSvg"));
    let token_types = json["semanticTokensProvider"]["legend"]["tokenTypes"]
        .as_array()
        .expect("semanticTokensProvider.legend.tokenTypes must be an array");
    assert!(token_types.iter().any(|t| t == "keyword"));
    assert!(json["semanticTokensProvider"]["full"]
        .as_bool()
        .unwrap_or(false));
    assert!(json["workspace"]["workspaceFolders"]["supported"]
        .as_bool()
        .unwrap_or(false));
}

#[test]
fn check_fixture_uses_fixture_loader_and_succeeds() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check-fixture", &fixture("single_valid.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn check_fixture_missing_file_maps_to_io_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            "/tmp/definitely-not-present-fixture-16.puml",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("failed to read fixture"));
}

#[test]
fn check_fixture_with_json_diagnostics_emits_warning_payload() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check-fixture",
            &fixture("styling/valid_skinparam_unsupported.puml"),
            "--diagnostics",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .get_output()
        .stderr
        .clone();

    let line = String::from_utf8(out).unwrap();
    let json: Value = serde_json::from_str(line.trim()).expect("valid json warning payload");
    assert_json_snapshot!("diagnostics_json_warning_contract_shape", json);
    let first = &json["diagnostics"][0];
    assert_eq!(json["schema"], "puml.diagnostics");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(first["code"], "W_SKINPARAM_UNSUPPORTED");
    assert_eq!(first["severity"], "warning");
    assert_eq!(first["line"], 2);
    assert_eq!(first["column"], 1);
    assert_eq!(first["snippet"], "skinparam TotallyUnknownColor red");
    assert!(first["message"]
        .as_str()
        .unwrap()
        .contains("W_SKINPARAM_UNSUPPORTED"));
}

#[test]
fn diagnostics_json_writes_only_to_stderr_and_not_stdout() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--diagnostics",
            "json",
            &fixture("invalid_single.puml"),
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::starts_with(
            "{\n  \"schema\": \"puml.diagnostics\"",
        ));
}

#[test]
fn stdin_empty_input_maps_to_validation_exit_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("no diagram content provided"));
}

#[test]
fn markdown_mdown_extension_auto_extracts_fenced_diagrams_without_flag() {
    let dir = tempfile::tempdir().expect("tempdir");
    let input = dir.path().join("input.mdown");
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
fn mermaid_loops_and_groups_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_loops_and_groups.mmd.txt"),
        ])
        .assert()
        .success();
}

// -- New diagram families: JSON / YAML / nwdiag / Archimate --------------------

// ---------------------------------------------------------------------------
// Creole inline formatting tests (#168)
// ---------------------------------------------------------------------------

// ─── State diagram advanced feature tests ────────────────────────────────────
