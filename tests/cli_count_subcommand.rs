use std::fs;

use assert_cmd::Command;

fn fixture_path(relative: &str) -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), relative)
}

#[test]
fn count_basic_sequence_diagram() {
    let fixture = fixture_path("docs/examples/sequence/01_basic.puml");
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["count", &fixture])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    assert!(
        stdout.trim().contains("nodes") && stdout.trim().contains("edges"),
        "expected 'N nodes, M edges' summary, got: {stdout:?}"
    );
}

#[test]
fn count_by_kind_sequence_uses_lowercase_labels_and_counts_actors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("sequence_with_actor.puml");
    fs::write(
        &fixture,
        "@startuml\nactor User\nparticipant Service\nUser -> Service: hi\n@enduml\n",
    )
    .expect("write fixture");

    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["count", "--by-kind", fixture.to_str().expect("utf-8 path")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    assert_eq!(stdout, "2 nodes, 1 edges\n  actor: 1\n  participant: 1\n");
}

#[test]
fn count_class_family_diagram_by_kind() {
    let fixture = fixture_path("docs/examples/class/01_basic.puml");
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["count", "--by-kind", &fixture])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    assert!(stdout.contains("nodes") && stdout.contains("edges"));
    assert!(
        stdout.contains("  class: 2"),
        "expected lowercase class kind, got: {stdout:?}"
    );
}

#[test]
fn count_family_pages_sums_all_pages() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = dir.path().join("pages.puml");
    fs::write(
        &fixture,
        "@startuml\nclass A\nnewpage\nclass B\nA --> B\n@enduml\n",
    )
    .expect("write fixture");

    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["count", "--by-kind", fixture.to_str().expect("utf-8 path")])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    assert_eq!(stdout, "2 nodes, 1 edges\n  class: 2\n");
}

#[test]
fn count_state_diagram_path() {
    let fixture = fixture_path("docs/examples/state/01_basic.puml");
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["count", "--by-kind", &fixture])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    assert!(stdout.contains("nodes") && stdout.contains("edges"));
    assert!(
        stdout.contains("  state:"),
        "expected lowercase state kind, got: {stdout:?}"
    );
}

#[test]
fn count_missing_file_exits_with_code_2() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["count", "does_not_exist_at_all.puml"])
        .assert()
        .code(2);
}
