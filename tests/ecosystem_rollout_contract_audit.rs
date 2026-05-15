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
