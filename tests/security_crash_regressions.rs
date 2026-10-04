//! Regression tests for the 2026-10 security/crash audit: dangerous link
//! schemes, preprocessor budgets, malformed activity input, UTF-8 labels,
//! and bare-filename include roots.

use assert_cmd::Command;
use std::fs;

fn puml() -> Command {
    Command::cargo_bin("puml").expect("puml binary")
}

fn render(src: &str) -> String {
    let out = puml()
        .args(["--format", "svg", "-"])
        .write_stdin(src.to_string())
        .output()
        .expect("run puml");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn dangerous_link_schemes_are_not_emitted_as_href() {
    for url in [
        "javascript:alert(1)",
        "JaVaScRiPt:alert(1)",
        "data:text/html,x",
        "vbscript:x",
        "file:///etc/passwd",
    ] {
        let svg = render(&format!("@startuml\nA -> B : [[{url} click]]\n@enduml\n"));
        assert!(!svg.contains("href=\"javascript"), "{url}: {svg}");
        assert!(!svg.to_ascii_lowercase().contains("href=\"data:"), "{url}");
        assert!(
            !svg.to_ascii_lowercase().contains("href=\"vbscript"),
            "{url}"
        );
        assert!(!svg.to_ascii_lowercase().contains("href=\"file:"), "{url}");
    }
}

#[test]
fn safe_link_schemes_are_kept() {
    let svg = render(
        "@startuml\nA -> B : [[https://example.com ok]] [[mailto:a@b.c m]] [[/rel r]]\n@enduml\n",
    );
    assert!(svg.contains("href=\"https://example.com\""));
    assert!(svg.contains("href=\"mailto:a@b.c\""));
    assert!(svg.contains("href=\"/rel\""));
}

#[test]
fn include_fan_out_hits_the_expansion_limit() {
    let dir = tempfile::tempdir().unwrap();
    for n in 1..=24 {
        fs::write(
            dir.path().join(format!("f{n}.puml")),
            format!("!include f{0}.puml\n!include f{0}.puml\n", n + 1),
        )
        .unwrap();
    }
    fs::write(dir.path().join("f25.puml"), "A -> B\n").unwrap();
    let main = dir.path().join("main.puml");
    fs::write(&main, "@startuml\n!include f1.puml\n@enduml\n").unwrap();
    puml()
        .args(["--check"])
        .arg(&main)
        .assert()
        .failure()
        .stderr(predicates::str::contains("E_PREPROC_"));
}

#[test]
fn nested_while_loops_share_a_total_budget() {
    puml()
        .args(["--check", "-"])
        .write_stdin("@startuml\n!$i=0\n!while $i<3000\n!$j=0\n!while $j<3000\n!$j=$j+1\n!endwhile\n!$i=$i+1\n!endwhile\nA->B\n@enduml\n")
        .assert()
        .failure()
        .stderr(predicates::str::contains("E_PREPROC_EXPANSION_LIMIT"));
}

#[test]
fn integer_overflow_in_preprocessor_does_not_abort() {
    let out = puml()
        .args(["--check", "-"])
        .write_stdin("@startuml\n!$a = -9223372036854775808 / -1\nA->B\n@enduml\n")
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(134));
    assert!(out.status.code().is_some());
}

#[test]
fn unbalanced_activity_keywords_do_not_abort() {
    for kw in [
        "else (no)",
        "endif",
        "fork again",
        "end fork",
        "end split",
        "endswitch",
        "case (a)",
        "split again",
        "elseif (x) then (y)",
    ] {
        let out = puml()
            .args(["--format", "svg", "-"])
            .write_stdin(format!("@startuml\nstart\n{kw}\n:y;\nstop\n@enduml\n"))
            .output()
            .unwrap();
        let code = out.status.code();
        assert!(matches!(code, Some(0..=2)), "{kw}: exit {code:?}");
    }
}

#[test]
fn multibyte_labels_do_not_panic_the_renderer() {
    for label in [
        "\u{1d518}\u{1d518}\u{1d518}\u{1d518}\u{1d518}\u{1d518}",
        "漢字漢字漢字漢字漢字漢字漢字",
        "😀😀😀😀😀😀😀😀",
    ] {
        let out = puml()
            .args(["--format", "svg", "-"])
            .write_stdin(format!(
                "@startuml\nA -> B : {label}\nnote right: {label}\n@enduml\n"
            ))
            .output()
            .unwrap();
        assert!(matches!(out.status.code(), Some(0..=2)));
    }
}

#[test]
fn bare_filename_resolves_includes() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("inc.puml"), "A -> B\n").unwrap();
    fs::write(
        dir.path().join("bare.puml"),
        "@startuml\n!include inc.puml\n@enduml\n",
    )
    .unwrap();
    puml()
        .current_dir(dir.path())
        .args(["--check", "bare.puml"])
        .assert()
        .success();
}
