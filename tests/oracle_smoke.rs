// oracle_smoke.rs — Deterministic sentinel test + env-gated JAR integration test.

use std::process::Command;

fn oracle_script() -> String {
    format!("{}/scripts/oracle.sh", env!("CARGO_MANIFEST_DIR"))
}

/// Smoke test: when PUML_ORACLE_JAR is not set the script exits 0 and emits
/// a JSON object with `"skipped": true` and `"reason"` matching the sentinel.
/// This test is always deterministic regardless of environment.
#[test]
fn oracle_skip_sentinel() {
    let output = Command::new("bash")
        .arg(oracle_script())
        .env_remove("PUML_ORACLE_JAR")
        .output()
        .expect("failed to invoke oracle.sh");

    assert!(
        output.status.success(),
        "oracle.sh should exit 0 when PUML_ORACLE_JAR is unset; status: {}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = stdout.trim();

    // Must be valid JSON.
    let v: serde_json::Value =
        serde_json::from_str(stdout).expect("oracle.sh output must be valid JSON");

    assert_eq!(
        v["skipped"].as_bool(),
        Some(true),
        "expected skipped=true, got: {stdout}"
    );
    let reason = v["reason"].as_str().unwrap_or("");
    assert!(
        reason.contains("PUML_ORACLE_JAR"),
        "reason should mention PUML_ORACLE_JAR, got: {reason}"
    );
}

/// Integration test: when PUML_ORACLE_JAR is set, invoke the oracle against
/// the fixtures corpus and assert the report has the expected shape.
/// Gated with `#[ignore]` — run with `cargo test -- --ignored oracle_with_jar`.
#[test]
#[ignore]
fn oracle_with_jar() {
    let jar = match std::env::var("PUML_ORACLE_JAR") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("PUML_ORACLE_JAR not set; skipping oracle_with_jar");
            return;
        }
    };

    let fixtures_dir = format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"));
    let report_file = format!(
        "{}/docs/benchmarks/oracle_report.json",
        env!("CARGO_MANIFEST_DIR")
    );

    let output = Command::new("bash")
        .arg(oracle_script())
        .arg("--corpus-dir")
        .arg(&fixtures_dir)
        .env("PUML_ORACLE_JAR", &jar)
        .output()
        .expect("failed to invoke oracle.sh with JAR");

    // Must exit 0 or 2 (1 = render mismatch → structural issue, not shape issue).
    let code = output.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 2,
        "oracle.sh exited {code}; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("oracle.sh output must be valid JSON");

    // Validate report shape.
    assert_eq!(
        v["skipped"].as_bool(),
        Some(false),
        "report must not be skipped when JAR is set"
    );
    assert!(v["total"].is_number(), "report must have numeric 'total'");
    assert!(
        v["identical"].is_number(),
        "report must have numeric 'identical'"
    );
    assert!(
        v["diff_count"].is_number(),
        "report must have numeric 'diff_count'"
    );
    assert!(
        v["render_mismatch_count"].is_number(),
        "report must have numeric 'render_mismatch_count'"
    );
    assert!(
        v["structural_drift_count"].is_number(),
        "report must have numeric 'structural_drift_count'"
    );
    assert!(v["diffs"].is_array(), "report must have 'diffs' array");

    // The written report must also be valid JSON and match stdout.
    if std::path::Path::new(&report_file).exists() {
        let written = std::fs::read_to_string(&report_file).expect("report file readable");
        let vr: serde_json::Value =
            serde_json::from_str(written.trim()).expect("report file must be valid JSON");
        assert_eq!(
            v["total"], vr["total"],
            "stdout and report file must agree on 'total'"
        );
    }
}
