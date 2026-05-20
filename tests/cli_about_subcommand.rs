use assert_cmd::Command;

#[test]
fn about_human_exits_zero() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["about"])
        .assert()
        .success();
}

#[test]
fn about_json_is_parseable() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["about", "--format", "json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let _: serde_json::Value =
        serde_json::from_slice(&out).expect("about --format json should produce valid JSON");
}
