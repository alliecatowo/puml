use assert_cmd::Command;
use insta::assert_json_snapshot;
use predicates::prelude::*;
use serde_json::Value;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn virtual_endpoint_fidelity_fixture_passes_check_mode() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("arrows/virtual_endpoint_fidelity.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn invalid_virtual_endpoint_combination_reports_diagnostic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_virtual_endpoint_combination.puml"),
        ])
        .assert()
        .code(1)
        .stderr(
            predicate::str::contains("line 2, column 1")
                .and(predicate::str::contains(
                    "virtual endpoint messages must include at least one concrete participant",
                ))
                .and(predicate::str::contains("E_ENDPOINT_COMBINATION")),
        );
}

#[test]
fn dump_mode_model_preserves_virtual_endpoint_found_lost_side_shape_semantics() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "model",
            &fixture("arrows/virtual_endpoint_fidelity.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "dump_mode_model_preserves_virtual_endpoint_found_lost_side_shape_semantics",
        json
    );
}

#[test]
fn dump_mode_scene_preserves_virtual_endpoint_found_lost_side_shape_semantics() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "scene",
            &fixture("arrows/virtual_endpoint_fidelity.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    assert_json_snapshot!(
        "dump_mode_scene_preserves_virtual_endpoint_found_lost_side_shape_semantics",
        json
    );
}
