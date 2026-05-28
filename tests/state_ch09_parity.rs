use puml::model::{NormalizedDocument, StateNodeKind};
use std::fs;

// ── Feature: inline transition color ────────────────────────────────────────
// PlantUML spec §3.36: tail-form inline style on state transitions.
// `From --> To #red : event` sets the transition stroke to red.
// `From --> To #line:blue;line.bold : event` sets blue bold stroke.
// Before this fix the `#red` / `#line:blue` was incorrectly appended to the
// target node name, creating phantom nodes like "To #red".
#[test]
fn state_transition_inline_tail_color_parses_correctly() {
    let src = r#"@startuml
[*] --> Idle
Idle --> Active #red : start
Active --> Idle #line:blue;line.bold : stop
Active --> Done #CC00FF
@enduml"#;
    let document = puml::parser::parse(src).expect("parse inline-color transitions");
    let NormalizedDocument::State(model) =
        puml::normalize_family(document).expect("normalize inline-color transitions")
    else {
        panic!("should normalize as State");
    };

    // No phantom nodes: only Idle, Active, Done, [*] and optionally [*]__end
    let node_names: Vec<&str> = model.nodes.iter().map(|n| n.name.as_str()).collect();
    assert!(
        !node_names.iter().any(|n| n.contains("#")),
        "no node name should contain '#'; got: {node_names:?}"
    );
    assert!(
        node_names.contains(&"Idle"),
        "Idle must exist as a clean node"
    );
    assert!(
        node_names.contains(&"Active"),
        "Active must exist as a clean node"
    );
    assert!(
        node_names.contains(&"Done"),
        "Done must exist as a clean node"
    );

    // Transition colours are captured
    let idle_to_active = model
        .transitions
        .iter()
        .find(|t| t.from == "Idle" && t.to == "Active")
        .expect("Idle→Active transition");
    assert_eq!(
        idle_to_active.line_color.as_deref(),
        Some("#ff0000"),
        "Idle→Active should be red"
    );
    assert_eq!(
        idle_to_active.label.as_deref(),
        Some("start"),
        "Idle→Active label should be 'start'"
    );

    let active_to_idle = model
        .transitions
        .iter()
        .find(|t| t.from == "Active" && t.to == "Idle")
        .expect("Active→Idle transition");
    assert_eq!(
        active_to_idle.line_color.as_deref(),
        Some("#0000ff"),
        "Active→Idle should be blue"
    );
    assert!(
        active_to_idle.thickness.is_some(),
        "Active→Idle should be bold"
    );

    let active_to_done = model
        .transitions
        .iter()
        .find(|t| t.from == "Active" && t.to == "Done")
        .expect("Active→Done transition");
    assert!(
        active_to_done.line_color.is_some(),
        "Active→Done should carry a line color"
    );
}

#[test]
fn state_transition_inline_tail_color_renders_correct_strokes() {
    let src = r#"@startuml
[*] --> Idle
Idle --> Active #red : start
Active --> Idle #line:blue;line.bold : stop
@enduml"#;
    let svg = puml::render_source_to_svg(src).expect("render inline-color state transitions");

    // State nodes must NOT include "#" in their name attribute
    assert!(
        !svg.contains("data-state-node=\"Active #"),
        "phantom 'Active #...' node must not appear in SVG"
    );
    assert!(
        !svg.contains("data-state-node=\"Idle #"),
        "phantom 'Idle #...' node must not appear in SVG"
    );

    // The correct stroke colour must appear on the Idle→Active path
    assert!(
        svg.contains("data-state-from=\"Idle\" data-state-to=\"Active\""),
        "Idle→Active path must be present"
    );
    // Red is rendered as #ff0000
    assert!(
        svg.contains("stroke=\"#ff0000\""),
        "red transition stroke (#ff0000) must appear in SVG"
    );
    // Bold blue
    assert!(
        svg.contains("stroke=\"#0000ff\""),
        "blue transition stroke (#0000ff) must appear in SVG"
    );
    assert!(
        svg.contains("stroke-width=\"3\""),
        "bold (stroke-width=3) transition must appear in SVG"
    );
}

// ── Feature: <<terminate>> pseudostate ──────────────────────────────────────
// UML terminate pseudostate: rendered as a circle with an X cross inside.
#[test]
fn state_terminate_stereotype_normalizes_to_correct_kind() {
    let src = r#"@startuml
state T <<terminate>>
[*] --> S
S --> T : done
@enduml"#;
    let document = puml::parser::parse(src).expect("parse terminate stereotype");
    let NormalizedDocument::State(model) =
        puml::normalize_family(document).expect("normalize terminate stereotype")
    else {
        panic!("should normalize as State");
    };

    let t_node = model
        .nodes
        .iter()
        .find(|n| n.name == "T")
        .expect("T node must exist");
    assert_eq!(
        t_node.kind,
        StateNodeKind::Terminate,
        "<<terminate>> should map to StateNodeKind::Terminate"
    );
}

#[test]
fn state_terminate_stereotype_renders_circle_with_x() {
    let src = r#"@startuml
state T <<terminate>>
[*] --> S
S --> T : done
@enduml"#;
    let svg = puml::render_source_to_svg(src).expect("render terminate state");

    assert!(
        svg.contains("data-state-kind=\"terminate\""),
        "terminate node must appear with data-state-kind=\"terminate\""
    );
    // Must NOT fall back to a rectangle (no <rect> immediately after terminate metadata)
    let doc = roxmltree::Document::parse(&svg).expect("SVG should parse");
    let elements: Vec<_> = doc.descendants().filter(|n| n.is_element()).collect();
    let term_meta_idx = elements
        .iter()
        .position(|n| {
            n.has_tag_name("metadata") && n.attribute("data-state-kind") == Some("terminate")
        })
        .expect("terminate metadata must exist");
    // The first shape after the metadata must be a circle (the outer ring), not a rect
    let first_shape = elements
        .iter()
        .skip(term_meta_idx + 1)
        .find(|n| matches!(n.tag_name().name(), "rect" | "circle" | "polygon" | "line"))
        .expect("a shape must follow terminate metadata");
    assert_eq!(
        first_shape.tag_name().name(),
        "circle",
        "terminate pseudostate must render as a circle, not a {:?}",
        first_shape.tag_name().name()
    );
    // The X cross lines must also be present in the SVG
    let has_cross_lines = elements
        .iter()
        .skip(term_meta_idx + 1)
        .take(5) // look at a few elements right after the circle
        .any(|n| n.has_tag_name("line"));
    assert!(
        has_cross_lines,
        "terminate pseudostate must render with X cross lines"
    );
}

// ── Feature: <<sdlreceive>> / <<sdlsend>> pseudostates ──────────────────────
// SDL signal stereotypes: distinctive polygon shapes instead of rounded rectangles.
#[test]
fn state_sdl_stereotypes_normalize_to_correct_kinds() {
    let src = r#"@startuml
state R <<sdlreceive>>
state S <<sdlsend>>
[*] --> R
R --> S
@enduml"#;
    let document = puml::parser::parse(src).expect("parse SDL stereotypes");
    let NormalizedDocument::State(model) =
        puml::normalize_family(document).expect("normalize SDL stereotypes")
    else {
        panic!("should normalize as State");
    };

    let r_node = model.nodes.iter().find(|n| n.name == "R").expect("R node");
    let s_node = model.nodes.iter().find(|n| n.name == "S").expect("S node");
    assert_eq!(
        r_node.kind,
        StateNodeKind::SdlReceive,
        "<<sdlreceive>> should map to StateNodeKind::SdlReceive"
    );
    assert_eq!(
        s_node.kind,
        StateNodeKind::SdlSend,
        "<<sdlsend>> should map to StateNodeKind::SdlSend"
    );
}

#[test]
fn state_sdl_stereotypes_render_polygon_shapes() {
    let src = r#"@startuml
state R <<sdlreceive>>
state S <<sdlsend>>
[*] --> R
R --> S
@enduml"#;
    let svg = puml::render_source_to_svg(src).expect("render SDL state stereotypes");

    assert!(
        svg.contains("data-state-kind=\"sdl-receive\""),
        "sdl-receive node must appear with correct data-state-kind"
    );
    assert!(
        svg.contains("data-state-kind=\"sdl-send\""),
        "sdl-send node must appear with correct data-state-kind"
    );

    let doc = roxmltree::Document::parse(&svg).expect("SVG should parse");
    let elements: Vec<_> = doc.descendants().filter(|n| n.is_element()).collect();

    for (kind, name) in [("sdl-receive", "R"), ("sdl-send", "S")] {
        let meta_idx = elements
            .iter()
            .position(|n| {
                n.has_tag_name("metadata") && n.attribute("data-state-kind") == Some(kind)
            })
            .unwrap_or_else(|| panic!("{kind} metadata must exist"));
        let first_shape = elements
            .iter()
            .skip(meta_idx + 1)
            .find(|n| matches!(n.tag_name().name(), "rect" | "circle" | "polygon"))
            .unwrap_or_else(|| panic!("a shape must follow {kind} metadata"));
        assert_eq!(
            first_shape.tag_name().name(),
            "polygon",
            "{name} (<<{kind}>>) must render as a polygon, not a {:?}",
            first_shape.tag_name().name()
        );
    }
}

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

#[test]
fn state_transition_labels_render_plantuml_line_break_escapes() {
    let svg = puml::render_source_to_svg(
        r"@startuml
[*] --> Source : file or stdin\ninput
Source --> Parsed : preprocess\lparse
Parsed --> [*] : output\rwritten
@enduml
",
    )
    .expect("render escaped state transition labels");
    let doc = roxmltree::Document::parse(&svg).expect("state SVG should parse");

    assert_eq!(
        state_label_tspan_text(&doc, r"file or stdin\ninput"),
        ["file or stdin", "input"]
    );
    assert_eq!(
        state_label_tspan_text(&doc, r"preprocess\lparse"),
        ["preprocess", "parse"]
    );
    assert_eq!(
        state_label_tspan_text(&doc, r"output\rwritten"),
        ["output", "written"]
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

fn state_label_tspan_text<'a>(doc: &'a roxmltree::Document<'_>, label: &str) -> Vec<&'a str> {
    state_label_node(doc, label)
        .children()
        .filter(|node| node.has_tag_name("tspan"))
        .filter_map(|node| node.text())
        .collect()
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
