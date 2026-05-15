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
fn family_routing_stub_renders_class_deterministically() {
    let src = "@startuml\nclass A\n@enduml\n";
    let first = render_source_to_svgs_for_family(src, DiagramFamily::Class)
        .expect("class routing should render via stub");
    let second = render_source_to_svgs_for_family(src, DiagramFamily::Class)
        .expect("class routing should render via stub");

    assert_eq!(first, second, "stub output should be deterministic");
    assert!(
        first[0].contains("Bootstrap stub for class diagrams"),
        "stub render should include family marker"
    );
}

#[test]
fn family_routing_rejects_mismatched_requested_family() {
    let src = "@startuml\nclass A\n@enduml\n";
    let err = render_source_to_svgs_for_family(src, DiagramFamily::Sequence)
        .expect_err("mismatched family should fail deterministically");
    assert!(err.message.contains("E_FAMILY_MISMATCH"));
}
