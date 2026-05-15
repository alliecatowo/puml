use std::process::Command;
use std::{fs, path::PathBuf};

use serde_json::Value;

#[test]
fn svg_bounds_audit_regression_corpus_passes() {
    let output = Command::new("python3")
        .args(["scripts/svg_bounds_audit.py", "--quiet"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run scripts/svg_bounds_audit.py");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "svg bounds audit failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout,
            stderr
        );
    }
}

#[test]
fn parity_harness_report_schema_is_stable() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("parity_harness_test_report.json");

    let output = Command::new("python3")
        .args([
            "scripts/parity_harness.py",
            "--quiet",
            "--output",
            path.to_str().expect("utf-8 path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run scripts/parity_harness.py");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "parity harness failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout,
            stderr
        );
    }

    let raw = fs::read_to_string(&path).expect("report should be written");
    let json: Value = serde_json::from_str(&raw).expect("report must be valid JSON");
    assert_eq!(json["schema_version"], "1.0.0");
    assert!(json.get("fixtures").and_then(Value::as_array).is_some());
    assert!(json.get("summary").is_some());
    assert!(json.get("oracle").is_some());
    assert!(json.get("doc_examples").is_some());
    assert!(
        json["doc_examples"]["entries"]
            .as_array()
            .map(|rows| !rows.is_empty())
            .unwrap_or(false),
        "doc_examples.entries should be non-empty"
    );
}
