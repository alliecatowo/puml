use std::fs;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn assert_tokens_in_order(spec_rel: &str, fixture_rel: &str) {
    let spec = fs::read_to_string(repo_path(spec_rel))
        .unwrap_or_else(|_| panic!("failed to read {spec_rel}"));
    let fixture = fs::read_to_string(repo_path(fixture_rel))
        .unwrap_or_else(|_| panic!("failed to read {fixture_rel}"));

    let mut search_from = 0usize;
    for raw in fixture.lines() {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }

        let found = spec[search_from..]
            .find(token)
            .unwrap_or_else(|| panic!("missing ecosystem contract token in spec: {token}"));
        search_from += found + token.len();
    }
}

fn extract_fixture_refs(doc: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for token in doc.split_whitespace() {
        let trimmed = token.trim_matches(|c: char| {
            c == '`'
                || c == ','
                || c == '.'
                || c == ':'
                || c == ';'
                || c == ')'
                || c == '('
                || c == ']'
                || c == '['
        });
        if trimmed.starts_with("tests/fixtures/") && trimmed.ends_with(".puml") {
            refs.push(trimmed.to_string());
        }
    }
    refs.sort();
    refs.dedup();
    refs
}

#[test]
fn lsp_spec_includes_current_runtime_surface_snapshot() {
    assert_tokens_in_order(
        "docs/specs/puml_lsp_spec.md",
        "tests/fixtures/contract/lsp_runtime_surface.txt",
    );
}

#[test]
fn vscode_spec_includes_current_runtime_surface_snapshot() {
    assert_tokens_in_order(
        "docs/specs/puml_vscode_extension_spec(1).md",
        "tests/fixtures/contract/vscode_runtime_surface.txt",
    );
}

#[test]
fn agent_pack_spec_includes_current_runtime_surface_snapshot() {
    assert_tokens_in_order(
        "docs/specs/puml_agent_plugin_mcp_spec.md",
        "tests/fixtures/contract/agent_pack_runtime_surface.txt",
    );
}

#[test]
fn parity_roadmap_fixture_paths_exist() {
    let roadmap = fs::read_to_string(repo_path("docs/parity-roadmap.md"))
        .expect("failed to read docs/parity-roadmap.md");
    let refs = extract_fixture_refs(&roadmap);
    assert!(
        !refs.is_empty(),
        "expected at least one fixture reference in docs/parity-roadmap.md"
    );

    let missing = refs
        .into_iter()
        .filter(|p| !repo_path(p).exists())
        .collect::<Vec<_>>();

    assert!(
        missing.is_empty(),
        "missing fixture path references in docs/parity-roadmap.md: {}",
        missing.join(", ")
    );
}

#[test]
fn decision_log_captures_current_preprocessor_contract() {
    let log = fs::read_to_string(repo_path("docs/decision-log.md"))
        .expect("failed to read docs/decision-log.md");
    assert!(
        log.contains("bounded preprocessing as executable today"),
        "decision log should describe bounded preprocessing as executable"
    );
    assert!(
        log.contains("plus simple `!define`/`!undef` token substitution"),
        "decision log should explicitly mention define/undef substitution support"
    );
}
