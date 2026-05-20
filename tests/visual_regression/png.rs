use image::ImageEncoder;
use std::fs;
use std::path::Path;

use crate::harness::{
    baseline_png_path, diff_png_path, render_svg, rendered_png_path, workspace_root, Failure,
};
use crate::manifest::{load_manifest, Fixture};

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

    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| format!("failed to allocate pixmap {width}x{height}"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
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
        let rendered_path = rendered_png_path(&root, fixture);
        let diff_dir = rendered_path.parent().unwrap();
        let _ = fs::create_dir_all(diff_dir);
        if let Ok(png) = rgba_to_png(width, height, &rgba) {
            let _ = fs::write(&rendered_path, &png);
        }
        return Some(Failure {
            fixture: fixture.path.clone(),
            reasons: vec![format!(
                "no baseline PNG at {} - run `cargo test --test visual_regression \
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
        return None;
    }

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
///   2. Rasterise to PNG at 96 DPI (scaled to <= 640 px wide).
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
/// The test is `#[ignore]` so it never runs automatically. You must pass
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
                report.push_str(&format!("  FAIL (render error): {} - {e}\n", fixture.path));
                failed += 1;
                continue;
            }
        };

        let (width, height, rgba) = match svg_to_rgba(&svg) {
            Ok(r) => r,
            Err(e) => {
                report.push_str(&format!(
                    "  FAIL (rasterise error): {} - {e}\n",
                    fixture.path
                ));
                failed += 1;
                continue;
            }
        };

        let png = match rgba_to_png(width, height, &rgba) {
            Ok(p) => p,
            Err(e) => {
                report.push_str(&format!("  FAIL (encode error): {} - {e}\n", fixture.path));
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

    println!("{report}");
    if failed > 0 {
        panic!("bless_baselines: {failed} fixture(s) could not be rendered - see report above.");
    }
}

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
    let tmp = tempfile::NamedTempFile::new().expect("tempfile");
    fs::write(tmp.path(), &png).expect("write tempfile");
    let (lw, lh, loaded) = load_png(tmp.path()).expect("load");
    assert_eq!((lw, lh), (width, height));
    assert_eq!(loaded, rgba);
}
