//! Visual regression smoke tests.
//!
//! Renders each fixture in `tests/visual_regression/manifest.json` to SVG via
//! the `puml` CLI and asserts (1) no empty `<text>` elements, (2) all
//! `expected_text` substrings appear, (3) at least `min_text_elements`
//! non-empty `<text>` elements are emitted.
//!
//! Catches the family of bugs where the renderer drops text labels.
//! See `tests/visual_regression/README.md`.

use assert_cmd::Command;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct Manifest {
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    path: String,
    family: String,
    expected_text: Vec<String>,
    min_text_elements: usize,
}

fn load_manifest() -> Manifest {
    let raw = include_str!("visual_regression/manifest.json");
    serde_json::from_str(raw).expect("manifest.json must be valid JSON")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn render_svg(fixture_path: &Path) -> Result<String, String> {
    let output = Command::cargo_bin("puml")
        .map_err(|e| format!("cargo_bin(puml) failed: {e}"))?
        .arg(fixture_path)
        .arg("--format")
        .arg("svg")
        .arg("--quiet")
        .output()
        .map_err(|e| format!("spawn puml failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "puml exited {:?}; stderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Extract the inner-text of every `<text ...>...</text>` element in the SVG.
/// Nested `<tspan>` and similar tags are stripped so we get the raw text
/// content. Returns one entry per `<text>` element (may be empty string).
fn extract_text_contents(svg: &str) -> Vec<String> {
    let bytes = svg.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = svg[i..].find("<text") {
        let start = i + rel;
        // Require word boundary after "<text" (so we don't match "<textarea>").
        let after = start + b"<text".len();
        if after >= bytes.len() {
            break;
        }
        let next_ch = bytes[after];
        if !(next_ch == b' ' || next_ch == b'\t' || next_ch == b'>' || next_ch == b'/') {
            i = after;
            continue;
        }
        // Find end of opening tag.
        let Some(gt_rel) = svg[after..].find('>') else {
            break;
        };
        let open_end = after + gt_rel + 1;
        // Self-closing `<text ... />` => empty content.
        if bytes[open_end - 2] == b'/' {
            out.push(String::new());
            i = open_end;
            continue;
        }
        // Find matching `</text>`.
        let Some(close_rel) = svg[open_end..].find("</text>") else {
            break;
        };
        let content_end = open_end + close_rel;
        let inner = &svg[open_end..content_end];
        out.push(strip_inner_tags(inner));
        i = content_end + "</text>".len();
    }
    out
}

/// Strip XML tags from the inner content of a `<text>` element, preserving
/// the visible text only.
fn strip_inner_tags(inner: &str) -> String {
    let bytes = inner.as_bytes();
    let mut out = String::with_capacity(inner.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Skip to matching '>'.
            let rest = &inner[i..];
            if let Some(j) = rest.find('>') {
                i += j + 1;
                continue;
            } else {
                break;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    decode_xml_entities(out.trim())
}

fn decode_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#10;", "\n")
}

struct Failure {
    fixture: String,
    reasons: Vec<String>,
}

fn check_fixture(fixture: &Fixture) -> Option<Failure> {
    let root = workspace_root();
    let path = root.join(&fixture.path);
    if !path.exists() {
        return Some(Failure {
            fixture: fixture.path.clone(),
            reasons: vec![format!(
                "fixture file not found: {} (resolve relative to workspace root)",
                path.display()
            )],
        });
    }
    let svg = match render_svg(&path) {
        Ok(s) => s,
        Err(e) => {
            return Some(Failure {
                fixture: fixture.path.clone(),
                reasons: vec![format!("render failed: {e}")],
            });
        }
    };

    // Persist the SVG to target/visual-diff/<family>/<basename>.svg for inspection.
    let diff_dir = root
        .join("target")
        .join("visual-diff")
        .join(&fixture.family);
    let _ = fs::create_dir_all(&diff_dir);
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        let _ = fs::write(diff_dir.join(format!("{stem}.svg")), &svg);
    }

    let mut reasons = Vec::new();
    let texts = extract_text_contents(&svg);

    // Check 1: no empty <text> elements (the missing-label bug class).
    let empty_count = texts.iter().filter(|t| t.is_empty()).count();
    if empty_count > 0 {
        reasons.push(format!(
            "found {} empty `<text>` element(s); rendered {} non-empty out of {} total. \
             This is the missing-label bug class (see #238). \
             Inspect the SVG at target/visual-diff/{}/{}.svg",
            empty_count,
            texts.len() - empty_count,
            texts.len(),
            fixture.family,
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("?"),
        ));
    }

    // Check 2: all expected_text substrings appear somewhere in the rendered text.
    let joined: String = texts.join("\n");
    for expected in &fixture.expected_text {
        if !joined.contains(expected) {
            reasons.push(format!(
                "expected text {:?} not found in any <text> element",
                expected
            ));
        }
    }

    // Check 3: at least min_text_elements non-empty <text> elements.
    let nonempty = texts.iter().filter(|t| !t.is_empty()).count();
    if nonempty < fixture.min_text_elements {
        reasons.push(format!(
            "expected ≥{} non-empty <text> elements, found {}",
            fixture.min_text_elements, nonempty
        ));
    }

    if reasons.is_empty() {
        None
    } else {
        Some(Failure {
            fixture: fixture.path.clone(),
            reasons,
        })
    }
}

// NOTE: This sweep is `#[ignore]` until the missing-label renderer bug
// (#238) is fixed — running it on main today would fail every fixture.
// Once #238 lands, remove the `#[ignore]` so this test guards the
// regression in CI. To run locally regardless:
//   cargo test --test visual_regression -- --ignored
#[test]
#[ignore]
fn visual_regression_all_fixtures() {
    let manifest = load_manifest();
    let mut failures: Vec<Failure> = Vec::new();
    for fixture in &manifest.fixtures {
        if let Some(f) = check_fixture(fixture) {
            failures.push(f);
        }
    }
    if !failures.is_empty() {
        let total = manifest.fixtures.len();
        let mut report = String::new();
        report.push_str(&format!(
            "\nVisual regression: {}/{} fixtures failed\n",
            failures.len(),
            total
        ));
        for f in &failures {
            report.push_str(&format!("\n  FIXTURE: {}\n", f.fixture));
            for r in &f.reasons {
                report.push_str(&format!("    - {}\n", r));
            }
        }
        report.push_str(
            "\nRendered SVGs are written to target/visual-diff/<family>/<fixture>.svg\n\
             for inspection. See tests/visual_regression/README.md for how to add\n\
             or update fixtures.\n",
        );
        panic!("{report}");
    }
}

#[test]
fn text_extractor_handles_nested_tspan() {
    let svg = r#"<svg><text x="0" y="0"><tspan>Hello</tspan> <tspan>World</tspan></text></svg>"#;
    let texts = extract_text_contents(svg);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "Hello World");
}

#[test]
fn text_extractor_handles_self_closing() {
    let svg = r#"<svg><text/><text x="0">Visible</text></svg>"#;
    let texts = extract_text_contents(svg);
    assert_eq!(texts.len(), 2);
    assert_eq!(texts[0], "");
    assert_eq!(texts[1], "Visible");
}

#[test]
fn text_extractor_ignores_textarea() {
    let svg = r#"<svg><textarea>noise</textarea><text x="0">Real</text></svg>"#;
    let texts = extract_text_contents(svg);
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0], "Real");
}

#[test]
fn text_extractor_decodes_entities() {
    let svg = r#"<svg><text>Foo &amp; Bar &lt;baz&gt;</text></svg>"#;
    let texts = extract_text_contents(svg);
    assert_eq!(texts[0], "Foo & Bar <baz>");
}
