use super::support::*;
use super::*;

#[test]
fn state_concurrent_regions_renders_svg_with_dashed_divider() {
    let src = fs::read_to_string(fixture("families/valid_state_concurrent.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render state concurrent SVG");
    assert!(svg.contains("<svg"), "expected SVG output");
    assert!(
        svg.contains("stroke-dasharray"),
        "expected dashed divider in concurrent state SVG"
    );
}

#[test]
fn creole_bold_italic_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("conformance/valid_creole_message_bold_italic.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn json_family_check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/valid_json.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_alt_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_alt.mmd.txt"),
        ])
        .assert()
        .success();
}

#[test]
fn json_family_renders_deterministic_svg() {
    let src = fs::read_to_string(fixture("non_sequence/valid_json.puml")).unwrap();
    let a = render_source_to_svg(&src).expect("render JSON");
    let b = render_source_to_svg(&src).expect("render JSON again");
    assert_eq!(a, b, "JSON render must be deterministic");
    assert!(a.starts_with("<svg"));
    assert!(a.contains("JSON"));
    assert!(a.contains("name"));
}

#[test]
fn creole_bold_italic_svg_contains_tspan_formatting() {
    let src =
        fs::read_to_string(fixture("conformance/valid_creole_message_bold_italic.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("render");
    assert!(
        svg.contains("font-weight=\"bold\""),
        "expected bold tspan in SVG"
    );
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic tspan in SVG"
    );
}

#[test]
fn creole_note_link_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("conformance/valid_creole_note_link.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn yaml_family_check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/valid_yaml.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_create_destroy_link_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_create_destroy_link.mmd.txt"),
        ])
        .assert()
        .success();
}

#[test]
fn yaml_family_renders_deterministic_svg() {
    let src = fs::read_to_string(fixture("non_sequence/valid_yaml.puml")).unwrap();
    let a = render_source_to_svg(&src).expect("render YAML");
    let b = render_source_to_svg(&src).expect("render YAML again");
    assert_eq!(a, b, "YAML render must be deterministic");
    assert!(a.contains("YAML"));
    assert!(a.contains("project:"));
}

#[test]
fn nwdiag_family_check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/valid_nwdiag.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn mermaid_box_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dialect",
            "mermaid",
            "--check",
            &fixture("mermaid/valid_box.mmd.txt"),
        ])
        .assert()
        .success();
}

#[test]
fn nwdiag_family_renders_deterministic_svg_with_networks() {
    let src = fs::read_to_string(fixture("non_sequence/valid_nwdiag.puml")).unwrap();
    let a = render_source_to_svg(&src).expect("render nwdiag");
    let b = render_source_to_svg(&src).expect("render nwdiag again");
    assert_eq!(a, b, "nwdiag render must be deterministic");
    assert!(a.contains("network dmz"));
    assert!(a.contains("web01"));
    assert!(a.contains("network internal"));
}

#[test]
fn nwdiag_multi_network_group_and_multi_address_layout_is_preserved() {
    let src = fs::read_to_string(fixture(
        "non_sequence/valid_nwdiag_multi_network_addresses.puml",
    ))
    .unwrap();
    let svg = render_source_to_svg(&src).expect("render nwdiag topology depth fixture");

    assert!(
        svg.contains("data-nwdiag-addresses=\"10.0.0.10, fd00:10::10\""),
        "public lb should preserve every bracketed address: {svg}"
    );
    assert!(
        svg.contains("edge lb [10.0.0.10, fd00:10::10]"),
        "multi-address label should render both values: {svg}"
    );
    assert!(
        svg.contains("stroke-dasharray=\"5 3\""),
        "dashed node style should reach SVG geometry"
    );
    assert!(
        svg.contains("width=\"240\""),
        "node width attribute should affect SVG geometry"
    );
    assert!(
        svg.contains("class=\"nwdiag-connector\""),
        "nwdiag nodes should render as boxes connected back to the network bar"
    );
    assert!(
        svg.contains("class=\"nwdiag-address\""),
        "nwdiag connector annotations should render node addresses near the link"
    );

    let public_y = svg_rect_y(
        &svg,
        "class=\"nwdiag-network\"",
        "network public (10.0.0.x/24)",
    )
    .expect("public network y");
    let private_y = svg_rect_y(
        &svg,
        "class=\"nwdiag-network\"",
        "network private (192.168.1.x/24)",
    )
    .expect("private network y");
    assert!(
        private_y > public_y,
        "private network should be laid out below public network"
    );

    let public_lb = svg_node_rect(&svg, "lb", "10.0.0.10, fd00:10::10").expect("public lb rect");
    let private_lb =
        svg_node_rect(&svg, "lb", "192.168.1.10, fd00:192::10").expect("private lb rect");
    assert_eq!(
        public_lb.x, private_lb.x,
        "shared node column should be stable"
    );
    assert!(
        private_lb.y > public_lb.y,
        "shared node should appear in each network row"
    );
    let private_app = svg_node_rect(&svg, "app01", "192.168.1.21").expect("private app rect");
    assert!(
        private_app.x > private_lb.x,
        "distinct nwdiag nodes should occupy separate horizontal columns instead of one vertical list"
    );

    // Groups now render as topology overlays positioned around their member nodes,
    // not as a flat list appended below the diagram. The group rect y-position
    // must be within the topology area, not beyond the last network row bottom edge.
    let group_y = svg_rect_y(&svg, "class=\"nwdiag-group\"", "group edge").expect("group y");
    assert!(
        group_y < private_y + 150,
        "group overlay should sit within the topology area, not appended below: group_y={group_y} private_y={private_y}"
    );
}
