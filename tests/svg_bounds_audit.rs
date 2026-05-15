use std::process::Command;

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
