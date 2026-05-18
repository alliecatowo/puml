use assert_cmd::Command;
use serde_json::Value;

const INVALID_MARKDOWN: &str = "# header\n```puml\n@startuml\nA -x B: bad\n@enduml\n```\n";

#[test]
fn color_never_disables_human_diagnostic_ansi() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "never", "--from-markdown", "--check", "-"])
        .write_stdin(INVALID_MARKDOWN)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("line 4, column 1"));
    assert!(
        !stderr.contains("\x1b["),
        "expected --color never to suppress ANSI escapes: {stderr:?}"
    );
}

#[test]
fn color_always_enables_human_diagnostic_ansi() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--color", "always", "--from-markdown", "--check", "-"])
        .write_stdin(INVALID_MARKDOWN)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();

    let stderr = String::from_utf8(out).unwrap();
    assert!(stderr.contains("line 4, column 1"));
    assert!(
        stderr.contains("\x1b["),
        "expected --color always to emit ANSI escapes: {stderr:?}"
    );
}

#[test]
fn color_always_does_not_color_json_or_stdrpt_diagnostics() {
    let json = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--color",
            "always",
            "--diagnostics",
            "json",
            "--from-markdown",
            "--check",
            "-",
        ])
        .write_stdin(INVALID_MARKDOWN)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let json_stderr = String::from_utf8(json).unwrap();
    assert!(serde_json::from_str::<Value>(&json_stderr).is_ok());
    assert!(!json_stderr.contains("\x1b["));

    let stdrpt = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--color",
            "always",
            "--stdrpt",
            "--from-markdown",
            "--check",
            "-",
        ])
        .write_stdin(INVALID_MARKDOWN)
        .assert()
        .code(1)
        .get_output()
        .stderr
        .clone();
    let stdrpt_stderr = String::from_utf8(stdrpt).unwrap();
    assert!(!stdrpt_stderr.contains("\x1b["));
    assert_eq!(stdrpt_stderr.lines().count(), 1);
}
