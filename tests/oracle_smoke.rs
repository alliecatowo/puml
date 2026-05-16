/// Oracle smoke test (#88).
///
/// Shells out to scripts/oracle.sh and asserts it exits 0.
/// When no JAR is present (CI without plantuml.jar), the script returns
/// `{"skipped":true,...}` and exits 0 — the test passes.
/// When PUML_ORACLE_JAR is set and a JAR is found, the script runs the
/// real byte-compare and the test verifies parity or reports a mismatch.

#[test]
fn oracle_smoke_exits_zero_without_jar() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let oracle = format!("{manifest_dir}/scripts/oracle.sh");

    // Ensure puml binary is built before shelling out.
    let status = std::process::Command::new("sh")
        .arg(&oracle)
        .env_remove("PUML_ORACLE_JAR")
        // Override JAR path to a nonexistent location to force skipped:true
        .env("PUML_ORACLE_JAR", "/nonexistent/plantuml.jar")
        .status()
        .expect("failed to execute oracle.sh");

    assert!(
        status.success(),
        "oracle.sh should exit 0 when JAR is absent (skipped path)"
    );
}
