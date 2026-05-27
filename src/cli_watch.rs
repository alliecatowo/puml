//! Watch mode: re-render a `.puml` file whenever its mtime changes.
//!
//! Invoked when `--watch` is passed on the CLI. Polls the file's metadata
//! on a fixed interval and re-invokes the render path on each detected change.

use crate::cli::{Cli, OutputFormat};
use puml::output::{
    render_artifact_export_content, render_artifact_output_bytes, RenderArtifactOutputMetadata,
    RenderedArtifactOutput,
};
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
        include_root,
        allow_url_includes: cli.allow_url_includes,
        inject_vars,
    };

    let doc = puml::parse_with_pipeline_options(&raw, &options).map_err(|d| d.message.clone())?;
    let model = puml::normalize_family(doc).map_err(|d| d.message.clone())?;

    let artifacts = puml::render_artifact_pages_from_model(&model);
    let Some(first_artifact) = artifacts.first() else {
        return Err("renderer produced no output pages".to_string());
    };

    let out_bytes = watch_output_bytes(first_artifact, cli.format, cli.dpi)?;

    // Determine output path: explicit --output or derive from input stem.
    let out_path = match &cli.output {
        Some(p) => p.clone(),
        None => {
            let p = cli.input.as_ref().unwrap();
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("diagram");
            let ext = cli.format.extension();
            p.with_file_name(format!("{stem}.{ext}"))
        }
    };

    fs::write(&out_path, &out_bytes)
        .map_err(|e| format!("failed to write \'{}\': {e}", out_path.display()))?;

    Ok(())
}

/// Convert a render artifact to watch-mode output bytes.
///
/// Watch mode keeps its existing supported-format policy, but delegates actual
/// SVG/HTML/raster conversion to the shared output backend.
fn watch_output_bytes(
    artifact: &puml::render::RenderArtifact,
    format: OutputFormat,
    dpi: f32,
) -> Result<Vec<u8>, String> {
    match format {
        OutputFormat::Svg
        | OutputFormat::Html
        | OutputFormat::Png
        | OutputFormat::Jpg
        | OutputFormat::Webp => {
            let output = RenderedArtifactOutput {
                name_hint: None,
                content: render_artifact_export_content(artifact, format),
                artifact: Some(RenderArtifactOutputMetadata::from_artifact(artifact)),
            };
            render_artifact_output_bytes(&output, format, dpi)
                .map(|output| output.bytes)
                .map_err(|err| err.message().to_string())
        }

        OutputFormat::Pdf => Err(
            "--watch does not yet support --format pdf; use --format svg or --format png"
                .to_string(),
        ),

        OutputFormat::Txt | OutputFormat::Atxt | OutputFormat::Utxt => Err(format!(
            "--watch does not support text output (--format {}); use svg or png",
            format.extension()
        )),
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
        assert_eq!(OutputFormat::Svg.extension(), "svg");
        assert_eq!(OutputFormat::Html.extension(), "html");
        assert_eq!(OutputFormat::Png.extension(), "png");
        assert_eq!(OutputFormat::Jpg.extension(), "jpg");
        assert_eq!(OutputFormat::Webp.extension(), "webp");
        assert_eq!(OutputFormat::Pdf.extension(), "pdf");
        assert_eq!(OutputFormat::Txt.extension(), "txt");
        assert_eq!(OutputFormat::Atxt.extension(), "atxt");
        assert_eq!(OutputFormat::Utxt.extension(), "utxt");

        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"/>";
        let artifact = puml::render::RenderArtifact::svg_only(svg.to_string());
        assert_eq!(
            watch_output_bytes(&artifact, OutputFormat::Svg, 96.0).unwrap(),
            svg.as_bytes()
        );
        let html = watch_output_bytes(&artifact, OutputFormat::Html, 96.0).unwrap();
        assert!(String::from_utf8(html)
            .unwrap()
            .starts_with("<!doctype html>"));
        assert!(watch_output_bytes(&artifact, OutputFormat::Pdf, 96.0)
            .unwrap_err()
            .contains("--watch does not yet support --format pdf"));
        assert!(watch_output_bytes(&artifact, OutputFormat::Txt, 96.0)
            .unwrap_err()
            .contains("--watch does not support text output"));
    }

    #[test]
    fn watch_raster_output_helpers_encode_supported_image_formats() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="2">
  <rect width="2" height="2" fill="#ffffff"/>
  <rect width="1" height="1" fill="#000000"/>
</svg>"##;

        let artifact = puml::render::RenderArtifact::svg_only(svg.to_string());

        let png = watch_output_bytes(&artifact, OutputFormat::Png, 96.0).expect("png bytes");
        assert!(png.starts_with(&[0x89, b'P', b'N', b'G']));

        let jpg = watch_output_bytes(&artifact, OutputFormat::Jpg, 96.0).expect("jpg bytes");
        assert!(jpg.starts_with(&[0xff, 0xd8]));

        let webp = watch_output_bytes(&artifact, OutputFormat::Webp, 96.0).expect("webp bytes");
        assert!(webp.starts_with(b"RIFF"));
        assert_eq!(&webp[8..12], b"WEBP");
    }

    #[test]
    fn render_once_reports_read_errors_without_panicking() {
        let tmp = tempdir().unwrap();
        let missing = tmp.path().join("missing.puml");
        let cli = Cli::try_parse_from(["puml", "--watch", missing.to_str().unwrap()])
            .expect("watch input should parse");

        let err = render_once(&cli, missing.to_str().unwrap()).expect_err("missing file");

        assert!(err.contains("failed to read"));
        assert!(err.contains("missing.puml"));
    }

    #[test]
    fn rasterize_rejects_degenerate_svg_dimensions() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="6">
  <rect x="0" y="0" width="8" height="6" fill="#fff"/>
</svg>"##;

        let artifact = puml::render::RenderArtifact::svg_only(svg.to_string());
        let err =
            watch_output_bytes(&artifact, OutputFormat::Png, 0.0).expect_err("degenerate output");

        assert!(err.contains("failed to rasterize PNG"));
    }

    #[test]
    fn chrono_hms_uses_fixed_width_time_fields() {
        let value = chrono_hms();

        assert_eq!(value.len(), 8);
        assert_eq!(value.as_bytes()[2], b':');
        assert_eq!(value.as_bytes()[5], b':');
        assert_eq!(value.chars().filter(|ch| ch.is_ascii_digit()).count(), 6);
    }
}
