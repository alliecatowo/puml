use assert_cmd::Command;

// Basic smoke test: the count subcommand should print a node/edge summary line
// and exit 0 for a valid .puml file.
#[test]
fn count_basic_sequence_diagram() {
    let fixture = format!(
        "{}/docs/examples/sequence/01_basic.puml",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["count", &fixture])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8(out).expect("utf-8 stdout");
    // Output must match "N nodes, M edges"
    assert!(
        stdout.trim().contains("nodes") && stdout.trim().contains("edges"),
        "expected 'N nodes, M edges' summary, got: {stdout:?}"
    );
}

// Verify that a non-existent file returns exit code 2 (IO error).
#[test]
fn count_missing_file_exits_with_code_2() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["count", "does_not_exist_at_all.puml"])
        .assert()
        .code(2);
}
