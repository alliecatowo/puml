use puml::model::{FamilyNodeKind, NormalizedDocument};
use puml::theme::ActorStyleKind;

const CH02_USECASE_SRC: &str = r##"@startuml
skinparam actorStyle awesome
(First usecase)/
:Business Actor:/ as BA
actor/ Worker
actor Customer
(Regular usecase)
Customer --> (Regular usecase)
BA --> (First usecase)
Worker --> (First usecase)
@enduml
"##;

const CH02_NEWPAGE_SRC: &str = r##"@startuml
title Page One
actor A1
(A1 usecase) as UC1
A1 --> UC1
newpage Second Page
actor A2
(A2 usecase) as UC2
A2 --> UC2
@enduml
"##;

#[test]
fn ch02_metadata_parses_actor_style_business_and_newpage() {
    let document = puml::parser::parse(CH02_USECASE_SRC).expect("parse ch02 usecase slice");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize ch02 usecase slice")
    else {
        panic!("usecase should normalize as a family document");
    };

    let class_style = match &model.family_style {
        Some(puml::model::FamilyStyle::Class(style)) => style,
        other => panic!("expected class family style, got {other:?}"),
    };
    assert_eq!(class_style.actor_style, ActorStyleKind::Awesome);

    let business_uc = model
        .nodes
        .iter()
        .find(|node| node.name == "First usecase")
        .expect("business usecase node");
    assert_eq!(business_uc.kind, FamilyNodeKind::UseCase);
    assert!(
        business_uc
            .members
            .iter()
            .any(|m| m.text.trim() == "<<business>>"),
        "business usecase should carry <<business>> stereotype"
    );

    let business_actor = model
        .nodes
        .iter()
        .find(|node| node.alias.as_deref() == Some("BA"))
        .expect("business actor by alias");
    assert_eq!(business_actor.kind, FamilyNodeKind::Actor);
    assert!(business_actor
        .members
        .iter()
        .any(|m| m.text.trim() == "<<business>>"));

    let worker = model
        .nodes
        .iter()
        .find(|node| node.name == "Worker")
        .expect("actor/ Worker");
    assert_eq!(worker.kind, FamilyNodeKind::Actor);
    assert!(worker
        .members
        .iter()
        .any(|m| m.text.trim() == "<<business>>"));

    let newpage_doc = puml::parser::parse(CH02_NEWPAGE_SRC).expect("parse ch02 newpage slice");
    let NormalizedDocument::Family(newpage_model) =
        puml::normalize_family(newpage_doc).expect("normalize ch02 newpage slice")
    else {
        panic!("newpage usecase should normalize as family");
    };
    let pages = puml::normalize::paginate_family(&newpage_model);
    assert_eq!(pages.len(), 2, "newpage should split into two pages");
    assert!(
        pages[0].nodes.iter().any(|n| n.name == "A1"),
        "first page should contain A1"
    );
    assert!(
        pages[1].nodes.iter().any(|n| n.name == "A2"),
        "second page should contain A2"
    );
    assert_eq!(pages[1].title.as_deref(), Some("Second Page"));
}

#[test]
fn ch02_render_emits_actor_styles_and_business_shapes() {
    let svg = puml::render_source_to_svg(CH02_USECASE_SRC).expect("render ch02 usecase slice");

    assert!(svg.contains("data-actor-style=\"awesome\""));
    assert!(svg.contains("data-actor-business=\"true\""));
    assert!(svg.contains("data-usecase-kind=\"business\""));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("<ellipse"));
}

#[test]
fn ch02_newpage_render_returns_multiple_svgs() {
    let pages = puml::render_source_to_svgs(CH02_NEWPAGE_SRC).expect("render ch02 newpage pages");
    assert_eq!(pages.len(), 2);
    assert!(pages[0].contains("A1"));
    assert!(pages[1].contains("A2"));
    assert!(pages[1].contains("Second Page") || pages[1].contains("A2 usecase"));
}
