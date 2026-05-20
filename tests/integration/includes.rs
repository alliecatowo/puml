use super::*;

#[test]
fn include_cycle_input_reports_cycle_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/error_include_cycle_self.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("include cycle detected"));
}

#[test]
fn include_cycle_chain_reports_cycle_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/error_include_chain_a.puml")])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("include cycle detected"));
}

#[test]
fn include_id_tag_extracts_local_block() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/include_with_tag_ok.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn include_many_expands_each_occurrence() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/include_many_ok.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let msg_count = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stmt| stmt["kind"]["Message"].is_object())
        .count();
    assert_eq!(msg_count, 2);
}

#[test]
fn include_once_expands_only_first_occurrence() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", &fixture("include/include_once_ok.puml")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let msg_count = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|stmt| stmt["kind"]["Message"].is_object())
        .count();
    assert_eq!(msg_count, 1);
}

#[test]
fn includesub_extracts_local_block() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("include/includesub_ok.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn include_id_missing_tag_reports_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_include_tag_missing.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_TAG_NOT_FOUND"))
        .stderr(predicate::str::contains(
            "include tag 'MISSING_TAG' was not found",
        ));
}

#[test]
fn include_url_is_rejected_with_deterministic_error_when_flag_set() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            "--no-url-includes",
            &fixture("errors/invalid_include_url.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_URL_DISABLED"));
}

#[test]
fn includesub_without_tag_is_rejected_with_deterministic_error() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_includesub_missing_tag.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDESUB_TAG_REQUIRED"))
        .stderr(predicate::str::contains(
            "!includesub requires a target tag",
        ));
}

#[test]
fn include_variants_url_policy_is_rejected_deterministically_when_flag_set() {
    for (case, _directive) in [
        ("errors/invalid_include_url.puml", "!include"),
        ("errors/invalid_include_once_url.puml", "!include_once"),
        ("errors/invalid_include_many_url.puml", "!include_many"),
        ("errors/invalid_includesub_url.puml", "!includesub"),
        ("errors/invalid_includeurl_url.puml", "!includeurl"),
    ] {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", "--no-url-includes", &fixture(case)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains("E_INCLUDE_URL_DISABLED"));
    }
}
