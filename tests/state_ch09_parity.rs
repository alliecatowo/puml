use puml::model::{NormalizedDocument, StateNodeKind};

const CH09_STATE_SRC: &str = r##"@startuml
title State ch09 parity slice
state Active #pink ##[dashed]blue
Active : waits for input
state Styled #back:lightgreen;line:red;line.bold;text:blue
state Parent {
  [*] --> Child
  state Child #LightBlue
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
}
