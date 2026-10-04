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

#[test]
fn bom_prefixed_input_is_accepted() {
    puml()
        .args(["--check", "-"])
        .write_stdin("\u{feff}@startuml\nA -> B\n@enduml\n")
        .assert()
        .success();
}

#[test]
fn output_dash_writes_to_stdout_not_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = puml()
        .current_dir(dir.path())
        .args(["--format", "svg", "-", "-o", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("<svg"));
    assert!(!dir.path().join("-").exists());
}

#[test]
fn lsp_survives_a_malformed_frame() {
    fn frame(body: &str) -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
    }
    let mut input = frame(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
    input.extend(frame("{bad}"));
    input.extend(b"content-length: 2\r\n\r\n{}".iter());
    input.extend(frame(r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#));
    let out = Command::cargo_bin("puml-lsp")
        .unwrap()
        .write_stdin(input)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("\"id\":1"));
    assert!(
        text.contains("\"id\":2"),
        "server died after bad frame: {text}"
    );
}

#[test]
fn lsp_resolves_includes_relative_to_the_document() {
    fn frame(body: &str) -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
    }
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("inc.puml"), "A -> B\n").unwrap();
    let uri = format!("file://{}/x.puml", dir.path().display());
    let open = serde_json::json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":uri,"version":1,"text":"@startuml\n!include inc.puml\n@enduml\n"}}}).to_string();
    let mut input = frame(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#);
    input.extend(frame(&open));
    input.extend(frame(r#"{"jsonrpc":"2.0","method":"exit"}"#));
    let out = Command::cargo_bin("puml-lsp")
        .unwrap()
        .write_stdin(input)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("publishDiagnostics"));
    assert!(!text.contains("E_INCLUDE_ROOT"), "{text}");
}

fn url_include_cmd(root: &std::path::Path, src: &str) -> std::process::Output {
    puml()
        .args(["--allow-url-includes", "--include-root"])
        .arg(root)
        .args(["--check", "-"])
        .write_stdin(src.to_string())
        .output()
        .expect("run puml")
}

#[test]
fn file_url_includes_respect_include_root() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("inside.puml"), "A -> B : ok\n").unwrap();
    let outside = tmp.path().join("outside.puml");
    fs::write(&outside, "A -> B : leaked\n").unwrap();

    // Absolute file:// path outside the root is rejected.
    let out = url_include_cmd(
        &root,
        &format!(
            "@startuml\n!include file://{}\n@enduml\n",
            outside.display()
        ),
    );
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("E_INCLUDE_ESCAPE"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // `..` traversal through a file:// URL is rejected too.
    let out = url_include_cmd(
        &root,
        &format!(
            "@startuml\n!include file://{}/../outside.puml\n@enduml\n",
            root.display()
        ),
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("E_INCLUDE_ESCAPE"));

    // The same read via !includeurl and !include_many is confined as well.
    for directive in ["!includeurl", "!include_many"] {
        let out = url_include_cmd(
            &root,
            &format!(
                "@startuml\n{directive} file://{}\n@enduml\n",
                outside.display()
            ),
        );
        assert!(!out.status.success(), "{directive}");
    }

    // A file inside the root still works.
    let out = url_include_cmd(
        &root,
        &format!(
            "@startuml\n!include file://{}\n@enduml\n",
            root.join("inside.puml").display()
        ),
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn file_url_include_without_root_is_rejected() {
    let out = puml()
        .args(["--allow-url-includes", "--check", "-"])
        .write_stdin("@startuml\n!include file:///etc/hostname\n@enduml\n".to_string())
        .output()
        .expect("run puml");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("E_INCLUDE_ROOT_REQUIRED"));
}

#[cfg(unix)]
#[test]
fn file_url_include_symlink_escape_is_rejected() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    let outside = tmp.path().join("outside.puml");
    fs::write(&outside, "A -> B : leaked\n").unwrap();
    std::os::unix::fs::symlink(&outside, root.join("link.puml")).unwrap();
    let out = url_include_cmd(
        &root,
        &format!(
            "@startuml\n!include file://{}\n@enduml\n",
            root.join("link.puml").display()
        ),
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("E_INCLUDE_ESCAPE"));
}
