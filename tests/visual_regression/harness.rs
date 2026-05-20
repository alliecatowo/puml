use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::Fixture;

pub(crate) struct Failure {
    pub(crate) fixture: String,
    pub(crate) reasons: Vec<String>,
}

pub(crate) fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(crate) fn render_svg(fixture_path: &Path) -> Result<String, String> {
    let source = fs::read_to_string(fixture_path)
        .map_err(|e| format!("read {} failed: {e}", fixture_path.display()))?;
    let include_root = fixture_path
        .parent()
        .ok_or_else(|| format!("fixture has no parent: {}", fixture_path.display()))?;

    let output = Command::cargo_bin("puml")
        .map_err(|e| format!("cargo_bin(puml) failed: {e}"))?
        .arg("-")
        .arg("--format")
        .arg("svg")
        .arg("--include-root")
        .arg(include_root)
        .arg("--quiet")
        .write_stdin(source)
        .output()
        .map_err(|e| format!("spawn puml failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "puml exited {:?}; stderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let svg = String::from_utf8_lossy(&output.stdout).into_owned();
    if !svg.contains("<svg") {
        return Err(format!(
            "puml produced no SVG on stdout ({} bytes); stderr:\n{}",
            output.stdout.len(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(svg)
}

pub(crate) fn svg_shape_failures(svg: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    let trimmed = svg.trim();
    if trimmed.is_empty() {
        reasons.push("rendered SVG is empty".to_string());
    }
    if !trimmed.contains("<svg") {
        reasons.push("rendered output does not contain an <svg> root".to_string());
    }
    if !trimmed.contains("</svg>") {
        reasons.push("rendered SVG is missing its closing </svg> tag".to_string());
    }
    if !trimmed.contains("viewBox=\"") {
        reasons.push("rendered SVG is missing a viewBox".to_string());
    }
    reasons
}

/// Where a fixture's baseline PNG lives (committed to git).
pub(crate) fn baseline_png_path(root: &Path, fixture: &Fixture) -> PathBuf {
    let file_path = Path::new(&fixture.path);
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    root.join("tests")
        .join("visual_baselines")
        .join(&fixture.family)
        .join(format!("{stem}.png"))
}

/// Where the freshly-rendered PNG is written for inspection on mismatch.
pub(crate) fn rendered_png_path(root: &Path, fixture: &Fixture) -> PathBuf {
    let file_path = Path::new(&fixture.path);
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    root.join("target")
        .join("visual-diff")
        .join(&fixture.family)
        .join(format!("{stem}.png.new"))
}

/// Where the diff image is written for inspection on mismatch.
pub(crate) fn diff_png_path(root: &Path, fixture: &Fixture) -> PathBuf {
    let file_path = Path::new(&fixture.path);
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    root.join("target")
        .join("visual-diff")
        .join(&fixture.family)
        .join(format!("{stem}.diff.png"))
}

pub(crate) fn rendered_svg_path(root: &Path, fixture: &Fixture) -> PathBuf {
    let file_path = Path::new(&fixture.path);
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    root.join("target")
        .join("visual-diff")
        .join(&fixture.family)
        .join(format!("{stem}.svg"))
}

#[test]
fn render_svg_uses_stdout_without_mutating_fixture_directory() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let fixture_path = tempdir.path().join("fixture.puml");
    fs::write(
        &fixture_path,
        "@startuml\nAlice -> Bob: hello from stdin\n@enduml\n",
    )
    .expect("write fixture");

    let svg = render_svg(&fixture_path).expect("render svg");

    assert!(svg.contains("<svg"), "rendered stdout should contain SVG");
    assert!(
        svg.contains("hello from stdin"),
        "rendered stdout should contain fixture text"
    );
    assert!(
        !fixture_path.with_extension("svg").exists(),
        "rendering through the visual harness must not create sibling SVGs"
    );
}
