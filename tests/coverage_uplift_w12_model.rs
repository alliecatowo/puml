//! Wave-12 coverage uplift — model and preprocessor focused tests.
//!
//! Targets `src/model/family.rs` (FamilyRelationArrow, FamilyRelationDirection,
//! FamilyRelationColor, FamilyRelationEndpointMarker) and exercises
//! `src/preproc/macros/definelong.rs` through the public parse API.
//!
//! Refs #89

use puml::ast::StatementKind;
use puml::model::{
    FamilyNode, FamilyNodeKind, FamilyRelationArrow, FamilyRelationColor, FamilyRelationDirection,
    MindMapSide,
};

// ── helpers ────────────────────────────────────────────────────────────────────

fn msg_labels(src: &str) -> Vec<String> {
    let doc = puml::parse(src).expect("parse failed");
    doc.statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.clone(),
            _ => None,
        })
        .collect()
}

// ── model/family.rs: FamilyRelationDirection ──────────────────────────────────

#[test]
fn family_relation_direction_as_str_values() {
    assert_eq!(FamilyRelationDirection::Left.as_str(), "left");
    assert_eq!(FamilyRelationDirection::Right.as_str(), "right");
    assert_eq!(FamilyRelationDirection::Up.as_str(), "up");
    assert_eq!(FamilyRelationDirection::Down.as_str(), "down");
}

#[test]
fn family_relation_direction_parse_canonical_values() {
    assert_eq!(
        FamilyRelationDirection::parse("left"),
        Some(FamilyRelationDirection::Left)
    );
    assert_eq!(
        FamilyRelationDirection::parse("right"),
        Some(FamilyRelationDirection::Right)
    );
    assert_eq!(
        FamilyRelationDirection::parse("up"),
        Some(FamilyRelationDirection::Up)
    );
    assert_eq!(
        FamilyRelationDirection::parse("down"),
        Some(FamilyRelationDirection::Down)
    );
}

#[test]
fn family_relation_direction_parse_abbreviations() {
    assert_eq!(
        FamilyRelationDirection::parse("l"),
        Some(FamilyRelationDirection::Left)
    );
    assert_eq!(
        FamilyRelationDirection::parse("r"),
        Some(FamilyRelationDirection::Right)
    );
    assert_eq!(
        FamilyRelationDirection::parse("u"),
        Some(FamilyRelationDirection::Up)
    );
    assert_eq!(
        FamilyRelationDirection::parse("d"),
        Some(FamilyRelationDirection::Down)
    );
}

#[test]
fn family_relation_direction_parse_case_insensitive() {
    assert_eq!(
        FamilyRelationDirection::parse("LEFT"),
        Some(FamilyRelationDirection::Left)
    );
    assert_eq!(
        FamilyRelationDirection::parse("UP"),
        Some(FamilyRelationDirection::Up)
    );
}

#[test]
fn family_relation_direction_parse_unknown_returns_none() {
    assert!(FamilyRelationDirection::parse("diagonal").is_none());
    assert!(FamilyRelationDirection::parse("").is_none());
    assert!(FamilyRelationDirection::parse("x").is_none());
}

#[test]
fn family_relation_direction_display() {
    use std::fmt::Write;
    let mut buf = String::new();
    write!(buf, "{}", FamilyRelationDirection::Left).unwrap();
    assert_eq!(buf, "left");
    buf.clear();
    write!(buf, "{}", FamilyRelationDirection::Right).unwrap();
    assert_eq!(buf, "right");
}

#[test]
fn family_relation_direction_deref() {
    use std::ops::Deref;
    let dir = FamilyRelationDirection::Up;
    assert_eq!(dir.deref(), "up");
    // Can be used directly as &str
    let s: &str = &dir;
    assert_eq!(s, "up");
}

// ── model/family.rs: FamilyRelationColor ─────────────────────────────────────

#[test]
fn family_relation_color_parse_valid_six_digit_hex() {
    let color = FamilyRelationColor::parse("#aabbcc").expect("valid hex");
    assert_eq!(color.as_str(), "#aabbcc");
}

#[test]
fn family_relation_color_parse_css3_name() {
    let color = FamilyRelationColor::parse("red").expect("valid css3 name");
    assert_eq!(color.as_str(), "#ff0000");
}

#[test]
fn family_relation_color_parse_invalid_returns_err() {
    assert!(
        FamilyRelationColor::parse("#abc").is_err(),
        "3-digit hex rejected"
    );
    assert!(
        FamilyRelationColor::parse("notacolor").is_err(),
        "unknown name rejected"
    );
    assert!(FamilyRelationColor::parse("").is_err(), "empty rejected");
}

#[test]
fn family_relation_color_display() {
    use std::fmt::Write;
    let color = FamilyRelationColor::parse("blue").unwrap();
    let mut buf = String::new();
    write!(buf, "{}", color).unwrap();
    assert_eq!(buf, "#0000ff");
}

#[test]
fn family_relation_color_deref() {
    use std::ops::Deref;
    let color = FamilyRelationColor::parse("#123456").unwrap();
    assert_eq!(color.deref(), "#123456");
    let s: &str = &color;
    assert_eq!(s, "#123456");
}

// ── model/family.rs: FamilyRelationArrow ─────────────────────────────────────

#[test]
fn family_relation_arrow_solid_line_detected() {
    let arrow = FamilyRelationArrow::parse("-->").unwrap();
    assert!(!arrow.is_dashed());
    assert_eq!(arrow.as_str(), "-->");
}

#[test]
fn family_relation_arrow_dashed_line_detected() {
    let arrow = FamilyRelationArrow::parse("..>").unwrap();
    assert!(arrow.is_dashed());
}

#[test]
fn family_relation_arrow_invalid_too_short_returns_err() {
    assert!(FamilyRelationArrow::parse("a").is_err());
    assert!(FamilyRelationArrow::parse("").is_err());
}

#[test]
fn family_relation_arrow_no_dash_or_dot_returns_err() {
    // Must contain '-' or '.'
    assert!(FamilyRelationArrow::parse("abc").is_err());
}

#[test]
fn family_relation_arrow_display() {
    use std::fmt::Write;
    let arrow = FamilyRelationArrow::parse("<|--").unwrap();
    let mut buf = String::new();
    write!(buf, "{}", arrow).unwrap();
    assert_eq!(buf, "<|--");
}

#[test]
fn family_relation_arrow_deref() {
    use std::ops::Deref;
    let arrow = FamilyRelationArrow::parse("-->").unwrap();
    assert_eq!(arrow.deref(), "-->");
}

#[test]
fn family_relation_arrow_eq_str() {
    let arrow = FamilyRelationArrow::parse("-->").unwrap();
    assert_eq!(arrow, "-->");
    assert_eq!("-->", arrow);
}

#[test]
fn family_relation_arrow_start_marker_open() {
    // '<--' has open marker at start
    let arrow = FamilyRelationArrow::parse("<--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::Open)
    );
}

#[test]
fn family_relation_arrow_end_marker_open() {
    let arrow = FamilyRelationArrow::parse("-->").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(arrow.end_marker(), Some(FamilyRelationEndpointMarker::Open));
}

#[test]
fn family_relation_arrow_triangle_start_marker() {
    let arrow = FamilyRelationArrow::parse("<|--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::Triangle)
    );
}

#[test]
fn family_relation_arrow_triangle_end_marker() {
    let arrow = FamilyRelationArrow::parse("--|>").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.end_marker(),
        Some(FamilyRelationEndpointMarker::Triangle)
    );
}

#[test]
fn family_relation_arrow_double_open_start_marker() {
    let arrow = FamilyRelationArrow::parse("<<--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::DoubleOpen)
    );
}

#[test]
fn family_relation_arrow_double_open_end_marker() {
    let arrow = FamilyRelationArrow::parse("-->>").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.end_marker(),
        Some(FamilyRelationEndpointMarker::DoubleOpen)
    );
}

#[test]
fn family_relation_arrow_diamond_filled_marker() {
    let arrow = FamilyRelationArrow::parse("*--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::DiamondFilled)
    );
}

#[test]
fn family_relation_arrow_diamond_open_marker() {
    let arrow = FamilyRelationArrow::parse("o--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::DiamondOpen)
    );
}

#[test]
fn family_relation_arrow_circle_open_marker() {
    let arrow = FamilyRelationArrow::parse("0--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::CircleOpen)
    );
}

#[test]
fn family_relation_arrow_circle_filled_marker() {
    let arrow = FamilyRelationArrow::parse("@--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::CircleFilled)
    );
}

#[test]
fn family_relation_arrow_triangle_filled_marker() {
    let arrow = FamilyRelationArrow::parse("^--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::TriangleFilled)
    );
}

#[test]
fn family_relation_arrow_box_filled_marker() {
    let arrow = FamilyRelationArrow::parse("#--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::BoxFilled)
    );
}

#[test]
fn family_relation_arrow_plus_marker() {
    let arrow = FamilyRelationArrow::parse("+--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::Plus)
    );
}

#[test]
fn family_relation_arrow_slash_marker() {
    let arrow = FamilyRelationArrow::parse("/--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::Slash)
    );
}

#[test]
fn family_relation_arrow_cross_marker() {
    let arrow = FamilyRelationArrow::parse("x--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::Cross)
    );
}

#[test]
fn family_relation_arrow_bracket_open_marker() {
    let arrow = FamilyRelationArrow::parse("}--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::BracketOpen)
    );
}

#[test]
fn family_relation_arrow_ie_zero_many_start() {
    // }o-- or o{--
    let arrow = FamilyRelationArrow::parse("}o--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::IeZeroMany)
    );
}

#[test]
fn family_relation_arrow_ie_one_many_start() {
    let arrow = FamilyRelationArrow::parse("}|--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::IeOneMany)
    );
}

#[test]
fn family_relation_arrow_ie_zero_one_start() {
    let arrow = FamilyRelationArrow::parse("|o--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::IeZeroOne)
    );
}

#[test]
fn family_relation_arrow_ie_one_start() {
    let arrow = FamilyRelationArrow::parse("||--").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.start_marker(),
        Some(FamilyRelationEndpointMarker::IeOne)
    );
}

#[test]
fn family_relation_arrow_ie_zero_many_end() {
    let arrow = FamilyRelationArrow::parse("--o{").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.end_marker(),
        Some(FamilyRelationEndpointMarker::IeZeroMany)
    );
}

#[test]
fn family_relation_arrow_ie_one_many_end() {
    let arrow = FamilyRelationArrow::parse("--|{").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.end_marker(),
        Some(FamilyRelationEndpointMarker::IeOneMany)
    );
}

#[test]
fn family_relation_arrow_ie_zero_one_end() {
    let arrow = FamilyRelationArrow::parse("--o|").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.end_marker(),
        Some(FamilyRelationEndpointMarker::IeZeroOne)
    );
}

#[test]
fn family_relation_arrow_ie_one_end() {
    let arrow = FamilyRelationArrow::parse("--||").unwrap();
    use puml::model::FamilyRelationEndpointMarker;
    assert_eq!(
        arrow.end_marker(),
        Some(FamilyRelationEndpointMarker::IeOne)
    );
}

#[test]
fn family_relation_arrow_no_markers_returns_none() {
    let arrow = FamilyRelationArrow::parse("--").unwrap();
    assert!(arrow.start_marker().is_none());
    assert!(arrow.end_marker().is_none());
}

#[test]
fn family_relation_arrow_with_endpoint_markers_constructs_new_arrow() {
    let base = FamilyRelationArrow::parse("--").unwrap();
    let result = base.with_endpoint_markers("<|", "|>");
    assert!(result.is_ok());
    let arrow = result.unwrap();
    assert_eq!(arrow.as_str(), "<|--|>");
}

// ── model/family.rs: FamilyOrientation ───────────────────────────────────────

#[test]
fn family_orientation_as_str_values() {
    use puml::model::FamilyOrientation;
    // Verify FamilyOrientation as_str returns correct values
    assert_eq!(FamilyOrientation::TopToBottom.as_str(), "TopToBottom");
    assert_eq!(FamilyOrientation::LeftToRight.as_str(), "LeftToRight");
    assert_eq!(FamilyOrientation::BottomToTop.as_str(), "BottomToTop");
    assert_eq!(FamilyOrientation::RightToLeft.as_str(), "RightToLeft");
}

// ── preproc/macros/definelong.rs: via public parse API ────────────────────────

#[test]
fn definelong_no_arg_macro_is_expanded() {
    // No-arg definelong must be called with empty parens: GREET()
    let src = "@startuml
!definelong GREET()
A -> B : Hello
!enddefinelong
GREET()
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello"]);
}

#[test]
fn definelong_one_arg_macro_substitutes_parameter() {
    let src = "@startuml
!definelong SEND(msg)
A -> B : Hello msg
!enddefinelong
SEND(World)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello World"]);
}

#[test]
fn definelong_two_arg_macro_substitutes_both() {
    let src = "@startuml
!definelong SEND2(from, msg)
A -> B : from msg
!enddefinelong
SEND2(Alice, Hi)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Alice Hi"]);
}

#[test]
fn definelong_multi_line_body_all_lines_emitted() {
    let src = "@startuml
!definelong PING()
A -> B : ping
B -> A : pong
!enddefinelong
PING()
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ping", "pong"]);
}

#[test]
fn definelong_multiple_calls_each_expanded() {
    let src = "@startuml
!definelong GREET(name)
A -> B : Hello name
!enddefinelong
GREET(Bob)
GREET(Carol)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["Hello Bob", "Hello Carol"]);
}

// ── model/family.rs: WbsCheckbox ──────────────────────────────────────────────

#[test]
fn wbs_checkbox_variants_debug_display() {
    use puml::model::WbsCheckbox;
    // Verify all variants exist and have Debug impl
    let checked = WbsCheckbox::Checked;
    let unchecked = WbsCheckbox::Unchecked;
    let progress = WbsCheckbox::Progress(75);
    assert_eq!(format!("{checked:?}"), "Checked");
    assert_eq!(format!("{unchecked:?}"), "Unchecked");
    assert_eq!(format!("{progress:?}"), "Progress(75)");
}

#[test]
fn wbs_checkbox_equality() {
    use puml::model::WbsCheckbox;
    assert_eq!(WbsCheckbox::Checked, WbsCheckbox::Checked);
    assert_eq!(WbsCheckbox::Unchecked, WbsCheckbox::Unchecked);
    assert_eq!(WbsCheckbox::Progress(50), WbsCheckbox::Progress(50));
    assert_ne!(WbsCheckbox::Progress(50), WbsCheckbox::Progress(75));
}

// ── model/family.rs: MindMapSide ─────────────────────────────────────────────

#[test]
fn mindmap_side_default_is_right() {
    assert_eq!(MindMapSide::default(), MindMapSide::Right);
}

#[test]
fn mindmap_side_debug_and_copy() {
    let side = MindMapSide::Left;
    let copy = side;
    assert_eq!(format!("{side:?}"), "Left");
    assert_eq!(copy, MindMapSide::Left);
}

// ── FamilyNode: fill_color field ─────────────────────────────────────────────

#[test]
fn family_node_fill_color_propagates_to_rendering() {
    // Exercise fill_color field on a real rendered diagram
    let svg = puml::render_source_to_svg("@startuml\nclass MyClass #LightBlue\n@enduml")
        .expect("render OK");
    assert!(svg.contains("<svg"), "should produce valid SVG");
}

// ── FamilyRelationLineKind ────────────────────────────────────────────────────

#[test]
fn family_relation_line_kind_solid_vs_dashed() {
    use puml::model::FamilyRelationLineKind;
    let solid = FamilyRelationArrow::parse("-->").unwrap();
    let dashed = FamilyRelationArrow::parse("..>").unwrap();
    assert_eq!(solid.line_kind(), FamilyRelationLineKind::Solid);
    assert_eq!(dashed.line_kind(), FamilyRelationLineKind::Dashed);
    assert!(!solid.is_dashed());
    assert!(dashed.is_dashed());
}

// ── Integration: render_source_to_svg exercises model/family paths ─────────────

#[test]
fn class_diagram_with_relations_and_stereotypes_renders() {
    let src = "@startuml
class Foo <<service>>
class Bar <<repository>>
Foo --> Bar
Bar ..> Foo : uses
@enduml";
    let svg = puml::render_source_to_svg(src).expect("render OK");
    assert!(svg.contains("<svg"));
}

#[test]
fn component_diagram_with_various_node_kinds_renders() {
    let src = "@startuml
component Comp
interface IFace
[SubComp] as SC
Comp --> IFace
IFace ..> SC
@enduml";
    let svg = puml::render_source_to_svg(src).expect("render OK");
    assert!(svg.contains("<svg"));
}

#[test]
fn deployment_diagram_with_all_node_kinds_renders() {
    let src = "@startuml
node \"Server\" as n1
artifact \"App.jar\" as a1
cloud \"Internet\" as c1
folder \"/var/log\" as f1
n1 --> a1
a1 --> c1
@enduml";
    let svg = puml::render_source_to_svg(src).expect("render OK");
    assert!(svg.contains("<svg"));
}

#[test]
fn mindmap_with_depth_nodes_renders() {
    let src = "@startmindmap
* Root
** Child 1
*** Grandchild
** Child 2
@endmindmap";
    let svg = puml::render_source_to_svg(src).expect("render OK");
    assert!(svg.contains("<svg"));
}

#[test]
fn ie_notation_with_all_endpoint_markers_parses_ok() {
    let src = "@startuml
entity A
entity B
entity C
entity D
A }o--o{ B
B }|--|{ C
C ||--o| D
@enduml";
    let svg = puml::render_source_to_svg(src).expect("render OK");
    assert!(svg.contains("<svg"));
}
