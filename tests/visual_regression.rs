//! Visual regression smoke tests.
//!
//! Renders each fixture in `tests/visual_regression/manifest.json` to SVG via
//! the `puml` CLI and asserts (1) no empty `<text>` elements, (2) all
//! `expected_text` substrings appear, (3) at least `min_text_elements`
//! non-empty `<text>` elements are emitted.
//!
//! Also provides PNG baseline-diff sweeps and a bless mechanism
//! (`bless_baselines`) for promoting renders to baselines after intentional
//! changes.
//!
//! Catches the family of bugs where the renderer drops text labels.
//! See `tests/visual_regression/README.md`.

use assert_cmd::Command;
use image::ImageEncoder;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

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
    #[serde(default)]
    structural_only_reason: Option<String>,
}

fn load_manifest() -> Manifest {
    let raw = include_str!("visual_regression/manifest.json");
    serde_json::from_str(raw).expect("manifest.json must be valid JSON")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ---------------------------------------------------------------------------
// SVG rendering via puml CLI
// ---------------------------------------------------------------------------

fn render_svg(fixture_path: &Path) -> Result<String, String> {
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

fn svg_shape_failures(svg: &str) -> Vec<String> {
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

// ---------------------------------------------------------------------------
// PNG rasterisation (resvg + tiny-skia, same chain as the CLI uses)
// ---------------------------------------------------------------------------

/// Fixed DPI for baseline PNG rasterisation. Must stay constant so regenerated
/// baselines are stable for a given renderer/font stack.
const BASELINE_DPI: f32 = 96.0;

/// Maximum width (px) for baseline PNGs. The SVG viewBox is scaled to fit
/// within this width so that git history stays small and diffs are readable.
const MAX_BASELINE_WIDTH_PX: u32 = 640;

/// Per-channel RGBA absolute-delta threshold for the pixel-diff comparison.
/// Each channel of each pixel must differ by no more than this value for the
/// test to pass. 0 = byte-perfect. Small values (~3) allow for sub-pixel
/// anti-aliasing differences that can occur between machines or resvg
/// versions while still catching real layout regressions.
const PIXEL_DIFF_THRESHOLD: u8 = 3;

/// Rasterise an SVG string to raw RGBA bytes at `BASELINE_DPI`, scaling the
/// image down if it would exceed `MAX_BASELINE_WIDTH_PX`.
///
/// Returns `(width, height, rgba_bytes)`.
fn svg_to_rgba(svg: &str) -> Result<(u32, u32, Vec<u8>), String> {
    let mut opt = resvg::usvg::Options::default();
    let fontdb = opt.fontdb_mut();
    fontdb.load_system_fonts();
    fontdb.set_monospace_family("Liberation Mono");
    let tree =
        resvg::usvg::Tree::from_str(svg, &opt).map_err(|e| format!("usvg parse failed: {e}"))?;

    let size = tree.size();
    let natural_w = (size.width() * (BASELINE_DPI / 96.0)).round() as u32;
    let natural_h = (size.height() * (BASELINE_DPI / 96.0)).round() as u32;
    if natural_w == 0 || natural_h == 0 {
        return Err("SVG has zero-size viewport".into());
    }

    // Scale down so baseline PNGs stay small.
    let scale = if natural_w > MAX_BASELINE_WIDTH_PX {
        MAX_BASELINE_WIDTH_PX as f32 / natural_w as f32
    } else {
        1.0_f32
    } * (BASELINE_DPI / 96.0);

    let width = (size.width() * scale).round().max(1.0) as u32;
    let height = (size.height() * scale).round().max(1.0) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| format!("failed to allocate pixmap {width}x{height}"))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok((width, height, pixmap.data().to_vec()))
}

/// Encode raw RGBA bytes to an in-memory PNG.
fn rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    image::codecs::png::PngEncoder::new(&mut buf)
        .write_image(rgba, width, height, image::ColorType::Rgba8.into())
        .map_err(|e| format!("PNG encode failed: {e}"))?;
    Ok(buf)
}

/// Decode a PNG file to `(width, height, rgba_bytes)`.
fn load_png(path: &Path) -> Result<(u32, u32, Vec<u8>), String> {
    let file_bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let img = image::load_from_memory_with_format(&file_bytes, image::ImageFormat::Png)
        .map_err(|e| format!("decode PNG {}: {e}", path.display()))?
        .to_rgba8();
    let (w, h) = img.dimensions();
    Ok((w, h, img.into_raw()))
}

// ---------------------------------------------------------------------------
// Pixel-diff helpers
// ---------------------------------------------------------------------------

/// Compare two RGBA buffers of identical dimensions. Returns the number of
/// pixels that exceeded `PIXEL_DIFF_THRESHOLD` in any channel, and also
/// writes a diff PNG where differing pixels are painted bright red.
fn pixel_diff(width: u32, height: u32, actual: &[u8], baseline: &[u8]) -> (u32, Vec<u8>) {
    assert_eq!(actual.len(), baseline.len());
    assert_eq!(actual.len(), (width * height * 4) as usize);

    let mut diff_rgba = vec![0u8; actual.len()];
    let mut differing_pixels: u32 = 0;

    for px in 0..(width * height) as usize {
        let base = px * 4;
        let ar = actual[base];
        let ag = actual[base + 1];
        let ab = actual[base + 2];
        let aa = actual[base + 3];

        let br = baseline[base];
        let bg = baseline[base + 1];
        let bb = baseline[base + 2];
        let ba = baseline[base + 3];

        let max_delta = [
            ar.abs_diff(br),
            ag.abs_diff(bg),
            ab.abs_diff(bb),
            aa.abs_diff(ba),
        ]
        .into_iter()
        .max()
        .unwrap_or(0);

        if max_delta > PIXEL_DIFF_THRESHOLD {
            differing_pixels += 1;
            // Paint differing pixels bright red so they're easy to spot.
            diff_rgba[base] = 255;
            diff_rgba[base + 1] = 0;
            diff_rgba[base + 2] = 0;
            diff_rgba[base + 3] = 255;
        } else {
            // Dim identical pixels so the red pops.
            diff_rgba[base] = ar / 3;
            diff_rgba[base + 1] = ag / 3;
            diff_rgba[base + 2] = ab / 3;
            diff_rgba[base + 3] = aa;
        }
    }
    (differing_pixels, diff_rgba)
}

// ---------------------------------------------------------------------------
// Baseline path helpers
// ---------------------------------------------------------------------------

/// Where a fixture's baseline PNG lives (committed to git).
fn baseline_png_path(root: &Path, fixture: &Fixture) -> PathBuf {
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
fn rendered_png_path(root: &Path, fixture: &Fixture) -> PathBuf {
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
fn diff_png_path(root: &Path, fixture: &Fixture) -> PathBuf {
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

fn rendered_svg_path(root: &Path, fixture: &Fixture) -> PathBuf {
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

// ---------------------------------------------------------------------------
// SVG text extraction (unchanged from PR #249)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Text-content sweep (from PR #249, unchanged)
// ---------------------------------------------------------------------------

struct Failure {
    fixture: String,
    reasons: Vec<String>,
}

struct FocusedTextFixture {
    path: &'static str,
    required_text: &'static [&'static str],
}

const NO_FOCUSED_TEXT_REQUIREMENTS: &[&str] = &[];

const FOCUSED_TEXT_SWEEP_FIXTURES: &[FocusedTextFixture] = &[
    FocusedTextFixture {
        path: "docs/examples/sequence/01_basic.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
    FocusedTextFixture {
        path: "docs/examples/sequence/05_alt_opt_loop.puml",
        required_text: &[
            "alt credentials valid",
            "invalid",
            "opt remember me",
            "loop 3 times",
            "authenticate",
            "token",
            "401 Unauthorized",
            "store session",
            "heartbeat",
            "pong",
        ],
    },
    FocusedTextFixture {
        path: "docs/examples/class/01_basic.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
    FocusedTextFixture {
        path: "docs/examples/class/02_inheritance.puml",
        required_text: &[
            "Vehicle",
            "Car",
            "Truck",
            "+make: String",
            "+model: String",
            "+start()",
            "+doors: Int",
            "+drive()",
            "+payload: Float",
            "+haul()",
        ],
    },
    FocusedTextFixture {
        path: "docs/examples/activity/01_simple_flow.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
    FocusedTextFixture {
        path: "docs/examples/activity/02_if_then_else.puml",
        required_text: &[
            "If-Then-Else Decision",
            "Receive Request",
            "authenticated? / yes",
            "(else) no",
            "Process",
            "Return 200",
            "Return 401",
            "(endif)",
        ],
    },
    FocusedTextFixture {
        path: "docs/examples/state/01_basic.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
    FocusedTextFixture {
        path: "docs/examples/sdl/02_with_transitions.puml",
        required_text: NO_FOCUSED_TEXT_REQUIREMENTS,
    },
];

const FAST_VISUAL_SMOKE_FIXTURES: &[&str] = &[
    "docs/examples/sequence/01_basic.puml",
    "docs/examples/sequence/05_alt_opt_loop.puml",
    "docs/examples/class/02_inheritance.puml",
    "docs/examples/activity/02_if_then_else.puml",
    "docs/examples/state/01_basic.puml",
    "docs/examples/component/01_basic.puml",
    "docs/examples/deployment/01_nodes.puml",
    "docs/examples/gantt/01_basic.puml",
    "docs/examples/mindmap/01_basic.puml",
    "docs/examples/wbs/01_basic.puml",
    "docs/examples/c4/01_context.puml",
    "docs/examples/chart/01_bar.puml",
];

fn check_fixture(fixture: &Fixture) -> Option<Failure> {
    check_fixture_with_required_text(fixture, NO_FOCUSED_TEXT_REQUIREMENTS)
}

fn check_fixture_with_required_text(
    fixture: &Fixture,
    focused_required_text: &[&str],
) -> Option<Failure> {
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
    let artifact_path = rendered_svg_path(&root, fixture);
    if let Some(diff_dir) = artifact_path.parent() {
        let _ = fs::create_dir_all(diff_dir);
    }
    let _ = fs::write(&artifact_path, &svg);

    let mut reasons = svg_shape_failures(&svg);
    let texts = extract_text_contents(&svg);

    // Check 1: no empty <text> elements (the missing-label bug class).
    let empty_count = texts.iter().filter(|t| t.is_empty()).count();
    if empty_count > 0 {
        reasons.push(format!(
            "found {} empty `<text>` element(s); rendered {} non-empty out of {} total. \
             This is the missing-label bug class. Inspect the SVG at {}",
            empty_count,
            texts.len() - empty_count,
            texts.len(),
            artifact_path.display(),
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
    for expected in focused_required_text {
        if !joined.contains(expected) {
            reasons.push(format!(
                "focused sweep expected text {:?} not found in any <text> element",
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

fn run_text_sweep<'a>(fixtures: impl IntoIterator<Item = &'a Fixture>, total: usize) {
    let mut failures: Vec<Failure> = Vec::new();
    for fixture in fixtures {
        if let Some(f) = check_fixture(fixture) {
            failures.push(f);
        }
    }
    if !failures.is_empty() {
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
fn manifest_requires_semantic_text_expectations_or_explicit_exception() {
    let manifest = load_manifest();
    let weak_fixtures = manifest
        .fixtures
        .iter()
        .filter(|fixture| {
            let has_exception = fixture
                .structural_only_reason
                .as_deref()
                .is_some_and(|reason| !reason.trim().is_empty());
            let has_blank_expected_text = fixture
                .expected_text
                .iter()
                .any(|expected| expected.trim().is_empty());

            has_blank_expected_text
                || (!has_exception
                    && (fixture.expected_text.is_empty() || fixture.min_text_elements == 0))
        })
        .map(|fixture| fixture.path.as_str())
        .collect::<Vec<_>>();

    assert!(
        weak_fixtures.is_empty(),
        "visual manifest fixtures must assert semantic expected_text and nonzero \
         min_text_elements, or include non-empty structural_only_reason for \
         machine/structural-only exceptions: {weak_fixtures:#?}"
    );
}

#[test]
fn visual_regression_focused_text_presence_sweep() {
    let manifest = load_manifest();
    let mut failures: Vec<Failure> = Vec::new();

    for focused_fixture in FOCUSED_TEXT_SWEEP_FIXTURES {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.path == focused_fixture.path)
            .unwrap_or_else(|| {
                panic!(
                    "focused visual text sweep fixture {} must exist in manifest",
                    focused_fixture.path
                )
            });
        if let Some(failure) =
            check_fixture_with_required_text(fixture, focused_fixture.required_text)
        {
            failures.push(failure);
        }
    }

    if !failures.is_empty() {
        let total = FOCUSED_TEXT_SWEEP_FIXTURES.len();
        let mut report = String::new();
        report.push_str(&format!(
            "\nFocused visual regression: {}/{} fixtures failed\n",
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
fn visual_smoke_representative_docs_examples_matrix() {
    let manifest = load_manifest();
    let mut fixtures = Vec::new();

    for path in FAST_VISUAL_SMOKE_FIXTURES {
        let fixture = manifest
            .fixtures
            .iter()
            .find(|fixture| fixture.path == *path)
            .unwrap_or_else(|| panic!("fast visual smoke fixture {path} must exist in manifest"));
        fixtures.push(fixture);
    }

    run_text_sweep(fixtures, FAST_VISUAL_SMOKE_FIXTURES.len());
}

#[test]
fn visual_regression_all_fixtures() {
    let manifest = load_manifest();
    run_text_sweep(manifest.fixtures.iter(), manifest.fixtures.len());
}

// ---------------------------------------------------------------------------
// PNG baseline diff sweep
// ---------------------------------------------------------------------------

/// Check one fixture against its stored PNG baseline.
///
/// Returns `None` on pass, or a `Failure` with actionable messages on diff.
fn check_png_fixture(fixture: &Fixture) -> Option<Failure> {
    let root = workspace_root();
    let path = root.join(&fixture.path);
    if !path.exists() {
        return Some(Failure {
            fixture: fixture.path.clone(),
            reasons: vec![format!("fixture file not found: {}", path.display())],
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

    let (width, height, rgba) = match svg_to_rgba(&svg) {
        Ok(r) => r,
        Err(e) => {
            return Some(Failure {
                fixture: fixture.path.clone(),
                reasons: vec![format!("rasterise failed: {e}")],
            });
        }
    };

    let baseline_path = baseline_png_path(&root, fixture);
    if !baseline_path.exists() {
        // No baseline yet — this is expected before `bless_baselines` is run.
        // Write the current render to target/visual-diff so developers can
        // inspect it and then bless it.
        let rendered_path = rendered_png_path(&root, fixture);
        let diff_dir = rendered_path.parent().unwrap();
        let _ = fs::create_dir_all(diff_dir);
        if let Ok(png) = rgba_to_png(width, height, &rgba) {
            let _ = fs::write(&rendered_path, &png);
        }
        return Some(Failure {
            fixture: fixture.path.clone(),
            reasons: vec![format!(
                "no baseline PNG at {} — run `cargo test --test visual_regression \
                 bless_baselines -- --ignored` to bless the current render as the baseline. \
                 The rendered PNG is at {}",
                baseline_path.display(),
                rendered_path.display(),
            )],
        });
    }

    let (bw, bh, baseline_rgba) = match load_png(&baseline_path) {
        Ok(r) => r,
        Err(e) => {
            return Some(Failure {
                fixture: fixture.path.clone(),
                reasons: vec![format!("failed to load baseline: {e}")],
            });
        }
    };

    if bw != width || bh != height {
        // Dimension mismatch — write the new render for inspection.
        let rendered_path = rendered_png_path(&root, fixture);
        let diff_dir = rendered_path.parent().unwrap();
        let _ = fs::create_dir_all(diff_dir);
        if let Ok(png) = rgba_to_png(width, height, &rgba) {
            let _ = fs::write(&rendered_path, &png);
        }
        return Some(Failure {
            fixture: fixture.path.clone(),
            reasons: vec![format!(
                "PNG dimensions changed: baseline is {}x{}, render is {}x{}. \
                 Inspect the new render at {}. \
                 If the change is intentional, run the bless command.",
                bw,
                bh,
                width,
                height,
                rendered_path.display(),
            )],
        });
    }

    let (differing_pixels, diff_rgba) = pixel_diff(width, height, &rgba, &baseline_rgba);
    if differing_pixels == 0 {
        return None; // All good.
    }

    // Write artefacts for inspection.
    let rendered_path = rendered_png_path(&root, fixture);
    let diff_path = diff_png_path(&root, fixture);
    let diff_dir = rendered_path.parent().unwrap();
    let _ = fs::create_dir_all(diff_dir);
    if let Ok(png) = rgba_to_png(width, height, &rgba) {
        let _ = fs::write(&rendered_path, &png);
    }
    if let Ok(png) = rgba_to_png(width, height, &diff_rgba) {
        let _ = fs::write(&diff_path, &png);
    }

    let total_pixels = width * height;
    let pct = differing_pixels as f64 / total_pixels as f64 * 100.0;
    Some(Failure {
        fixture: fixture.path.clone(),
        reasons: vec![format!(
            "{differing_pixels}/{total_pixels} pixels differ (>{PIXEL_DIFF_THRESHOLD} delta, \
             {pct:.2}%). \
             Rendered PNG: {}  Diff PNG (red = changed): {}. \
             If intentional, run the bless command to promote the new render.",
            rendered_path.display(),
            diff_path.display(),
        )],
    })
}

fn run_png_sweep<'a>(label: &str, fixtures: impl IntoIterator<Item = &'a Fixture>, total: usize) {
    let mut failures: Vec<Failure> = Vec::new();
    for fixture in fixtures {
        if let Some(f) = check_png_fixture(fixture) {
            failures.push(f);
        }
    }
    if !failures.is_empty() {
        let mut report = format!("\n{label}: {}/{} fixtures failed\n", failures.len(), total);
        for f in &failures {
            report.push_str(&format!("\n  FIXTURE: {}\n", f.fixture));
            for r in &f.reasons {
                report.push_str(&format!("    - {}\n", r));
            }
        }
        report.push_str(
            "\nTo bless changed renders as new baselines (after verifying the\n\
             changes are intentional):\n\n  \
             cargo test --test visual_regression bless_baselines -- --ignored\n\n\
             Diff artefacts are written to target/visual-diff/. On PR Gate, \
             download the pr-visual-smoke-<run_number> artifact from the \
             visual smoke fixture matrix job. See \
             tests/visual_regression/README.md for full workflow.\n",
        );
        panic!("{report}");
    }
}

/// Compare every reviewed PNG baseline currently committed to git.
#[test]
fn png_regression_committed_baselines() {
    let manifest = load_manifest();
    let root = workspace_root();
    let fixtures = manifest
        .fixtures
        .iter()
        .filter(|fixture| baseline_png_path(&root, fixture).exists())
        .collect::<Vec<_>>();
    let total = fixtures.len();

    assert!(
        total > 0,
        "visual regression should include at least one committed PNG baseline; \
         run `cargo test --test visual_regression bless_baselines -- --ignored` \
         and commit a reviewed baseline from tests/visual_baselines/"
    );

    run_png_sweep("Committed PNG regression", fixtures, total);
}

/// PNG perceptual baseline sweep.
///
/// For every fixture in `manifest.json`:
///   1. Render SVG via `puml`.
///   2. Rasterise to PNG at 96 DPI (scaled to ≤640 px wide).
///   3. Load the stored baseline from `tests/visual_baselines/<family>/<fixture>.png`.
///   4. Run a per-pixel RGBA diff with threshold `PIXEL_DIFF_THRESHOLD`.
///   5. On any mismatch, write `target/visual-diff/<family>/<fixture>.png.new`
///      (current render) and `<fixture>.diff.png` (diff overlay, changed
///      pixels in red).
///
/// This runs by default because every current manifest fixture has a reviewed
/// PNG baseline. Keep it unignored when adding new manifest fixtures: either
/// commit the matching reviewed baseline in the same change, or split the new
/// fixture into a text-only manifest change once it has a documented reason.
#[test]
fn png_regression_all_fixtures() {
    let manifest = load_manifest();
    run_png_sweep(
        "PNG regression",
        manifest.fixtures.iter(),
        manifest.fixtures.len(),
    );
}

// ---------------------------------------------------------------------------
// Bless workflow
// ---------------------------------------------------------------------------

/// Bless (promote) current renders as new PNG baselines.
///
/// Run this test explicitly when you have intentionally changed the renderer's
/// output (skinparam tweak, layout fix, new feature, etc.) and want to update
/// the stored baselines so the PNG regression sweep does not flag the change
/// on subsequent runs.
///
/// Command:
///   cargo test --test visual_regression bless_baselines -- --ignored
///
/// The command re-renders every fixture in `manifest.json`, writes the PNG to
/// `tests/visual_baselines/<family>/<fixture>.png`, and reports what changed.
/// You should then:
///   1. Review the new baseline PNGs (they're committed to git so PR diffs
///      show the visual change).
///   2. `git add tests/visual_baselines/`
///   3. Commit and open a PR explaining why the visual output changed.
///
/// The test is `#[ignore]` so it never runs automatically — you must pass
/// `-- --ignored` (or `bless_baselines -- --ignored`) explicitly.
#[test]
#[ignore]
fn bless_baselines() {
    let manifest = load_manifest();
    let root = workspace_root();
    let mut blessed = 0u32;
    let mut failed = 0u32;
    let mut report = String::from("\nBless baselines\n");
    report.push_str(&format!(
        "  Threshold: {} per-channel delta\n",
        PIXEL_DIFF_THRESHOLD
    ));
    report.push_str(&format!("  Max width: {} px\n", MAX_BASELINE_WIDTH_PX));
    report.push_str(&format!("  DPI: {}\n\n", BASELINE_DPI));

    for fixture in &manifest.fixtures {
        let path = root.join(&fixture.path);
        if !path.exists() {
            report.push_str(&format!("  SKIP (fixture not found): {}\n", fixture.path));
            failed += 1;
            continue;
        }

        let svg = match render_svg(&path) {
            Ok(s) => s,
            Err(e) => {
                report.push_str(&format!("  FAIL (render error): {} — {e}\n", fixture.path));
                failed += 1;
                continue;
            }
        };

        let (width, height, rgba) = match svg_to_rgba(&svg) {
            Ok(r) => r,
            Err(e) => {
                report.push_str(&format!(
                    "  FAIL (rasterise error): {} — {e}\n",
                    fixture.path
                ));
                failed += 1;
                continue;
            }
        };

        let png = match rgba_to_png(width, height, &rgba) {
            Ok(p) => p,
            Err(e) => {
                report.push_str(&format!("  FAIL (encode error): {} — {e}\n", fixture.path));
                failed += 1;
                continue;
            }
        };

        let baseline_path = baseline_png_path(&root, fixture);
        if let Some(parent) = baseline_path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                report.push_str(&format!("  FAIL (mkdir {}): {e}\n", parent.display()));
                failed += 1;
                continue;
            }
        }

        // Check if this is a new baseline or an update.
        let action = if baseline_path.exists() {
            "updated"
        } else {
            "created"
        };

        if let Err(e) = fs::write(&baseline_path, &png) {
            report.push_str(&format!(
                "  FAIL (write {}): {e}\n",
                baseline_path.display()
            ));
            failed += 1;
            continue;
        }

        report.push_str(&format!(
            "  OK ({action} {width}x{height}px): {}\n",
            baseline_path.display()
        ));
        blessed += 1;
    }

    report.push_str(&format!(
        "\nBlessed {blessed}/{} baselines",
        manifest.fixtures.len()
    ));
    if failed > 0 {
        report.push_str(&format!(", {failed} failed"));
    }
    report.push_str(
        ".\n\nNext steps:\n  \
         git add tests/visual_baselines/\n  \
         git commit -m \"test: bless PNG baselines after <describe change>\"\n\
         Then open a PR so the visual diff is reviewable.\n",
    );

    // Always print the report (even on success) so the developer sees what changed.
    println!("{report}");
    if failed > 0 {
        panic!("bless_baselines: {failed} fixture(s) could not be rendered — see report above.");
    }
}

// ---------------------------------------------------------------------------
// Unit tests for SVG text extraction helpers (run by default, no #[ignore])
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Unit tests for PNG diffing helpers (run by default, no #[ignore])
// ---------------------------------------------------------------------------

#[test]
fn pixel_diff_identical_images_pass() {
    let rgba = vec![128u8, 64, 200, 255, 10, 20, 30, 255];
    let (differing, _) = pixel_diff(2, 1, &rgba, &rgba);
    assert_eq!(
        differing, 0,
        "identical RGBA should report 0 differing pixels"
    );
}

#[test]
fn pixel_diff_within_threshold_pass() {
    let actual = vec![100u8, 100, 100, 255];
    let baseline = vec![100u8 + PIXEL_DIFF_THRESHOLD, 100, 100, 255];
    let (differing, _) = pixel_diff(1, 1, &actual, &baseline);
    assert_eq!(differing, 0, "delta == threshold should still pass");
}

#[test]
fn pixel_diff_above_threshold_fails() {
    let actual = vec![100u8, 100, 100, 255];
    let baseline = vec![100u8 + PIXEL_DIFF_THRESHOLD + 1, 100, 100, 255];
    let (differing, diff_rgba) = pixel_diff(1, 1, &actual, &baseline);
    assert_eq!(differing, 1, "delta > threshold should count as differing");
    // Differing pixel should be painted red.
    assert_eq!(
        diff_rgba[0], 255,
        "red channel should be 255 for differing pixel"
    );
    assert_eq!(
        diff_rgba[1], 0,
        "green channel should be 0 for differing pixel"
    );
    assert_eq!(
        diff_rgba[2], 0,
        "blue channel should be 0 for differing pixel"
    );
}

#[test]
fn svg_to_rgba_produces_deterministic_output() {
    // A trivial SVG — we just check that two calls return identical bytes.
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="blue"/></svg>"#;
    let (w1, h1, rgba1) = svg_to_rgba(svg).expect("rasterise call 1");
    let (w2, h2, rgba2) = svg_to_rgba(svg).expect("rasterise call 2");
    assert_eq!((w1, h1), (w2, h2), "dimensions must be deterministic");
    assert_eq!(rgba1, rgba2, "pixel data must be deterministic");
}

#[test]
fn svg_to_rgba_renders_text_pixels() {
    let with_text = r#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="50"><rect width="120" height="50" fill="white"/><text x="8" y="30" font-family="monospace" font-size="20" fill="black">Text</text></svg>"#;
    let without_text = r#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="50"><rect width="120" height="50" fill="white"/></svg>"#;

    let (text_w, text_h, text_rgba) = svg_to_rgba(with_text).expect("rasterise with text");
    let (blank_w, blank_h, blank_rgba) = svg_to_rgba(without_text).expect("rasterise blank");

    assert_eq!((text_w, text_h), (blank_w, blank_h));
    assert_ne!(
        text_rgba, blank_rgba,
        "rasterized text should change output pixels"
    );
}

#[test]
fn png_roundtrip_preserves_dimensions() {
    let width = 4u32;
    let height = 2u32;
    let rgba: Vec<u8> = (0..(width * height * 4)).map(|i| (i % 256) as u8).collect();
    let png = rgba_to_png(width, height, &rgba).expect("encode");
    // Write to a tempfile and load back.
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    fs::write(tmp.path(), &png).expect("write tempfile");
    let (lw, lh, loaded) = load_png(tmp.path()).expect("load");
    assert_eq!((lw, lh), (width, height));
    assert_eq!(loaded, rgba);
}
