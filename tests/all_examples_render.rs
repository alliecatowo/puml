//! Every .puml under docs/examples/ must render successfully.
//!
//! This test walks the entire `docs/examples/` tree and asserts that every
//! `.puml` file renders cleanly via the `puml` binary with `--format svg`.
//! A failure here means either an example was broken (fix the example) or a
//! renderer regression was introduced (fix the renderer).

use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::tempdir;

/// Recursively collect all `.puml` files under `root`.
fn collect_puml_files(root: &std::path::Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return result;
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            result.extend(collect_puml_files(&path));
        } else if path.extension().is_some_and(|e| e == "puml") {
            result.push(path);
        }
    }
    result
}

#[test]
fn all_docs_examples_render_cleanly() {
    let examples_dir = format!("{}/docs/examples", env!("CARGO_MANIFEST_DIR"));
    let examples_path = std::path::Path::new(&examples_dir);

    assert!(
        examples_path.exists(),
        "docs/examples/ directory not found at {examples_dir}"
    );

    let puml_files = collect_puml_files(examples_path);
    assert!(
        !puml_files.is_empty(),
        "No .puml files found under docs/examples/"
    );

    // Use a temp directory for output to avoid /dev/null sandbox restrictions.
    let tmp = tempdir().expect("create temp dir for render outputs");
    let out_svg = tmp.path().join("render_out.svg");

    let mut failures: Vec<String> = Vec::new();

    for path in &puml_files {
        let output = Command::cargo_bin("puml")
            .expect("puml binary must be available")
            .args(["--format", "svg"])
            .arg(path)
            .args(["-o", out_svg.to_str().unwrap()])
            .output()
            .unwrap_or_else(|e| panic!("failed to invoke puml on {}: {e}", path.display()));

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            failures.push(format!(
                "  {}: {}",
                path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
                    .unwrap_or(path)
                    .display(),
                stderr.trim()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} examples failed to render:\n{}",
        failures.len(),
        puml_files.len(),
        failures.join("\n")
    );
}
