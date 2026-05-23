//! Watch mode: re-render a `.puml` file whenever its mtime changes.
//!
//! Invoked when `--watch` is passed on the CLI. Polls the file's metadata
//! on a fixed interval and re-invokes the render path on each detected change.

use crate::cli::{Cli, OutputFormat};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Result type alias used throughout this module.
pub type WatchResult = Result<i32, String>;

/// Entry point for watch mode. Loops indefinitely, polling `args.input` for
/// mtime changes and re-rendering on each detected change.
///
/// The caller is responsible for ensuring `args.input` is `Some` before
/// calling this function.
pub fn run_watch(cli: &Cli) -> WatchResult {
    let path: PathBuf = cli
        .input
        .clone()
        .ok_or_else(|| "--watch requires an input file path".to_string())?;

    eprintln!(
        "watching {} for changes\u{2026} (Ctrl-C to stop)",
        path.display()
    );

    // Note: we leak the file handle on Ctrl-C, but the OS will reap it.
    let mut last_mtime: Option<SystemTime> = None;

    loop {
        // Fix #2: log and retry instead of panicking when the file disappears.
        // Atomic-save editors (vim, emacs, many IDEs) briefly unlink the file
        // mid-write; crashing here would abort the watch for a routine save.
        // CLAUDE.md §6: no panic!() on user-observable file conditions.
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("watch: cannot stat \'{}\': {e} — retrying…", path.display());
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
        };

        let new_mtime = meta
            .modified()
            .map_err(|e| format!("cannot read mtime for \'{}\': {e}", path.display()))?;

        // Fix #1: use != not >= so the watcher fires only on actual changes.
        // The None arm handles the initial render on startup; once last_mtime
        // is set to new_mtime, `>=` would be true on every poll tick even when
        // the file has not been touched.
        let changed = match last_mtime {
            None => true,
            Some(prev) => new_mtime != prev,
        };

        if changed {
            last_mtime = Some(new_mtime);

            let path_str = format!("{}", path.display());

            match render_once(cli, &path_str) {
                Ok(()) => {
                    let now = chrono_hms();
                    println!("rendered at {now}  ← {path_str}");
                }
                Err(msg) => {
                    eprintln!("render error: {msg}");
                }
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }
}

/// Re-render the file at `path_str` using the output settings from `cli`.
///
/// Re-reads the file content on each call so that changes are picked up.
fn render_once(cli: &Cli, path_str: &str) -> Result<(), String> {
    use std::collections::BTreeMap;

    let raw =
        fs::read_to_string(path_str).map_err(|e| format!("failed to read \'{path_str}\': {e}"))?;

    let inject_vars: BTreeMap<String, String> = cli.defines.iter().cloned().collect();
    let include_root = cli
        .input
        .as_ref()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    let options = puml::ParsePipelineOptions {
        frontend: puml::FrontendSelection::Auto,
        compat: puml::CompatMode::Strict,
        determinism: puml::DeterminismMode::Strict,
        include_root,
        allow_url_includes: cli.allow_url_includes,
        inject_vars,
    };

    let doc = puml::parse_with_pipeline_options(&raw, &options).map_err(|d| d.message.clone())?;
    let model = puml::normalize_family(doc).map_err(|d| d.message.clone())?;

    let svg_pages = puml::render_svg_pages_from_model(&model);
    if svg_pages.is_empty() {
        return Err("renderer produced no output pages".to_string());
    }

    // Fix #3: convert the SVG to the requested output format rather than always
    // writing raw SVG bytes. PNG/JPG/WebP are rasterised inline; other formats
    // either pass through or return a clear error.
    //
    // TODO: extract a shared `svg_to_output_bytes(svg, format, dpi)` helper out
    // of main.rs::render_output_bytes so this path can be deduplicated.
    let out_bytes = svg_to_output_bytes(&svg_pages[0], cli.format, cli.dpi)?;

    // Determine output path: explicit --output or derive from input stem.
    let out_path = match &cli.output {
        Some(p) => p.clone(),
        None => {
            let p = cli.input.as_ref().unwrap();
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("diagram");
            let ext = format_extension(cli.format);
            p.with_file_name(format!("{stem}.{ext}"))
        }
    };

    fs::write(&out_path, &out_bytes)
        .map_err(|e| format!("failed to write \'{}\': {e}", out_path.display()))?;

    Ok(())
}

/// Convert an SVG string to output bytes in the requested format.
///
/// PNG, JPG, and WebP are rasterised via `resvg`. SVG/HTML pass through as
/// UTF-8 bytes. PDF and text formats return a clear unsupported error rather
/// than silently writing a corrupt file.
///
/// TODO: once `main.rs::render_output_bytes` is extracted into a shared
/// helper, replace this function with a call to that helper.
fn svg_to_output_bytes(svg: &str, format: OutputFormat, dpi: f32) -> Result<Vec<u8>, String> {
    match format {
        OutputFormat::Svg | OutputFormat::Html => Ok(svg.as_bytes().to_vec()),

        OutputFormat::Png | OutputFormat::Jpg | OutputFormat::Webp => rasterize(svg, format, dpi),

        OutputFormat::Pdf => Err(
            "--watch does not yet support --format pdf; use --format svg or --format png"
                .to_string(),
        ),

        OutputFormat::Txt | OutputFormat::Atxt | OutputFormat::Utxt => Err(format!(
            "--watch does not support text output (--format {}); use svg or png",
            format_extension(format)
        )),
    }
}

/// Rasterise an SVG string to PNG, JPG, or WebP bytes using `resvg`.
///
/// Mirrors the logic in `main.rs::svg_to_raster_bytes` / `encode_*`.
fn rasterize(svg: &str, format: OutputFormat, dpi: f32) -> Result<Vec<u8>, String> {
    use image::ImageEncoder as _;

    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_str(svg, &opt)
        .map_err(|e| format!("failed to parse SVG for raster output: {e}"))?;

    let scale = dpi / 96.0;
    let size = tree.size();
    let width = (size.width() * scale).ceil() as u32;
    let height = (size.height() * scale).ceil() as u32;

    if width == 0 || height == 0 {
        return Err(format!("degenerate SVG dimensions {width}x{height}"));
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| format!("failed to allocate raster surface {width}x{height}"))?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    match format {
        OutputFormat::Png => {
            let mut buf = Vec::new();
            image::codecs::png::PngEncoder::new(&mut buf)
                .write_image(pixmap.data(), width, height, image::ColorType::Rgba8.into())
                .map_err(|e| format!("PNG encoding failed: {e}"))?;
            Ok(buf)
        }
        OutputFormat::Jpg => {
            // Flatten alpha over white before JPEG encoding (no alpha channel).
            let rgba = pixmap.data();
            let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
            for pixel in rgba.chunks_exact(4) {
                let a = pixel[3] as u16;
                for &c in &pixel[..3] {
                    rgb.push(((c as u16 * a + 255 * (255 - a) + 127) / 255) as u8);
                }
            }
            let mut buf = Vec::new();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90)
                .write_image(&rgb, width, height, image::ColorType::Rgb8.into())
                .map_err(|e| format!("JPEG encoding failed: {e}"))?;
            Ok(buf)
        }
        OutputFormat::Webp => {
            let mut buf = Vec::new();
            image::codecs::webp::WebPEncoder::new_lossless(&mut buf)
                .write_image(pixmap.data(), width, height, image::ColorType::Rgba8.into())
                .map_err(|e| format!("WebP encoding failed: {e}"))?;
            Ok(buf)
        }
        _ => unreachable!("rasterize called with non-raster format"),
    }
}

/// Return a simple `HH:MM:SS` timestamp string for the current local time.
fn chrono_hms() -> String {
    // Use SystemTime directly to avoid pulling in a time-zone dependency.
    use std::time::UNIX_EPOCH;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    format!("{h:02}:{m:02}:{s:02}")
}

fn format_extension(fmt: OutputFormat) -> &'static str {
    match fmt {
        OutputFormat::Svg => "svg",
        OutputFormat::Html => "html",
        OutputFormat::Png => "png",
        OutputFormat::Jpg => "jpg",
        OutputFormat::Webp => "webp",
        OutputFormat::Pdf => "pdf",
        OutputFormat::Txt => "txt",
        OutputFormat::Atxt => "atxt",
        OutputFormat::Utxt => "utxt",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use clap::Parser;
    use tempfile::tempdir;

    #[test]
    fn watch_mode_requires_positional_input_before_looping() {
        let cli = Cli::try_parse_from(["puml", "--watch"]).expect("watch flag should parse");
        let err = run_watch(&cli).expect_err("missing input should fail before polling");

        assert_eq!(err, "--watch requires an input file path");
    }

    #[test]
    fn render_once_writes_default_svg_and_explicit_html_outputs() {
        let tmp = tempdir().unwrap();
        let input = tmp.path().join("watch-me.puml");
        fs::write(&input, "@startuml\nAlice -> Bob : hello\n@enduml\n").unwrap();

        let cli = Cli::try_parse_from(["puml", "--watch", input.to_str().unwrap()])
            .expect("watch input should parse");
        render_once(&cli, input.to_str().unwrap()).expect("svg render should succeed");
        let svg = fs::read_to_string(tmp.path().join("watch-me.svg")).unwrap();
        assert!(svg.contains("<svg"));

        let html = tmp.path().join("watch-me.html");
        let cli = Cli::try_parse_from([
            "puml",
            "--watch",
            "--format",
            "html",
            "--output",
            html.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .expect("watch html output should parse");
        render_once(&cli, input.to_str().unwrap()).expect("html render should succeed");
        let html = fs::read_to_string(html).unwrap();
        assert!(html.contains("<svg"));
    }

    #[test]
    fn watch_output_format_helpers_report_supported_extensions_and_errors() {
        assert_eq!(format_extension(OutputFormat::Svg), "svg");
        assert_eq!(format_extension(OutputFormat::Html), "html");
        assert_eq!(format_extension(OutputFormat::Png), "png");
        assert_eq!(format_extension(OutputFormat::Jpg), "jpg");
        assert_eq!(format_extension(OutputFormat::Webp), "webp");
        assert_eq!(format_extension(OutputFormat::Pdf), "pdf");
        assert_eq!(format_extension(OutputFormat::Txt), "txt");
        assert_eq!(format_extension(OutputFormat::Atxt), "atxt");
        assert_eq!(format_extension(OutputFormat::Utxt), "utxt");

        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"/>";
        assert_eq!(
            svg_to_output_bytes(svg, OutputFormat::Svg, 96.0).unwrap(),
            svg.as_bytes()
        );
        assert!(svg_to_output_bytes(svg, OutputFormat::Pdf, 96.0)
            .unwrap_err()
            .contains("--watch does not yet support --format pdf"));
        assert!(svg_to_output_bytes(svg, OutputFormat::Txt, 96.0)
            .unwrap_err()
            .contains("--watch does not support text output"));
    }

    #[test]
    fn watch_raster_output_helpers_encode_supported_image_formats() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2">
  <rect width="2" height="2" fill="#ffffff"/>
  <rect width="1" height="1" fill="#000000"/>
</svg>"##;

        let png = svg_to_output_bytes(svg, OutputFormat::Png, 96.0).expect("png bytes");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));

        let jpg = svg_to_output_bytes(svg, OutputFormat::Jpg, 96.0).expect("jpg bytes");
        assert!(jpg.starts_with(&[0xff, 0xd8]));

        let webp = svg_to_output_bytes(svg, OutputFormat::Webp, 96.0).expect("webp bytes");
        assert!(webp.starts_with(b"RIFF"));
        assert_eq!(&webp[8..12], b"WEBP");
    }
}
