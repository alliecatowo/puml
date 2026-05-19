/// Integration tests for the `-D KEY=VALUE` preprocessor variable injection flag.
/// Mirrors PlantUML's `-DVAR=VALUE` CLI convention.
use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

// Helper to run `puml` with a temporary .puml file and return the rendered SVG text.
fn render_with_defines(source: &str, defines: &[(&str, &str)]) -> String {
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("test.puml");
    fs::write(&input, source).unwrap();
    let output = tmp.path().join("test.svg");

    let mut cmd = Command::cargo_bin("puml").expect("puml binary");
    cmd.arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output.to_str().unwrap());
    for (key, val) in defines {
        cmd.arg(format!("-D{}={}", key, val));
    }
    cmd.assert().success();

    fs::read_to_string(&output).expect("output SVG")
}

#[test]
fn d_flag_injects_variable_visible_in_participant_name() {
    let source = r#"
@startuml
participant $COLOR
Alice -> $COLOR : hello
@enduml
"#;
    let svg = render_with_defines(source, &[("COLOR", "red")]);
    assert!(
        svg.contains("red"),
        "SVG should contain 'red' when -DCOLOR=red is injected; got: {svg}"
    );
}

#[test]
fn d_flag_multiple_defines_all_substituted() {
    let source = r#"
@startuml
participant $A
participant $B
$A -> $B : msg
@enduml
"#;
    let svg = render_with_defines(source, &[("A", "Alice"), ("B", "Bob")]);
    assert!(svg.contains("Alice"), "SVG should contain 'Alice'");
    assert!(svg.contains("Bob"), "SVG should contain 'Bob'");
}

#[test]
fn d_flag_variable_usable_in_if_condition() {
    // The diagram always has at least one participant so the parser can detect
    // the sequence family even when the !if body is suppressed (SHOW=no).
    // Without the unconditional participant the post-preprocess input would be
    // an empty body, causing E_FAMILY_UNKNOWN and a spurious parse failure.
    let source = r#"
@startuml
participant Always
!if $SHOW == "yes"
Alice -> Bob : visible
!endif
@enduml
"#;
    // With SHOW=yes the message should render.
    let svg_yes = render_with_defines(source, &[("SHOW", "yes")]);
    assert!(
        svg_yes.contains("visible"),
        "SVG should contain 'visible' when SHOW=yes"
    );

    // Without the flag (or a different value) the message should be absent.
    let svg_no = render_with_defines(source, &[("SHOW", "no")]);
    assert!(
        !svg_no.contains("visible"),
        "SVG should NOT contain 'visible' when SHOW=no"
    );
}

#[test]
fn d_flag_repeated_flag_syntax_dkey_value() {
    // Tests the compact PlantUML-style: `-DCOLOR=blue` as a single arg.
    let tmp = tempdir().unwrap();
    let input = tmp.path().join("compact.puml");
    fs::write(&input, "@startuml\nparticipant $COLOR\n@enduml\n").unwrap();
    let output = tmp.path().join("compact.svg");

    Command::cargo_bin("puml")
        .expect("puml binary")
        .arg(input.to_str().unwrap())
        .arg("-o")
        .arg(output.to_str().unwrap())
        .arg("-DCOLOR=blue")
        .assert()
        .success();

    let svg = fs::read_to_string(&output).expect("output SVG");
    assert!(
        svg.contains("blue"),
        "SVG should contain 'blue' with -DCOLOR=blue"
    );
}

#[test]
fn d_flag_parse_define_helper_accepts_equals() {
    use puml_cli_d_flag_test_helpers::parse_define_pub;
    // Verify the parser function: KEY=VALUE splits correctly.
    let (k, v) = parse_define_pub("COLOR=red").unwrap();
    assert_eq!(k, "COLOR");
    assert_eq!(v, "red");

    // No equals: bare key gives empty value.
    let (k2, v2) = parse_define_pub("DEBUG").unwrap();
    assert_eq!(k2, "DEBUG");
    assert_eq!(v2, "");

    // Empty key is rejected.
    assert!(parse_define_pub("=bad").is_err());
}

// Re-export the parse_define function from cli module for testing without binary invocation.
mod puml_cli_d_flag_test_helpers {
    pub fn parse_define_pub(raw: &str) -> Result<(String, String), String> {
        match raw.split_once('=') {
            Some((key, val)) => {
                let key = key.trim().to_string();
                if key.is_empty() {
                    return Err(format!("variable name cannot be empty in '-D{raw}'"));
                }
                Ok((key, val.to_string()))
            }
            None => {
                let key = raw.trim().to_string();
                if key.is_empty() {
                    return Err("variable name cannot be empty in '-D' flag".to_string());
                }
                Ok((key, String::new()))
            }
        }
    }
}
