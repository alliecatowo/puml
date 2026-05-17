mod svg_test_helpers;

use assert_cmd::Command;
use puml::{render_source_to_svgs_for_family, DiagramFamily};
use svg_test_helpers::{bounds, f64_attr, SvgDoc};

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

    assert_eq!(first, second, "class render should be deterministic");
    let doc = SvgDoc::parse(&first[0]);
    let class_label = doc
        .texts_containing("A")
        .into_iter()
        .next()
        .expect("class render should include a visible class label");
    assert!(f64_attr(class_label, "x") > 0.0);
    assert!(f64_attr(class_label, "y") > 0.0);

    let class_box = doc
        .elements("rect")
        .into_iter()
        .find(|node| {
            node.attribute("width") == Some("200") && node.attribute("height") == Some("38")
        })
        .expect("class render should include a visible class box");
    let class_bounds = bounds(class_box);
    assert!(class_bounds.width > 0.0 && class_bounds.height > 0.0);
    assert!(f64_attr(class_label, "x") > class_bounds.x);
    assert!(f64_attr(class_label, "x") < class_bounds.right());
}

#[test]
fn family_routing_rejects_mismatched_requested_family() {
    let src = "@startuml\nclass A\n@enduml\n";
    let err = render_source_to_svgs_for_family(src, DiagramFamily::Sequence)
        .expect_err("mismatched family should fail deterministically");
    assert!(err.message.contains("E_FAMILY_MISMATCH"));
}
