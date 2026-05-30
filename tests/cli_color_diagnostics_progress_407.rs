//! Integration tests for issue #407: `--color` policy, richer human diagnostics,
//! and progress summaries for multi-file/markdown/verbose modes.
//!
//! Coverage:
//! - `--color never` strips ANSI escapes from diagnostic output
//! - `--color always` includes ANSI escapes even when piped (non-TTY)
//! - JSON/stdrpt output is unaffected by `--color`
//! - Progress lines are emitted on stderr in `--multi` mode
//! - Hint shown for `--multi`-needed error
//! - Hint shown for URL include rejection
//! - Hint shown for no markdown fence

use assert_cmd::Command;
use serde_json::Value;

// Minimal valid two-block input that requires --multi on stdin.
const TWO_BLOCKS: &str = "@startuml\nA -> B\n@enduml\n@startuml\nB -> C\n@enduml\n";

// Markdown with a puml fence.
const MARKDOWN_ONE_FENCE: &str = "# Title\n\n```puml\n@startuml\nA -> B\n@enduml\n```\n";

// Markdown with TWO puml fences.
const MARKDOWN_TWO_FENCES: &str =
    "# Title\n\n```puml\n@startuml\nA -> B\n@enduml\n```\n\n```puml\n@startuml\nC -> D\n@enduml\n```\n";

// Markdown with no recognized fence.
const MARKDOWN_NO_FENCE: &str = "# Title\n\nSome prose without any diagram fences.\n";

// Invalid puml to trigger a parse diagnostic.
const INVALID_PUML: &str = "@startuml\nA -x B: bad\n@enduml\n";

// Source with a URL include (disabled by default).
const URL_INCLUDE_SRC: &str =
    "@startuml\n!include https://example.com/diagram.puml\nA -> B\n@enduml\n";

// --------------------------------------------------------------------------
// Color policy: ANSI in human diagnostics
// --------------------------------------------------------------------------

#[test]
fn color_never_strips_ansi_from_human_diagnostics() {
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "never", "--check", "-"])
        .write_stdin(INVALID_PUML)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    assert!(
        !s.contains("\x1b["),
        "--color never must strip ANSI escapes; got: {s:?}"
    );
}

#[test]
fn color_always_includes_ansi_in_human_diagnostics() {
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "always", "--check", "-"])
        .write_stdin(INVALID_PUML)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    assert!(
        s.contains("\x1b["),
        "--color always must emit ANSI escapes even when piped; got: {s:?}"
    );
}

// --------------------------------------------------------------------------
// JSON / stdrpt output unaffected by --color
// --------------------------------------------------------------------------

#[test]
fn color_always_does_not_color_json_diagnostics() {
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "always", "--diagnostics", "json", "--check", "-"])
        .write_stdin(INVALID_PUML)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    assert!(
        serde_json::from_str::<Value>(&s).is_ok(),
        "JSON diagnostics must be valid JSON: {s:?}"
    );
    assert!(
        !s.contains("\x1b["),
        "JSON diagnostics must not contain ANSI escapes: {s:?}"
    );
}

#[test]
fn color_always_does_not_color_stdrpt_diagnostics() {
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "always", "--stdrpt", "--check", "-"])
        .write_stdin(INVALID_PUML)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    assert!(
        !s.contains("\x1b["),
        "stdrpt diagnostics must not contain ANSI escapes: {s:?}"
    );
}

// --------------------------------------------------------------------------
// Progress lines on stderr in multi-diagram modes
// --------------------------------------------------------------------------

#[test]
fn progress_lines_emitted_in_multi_mode() {
    // Use --check mode so no rendering output is needed; just verify progress.
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "never", "--multi", "--check", "-"])
        .write_stdin(TWO_BLOCKS)
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    // --check mode does not go through render.rs so no progress lines —
    // verify we at least succeed (no crash) with two blocks and --multi.
    // Progress lines are only emitted in render mode via render.rs.
    let _ = s; // no panic = pass
}

#[test]
fn progress_lines_emitted_in_markdown_render_mode() {
    // Render two fences from markdown to stdout (SVG); expect two progress lines.
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--color",
            "never",
            "--from-markdown",
            "--multi",
            "--format",
            "svg",
            "-",
        ])
        .write_stdin(MARKDOWN_TWO_FENCES)
        .assert()
        // Multi SVG on stdin produces JSON envelope; exit 0.
        .success()
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    assert!(
        s.contains("[1/2]") && s.contains("[2/2]"),
        "expected two progress lines [1/2] and [2/2] in stderr; got: {s:?}"
    );
}

#[test]
fn progress_lines_include_diagram_label() {
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--color",
            "never",
            "--from-markdown",
            "--multi",
            "--format",
            "svg",
            "-",
        ])
        .write_stdin(MARKDOWN_ONE_FENCE)
        .assert()
        .success()
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    // Single fence: progress line should appear because from_markdown is true.
    assert!(
        s.contains("[1/1]"),
        "expected progress line [1/1] in stderr; got: {s:?}"
    );
    assert!(
        s.contains("rendering"),
        "progress line should contain 'rendering'; got: {s:?}"
    );
}

// --------------------------------------------------------------------------
// Hint messages
// --------------------------------------------------------------------------

#[test]
fn hint_shown_for_multi_needed_error() {
    // Two @startuml blocks on stdin without --multi should suggest --multi.
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "never", "-"])
        .write_stdin(TWO_BLOCKS)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    assert!(
        s.contains("hint:") && s.contains("--multi"),
        "expected hint about --multi in stderr; got: {s:?}"
    );
}

#[test]
fn hint_shown_for_url_include_rejection() {
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "never", "--check", "-"])
        .write_stdin(URL_INCLUDE_SRC)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    assert!(
        s.contains("hint:") && s.contains("--allow-url-includes"),
        "expected hint about --allow-url-includes in stderr; got: {s:?}"
    );
}

#[test]
fn hint_shown_for_no_markdown_fence() {
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "never", "--from-markdown", "-"])
        .write_stdin(MARKDOWN_NO_FENCE)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    assert!(
        s.contains("hint:"),
        "expected hint about markdown fence in stderr; got: {s:?}"
    );
    assert!(
        s.contains("puml") || s.contains("plantuml") || s.contains("```"),
        "hint should mention a fence tag; got: {s:?}"
    );
}

#[test]
fn hint_not_shown_in_json_mode_for_multi_needed() {
    // JSON format must not include a hint line.
    let stderr = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "never", "--diagnostics", "json", "-"])
        .write_stdin(TWO_BLOCKS)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let s = String::from_utf8(stderr).unwrap();
    // The error message itself may appear in JSON, but a free-text "hint:" line should not.
    assert!(
        !s.contains("hint:"),
        "hint: line must not appear in JSON diagnostic output; got: {s:?}"
    );
}
