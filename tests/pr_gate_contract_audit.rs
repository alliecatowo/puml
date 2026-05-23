use std::fs;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn pr_gate_coverage_command_stays_aligned_with_release_gate() {
    let release_gate = fs::read_to_string(repo_path("scripts/check-all.sh"))
        .expect("failed to read scripts/check-all.sh");
    let pr_gate = fs::read_to_string(repo_path(".github/workflows/pr-gate.yml"))
        .expect("failed to read .github/workflows/pr-gate.yml");

    let expected_threshold = "cargo llvm-cov --all-features --workspace --fail-under-lines 87";
    let expected_ignore = r#"src/(main|bin/puml-lsp|lib|parser|preproc|normalize|render|specialized)\.rs|src/(frontend|normalize|parser|render|specialized)/.*\.rs"#;

    assert!(
        release_gate.contains(expected_threshold),
        "release check-all gate must keep coverage threshold at 87"
    );
    assert!(
        release_gate.contains(expected_ignore),
        "release check-all gate must keep existing scoped ignore regex"
    );

    assert!(
        pr_gate.contains(expected_threshold),
        "PR gate coverage step should use the same threshold as release check-all"
    );
    assert!(
        pr_gate.contains(expected_ignore),
        "PR gate coverage step should keep the same scoped ignore regex as release check-all"
    );
    assert!(
        pr_gate.contains(
            "if: always() && needs.changes.outputs.run_full_gate == 'true' && needs.artifact_regen.outputs.pushed_regen_commit != 'true' && github.actor != 'dependabot[bot]' && github.actor != 'renovate[bot]'"
        ),
        "PR gate coverage job must stay scoped to full-gate runs on non-bot actors"
    );
}
