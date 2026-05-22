use assert_cmd::Command;

fn fixture_path(relative: &str) -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), relative)
}

#[test]
fn stats_human_output_for_simple_sequence() {
    let fixture = fixture_path("docs/examples/sequence/02_participants.puml");
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["stats", &fixture])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    assert!(stdout.contains("nodes:"), "human output mentions nodes");
    assert!(stdout.contains("edges:"), "human output mentions edges");
    assert!(
        stdout.contains("sequence"),
        "human output mentions family `sequence`"
    );
}

#[test]
fn stats_json_format_produces_valid_json() {
    let fixture = fixture_path("docs/examples/class/01_basic.puml");
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["stats", "--format", "json", &fixture])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should parse as JSON");
    assert_eq!(value["families"][0], "class");
    assert!(value["node_count"].as_u64().expect("node_count") >= 1);
    assert!(value["edge_count"].as_u64().expect("edge_count") >= 1);
    assert!(value["max_nesting_depth"].as_u64().is_some());
    assert!(value["node_kinds"]
        .as_object()
        .expect("node_kinds")
        .contains_key("class"));
}

#[test]
fn stats_counts_nested_packages_and_stable_kind_labels() {
    let fixture = fixture_path("docs/examples/class/14_nested_packages.puml");
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["stats", "--format", "json", &fixture])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should parse as JSON");
    assert!(value["node_count"].as_u64().expect("node_count") >= 1);
    assert!(value["edge_count"].as_u64().expect("edge_count") >= 1);
    assert!(
        value["max_nesting_depth"]
            .as_u64()
            .expect("max_nesting_depth")
            >= 1
    );
    assert!(value["node_kinds"]
        .as_object()
        .expect("node_kinds")
        .contains_key("package"));
}

#[test]
fn stats_uses_qualified_scope_depth_for_flattened_class_groups() {
    let fixture = fixture_path("docs/examples/class/32_association_class_deep_packages.puml");
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["stats", "--format", "json", &fixture])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should parse as JSON");
    assert!(
        value["max_nesting_depth"]
            .as_u64()
            .expect("max_nesting_depth")
            >= 3
    );
    assert!(value["node_kinds"]
        .as_object()
        .expect("node_kinds")
        .contains_key("member"));
}

#[test]
fn stats_missing_file_exits_with_code_2() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["stats", "does_not_exist_at_all.puml"])
        .assert()
        .code(2);
}
