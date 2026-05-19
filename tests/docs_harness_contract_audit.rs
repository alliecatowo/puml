use std::fs;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn internal_agent_runbooks_present_after_docs_reorg() {
    // README and CONTRIBUTING are user-facing post-docs-reorg. Agent runbook
    // material lives in docs/internal/agents/ — verify the canonical files exist
    // and carry the key script-runbook commands.
    for path in [
        "docs/internal/agents/codex-workflow.md",
        "docs/internal/agents/autonomous-workflow-cookbook.md",
    ] {
        assert!(
            repo_path(path).exists(),
            "internal agent runbook should exist after docs reorg: {path}"
        );
    }
}

#[test]
fn codex_workflow_doc_has_codex_claude_runbook_and_gallery_refresh_commands() {
    let doc = fs::read_to_string(repo_path("docs/internal/agents/codex-workflow.md"))
        .expect("read docs/internal/agents/codex-workflow.md");
    for needle in [
        "Codex and Claude",
        "## Exact Codex + Claude Runbook",
        "./scripts/harness-check.sh --dry",
        "./scripts/harness-check.sh --quick",
        "./scripts/harness-check.sh",
        "./scripts/autonomy-check.sh --quick",
        "./scripts/autonomy-check.sh",
        "for f in docs/examples/*.puml; do cargo run -- \"$f\"; done",
        "cargo run -- --from-markdown --multi docs/examples/sequence/README.md",
        "--fail-on-doc-drift",
    ] {
        assert!(
            doc.contains(needle),
            "docs/codex-workflow.md missing: {needle}"
        );
    }
}

#[test]
fn autonomous_cookbook_documents_dedicated_worktree_issue_flow() {
    let doc = fs::read_to_string(repo_path(
        "docs/internal/agents/autonomous-workflow-cookbook.md",
    ))
    .expect("read docs/internal/agents/autonomous-workflow-cookbook.md");
    for needle in [
        "git worktree add ../puml-issue-131 -b feat/issue-131-docs-harness origin/main",
        "./scripts/harness-check.sh --quick",
        "./scripts/autonomy-check.sh --quick",
        "./scripts/autonomy-check.sh",
    ] {
        assert!(
            doc.contains(needle),
            "docs/autonomous-workflow-cookbook.md missing: {needle}"
        );
    }
}

#[test]
fn referenced_harness_scripts_exist() {
    for script in [
        "scripts/harness-check.sh",
        "scripts/autonomy-check.sh",
        "scripts/render_check.py",
    ] {
        assert!(
            repo_path(script).exists(),
            "referenced harness command target should exist: {script}"
        );
    }
}
