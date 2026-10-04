//! Regression tests: XML-invalid numeric references, nested markdown fences,
//! and splitting on every @start* marker.

use assert_cmd::Command;

fn puml() -> Command {
    Command::cargo_bin("puml").expect("puml binary")
}

fn render(src: &str) -> Vec<u8> {
    puml()
        .args(["--format", "svg", "-"])
        .write_stdin(src.to_string())
        .output()
        .expect("run puml")
        .stdout
}

#[test]
fn xml_invalid_numeric_references_never_reach_svg() {
    for r in [
        "&#0;", "&#1;", "&#x8;", "&#xB;", "&#xFFFE;", "&#xFFFF;", "&#x0;",
    ] {
        let out = render(&format!("@startuml\nA -> B : x {r} y\n@enduml\n"));
        assert!(!out.is_empty(), "{r}");
        assert!(
            out.iter()
                .all(|b| *b >= 0x20 || matches!(b, b'\n' | b'\r' | b'\t')),
            "{r}: control byte in SVG"
        );
        let s = String::from_utf8(out).expect("utf8");
        assert!(!s.contains('\u{FFFE}') && !s.contains('\u{FFFF}'), "{r}");
    }
}

#[test]
fn valid_numeric_references_still_decode() {
    let s = String::from_utf8(render("@startuml\nA -> B : &#65;&#x42;&#9731;\n@enduml\n")).unwrap();
    assert!(s.contains("AB\u{2603}"), "{s}");
}

#[test]
fn nested_fenced_samples_are_not_extracted() {
    let md = "````markdown\n```puml\n@startuml\nA -> B : inert\n@enduml\n```\n````\n\n```puml\n@startuml\nA -> B : real\n@enduml\n```\n";
    let out = puml()
        .args(["--from-markdown", "--extract", "-o", "-", "-"])
        .write_stdin(md.to_string())
        .output()
        .expect("run");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("real"), "{text}");
    assert!(!text.contains("inert"), "{text}");
}

#[test]
fn library_extraction_skips_diagrams_inside_other_fences() {
    let found = puml::extract_markdown_diagrams(
        "~~~md
```puml
@startuml
A->B
@enduml
```
~~~
",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn mixed_start_markers_each_produce_a_diagram() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("mixed.puml");
    std::fs::write(
        &input,
        "@startuml\nAlice -> Bob : seq\n@enduml\n\n@startmindmap\n* root\n** child\n@endmindmap\n",
    )
    .unwrap();
    let out = puml()
        .current_dir(dir.path())
        .args(["--format", "svg", "mixed.puml"])
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let svgs: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "svg"))
        .collect();
    assert_eq!(svgs.len(), 2, "{svgs:?}");
}

#[test]
fn unmatched_non_uml_marker_is_reported() {
    let out = puml()
        .args(["--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n@startmindmap\n* a\n".to_string())
        .output()
        .expect("run");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("@startmindmap"));
}
