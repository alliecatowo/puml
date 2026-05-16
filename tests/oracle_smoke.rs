/// Oracle smoke tests — differential parity harness (#88)
///
/// The `oracle_skip_sentinel` test runs `scripts/oracle.sh` with no
/// `PUML_ORACLE_JAR` env var and verifies the deterministic skip contract:
/// exit 0 + stdout containing `"skipped":true`.
///
/// The `oracle_with_jar` test is `#[ignore]`-gated and only runs when
/// `PUML_ORACLE_JAR` points to a real PlantUML JAR.

#[test]
fn oracle_skip_sentinel() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let script = format!("{manifest_dir}/scripts/oracle.sh");

    let output = std::process::Command::new("bash")
        .arg(&script)
        .env_remove("PUML_ORACLE_JAR")
        .output()
        .expect("failed to run oracle.sh");

    assert!(
        output.status.success(),
        "oracle.sh should exit 0 in skip mode, got: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"skipped\":true"),
        "oracle.sh skip mode should output {{\"skipped\":true,...}}, got: {stdout}"
    );
}

#[test]
#[ignore = "requires PUML_ORACLE_JAR=/path/to/plantuml.jar"]
fn oracle_with_jar() {
    let jar = std::env::var("PUML_ORACLE_JAR").expect("PUML_ORACLE_JAR must be set");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let script = format!("{manifest_dir}/scripts/oracle.sh");

    let output = std::process::Command::new("bash")
        .arg(&script)
        .env("PUML_ORACLE_JAR", &jar)
        .output()
        .expect("failed to run oracle.sh");

    assert!(
        output.status.success(),
        "oracle.sh should exit 0 when JAR is present and all checks pass\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"skipped\":false"),
        "oracle.sh with JAR should output {{\"skipped\":false,...}}, got: {stdout}"
    );
}
