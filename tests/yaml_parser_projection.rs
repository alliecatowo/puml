use puml::{render_source_to_svg, render_source_to_text, TextOutputMode};

#[test]
fn standalone_yaml_uses_parser_backed_sequence_projection() {
    let src = r#"@startyaml
servers:
  - name: api
    ports:
      - 80
      - 443
  - name: worker
enabled: true
@endyaml
"#;

    let text = render_source_to_text(src, TextOutputMode::Txt).expect("YAML should render");

    assert!(text.contains("servers: [...]"));
    assert!(text.contains("[0]: {...}"));
    assert!(text.contains("name: api"));
    assert!(text.contains("ports: [...]"));
    assert!(text.contains("[0]: 80"));
    assert!(text.contains("[1]: 443"));
    assert!(text.contains("enabled: true"));
}

#[test]
fn family_yaml_projection_flattens_nested_sequences_with_paths() {
    let src = r#"@startuml
yaml $cfg {
servers:
  - name: api
    ports:
      - 80
      - 443
  - name: worker
}
@enduml
"#;

    let svg = render_source_to_svg(src).expect("YAML projection should render");

    assert!(svg.contains("$cfg"));
    assert!(svg.contains("servers"));
    assert!(svg.contains("name: api"));
    assert!(svg.contains("ports"));
    assert!(svg.contains("80"));
    assert!(svg.contains("443"));
    assert!(svg.contains("name: worker"));
}

#[test]
fn invalid_yaml_keeps_indentation_fallback() {
    let src = r#"@startyaml
root:
  ok: true
  bad: [unterminated
@endyaml
"#;

    let text = render_source_to_text(src, TextOutputMode::Txt)
        .expect("invalid YAML should keep legacy fallback rendering");

    assert!(text.contains("root:"));
    assert!(text.contains("ok: true"));
    assert!(text.contains("bad: [unterminated"));
}

#[test]
fn invalid_yaml_strips_highlight_and_style_before_fallback() {
    let src = r##"@startyaml
#highlight "root" <<bad>>
<style>
.bad {
  BackGroundColor #dc2626
}
</style>
root:
  bad: [unterminated
@endyaml
"##;

    let text = render_source_to_text(src, TextOutputMode::Txt)
        .expect("invalid YAML should strip controls before fallback rendering");

    assert!(!text.contains("#highlight"));
    assert!(!text.contains("<style>"));
    assert!(text.contains("root:"));
    assert!(text.contains("bad: [unterminated"));
}
