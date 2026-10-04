//! Watch mode: re-render a `.puml` file whenever its mtime changes.
//!
//! Invoked when `--watch` is passed on the CLI. Polls the file's metadata
//! on a fixed interval and re-invokes the render path on each detected change.

use crate::cli::Cli;
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

/// Re-render the file at `path_str` through the same pipeline as a one-shot run
/// (dialect, compat, style, defines, include root, multi-diagram and multi-page
/// output), so watch output never diverges from `puml <file>`.
fn render_once(cli: &Cli, path_str: &str) -> Result<(), String> {
    let mut once = cli.clone();
    once.watch = false;
    once.input = Some(PathBuf::from(path_str));
    if once.output.is_none() && !once.pipe {
        // Derive `<stem>.<ext>` next to the input, as watch always has.
        let p = PathBuf::from(path_str);
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("diagram");
        once.output = Some(p.with_file_name(format!("{stem}.{}", cli.format.extension())));
    }
    crate::cli_run::run(once).map_err(|(_code, msg)| msg)
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
    fn render_once_uses_the_normal_render_path() {
        // Include root, defines and dialect handling now come from the shared
        // one-shot pipeline instead of a divergent copy.
        let tmp = tempdir().unwrap();
        fs::write(tmp.path().join("part.puml"), "Alice -> Bob : included\n").unwrap();
        let input = tmp.path().join("inc.puml");
        fs::write(&input, "@startuml\n!include part.puml\n@enduml\n").unwrap();
        let cli = Cli::try_parse_from(["puml", "--watch", input.to_str().unwrap()]).unwrap();
        render_once(&cli, input.to_str().unwrap()).expect("include render");
        let svg = fs::read_to_string(tmp.path().join("inc.svg")).unwrap();
        assert!(svg.contains("included"));

        let mm = tmp.path().join("flow.mmd");
        fs::write(&mm, "sequenceDiagram\n  Alice->>Bob: hi\n").unwrap();
        let cli = Cli::try_parse_from(["puml", "--watch", mm.to_str().unwrap()]).unwrap();
        render_once(&cli, mm.to_str().unwrap()).expect("mermaid render by extension");
        assert!(fs::read_to_string(tmp.path().join("flow.svg"))
            .unwrap()
            .contains("<svg"));
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
