use std::fs;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn branch_protection_script_contains_required_checks() {
    let script = fs::read_to_string(repo_path("scripts/branch-protection.sh"))
        .expect("failed to read scripts/branch-protection.sh");
    let fixture = fs::read_to_string(repo_path(
        "tests/fixtures/contract/branch_protection_required_checks.txt",
    ))
    .expect("failed to read branch protection check fixture");

    for raw in fixture.lines() {
        let check = raw.trim();
        if check.is_empty() {
            continue;
        }
        assert!(
            script.contains(check),
            "branch protection script must require status check context: {check}"
        );
    }
}

#[test]
fn docs_capture_branch_protection_validation_command() {
    let readme = fs::read_to_string(repo_path("README.md")).expect("failed to read README.md");
    let checklist = fs::read_to_string(repo_path("docs/release-checklist.md"))
        .expect("failed to read docs/release-checklist.md");
    let policy = fs::read_to_string(repo_path("docs/branch-protection.md"))
        .expect("failed to read docs/branch-protection.md");

    assert!(
        readme.contains("./scripts/branch-protection.sh verify"),
        "README should document the branch protection verification command"
    );
    assert!(
        checklist.contains("./scripts/branch-protection.sh verify"),
        "release checklist should include branch protection verification"
    );
    assert!(
        policy.contains("./scripts/branch-protection.sh apply"),
        "branch protection doc should include apply command"
    );
    assert!(
        policy.contains("Audited Fallback When API Writes Are Blocked"),
        "branch protection doc should describe audited fallback behavior"
    );
}
