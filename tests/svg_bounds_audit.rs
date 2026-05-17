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
const DOC_SOURCE_EXTS: &[&str] = &["puml", "plantuml", "picouml"];

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

    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
                    continue;
                };
                if !DOC_SOURCE_EXTS.contains(&ext) {
                    continue;
                }
                let artifact = path.with_extension("svg");
                expected.push(DocExampleKey {
                    source_markdown: "".to_string(),
                    source_kind: "source_file".to_string(),
                    source_ref: path
                        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                        .expect("repo-relative source path")
                        .to_string_lossy()
                        .to_string(),
                    artifact_svg: artifact
                        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                        .expect("repo-relative artifact path")
                        .to_string_lossy()
                        .to_string(),
                });
            }
        }
    }

    for md_path in md_files {
        let raw = fs::read_to_string(&md_path).expect("docs/examples markdown should be readable");
        let rel_md = md_path
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .expect("repo-relative markdown path")
            .to_string_lossy()
            .to_string();

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
    assert_eq!(
        json["doc_examples"]["summary"]["excluded"].as_u64(),
        Some(4),
        "intentionally excluded docs examples should be explicit and rare"
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
        "parity harness doc gallery discovery should stay in lock-step with all docs/examples source files and supported fenced snippets"
    );

    let entries = json["doc_examples"]["entries"]
        .as_array()
        .expect("doc_examples.entries should be an array");
    let excluded = entries
        .iter()
        .find(|entry| {
            entry["source_ref"].as_str()
                == Some("docs/examples/nonuml_parity_gantt_chart_topology.puml")
        })
        .expect("mixed-family topology source should be reported");
    assert_eq!(excluded["status"].as_str(), Some("excluded"));
    assert!(
        excluded["exclusion_reason"]
            .as_str()
            .map(|reason| reason.contains("mixed-family"))
            .unwrap_or(false),
        "excluded docs example should carry a concrete reason"
    );
}

#[test]
fn differential_oracle_smoke_report_schema_is_stable_in_dry_mode() {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("oracle_smoke_test_report.json");

    let output = Command::new("python3")
        .args([
            "scripts/differential_oracle_smoke.py",
            "--quick",
            "--dry",
            "--quiet",
            "--output",
            path.to_str().expect("utf-8 path"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run scripts/differential_oracle_smoke.py");

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "differential oracle smoke harness failed\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            stdout,
            stderr
        );
    }

    let raw = fs::read_to_string(&path).expect("report should be written");
    let json: Value = serde_json::from_str(&raw).expect("report must be valid JSON");
    assert_eq!(json["schema_version"], "1.2.0");
    assert_eq!(json["generated_at_utc"], "1970-01-01T00:00:00Z");
    assert_eq!(json["tool"]["cwd"], "repo-root");
    assert_eq!(json["oracle"]["mode"], "metadata-dry-run");
    assert_eq!(json["tool"]["dry_run"], true);
    assert_eq!(json["tool"]["quick_mode"], true);
    let fixtures = json["fixtures"]
        .as_array()
        .expect("fixtures should be an array");
    assert!(!fixtures.is_empty(), "expected non-empty fixture corpus");
    assert_eq!(
        json["summary"]["total"].as_u64(),
        Some(fixtures.len() as u64)
    );
    assert!(
        json["summary"]["top_expected_drift_categories"]
            .as_array()
            .map(|rows| rows.iter().all(|row| row["fixture_count"].is_u64()))
            .unwrap_or(false),
        "expected deterministic top drift category rows"
    );
}
