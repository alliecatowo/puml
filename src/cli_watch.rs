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

    eprintln!("watching {} for changes… (Ctrl-C to stop)", path.display());

    // Note: we leak the file handle on Ctrl-C, but the OS will reap it.
    let mut last_mtime: Option<SystemTime> = None;

    loop {
        let meta = fs::metadata(&path)
            .unwrap_or_else(|e| panic!("watch target disappeared: {:?} — {e}", path));

        let new_mtime = meta
            .modified()
            .map_err(|e| format!("cannot read mtime for '{}': {e}", path.display()))?;

        let changed = match last_mtime {
            None => true,
            Some(prev) => new_mtime >= prev,
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
        fs::read_to_string(path_str).map_err(|e| format!("failed to read '{path_str}': {e}"))?;

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

    fs::write(&out_path, svg_pages[0].as_bytes())
        .map_err(|e| format!("failed to write '{}': {e}", out_path.display()))?;

    Ok(())
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
