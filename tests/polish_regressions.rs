//! Regression tests: !ifdef sees variables, raster pixel cap, lint include root.

use assert_cmd::Command;

fn puml() -> Command {
    Command::cargo_bin("puml").expect("puml binary")
}

#[test]
fn ifdef_consults_variables_like_defined() {
    let src = "@startuml\n!$x = 1\n!ifdef $x\nA -> B : set\n!endif\n!ifndef $x\nA -> B : unset\n!endif\n@enduml\n";
    let out = puml()
        .args(["--format", "svg", "-"])
        .write_stdin(src.to_string())
        .output()
        .unwrap();
    let svg = String::from_utf8_lossy(&out.stdout);
    assert!(svg.contains("set") && !svg.contains("unset"), "{svg}");
}

#[test]
fn raster_output_is_capped() {
    let mut src = String::from("@startuml\n");
    for i in 0..60 {
        src.push_str(&format!("participant P{i}\n"));
    }
    src.push_str("P0 -> P59 : x\n@enduml\n");
    let out = puml()
        .args(["--format", "png", "--dpi", "1200", "-o", "-", "-"])
        .write_stdin(src)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("pixel limit"));
}

#[test]
fn lint_honours_include_root() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("part.puml"), "A -> B : hi\n").unwrap();
    std::fs::write(
        dir.path().join("main.puml"),
        "@startuml\n!include part.puml\n@enduml\n",
    )
    .unwrap();
    puml()
        .current_dir(dir.path())
        .args(["lint", "main.puml"])
        .assert()
        .success();
}
