use std::fs;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn studio_spec_includes_current_runtime_contract_snapshot() {
    let spec = fs::read_to_string(repo_path("docs/specs/puml_studio_spa_spec.md"))
        .expect("failed to read docs/specs/puml_studio_spa_spec.md");
    let fixture = fs::read_to_string(repo_path(
        "tests/fixtures/contract/studio_spa_runtime_surface.txt",
    ))
    .expect("failed to read studio runtime contract fixture");

    let mut search_from = 0usize;
    for raw in fixture.lines() {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }

        let found = spec[search_from..]
            .find(token)
            .unwrap_or_else(|| panic!("missing studio runtime contract token in spec: {token}"));
        search_from += found + token.len();
    }
}

#[test]
fn studio_spec_marks_target_api_sections_explicitly() {
    let spec = fs::read_to_string(repo_path("docs/specs/puml_studio_spa_spec.md"))
        .expect("failed to read docs/specs/puml_studio_spa_spec.md");

    assert!(
        spec.contains("## WASM API (Target)"),
        "studio spec should distinguish target wasm api from current shipped runtime"
    );
    assert!(
        spec.contains("## Worker protocol (Target)"),
        "studio spec should distinguish target worker protocol from current shipped runtime"
    );
}
