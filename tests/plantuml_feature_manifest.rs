use std::path::PathBuf;
use std::process::Command;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn plantuml_feature_manifest_is_executable_and_fixture_backed() {
    let output = Command::new("python3")
        .arg(repo_path("scripts/plantuml_feature_manifest.py"))
        .arg("--manifest")
        .arg(repo_path("tests/plantuml_feature_manifest.json"))
        .arg("--json")
        .output()
        .expect("manifest validator should run");

    assert!(
        output.status.success(),
        "manifest validator failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let summary: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("validator should emit JSON");
    assert!(
        summary["entries"].as_u64().unwrap_or(0) >= 50,
        "manifest should seed at least 50 high-value feature entries"
    );

    for family in [
        "sequence",
        "class",
        "component",
        "deployment",
        "c4",
        "state",
        "activity",
        "salt",
        "mindmap",
        "wbs",
        "timing",
        "stdlib",
        "include",
    ] {
        assert!(
            summary["families"][family].as_u64().unwrap_or(0) > 0,
            "manifest should cover required family {family}"
        );
    }

    assert!(
        summary["statuses"]["known_visual_risk"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "manifest should classify high-risk visual cases explicitly"
    );
    assert!(
        summary["statuses"]["unsupported"].as_u64().unwrap_or(0) > 0,
        "manifest should keep unsupported diagnostics in the executable matrix"
    );
    assert!(
        summary["oracle_statuses"]["promoted-blocking"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "manifest should name blocking oracle-promoted coverage"
    );
    assert!(
        summary["oracle_statuses"]["promoted-report"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "manifest should name report-only oracle-promoted coverage"
    );
}
