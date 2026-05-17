// oracle_smoke.rs — Deterministic sentinel tests + env-gated JAR integration test.

use std::path::PathBuf;
use std::process::Command;

fn oracle_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/oracle.sh")
}

fn differential_oracle_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/differential_oracle_smoke.py")
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
        .arg("--report-file")
        .arg(repo_path("target/oracle_skip_sentinel_report.json"))
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

    assert_eq!(
        v["comparison_only"].as_bool(),
        Some(true),
        "oracle sentinel must identify comparison-only tooling"
    );
    assert_eq!(
        v["runtime_dependency"].as_bool(),
        Some(false),
        "oracle sentinel must not describe a runtime dependency"
    );
    assert_eq!(
        v["build_dependency"].as_bool(),
        Some(false),
        "oracle sentinel must not describe a build dependency"
    );
    assert_eq!(
        v["java_attempted"].as_bool(),
        Some(false),
        "PUML_ORACLE_JAR-unset sentinel must not attempt java"
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
    let report_file = repo_path("target/oracle_report_schema_test.json");

    let output = Command::new("bash")
        .arg(oracle_script())
        .arg("--corpus-dir")
        .arg(&fixtures_dir)
        .arg("--report-file")
        .arg(&report_file)
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
        matches!(code, 0..=2),
        "oracle.sh must exit 0, 1, or 2 (got {code}); stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// ---------------------------------------------------------------------------
// differential_oracle_dry_run_schema_lists_fixture_categories
// ---------------------------------------------------------------------------

/// The Python smoke harness must be usable as Java-free, comparison-only
/// metadata tooling. Dry run mode should not invoke cargo rendering, PlantUML,
/// Java, or a JAR; it should still publish the fixture-backed drift categories
/// that explain known partial PlantUML gaps.
#[test]
fn differential_oracle_dry_run_schema_lists_fixture_categories() {
    let report_file = repo_path("target/oracle_smoke_dry_test_report.json");
    let output = Command::new("python3")
        .arg(differential_oracle_script())
        .arg("--dry-run")
        .arg("--quiet")
        .arg("--output")
        .arg(&report_file)
        .output()
        .expect("failed to invoke differential_oracle_smoke.py");

    assert!(
        output.status.success(),
        "dry-run oracle smoke should succeed without Java/JAR; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let written =
        std::fs::read_to_string(&report_file).expect("dry-run oracle report must be readable");
    let v: serde_json::Value =
        serde_json::from_str(written.trim()).expect("dry-run oracle report must be valid JSON");

    assert_eq!(
        v["schema_version"].as_str(),
        Some("1.2.0"),
        "dry-run schema version should capture fixture category metadata"
    );
    assert_eq!(v["generated_at_utc"].as_str(), Some("1970-01-01T00:00:00Z"));
    assert_eq!(v["tool"]["cwd"].as_str(), Some("repo-root"));
    assert_eq!(v["tool"]["dry_run"].as_bool(), Some(true));
    assert_eq!(v["oracle"]["enabled"].as_bool(), Some(false));
    assert_eq!(v["oracle"]["comparison_only"].as_bool(), Some(true));
    assert_eq!(v["oracle"]["runtime_dependency"].as_bool(), Some(false));
    assert_eq!(v["oracle"]["build_dependency"].as_bool(), Some(false));
    assert_eq!(
        v["oracle"]["normal_cargo_test_uses_oracle"].as_bool(),
        Some(false)
    );

    let total = v["summary"]["total"].as_u64().unwrap_or(0);
    assert!(total >= 8, "expected expanded oracle fixture corpus");
    assert_eq!(v["summary"]["not_run"].as_u64(), Some(total));
    assert_eq!(v["summary"]["failed"].as_u64(), Some(0));
    assert!(
        v["summary"]["by_expected_oracle_category"]["drift"]
            .as_u64()
            .unwrap_or(0)
            >= 3,
        "expected partial PlantUML gaps to be represented as drift fixtures"
    );
    assert!(
        v["summary"]["by_expected_oracle_category"]["jar-only"]
            .as_u64()
            .unwrap_or(0)
            >= 1,
        "expected unsupported advanced preprocessor gap to be fixture-backed"
    );
    let top_categories = v["summary"]["top_expected_drift_categories"]
        .as_array()
        .expect("dry-run report should rank expected drift categories");
    assert!(
        top_categories.iter().any(|category| {
            category["category"].as_str() == Some("family-partial")
                && category["fixture_count"].as_u64().unwrap_or(0) >= 3
        }),
        "expected family partials to rank as a top drift category"
    );
    let top_areas = v["summary"]["top_expected_drift_areas"]
        .as_array()
        .expect("dry-run report should rank expected drift areas");
    for expected_area in [
        "Salt widget breadth",
        "chart axis legend style",
        "dynamic preprocessor invocation",
        "mindmap orientation layout",
        "unsupported skinparam styling",
    ] {
        assert!(
            top_areas
                .iter()
                .any(|area| area["drift_area"].as_str() == Some(expected_area)),
            "expected drift area should be listed in dry-run top drift areas: {expected_area}"
        );
    }

    let fixtures = v["fixtures"]
        .as_array()
        .expect("dry-run report should contain fixture array");
    let mut saw_salt = false;
    let mut saw_preproc = false;
    for fixture in fixtures {
        assert_eq!(fixture["local"]["attempted"].as_bool(), Some(false));
        assert_eq!(fixture["oracle"]["attempted"].as_bool(), Some(false));
        assert_eq!(fixture["comparison"]["state"].as_str(), Some("not-run"));

        let rel = fixture["fixture"]
            .as_str()
            .expect("fixture entries should include relative paths");
        assert!(
            repo_path(&format!("tests/fixtures/{rel}")).exists(),
            "dry-run fixture path should exist: {rel}"
        );

        if rel == "families/valid_salt_login_form.puml" {
            saw_salt = true;
            assert_eq!(
                fixture["classification"]["support_status"].as_str(),
                Some("partial")
            );
            assert_eq!(
                fixture["classification"]["expected_oracle_category"].as_str(),
                Some("drift")
            );
        }
        if rel == "errors/invalid_preproc_dynamic_invoke.puml" {
            saw_preproc = true;
            assert_eq!(
                fixture["classification"]["expected_oracle_category"].as_str(),
                Some("jar-only")
            );
        }
    }

    assert!(saw_salt, "Salt partial fixture should be in dry-run corpus");
    assert!(
        saw_preproc,
        "advanced preprocessor partial fixture should be in dry-run corpus"
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
    assert!(
        contents.contains("comparison-only"),
        "docs/oracle-thresholds.md must document comparison-only oracle usage"
    );
    assert!(
        contents.contains("dry-run"),
        "docs/oracle-thresholds.md must document the Java-free dry-run schema"
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
