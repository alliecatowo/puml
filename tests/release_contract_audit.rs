use std::fs;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn full_gate_contains_required_release_commands_in_order() {
    let script = fs::read_to_string(repo_path("scripts/check-all.sh"))
        .expect("failed to read scripts/check-all.sh");
    let fixture = fs::read_to_string(repo_path(
        "tests/fixtures/contract/release_gate_full_commands.txt",
    ))
    .expect("failed to read release gate command fixture");

    let mut search_from = 0usize;
    for raw in fixture.lines() {
        let cmd = raw.trim();
        if cmd.is_empty() {
            continue;
        }

        let found = script[search_from..]
            .find(cmd)
            .unwrap_or_else(|| panic!("missing release-gate command in check-all.sh: {cmd}"));
        search_from += found + cmd.len();
    }
}

#[test]
fn release_docs_capture_release_gate_contract() {
    let script = fs::read_to_string(repo_path("scripts/check-all.sh"))
        .expect("failed to read scripts/check-all.sh");
    let bench =
        fs::read_to_string(repo_path("scripts/bench.sh")).expect("failed to read scripts/bench.sh");
    let checklist = fs::read_to_string(repo_path("docs/release-checklist.md"))
        .expect("failed to read docs/release-checklist.md");
    let bench_docs = fs::read_to_string(repo_path("docs/benchmarks/README.md"))
        .expect("failed to read docs/benchmarks/README.md");
    let readme = fs::read_to_string(repo_path("README.md")).expect("failed to read README.md");
    let coverage = fs::read_to_string(repo_path("docs/internal/coverage-status.md"))
        .expect("failed to read docs/internal/coverage-status.md");

    assert!(
        checklist.contains("cargo build --release"),
        "release checklist must include release build validation"
    );
    assert!(
        checklist.contains("cargo llvm-cov --all-features --workspace --fail-under-lines 85"),
        "release checklist must include coverage gate command"
    );
    assert!(
        script.contains("src/(main|bin/puml-lsp|lib|parser|preproc|normalize|render|specialized)\\.rs|src/(frontend|normalize|parser|render|specialized)/.*\\.rs"),
        "full gate should scope coverage away from entrypoint and high-churn parity implementation modules"
    );
    assert!(
        coverage.contains("Coverage scope excludes entrypoint binaries, library facade, and high-churn parity implementation modules"),
        "coverage status doc should capture scoped coverage policy"
    );
    assert!(
        bench.contains("BINARY_LIMIT_BYTES_FULL=12000000"),
        "bench gate should define the post-url-include full-mode binary ceiling"
    );
    assert!(
        bench.contains("BINARY_LIMIT_BYTES_QUICK=12000000"),
        "bench gate should define the post-url-include quick-mode binary ceiling"
    );
    assert!(
        bench.contains("REGRESSION_MIN_DELTA_MS_FULL=40"),
        "bench gate should define a full-mode regression delta floor"
    );
    assert!(
        bench.contains("REGRESSION_MIN_DELTA_MS_QUICK=50"),
        "bench gate should define a quick-mode regression delta floor"
    );
    assert!(
        bench.contains("baseline_quick.json") && bench.contains("baseline_full.json"),
        "bench gate should use mode-scoped baseline artifacts"
    );
    assert!(
        bench.contains("--update-baseline"),
        "bench gate should require explicit baseline updates"
    );
    assert!(
        bench.contains("--check-artifacts") && bench.contains("validate-artifacts"),
        "bench gate should expose checked-in artifact policy validation"
    );
    assert!(
        bench_docs.contains("./scripts/bench.sh --check-artifacts"),
        "bench docs should describe artifact policy validation"
    );
    assert!(
        checklist.contains("./scripts/bench.sh --check-artifacts"),
        "release checklist should require artifact policy validation"
    );
    assert!(
        bench_docs.contains("absolute delta floor `>40ms`"),
        "bench docs should describe full-mode regression delta floor"
    );
    assert!(
        bench_docs.contains("absolute delta floor `>50ms`"),
        "bench docs should describe quick-mode regression delta floor"
    );
    assert!(
        bench_docs.contains("binary size limit (`target/release/puml`): `12,000,000` bytes"),
        "bench docs should describe recalibrated binary ceiling"
    );
    assert!(
        bench_docs.contains("baseline_full.json") && bench_docs.contains("baseline_quick.json"),
        "bench docs should describe mode-scoped baselines"
    );
    // README was rewritten as a user-facing landing page (#457); developer gate docs
    // moved to docs/release-checklist.md and CONTRIBUTING.md.
    assert!(
        readme.contains("./scripts/check-all.sh --quick")
            || checklist.contains("./scripts/check-all.sh --quick"),
        "README or release checklist should document quick gate usage"
    );
    assert!(
        readme.contains("--update-baseline")
            || checklist.contains("--update-baseline")
            || bench.contains("--update-baseline"),
        "README should document explicit baseline refresh command"
    );
}
