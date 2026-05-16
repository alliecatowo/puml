use std::process::Command;
use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

const GALLERY_FENCE_LANGS: &[&str] = &[
    "puml",
    "pumlx",
    "picouml",
    "plantuml",
    "uml",
    "puml-sequence",
    "uml-sequence",
    "mermaid",
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DocExampleKey {
    source_markdown: String,
    source_kind: String,
    source_ref: String,
    artifact_svg: String,
}

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn markdown_files_under(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    files
}

fn parse_expected_gallery_entries() -> Vec<DocExampleKey> {
    let root = repo_path("docs/examples");
    let md_files = markdown_files_under(&root);
    let mut expected = Vec::new();

    for md_path in md_files {
        let raw = fs::read_to_string(&md_path).expect("docs/examples markdown should be readable");
        let rel_md = md_path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .expect("repo-relative markdown path")
            .to_string_lossy()
            .to_string();

        let mut cursor = 0usize;
        while let Some(open_idx) = raw[cursor..].find("](") {
            let start = cursor + open_idx + 2;
            let Some(close_rel) = raw[start..].find(')') else {
                break;
            };
            let end = start + close_rel;
            let trimmed = &raw[start..end];
            cursor = end + 1;
            if !trimmed.ends_with(".puml") {
                continue;
            }
            let puml_path = md_path
                .parent()
                .expect("markdown parent")
                .join(trimmed)
                .canonicalize()
                .expect("linked .puml path should exist");
            let artifact = puml_path.with_extension("svg");
            let source_ref = puml_path
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .expect("repo-relative .puml path")
                .to_string_lossy()
                .to_string();
            let artifact_svg = artifact
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .expect("repo-relative artifact path")
                .to_string_lossy()
                .to_string();
            expected.push(DocExampleKey {
                source_markdown: rel_md.clone(),
                source_kind: "linked_file".to_string(),
                source_ref,
                artifact_svg,
            });
        }

        let mut snippet_index = 0usize;
        for line in raw.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with("```") {
                continue;
            }
            let lang = trimmed
                .trim_start_matches("```")
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_ascii_lowercase();
            if !GALLERY_FENCE_LANGS.contains(&lang.as_str()) {
                continue;
            }
            snippet_index += 1;
            let artifact = md_path.with_file_name(format!(
                "{}_snippet_{}.svg",
                md_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("README"),
                snippet_index
            ));
            let artifact_svg = artifact
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .expect("repo-relative snippet artifact path")
                .to_string_lossy()
                .to_string();
            expected.push(DocExampleKey {
                source_markdown: rel_md.clone(),
                source_kind: "inline_snippet".to_string(),
                source_ref: format!("{rel_md}#snippet-{snippet_index}"),
                artifact_svg,
            });
        }
    }

    expected.sort();
    expected.dedup();
    expected
}

#[test]
fn svg_bounds_audit_regression_corpus_passes() {
    let output = Command::new("python3")
        .args(["scripts/svg_bounds_audit.py", "--quiet"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run scripts/svg_bounds_audit.py");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "svg bounds audit failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout,
            stderr
        );
    }
}

#[test]
fn parity_harness_report_schema_is_stable() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("parity_harness_test_report.json");

    let output = Command::new("python3")
        .args([
            "scripts/parity_harness.py",
            "--quiet",
            "--output",
            path.to_str().expect("utf-8 path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run scripts/parity_harness.py");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "parity harness failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout,
            stderr
        );
    }

    let raw = fs::read_to_string(&path).expect("report should be written");
    let json: Value = serde_json::from_str(&raw).expect("report must be valid JSON");
    assert_eq!(json["schema_version"], "1.0.0");
    assert!(json.get("fixtures").and_then(Value::as_array).is_some());
    assert!(json.get("summary").is_some());
    assert!(json.get("oracle").is_some());
    assert!(json.get("doc_examples").is_some());
    assert_eq!(json["doc_examples"]["summary"]["failed"], 0);
    assert!(
        json["doc_examples"]["entries"]
            .as_array()
            .map(|rows| !rows.is_empty())
            .unwrap_or(false),
        "doc_examples.entries should be non-empty"
    );
    assert_eq!(
        json["doc_examples"]["summary"]["failed"].as_u64(),
        Some(0),
        "doc example SVG artifacts should match current renderer output"
    );

    let expected = parse_expected_gallery_entries();
    let mut discovered = json["doc_examples"]["entries"]
        .as_array()
        .expect("doc_examples.entries should be an array")
        .iter()
        .map(|entry| DocExampleKey {
            source_markdown: entry["source_markdown"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            source_kind: entry["source_kind"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            source_ref: entry["source_ref"].as_str().unwrap_or_default().to_string(),
            artifact_svg: entry["artifact_svg"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        })
        .collect::<Vec<_>>();
    discovered.sort();
    discovered.dedup();

    assert_eq!(
        discovered, expected,
        "parity harness doc gallery discovery should stay in lock-step with docs/examples markdown links and supported fenced snippets"
    );
}
