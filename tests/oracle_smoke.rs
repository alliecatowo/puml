// oracle_smoke.rs — Deterministic sentinel tests + env-gated JAR integration test.

use std::path::PathBuf;
use std::process::Command;

fn oracle_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/oracle.sh")
}

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

// ---------------------------------------------------------------------------
// oracle_skip_sentinel
// ---------------------------------------------------------------------------

/// When `PUML_ORACLE_JAR` is not set the script must exit 0 and emit a JSON
/// object with `"skipped": true` and a `"reason"` that mentions `PUML_ORACLE_JAR`.
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

// ---------------------------------------------------------------------------
// oracle_report_schema_is_stable
// ---------------------------------------------------------------------------

/// When `PUML_ORACLE_JAR` is set and valid, the report JSON must contain all
/// required top-level fields matching schema_version "1.0".
///
/// Gated with `#[ignore]` — run with:
///   PUML_ORACLE_JAR=/path/to/plantuml.jar cargo test -- --ignored oracle_report_schema_is_stable
#[test]
#[ignore]
fn oracle_report_schema_is_stable() {
    let jar = match std::env::var("PUML_ORACLE_JAR") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("PUML_ORACLE_JAR not set; skipping oracle_report_schema_is_stable");
            return;
        }
    };

    let fixtures_dir = repo_path("tests/fixtures");
    let report_file = repo_path("docs/benchmarks/oracle_report.json");

    let output = Command::new("bash")
        .arg(oracle_script())
        .arg("--corpus-dir")
        .arg(&fixtures_dir)
        .env("PUML_ORACLE_JAR", &jar)
        .output()
        .expect("failed to invoke oracle.sh with JAR");

    // Exit 0 (≥80% match) or 1 (50-79%) are acceptable; exit 2 (<50%) is a
    // hard failure but we still validate the JSON shape regardless.
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("oracle.sh output must be valid JSON; err={e}; stdout={stdout}")
    });

    // schema_version must be "1.0"
    assert_eq!(
        v["schema_version"].as_str(),
        Some("1.0"),
        "report must have schema_version=\"1.0\""
    );

    // timestamp must be present and look like an ISO-8601 UTC string
    let timestamp = v["timestamp"].as_str().unwrap_or("");
    assert!(
        timestamp.contains('T') && timestamp.ends_with('Z'),
        "timestamp should be ISO-8601 UTC, got: {timestamp}"
    );

    // jar_version must be present
    assert!(
        v["jar_version"].is_string(),
        "report must have string 'jar_version'"
    );

    // summary must contain all expected fields
    let summary = &v["summary"];
    for field in &[
        "total",
        "match",
        "drift",
        "puml_only",
        "jar_only",
        "both_fail",
    ] {
        assert!(
            summary[field].is_number(),
            "summary must have numeric '{field}'"
        );
    }

    // fixtures must be an array
    assert!(
        v["fixtures"].is_array(),
        "report must have 'fixtures' array"
    );

    // Each fixture entry must have path, category, and metrics
    if let Some(entries) = v["fixtures"].as_array() {
        for entry in entries {
            let path = entry["path"].as_str().unwrap_or("");
            assert!(!path.is_empty(), "fixture entry must have non-empty 'path'");

            let category = entry["category"].as_str().unwrap_or("");
            assert!(
                matches!(
                    category,
                    "match" | "drift" | "puml-only" | "jar-only" | "both-fail"
                ),
                "fixture category must be one of the five valid values, got: {category} (path={path})"
            );

            assert!(
                entry["metrics"].is_object(),
                "fixture entry must have 'metrics' object (path={path})"
            );
        }
    }

    // The written report file must also be valid JSON and agree on total.
    if report_file.exists() {
        let written =
            std::fs::read_to_string(&report_file).expect("oracle report file must be readable");
        let vr: serde_json::Value =
            serde_json::from_str(written.trim()).expect("oracle report file must be valid JSON");
        assert_eq!(
            v["summary"]["total"], vr["summary"]["total"],
            "stdout and report file must agree on summary.total"
        );
    }

    // Soft-assert: exit code must be 0, 1, or 2
    assert!(
        matches!(code, 0 | 1 | 2),
        "oracle.sh must exit 0, 1, or 2 (got {code}); stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// oracle_drift_threshold_documented
// ---------------------------------------------------------------------------

/// Verify that `docs/oracle-thresholds.md` exists and documents both the 80%
/// and 50% exit-code thresholds so the semantics are always discoverable.
#[test]
fn oracle_drift_threshold_documented() {
    let doc = repo_path("docs/oracle-thresholds.md");

    assert!(
        doc.exists(),
        "docs/oracle-thresholds.md must exist (see issue #212)"
    );

    let contents =
        std::fs::read_to_string(&doc).expect("docs/oracle-thresholds.md must be readable");

    assert!(
        contents.contains("80"),
        "docs/oracle-thresholds.md must document the 80% match threshold"
    );

    assert!(
        contents.contains("50"),
        "docs/oracle-thresholds.md must document the 50% match threshold"
    );

    // Verify the exit codes are documented
    assert!(
        contents.contains("exit"),
        "docs/oracle-thresholds.md must document exit codes"
    );

    // Verify the script actually encodes these thresholds too
    let script = repo_path("scripts/oracle.sh");
    assert!(script.exists(), "scripts/oracle.sh must exist");

    let script_contents =
        std::fs::read_to_string(&script).expect("scripts/oracle.sh must be readable");

    assert!(
        script_contents.contains("80"),
        "scripts/oracle.sh must encode the 80% threshold"
    );

    assert!(
        script_contents.contains("50"),
        "scripts/oracle.sh must encode the 50% threshold"
    );
}
