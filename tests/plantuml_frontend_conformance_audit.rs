use std::fs;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn conformance_matrix_has_required_sections_and_scenarios() {
    let doc = fs::read_to_string(repo_path("docs/plantuml_frontend_conformance_matrix.md"))
        .expect("failed to read conformance matrix doc");
    for needle in [
        "# PlantUML Frontend Conformance Matrix",
        "## Matrix",
        "@startuml` optional suffix",
        "apostrophe inside quoted label preserved",
        "unknown preprocessor directive fails deterministically",
        "trailing unterminated block in `--multi`",
    ] {
        assert!(
            doc.contains(needle),
            "conformance matrix missing required scenario text: {needle}"
        );
    }
}

#[test]
fn conformance_matrix_fixture_paths_exist_and_test_anchors_resolve() {
    let doc = fs::read_to_string(repo_path("docs/plantuml_frontend_conformance_matrix.md"))
        .expect("failed to read conformance matrix doc");

    let mut fixtures = Vec::new();
    let mut anchors = Vec::new();
    for line in doc.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        if trimmed.contains("---") || trimmed.contains("Fixture") {
            continue;
        }
        let cols = trimmed
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim().trim_matches('`'))
            .collect::<Vec<_>>();
        if cols.len() < 5 {
            continue;
        }
        fixtures.push(cols[2].to_string());
        anchors.push(cols[4].to_string());
    }

    assert!(
        !fixtures.is_empty(),
        "expected at least one fixture row in conformance matrix"
    );

    for fixture_cell in fixtures {
        for token in fixture_cell
            .split('+')
            .map(str::trim)
            .map(|t| t.trim_matches('`'))
        {
            if !token.starts_with("tests/fixtures/") {
                continue;
            }
            assert!(
                repo_path(token).exists(),
                "fixture path referenced by conformance matrix should exist: {token}"
            );
        }
    }

    let integration = fs::read_to_string(repo_path("tests/integration.rs"))
        .expect("failed to read tests/integration.rs");
    let coverage_edges = fs::read_to_string(repo_path("tests/coverage_edges.rs"))
        .expect("failed to read tests/coverage_edges.rs");
    let parser =
        fs::read_to_string(repo_path("src/parser.rs")).expect("failed to read src/parser.rs");
    for anchor in anchors {
        let Some((file_hint, symbol)) = anchor.split_once("::") else {
            continue;
        };
        let needle = format!("fn {symbol}");
        match file_hint {
            "tests/integration.rs" => assert!(
                integration.contains(&needle),
                "missing integration anchor referenced by matrix: {anchor}"
            ),
            "tests/coverage_edges.rs" => assert!(
                coverage_edges.contains(&needle),
                "missing coverage_edges anchor referenced by matrix: {anchor}"
            ),
            "src/parser.rs" => assert!(
                parser.contains(&needle),
                "missing parser anchor referenced by matrix: {anchor}"
            ),
            _ => panic!("unsupported matrix anchor file hint: {file_hint}"),
        }
    }
}
