//! Integration tests for `puml env` subcommand.
//!
//! These tests run the compiled binary via [`assert_cmd`] and assert meaningful
//! properties about its stdout/stderr/exit-code behaviour.

use assert_cmd::Command;
use predicates::prelude::*;

// ── Helper ────────────────────────────────────────────────────────────────────

fn puml() -> Command {
    Command::cargo_bin("puml").expect("puml binary must be present")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Human output (the default) must list every tracked variable name.
#[test]
fn human_output_contains_every_tracked_var_name() {
    puml()
        .args(["env"])
        .assert()
        .success()
        .stdout(predicate::str::contains("PUML_STDLIB_PATH"))
        .stdout(predicate::str::contains("NO_COLOR"))
        .stdout(predicate::str::contains("RUST_LOG"))
        .stdout(predicate::str::contains("PUML_CONFIG"))
        .stderr(predicate::str::is_empty());
}

/// Human output must end with a trailing newline (CLAUDE.md §hard-requirements).
#[test]
fn human_output_ends_with_newline() {
    let output = puml().args(["env"]).output().expect("puml env must run");
    assert!(output.status.success(), "exit code must be 0");
    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    assert!(
        stdout.ends_with('\n'),
        "human output must end with a trailing newline; got: {stdout:?}"
    );
}

/// `--format json` must produce valid JSON with the expected top-level schema.
///
/// Asserts:
/// - output is valid JSON
/// - top-level key `vars` is an object
/// - each entry has `name`, `value` (string or null), and `source` fields
#[test]
fn json_output_has_expected_schema() {
    let output = puml()
        .args(["env", "--format", "json"])
        .output()
        .expect("puml env --format json must run");
    assert!(output.status.success(), "exit code must be 0");

    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");

    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON");

    // Top-level `vars` must be an object.
    let vars = parsed
        .get("vars")
        .expect("JSON must have a top-level 'vars' key")
        .as_object()
        .expect("'vars' must be a JSON object");

    // All tracked variable names must be present.
    for name in &["PUML_STDLIB_PATH", "NO_COLOR", "RUST_LOG", "PUML_CONFIG"] {
        let entry = vars
            .get(*name)
            .unwrap_or_else(|| panic!("vars must contain '{name}'"));

        // `name` field must match the key.
        assert_eq!(
            entry.get("name").and_then(|v| v.as_str()),
            Some(*name),
            "entry.name must equal '{name}'"
        );

        // `value` field must be a string or null.
        let value_field = entry
            .get("value")
            .unwrap_or_else(|| panic!("entry for '{name}' must have a 'value' field"));
        assert!(
            value_field.is_null() || value_field.is_string(),
            "entry.value for '{name}' must be a string or null, got: {value_field:?}"
        );

        // `source` field must be a non-empty string.
        let source = entry
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("entry for '{name}' must have a string 'source' field"));
        assert!(
            !source.is_empty(),
            "entry.source for '{name}' must not be empty"
        );
    }
}

/// Setting `PUML_STDLIB_PATH` in the environment must cause that value to appear
/// in the human output and be reported with source `env`.
#[test]
fn custom_puml_stdlib_path_appears_in_human_output() {
    let expected_value = "/tmp/my-custom-stdlib";

    let output = puml()
        .args(["env"])
        .env("PUML_STDLIB_PATH", expected_value)
        .output()
        .expect("puml env must run");
    assert!(output.status.success(), "exit code must be 0");

    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    assert!(
        stdout.contains(expected_value),
        "output must contain the custom stdlib path '{expected_value}'; got:\n{stdout}"
    );
    assert!(
        stdout.contains("[env]"),
        "output must contain '[env]' source tag when PUML_STDLIB_PATH is set; got:\n{stdout}"
    );
}

/// Setting `PUML_STDLIB_PATH` in the environment must cause its value to appear
/// in JSON output with `source: "env"`.
#[test]
fn custom_puml_stdlib_path_appears_in_json_output() {
    let expected_value = "/tmp/my-json-stdlib";

    let output = puml()
        .args(["env", "--format", "json"])
        .env("PUML_STDLIB_PATH", expected_value)
        .output()
        .expect("puml env --format json must run");
    assert!(output.status.success(), "exit code must be 0");

    let stdout = String::from_utf8(output.stdout).expect("stdout must be UTF-8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON");

    let entry = parsed["vars"]["PUML_STDLIB_PATH"]
        .as_object()
        .expect("PUML_STDLIB_PATH entry must be an object");

    assert_eq!(
        entry["value"].as_str(),
        Some(expected_value),
        "PUML_STDLIB_PATH.value must equal the env-set value"
    );
    assert_eq!(
        entry["source"].as_str(),
        Some("env"),
        "PUML_STDLIB_PATH.source must be 'env' when set in environment"
    );
}

/// An unset variable must appear in output with `<unset>` in human mode and
/// `null` value in JSON mode.
#[test]
fn unset_var_appears_as_unset_in_human_and_null_in_json() {
    // Use PUML_CONFIG which is unlikely to be set in CI environments.
    // Explicitly remove it to be sure.
    let human_output = puml()
        .args(["env"])
        .env_remove("PUML_CONFIG")
        .output()
        .expect("puml env must run");
    assert!(human_output.status.success());
    let human_stdout = String::from_utf8(human_output.stdout).expect("utf-8");
    assert!(
        human_stdout.contains("<unset>"),
        "human output must contain '<unset>' for unset variables; got:\n{human_stdout}"
    );

    let json_output = puml()
        .args(["env", "--format", "json"])
        .env_remove("PUML_CONFIG")
        .output()
        .expect("puml env --format json must run");
    assert!(json_output.status.success());
    let json_stdout = String::from_utf8(json_output.stdout).expect("utf-8");
    let parsed: serde_json::Value = serde_json::from_str(&json_stdout).expect("valid JSON");
    assert!(
        parsed["vars"]["PUML_CONFIG"]["value"].is_null(),
        "JSON value for unset PUML_CONFIG must be null; got: {:?}",
        parsed["vars"]["PUML_CONFIG"]["value"]
    );
}
