use assert_cmd::Command;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn write_json(path: &Path, value: serde_json::Value) {
    fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
}

#[test]
fn failures_report_binary_absolute_and_regression_for_matching_mode() {
    let temp = TempDir::new().unwrap();
    let current = temp.path().join("current.json");
    let previous = temp.path().join("previous.json");

    write_json(
        &current,
        json!({
            "timestamp_utc": "2026-05-15T00:00:00Z",
            "mode": "full",
            "binary": "/tmp/puml",
            "scenarios": [
                {"name": "render_stdin", "mean_ms": 130.0, "stddev_ms": 1.0, "runs": 5}
            ]
        }),
    );
    write_json(
        &previous,
        json!({
            "timestamp_utc": "2026-05-14T00:00:00Z",
            "mode": "full",
            "binary": "/tmp/puml",
            "scenarios": [
                {"name": "render_stdin", "mean_ms": 100.0, "stddev_ms": 1.0, "runs": 5}
            ]
        }),
    );

    let output = Command::new("python3")
        .arg(repo_path("scripts/bench_gate.py"))
        .arg("failures")
        .arg("--current")
        .arg(&current)
        .arg("--previous")
        .arg(&previous)
        .arg("--mode")
        .arg("full")
        .arg("--abs-limit")
        .arg("120")
        .arg("--regression-limit-pct")
        .arg("10")
        .arg("--regression-min-delta-ms")
        .arg("20")
        .arg("--binary-bytes")
        .arg("210")
        .arg("--binary-limit-bytes")
        .arg("200")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out = String::from_utf8(output).unwrap();
    assert!(
        out.contains("binary size 210B exceeds 200B"),
        "expected binary limit failure, got: {out}"
    );
    assert!(
        out.contains("render_stdin: mean 130.000ms exceeds absolute limit 120.000ms"),
        "expected absolute mean failure, got: {out}"
    );
    assert!(
        out.contains("render_stdin: regression 30.000% exceeds limit 10.000%"),
        "expected regression failure, got: {out}"
    );
}

#[test]
fn failures_skip_regression_when_baseline_mode_mismatches() {
    let temp = TempDir::new().unwrap();
    let current = temp.path().join("current.json");
    let previous = temp.path().join("previous.json");

    write_json(
        &current,
        json!({
            "timestamp_utc": "2026-05-15T00:00:00Z",
            "mode": "full",
            "binary": "/tmp/puml",
            "scenarios": [
                {"name": "render_stdin", "mean_ms": 130.0, "stddev_ms": 1.0, "runs": 5}
            ]
        }),
    );
    write_json(
        &previous,
        json!({
            "timestamp_utc": "2026-05-14T00:00:00Z",
            "mode": "quick",
            "binary": "/tmp/puml",
            "scenarios": [
                {"name": "render_stdin", "mean_ms": 80.0, "stddev_ms": 1.0, "runs": 5}
            ]
        }),
    );

    let output = Command::new("python3")
        .arg(repo_path("scripts/bench_gate.py"))
        .arg("failures")
        .arg("--current")
        .arg(&current)
        .arg("--previous")
        .arg(&previous)
        .arg("--mode")
        .arg("full")
        .arg("--abs-limit")
        .arg("200")
        .arg("--regression-limit-pct")
        .arg("10")
        .arg("--regression-min-delta-ms")
        .arg("20")
        .arg("--binary-bytes")
        .arg("100")
        .arg("--binary-limit-bytes")
        .arg("200")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out = String::from_utf8(output).unwrap();
    assert!(
        out.trim().is_empty(),
        "expected no failures when only mismatch regression would fail, got: {out}"
    );
}

#[test]
fn trend_marks_baseline_unavailable_for_mode_mismatch() {
    let temp = TempDir::new().unwrap();
    let current = temp.path().join("current.json");
    let previous = temp.path().join("previous.json");
    let trend_json = temp.path().join("trend.json");
    let trend_md = temp.path().join("trend.md");

    write_json(
        &current,
        json!({
            "timestamp_utc": "2026-05-15T00:00:00Z",
            "mode": "full",
            "binary": "/tmp/puml",
            "scenarios": [
                {"name": "render_stdin", "mean_ms": 130.0, "stddev_ms": 1.0, "runs": 5}
            ]
        }),
    );
    write_json(
        &previous,
        json!({
            "timestamp_utc": "2026-05-14T00:00:00Z",
            "mode": "quick",
            "binary": "/tmp/puml",
            "scenarios": [
                {"name": "render_stdin", "mean_ms": 80.0, "stddev_ms": 1.0, "runs": 5}
            ]
        }),
    );

    Command::new("python3")
        .arg(repo_path("scripts/bench_gate.py"))
        .arg("trend")
        .arg("--current")
        .arg(&current)
        .arg("--previous")
        .arg(&previous)
        .arg("--output-json")
        .arg(&trend_json)
        .arg("--output-md")
        .arg(&trend_md)
        .arg("--timestamp-utc")
        .arg("2026-05-15T00:10:00Z")
        .arg("--mode")
        .arg("full")
        .arg("--abs-limit")
        .arg("250")
        .arg("--regression-limit-pct")
        .arg("10")
        .arg("--regression-min-delta-ms")
        .arg("20")
        .arg("--binary-bytes")
        .arg("100")
        .arg("--binary-limit-bytes")
        .arg("200")
        .arg("--host")
        .arg("local")
        .arg("--os-name")
        .arg("Linux")
        .arg("--kernel")
        .arg("test")
        .arg("--arch")
        .arg("x86_64")
        .arg("--rustc")
        .arg("rustc test")
        .arg("--timing-tool")
        .arg("python-perf-counter")
        .assert()
        .success();

    let trend: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(trend_json).unwrap()).unwrap();
    assert_eq!(trend["baseline"]["available"], json!(false));
    assert_eq!(trend["baseline"]["mode_match"], json!(false));
    assert_eq!(
        trend["scenarios"][0]["previous_mean_ms"],
        serde_json::Value::Null
    );
    assert_eq!(trend["scenarios"][0]["delta_ms"], serde_json::Value::Null);
    assert_eq!(trend["scenarios"][0]["delta_pct"], serde_json::Value::Null);
}

#[test]
fn trend_embeds_benchmark_policy_metadata() {
    let temp = TempDir::new().unwrap();
    let current = temp.path().join("current.json");
    let previous = temp.path().join("previous.json");
    let trend_json = temp.path().join("trend.json");
    let trend_md = temp.path().join("trend.md");

    write_json(
        &current,
        json!({
            "timestamp_utc": "2026-05-17T00:00:00Z",
            "mode": "full",
            "binary": "/tmp/puml",
            "benchmark_policy": {
                "version": "bench-gate-v2-2026-05-17",
                "mode": "full",
                "absolute_mean_ms_limit": 250.0,
                "regression_pct_limit": 10.0,
                "regression_min_delta_ms": 40.0,
                "binary_limit_bytes": 16000000
            },
            "scenarios": [
                {"name": "render_stdin", "mean_ms": 90.0, "stddev_ms": 1.0, "runs": 5}
            ]
        }),
    );
    write_json(
        &previous,
        json!({
            "timestamp_utc": "2026-05-16T00:00:00Z",
            "mode": "full",
            "scenarios": [
                {"name": "render_stdin", "mean_ms": 90.0, "stddev_ms": 1.0, "runs": 5}
            ]
        }),
    );

    Command::new("python3")
        .arg(repo_path("scripts/bench_gate.py"))
        .arg("trend")
        .arg("--current")
        .arg(&current)
        .arg("--previous")
        .arg(&previous)
        .arg("--output-json")
        .arg(&trend_json)
        .arg("--output-md")
        .arg(&trend_md)
        .arg("--timestamp-utc")
        .arg("2026-05-17T00:10:00Z")
        .arg("--mode")
        .arg("full")
        .arg("--abs-limit")
        .arg("250")
        .arg("--regression-limit-pct")
        .arg("10")
        .arg("--regression-min-delta-ms")
        .arg("40")
        .arg("--binary-bytes")
        .arg("10035664")
        .arg("--binary-limit-bytes")
        .arg("16000000")
        .arg("--host")
        .arg("local")
        .arg("--os-name")
        .arg("Linux")
        .arg("--kernel")
        .arg("test")
        .arg("--arch")
        .arg("x86_64")
        .arg("--rustc")
        .arg("rustc test")
        .arg("--timing-tool")
        .arg("python-perf-counter")
        .assert()
        .success();

    let trend: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(trend_json).unwrap()).unwrap();
    assert_eq!(
        trend["benchmark_policy"]["version"],
        json!("bench-gate-v2-2026-05-17")
    );
    assert_eq!(
        trend["benchmark_policy"]["binary_limit_bytes"],
        json!(16000000)
    );
    assert_eq!(
        trend["benchmark_policy"]["regression_min_delta_ms"],
        json!(40.0)
    );
}

#[test]
fn validate_artifacts_rejects_stale_policy_metadata() {
    let temp = TempDir::new().unwrap();
    let stale = temp.path().join("stale.json");
    let current = temp.path().join("current.json");

    write_json(
        &stale,
        json!({
            "timestamp_utc": "2026-05-15T20:17:06Z",
            "mode": "full",
            "benchmark_policy": {
                "version": "bench-gate-v1",
                "mode": "full",
                "absolute_mean_ms_limit": 250.0,
                "regression_pct_limit": 10.0,
                "regression_min_delta_ms": 20.0,
                "binary_limit_bytes": 2000000
            },
            "gates": {
                "absolute_mean_ms_limit": 250.0,
                "regression_pct_limit": 10.0,
                "regression_min_delta_ms": 20.0
            },
            "binary": {
                "limit_bytes": 2000000
            },
            "scenarios": []
        }),
    );
    write_json(
        &current,
        json!({
            "timestamp_utc": "2026-05-17T20:17:06Z",
            "mode": "quick",
            "benchmark_policy": {
                "version": "bench-gate-v2-2026-05-17",
                "mode": "quick",
                "absolute_mean_ms_limit": 350.0,
                "regression_pct_limit": 20.0,
                "regression_min_delta_ms": 50.0,
                "binary_limit_bytes": 16000000
            },
            "scenarios": []
        }),
    );

    let output = Command::new("python3")
        .arg(repo_path("scripts/bench_gate.py"))
        .arg("validate-artifacts")
        .arg(&stale)
        .arg(&current)
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();

    let out = String::from_utf8(output).unwrap();
    assert!(
        out.contains("benchmark_policy.version is 'bench-gate-v1'"),
        "expected stale policy version failure, got: {out}"
    );
    assert!(
        out.contains("binary.limit_bytes is 2000000"),
        "expected stale binary limit failure, got: {out}"
    );
}
