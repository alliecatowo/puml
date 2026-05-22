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
        .find(|node| node.label.as_deref() == Some("note right: keep evidence"))
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
