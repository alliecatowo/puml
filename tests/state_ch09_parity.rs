use puml::model::{NormalizedDocument, StateNodeKind};
use std::fs;

const CH09_STATE_SRC: &str = r##"@startuml
title State ch09 parity slice
hide empty description
state Active #pink ##[dashed]blue
Active : waits for input
state Empty
state Styled #back:lightgreen;line:red;line.bold;text:blue
state Parent {
  state entryIn <<entryPoint>>
  state exitOut <<exitPoint>>
  [*] --> Child
  entryIn --> Child : enter
  state Child #LightBlue
  Child --> exitOut : leave
  json $payload {
    "fruit": "Apple",
    "count": 3
  }
  yaml $meta {
    fruit: Apple
    count: 3
  }
}
state entry1 <<entryPoint>>
state exit1 <<exitPoint>>
state in1 <<inputPin>>
state out1 <<outputPin>>
state expIn <<expansionInput>>
state expOut <<expansionOutput>>
state Done[H*]
Active -[#DD00AA,dashed]-> entry1 : colored
entry1 --> Parent
Parent --> exit1
exit1 --> Styled
Styled --> Done[H*]
note left of Active: attached note
note on link: link note
note right on link: second link note
note right of Styled
  multiline attached note
end note
note on link
  multiline link note
end note
json $cfg {
  "outer": {
    "ok": true
  }
}
yaml $settings {
  mode: ready
  retries: 2
}
@enduml
"##;

#[test]
fn state_ch09_metadata_preserves_notes_ports_styles_and_json() {
    let document = puml::parser::parse(CH09_STATE_SRC).expect("parse state ch09 slice");
    let NormalizedDocument::State(model) =
        puml::normalize_family(document).expect("normalize state ch09 slice")
    else {
        panic!("state should normalize as a state document");
    };
    assert!(model.hide_empty_description);

    let active = model
        .nodes
        .iter()
        .find(|node| node.name == "Active")
        .expect("Active state");
    assert_eq!(active.style.fill_color.as_deref(), Some("pink"));
    assert_eq!(active.style.border_color.as_deref(), Some("blue"));
    assert!(active.style.border_dashed);
    assert!(active
        .internal_actions
        .iter()
        .any(|action| action.kind == "waits for input"));

    let styled = model
        .nodes
        .iter()
        .find(|node| node.name == "Styled")
        .expect("Styled state");
    assert_eq!(styled.style.fill_color.as_deref(), Some("lightgreen"));
    assert_eq!(styled.style.border_color.as_deref(), Some("red"));
    assert_eq!(styled.style.border_thickness, Some(3));
    assert_eq!(styled.style.text_color.as_deref(), Some("blue"));

    for (name, kind) in [
        ("entry1", StateNodeKind::EntryPoint),
        ("exit1", StateNodeKind::ExitPoint),
        ("in1", StateNodeKind::InputPin),
        ("out1", StateNodeKind::OutputPin),
        ("expIn", StateNodeKind::ExpansionInput),
        ("expOut", StateNodeKind::ExpansionOutput),
        ("Done[H*]", StateNodeKind::HistoryDeep),
    ] {
        let node = model
            .nodes
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("missing state node {name}"));
        assert_eq!(&node.kind, &kind, "state node kind for {name}");
    }

    assert!(model
        .nodes
        .iter()
        .any(|node| node.kind == StateNodeKind::Note
            && node.display.as_deref() == Some("attached note")));
    assert!(model.nodes.iter().any(
        |node| node.kind == StateNodeKind::Note && node.display.as_deref() == Some("link note")
    ));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.kind == StateNodeKind::Note
            && node.display.as_deref() == Some("second link note")));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.kind == StateNodeKind::Note
            && node.display.as_deref() == Some("multiline attached note")));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.kind == StateNodeKind::Note
            && node.display.as_deref() == Some("multiline link note")));
    assert!(model.transitions.iter().any(|transition| {
        transition.to.starts_with("__state_note_")
            && transition
                .direction
                .as_deref()
                .is_some_and(|direction| direction.starts_with("on-link|over|"))
    }));
    assert!(model.transitions.iter().any(|transition| {
        transition.to.starts_with("__state_note_")
            && transition
                .direction
                .as_deref()
                .is_some_and(|direction| direction.starts_with("on-link|right|"))
    }));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.kind == StateNodeKind::JsonProjection
            && node
                .display
                .as_deref()
                .is_some_and(|text| text.contains("$cfg"))));
    assert!(model
        .nodes
        .iter()
        .any(|node| node.kind == StateNodeKind::JsonProjection
            && node.stereotype.as_deref() == Some("yaml")
            && node
                .display
                .as_deref()
                .is_some_and(|text| text.contains("$settings"))));
    let parent = model
        .nodes
        .iter()
        .find(|node| node.name == "Parent")
        .expect("Parent composite");
    assert!(parent
        .regions
        .iter()
        .flatten()
        .any(|node| node.name == "entryIn" && node.kind == StateNodeKind::EntryPoint));
    assert!(parent
        .regions
        .iter()
        .flatten()
        .any(|node| node.name == "exitOut" && node.kind == StateNodeKind::ExitPoint));
    assert!(parent
        .regions
        .iter()
        .flatten()
        .any(|node| node.kind == StateNodeKind::JsonProjection
            && node
                .display
                .as_deref()
                .is_some_and(|text| text.contains("$payload"))));
    assert!(parent.regions.iter().flatten().any(|node| {
        node.kind == StateNodeKind::JsonProjection
            && node.stereotype.as_deref() == Some("yaml")
            && node
                .display
                .as_deref()
                .is_some_and(|text| text.contains("$meta"))
    }));
}

#[test]
fn state_ch09_render_emits_visual_shapes_styles_and_labels() {
    let svg = puml::render_source_to_svg(CH09_STATE_SRC).expect("render state ch09 slice");

    assert!(svg.contains("data-state-kind=\"entry-point\""));
    assert!(svg.contains("data-state-kind=\"exit-point\""));
    assert!(svg.contains("data-state-kind=\"input-pin\""));
    assert!(svg.contains("data-state-kind=\"output-pin\""));
    assert!(svg.contains("data-state-kind=\"expansion-input\""));
    assert!(svg.contains("data-state-kind=\"expansion-output\""));
    assert!(svg.contains("data-state-kind=\"history-deep\""));
    assert!(svg.contains("class=\"state-note\""));
    assert!(svg.contains("class=\"state-note-connector\""));
    assert!(svg.contains(">attached note<"));
    assert!(svg.contains(">link note<"));
    assert!(svg.contains(">second link note<"));
    assert!(svg.contains(">multiline attached note<"));
    assert!(svg.contains(">multiline link note<"));
    assert!(svg.contains("class=\"state-json-projection\""));
    assert!(svg.contains("data-state-projection-format=\"json\""));
    assert!(svg.contains("data-state-projection-format=\"yaml\""));
    assert!(svg.contains(">fruit: Apple<"));
    assert!(svg.contains(">mode: ready<"));
    assert!(svg.contains("fill=\"pink\""));
    assert!(svg.contains("stroke=\"blue\""));
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("stroke=\"#dd00aa\""));
    assert!(svg.contains(">colored<"));

    let doc = roxmltree::Document::parse(&svg).expect("state ch09 SVG should parse");
    assert!(
        !state_elements_after_metadata(&doc, "Empty")
            .iter()
            .any(|node| node.has_tag_name("line")),
        "hide empty description should keep an actionless state as a simple box"
    );
    assert!(
        state_elements_after_metadata(&doc, "Active")
            .iter()
            .any(|node| node.has_tag_name("line")),
        "states with descriptions/internal actions should keep their compartment divider"
    );
}

#[test]
fn hide_empty_description_before_transition_detects_state_family() {
    let src = r##"@startuml
hide empty description
[*] --> Empty
Empty --> [*]
@enduml
"##;
    let document = puml::parser::parse(src).expect("parse hide empty description before state");
    let NormalizedDocument::State(model) =
        puml::normalize_family(document).expect("normalize hide empty description before state")
    else {
        panic!("state should normalize as a state document");
    };
    assert!(model.hide_empty_description);
    assert!(model.nodes.iter().any(|node| node.name == "Empty"));
}

#[test]
fn state_entry_and_exit_points_snap_to_composite_boundary() {
    let svg = puml::render_source_to_svg(CH09_STATE_SRC).expect("render state ch09 slice");
    let doc = roxmltree::Document::parse(&svg).expect("state ch09 SVG should parse");

    let parent_rect = state_shape_after_metadata(&doc, "Parent", "rect");
    let parent_x = svg_attr_i32(parent_rect, "x");
    let parent_right = parent_x + svg_attr_i32(parent_rect, "width");

    let entry_circle = state_shape_after_metadata(&doc, "entryIn", "circle");
    assert_eq!(
        svg_attr_i32(entry_circle, "cx"),
        parent_x,
        "entryPoint child should be centered on the composite's left boundary"
    );

    let exit_circle = state_shape_after_metadata(&doc, "exitOut", "circle");
    assert_eq!(
        svg_attr_i32(exit_circle, "cx"),
        parent_right,
        "exitPoint child should be centered on the composite's right boundary"
    );
}

#[test]
fn state_transition_labels_clear_crossing_arrow_lanes_issue_483() {
    let src = fs::read_to_string("docs/examples/state/02_transitions.puml")
        .expect("state transition example");
    let svg = puml::render_source_to_svg(&src).expect("render state transition example");
    let doc = roxmltree::Document::parse(&svg).expect("state transition SVG should parse");

    let revise_edge = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("Rejected")
                && node.attribute("data-state-to") == Some("Draft")
        })
        .expect("Rejected -> Draft edge should render");
    let revise_path = revise_edge
        .attribute("d")
        .expect("Rejected -> Draft edge should have path data");
    assert!(
        revise_path.contains("L 298 100"),
        "upward diagonal transition should use the target-side lane; d={revise_path:?}"
    );
    assert!(
        !revise_path.contains("L 170 193 L 310 193"),
        "Rejected -> Draft must not reuse the Submitted -> Approved horizontal lane"
    );

    let submit_label = state_label_node(&doc, "submit()");
    let revise_label = state_label_node(&doc, "revise()");
    let approve_label = state_label_node(&doc, "approve()");

    let submit_right = state_label_center_x(submit_label) + state_label_half_width("submit()");
    let revise_left = state_label_center_x(revise_label) - state_label_half_width("revise()");
    assert!(
        submit_right < 298,
        "submit() label should stay left of the upward revise lane"
    );
    assert!(
        revise_left > 298,
        "revise() label should be offset clear of its own vertical shaft"
    );
    assert!(
        state_label_center_y(approve_label) > state_label_center_y(revise_label),
        "approve() and revise() labels should not stack in the crossing corridor"
    );
}

fn state_label_node<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    label: &str,
) -> roxmltree::Node<'a, 'input> {
    doc.descendants()
        .find(|node| node.has_tag_name("text") && node.attribute("data-state-label") == Some(label))
        .unwrap_or_else(|| panic!("missing state transition label {label}"))
}

fn state_label_center_x(node: roxmltree::Node<'_, '_>) -> i32 {
    node.attribute("x")
        .expect("state transition label should have x")
        .parse()
        .expect("state transition label x should be numeric")
}

fn state_label_center_y(node: roxmltree::Node<'_, '_>) -> i32 {
    node.attribute("y")
        .expect("state transition label should have y")
        .parse()
        .expect("state transition label y should be numeric")
}

fn state_label_half_width(label: &str) -> i32 {
    (label.chars().count() as i32 * 6) / 2
}

fn state_shape_after_metadata<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    node_name: &str,
    shape_name: &str,
) -> roxmltree::Node<'a, 'input> {
    let elements: Vec<_> = doc.descendants().filter(|node| node.is_element()).collect();
    let metadata_idx = elements
        .iter()
        .position(|node| {
            node.has_tag_name("metadata") && node.attribute("data-state-node") == Some(node_name)
        })
        .unwrap_or_else(|| panic!("missing metadata for state node {node_name}"));
    elements
        .iter()
        .skip(metadata_idx + 1)
        .find(|node| node.has_tag_name(shape_name))
        .copied()
        .unwrap_or_else(|| panic!("missing {shape_name} after state metadata {node_name}"))
}

fn state_elements_after_metadata<'a, 'input>(
    doc: &'a roxmltree::Document<'input>,
    node_name: &str,
) -> Vec<roxmltree::Node<'a, 'input>> {
    let elements: Vec<_> = doc.descendants().filter(|node| node.is_element()).collect();
    let metadata_idx = elements
        .iter()
        .position(|node| {
            node.has_tag_name("metadata") && node.attribute("data-state-node") == Some(node_name)
        })
        .unwrap_or_else(|| panic!("missing metadata for state node {node_name}"));
    let next_metadata_idx = elements
        .iter()
        .enumerate()
        .skip(metadata_idx + 1)
        .find_map(|(idx, node)| node.has_tag_name("metadata").then_some(idx))
        .unwrap_or(elements.len());
    elements[metadata_idx + 1..next_metadata_idx].to_vec()
}

fn svg_attr_i32(node: roxmltree::Node<'_, '_>, attr: &str) -> i32 {
    node.attribute(attr)
        .unwrap_or_else(|| panic!("missing SVG attr {attr}"))
        .parse()
        .unwrap_or_else(|_| panic!("SVG attr {attr} should be an integer"))
}

#[test]
fn state_history_endpoints_scope_to_composite_owner() {
    let src = r##"@startuml
state Running
state Session {
  [*] --> Idle
  Idle --> Busy
}
Running --> Session[H] : resume
Session[H*] --> Running : deep-resume
@enduml
"##;
    let document = puml::parser::parse(src).expect("parse scoped history slice");
    let NormalizedDocument::State(model) =
        puml::normalize_family(document).expect("normalize scoped history slice")
    else {
        panic!("state should normalize as a state document");
    };

    let session = model
        .nodes
        .iter()
        .find(|node| node.name == "Session")
        .expect("Session composite");
    let region_nodes: Vec<&str> = session
        .regions
        .iter()
        .flatten()
        .map(|node| node.name.as_str())
        .collect();
    assert!(region_nodes.contains(&"Session[H]"));
    assert!(region_nodes.contains(&"Session[H*]"));

    let svg = puml::render_source_to_svg(src).expect("render scoped history slice");
    assert!(svg.contains("data-state-from=\"Running\" data-state-to=\"Session[H]\""));
    assert!(svg.contains("data-state-from=\"Session[H*]\" data-state-to=\"Running\""));
}
