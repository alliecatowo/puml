//! Integration tests for the `puml stats <file>` subcommand.

use std::process::Command;

fn puml_bin() -> &'static str {
    env!("CARGO_BIN_EXE_puml")
}

#[test]
fn stats_human_output_for_simple_sequence() {
    let out = Command::new(puml_bin())
        .args(["stats", "docs/examples/sequence/02_participants.puml"])
        .output()
        .expect("stats command should run");
    assert!(
        out.status.success(),
        "stats should exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf-8 stdout");
    assert!(stdout.contains("nodes:"), "human output mentions nodes");
    assert!(stdout.contains("edges:"), "human output mentions edges");
    assert!(
        stdout.contains("sequence"),
        "human output mentions family `sequence`"
    );
}

#[test]
fn stats_counts_nodes_and_depth_for_nested_packages() {
    let out = Command::new(puml_bin())
        .args([
            "stats",
            "docs/examples/class/14_nested_packages.puml",
            "--format",
            "json",
        ])
        .output()
        .expect("stats command should run");
    assert!(
        out.status.success(),
        "stats should exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf-8 stdout");
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should be valid JSON");
    let node_count = value
        .get("node_count")
        .and_then(|v| v.as_u64())
        .expect("node_count field");
    assert!(node_count >= 1, "should find at least one node");
    let edge_count = value
        .get("edge_count")
        .and_then(|v| v.as_u64())
        .expect("edge_count field");
    assert!(edge_count >= 1, "should find at least one edge");
    assert_eq!(
        value
            .get("families")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str()),
        Some("class"),
        "family should be class"
    );
}

#[test]
fn stats_json_format_produces_valid_json() {
    let out = Command::new(puml_bin())
        .args([
            "stats",
            "docs/examples/class/01_basic.puml",
            "--format",
            "json",
        ])
        .output()
        .expect("stats command should run");
    assert!(out.status.success(), "stats should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("utf-8 stdout");
    let value: serde_json::Value =
        serde_json::from_str(&stdout).expect("output should parse as JSON");
    assert!(value.get("node_count").is_some());
    assert!(value.get("edge_count").is_some());
    assert!(value.get("families").is_some());
    assert!(value.get("max_nesting_depth").is_some());
    assert!(value.get("node_kinds").is_some());
}
