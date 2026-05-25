use assert_cmd::Command;
use predicates::prelude::*;

const SIMPLE: &str = "@startuml\nAlice -> Bob: hi\n@enduml\n";

#[test]
fn single_dash_thtml_alias_selects_html_output() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["-thtml", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::starts_with("<!doctype html>"))
        .stdout(predicate::str::contains("<svg"))
        .stdout(predicate::str::contains("hi"));
}

#[test]
fn output_format_alias_selects_supported_text_output() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--output-format", "txt", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("Alice -> Bob: hi"));
}

#[test]
fn plantuml_text_format_aliases_remain_supported() {
    for alias in ["-ttxt", "-tutxt", "-txt", "-utxt"] {
        Command::cargo_bin("puml")
            .expect("puml binary")
            .args([alias, "-"])
            .write_stdin(SIMPLE)
            .assert()
            .success()
            .stderr(predicate::str::is_empty())
            .stdout(predicate::str::contains("Alice"));
    }
}

#[test]
fn threads_and_failfast2_are_deterministic_noops() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--threads", "4", "--failfast2", "--check", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn unsupported_latex_alias_reports_supported_formats_with_exit_2() {
    for args in [
        vec!["-tlatex", "-"],
        vec!["-tlatex:nopreamble", "-"],
        vec!["--format", "latex", "-"],
        vec!["--output-format", "latex:nopreamble", "-"],
    ] {
        Command::cargo_bin("puml")
            .expect("puml binary")
            .args(args)
            .write_stdin(SIMPLE)
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("E_OUTPUT_FORMAT_UNSUPPORTED"))
            .stderr(predicate::str::contains("supported formats: svg, html"));
    }
}

#[test]
fn unsupported_parity_flags_report_deterministic_messages_with_exit_2() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--extract", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("E_FLAG_UNSUPPORTED"))
        .stderr(predicate::str::contains("--extract"));

    Command::cargo_bin("puml")
        .expect("puml binary")
        .args(["--pattern", ".*\\.puml", "-"])
        .write_stdin(SIMPLE)
        .assert()
        .code(2)
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("E_FLAG_UNSUPPORTED"))
        .stderr(predicate::str::contains("--pattern"))
        .stderr(predicate::str::contains(".*\\.puml"));
}

#[test]
fn help_documents_parity_flags_and_output_format_alias() {
    Command::cargo_bin("puml")
        .expect("puml binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--threads <N>"))
        .stdout(predicate::str::contains("--failfast2"))
        .stdout(predicate::str::contains("--extract"))
        .stdout(predicate::str::contains("--pattern <REGEX>"))
        .stdout(predicate::str::contains("--output-format"));
}
