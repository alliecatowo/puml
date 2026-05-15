use assert_cmd::Command;
use puml::{render_source_to_svgs_for_family, DiagramFamily};

#[test]
fn exit_code_contract() {
    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--help")
        .assert()
        .code(0);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("--definitely-invalid-flag")
        .assert()
        .code(1);

    Command::cargo_bin("puml")
        .expect("binary")
        .arg("/tmp/definitely-not-present-12345.puml")
        .assert()
        .code(2);

    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("")
        .assert()
        .code(1);

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin("invalid")
        .assert()
        .code(1);
}

#[test]
fn family_routing_stub_is_deterministic_for_sequence() {
    let src = "@startuml\nAlice -> Bob: hi\n@enduml\n";
    let first = render_source_to_svgs_for_family(src, DiagramFamily::Sequence)
        .expect("sequence should render");
    let second = render_source_to_svgs_for_family(src, DiagramFamily::Sequence)
        .expect("sequence should render");
    assert_eq!(first, second, "sequence routing should be deterministic");
}

#[test]
fn family_routing_stub_rejects_non_sequence_for_now() {
    let src = "@startuml\nclass A\n@enduml\n";
    let first = render_source_to_svgs_for_family(src, DiagramFamily::Class)
        .expect_err("class routing should be rejected");
    let second = render_source_to_svgs_for_family(src, DiagramFamily::Class)
        .expect_err("class routing should be rejected");

    assert_eq!(
        first.message, second.message,
        "rejection should be deterministic"
    );
    assert!(
        first
            .message
            .contains("diagram family `class` is not implemented yet"),
        "rejection should mention unsupported family"
    );
}
