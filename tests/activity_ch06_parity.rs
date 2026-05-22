use puml::model::{FamilyNodeKind, NormalizedDocument};

const PARITY_SRC: &str = r##"@startuml
title Activity ch06 parity slice
start
#LightBlue:Collect;
-[#red,dashed]-> reviewed;
:Review;
note right: keep evidence
#pink:(A)
partition #LightYellow Ops {
  :Ship;
}
detach
:After detach;
@enduml
"##;

#[test]
fn activity_ch06_metadata_survives_normalization() {
    let document = puml::parser::parse(PARITY_SRC).expect("parse activity parity slice");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize activity parity slice")
    else {
        panic!("activity should normalize as a family document");
    };

    let collect = model
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("Collect"))
        .expect("colored action node");
    assert_eq!(collect.fill_color.as_deref(), Some("LightBlue"));

    let note = model
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("keep evidence"))
        .expect("activity note node");
    assert_eq!(note.kind, FamilyNodeKind::Note);

    let connector = model
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("(A)"))
        .expect("connector node");
    assert_eq!(connector.kind, FamilyNodeKind::ActivityAction);
    assert_eq!(connector.fill_color.as_deref(), Some("pink"));

    let lane = model
        .nodes
        .iter()
        .find(|node| {
            node.label.as_deref() == Some("Ops")
                && matches!(node.kind, FamilyNodeKind::ActivityPartition)
        })
        .expect("colored partition marker");
    assert_eq!(lane.fill_color.as_deref(), Some("LightYellow"));
}

#[test]
fn activity_ch06_render_applies_arrow_color_note_connector_and_detach() {
    let svg = puml::render_source_to_svg(PARITY_SRC).expect("render activity parity slice");

    assert!(svg.contains("fill=\"LightBlue\""));
    assert!(svg.contains("fill=\"LightYellow\""));
    assert!(svg.contains("fill=\"pink\""));
    assert!(svg.contains("stroke=\"red\""));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(svg.contains(">reviewed<"));
    assert!(svg.contains("data-activity-kind=\"Note\""));
    assert!(svg.contains("data-activity-kind=\"Connector\""));
    assert!(svg.contains("data-activity-kind=\"Detach\""));
    assert!(svg.contains(">After detach<"));

    assert!(
        !svg.contains("y1=\"488\""),
        "detach should suppress the outgoing arrow into the following action"
    );
}

#[test]
fn activity_inline_note_renders_payload_without_directive_prefix() {
    let src = include_str!("fixtures/non_sequence/valid_activity_inline_note_text.puml");

    let document = puml::parser::parse(src).expect("parse activity inline note");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize activity inline note")
    else {
        panic!("activity should normalize as a family document");
    };
    let note = model
        .nodes
        .iter()
        .find(|node| node.kind == FamilyNodeKind::Note)
        .expect("activity note node");
    assert_eq!(note.label.as_deref(), Some("baseline note"));
    assert!(
        note.alias.as_deref().is_some_and(|alias| {
            alias.contains("activity::Note")
                && alias.contains("position=right")
                && alias.contains("lane=default")
        }),
        "activity notes should stay out of the active partition lane"
    );

    let svg = puml::render_source_to_svg(src).expect("render activity inline note");
    assert!(svg.contains("data-activity-kind=\"Note\" data-activity-lane=\"default\""));
    assert!(svg.contains(">baseline note<"));
    assert!(!svg.contains(">note right: baseline note<"));
    assert!(
        svg.contains("<line x1=\"344\" y1=\"406\" x2=\"344\" y2=\"415\"")
            && svg.contains("<line x1=\"344\" y1=\"415\" x2=\"136\" y2=\"415\"")
            && svg.contains("<line x1=\"136\" y1=\"415\" x2=\"136\" y2=\"424\""),
        "annotation connector should route from detach to the out-of-lane note"
    );
}
