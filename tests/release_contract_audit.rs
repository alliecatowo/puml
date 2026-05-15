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
    let checklist = fs::read_to_string(repo_path("docs/release-checklist.md"))
        .expect("failed to read docs/release-checklist.md");
    let readme = fs::read_to_string(repo_path("README.md")).expect("failed to read README.md");

    assert!(
        checklist.contains("cargo build --release"),
        "release checklist must include release build validation"
    );
    assert!(
        checklist.contains("cargo llvm-cov --all-features --workspace --fail-under-lines 90"),
        "release checklist must include coverage gate command"
    );
    assert!(
        readme.contains("./scripts/check-all.sh --quick"),
        "README should document quick gate usage"
    );
}
