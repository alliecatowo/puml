use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn classify(paths: &[&str]) -> String {
    let mut child = Command::new("python3")
        .arg(repo_path("scripts/ci-classify-changes.py"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn ci-classify-changes.py");

    child
        .stdin
        .as_mut()
        .expect("classifier stdin should be piped")
        .write_all(paths.join("\n").as_bytes())
        .expect("failed to write changed file list to classifier stdin");

    let output = child
        .wait_with_output()
        .expect("failed to run ci-classify-changes.py");

    assert!(
        output.status.success(),
        "classifier failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("classifier output should be utf-8")
}

fn assert_output_contains(output: &str, expected: &[&str]) {
    for line in expected {
        assert!(
            output.lines().any(|actual| actual == *line),
            "missing classifier output line `{line}` in:\n{output}"
        );
    }
}

#[test]
fn site_content_changes_skip_full_rust_gate() {
    let output = classify(&["site/content/guide/getting-started.md"]);

    assert_output_contains(
        &output,
        &[
            "run_full_gate=false",
            "docs_examples_changed=false",
            "run_docs_examples_drift=false",
            "run_wasm_check=false",
            "run_site_smoke=true",
            "run_wasm_site_smoke=false",
        ],
    );
}

#[test]
fn wasm_crate_changes_run_full_and_wasm_site_smoke() {
    let output = classify(&["crates/puml-wasm/src/lib.rs"]);

    assert_output_contains(
        &output,
        &[
            "run_full_gate=true",
            "docs_examples_changed=false",
            "run_docs_examples_drift=false",
            "run_wasm_check=true",
            "run_site_smoke=true",
            "run_wasm_site_smoke=true",
        ],
    );
}

#[test]
fn docs_examples_changes_keep_drift_and_site_coverage() {
    let output = classify(&["docs/examples/sequence/basic.puml"]);

    assert_output_contains(
        &output,
        &[
            "run_full_gate=true",
            "docs_examples_changed=true",
            "run_docs_examples_drift=true",
            "run_wasm_check=false",
            "run_site_smoke=true",
            "run_wasm_site_smoke=false",
        ],
    );
}

#[test]
fn rust_test_only_changes_skip_unrelated_expensive_shards() {
    let output = classify(&["tests/integration.rs"]);

    assert_output_contains(
        &output,
        &[
            "run_full_gate=true",
            "docs_examples_changed=false",
            "run_docs_examples_drift=false",
            "run_wasm_check=false",
            "run_site_smoke=false",
            "run_wasm_site_smoke=false",
        ],
    );
}

#[test]
fn renderer_changes_keep_wasm_and_docs_example_drift_coverage() {
    let output = classify(&["src/render/mod.rs"]);

    assert_output_contains(
        &output,
        &[
            "run_full_gate=true",
            "docs_examples_changed=false",
            "run_docs_examples_drift=true",
            "run_wasm_check=true",
            "run_site_smoke=false",
            "run_wasm_site_smoke=false",
        ],
    );
}

#[test]
fn empty_change_sets_take_the_conservative_path() {
    let output = classify(&[]);

    assert_output_contains(
        &output,
        &[
            "run_full_gate=true",
            "docs_examples_changed=false",
            "run_docs_examples_drift=true",
            "run_wasm_check=true",
            "run_site_smoke=true",
            "run_wasm_site_smoke=false",
        ],
    );
}
