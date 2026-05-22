//! Chapter 2 use-case parity tests.
//!
//! Covers the focused slice implemented here:
//! - `skinparam actorStyle awesome|hollow` changes use-case actor glyphs (2.3).

use puml::model::{FamilyStyle, NormalizedDocument};
use puml::theme::ActorStyle;

const ACTOR_STYLE_AWESOME_SRC: &str = r##"@startuml
skinparam actorStyle awesome
(Login)
actor User
User --> (Login)
@enduml
"##;

const ACTOR_STYLE_HOLLOW_SRC: &str = r##"@startuml
skinparam actorStyle hollow
(Login)
actor User
User --> (Login)
@enduml
"##;

#[test]
fn actor_style_awesome_renders_distinct_actor_glyph() {
    let svg =
        puml::render_source_to_svg(ACTOR_STYLE_AWESOME_SRC).expect("render awesome actorStyle");

    assert!(
        svg.contains("data-uml-actor-style=\"awesome\""),
        "awesome actorStyle should mark the alternate glyph; svg={svg}"
    );
    assert!(
        svg.contains("class=\"uml-actor-glyph\""),
        "awesome actorStyle should render an actor glyph path/circle; svg={svg}"
    );
}

#[test]
fn actor_style_hollow_renders_distinct_actor_glyph() {
    let svg = puml::render_source_to_svg(ACTOR_STYLE_HOLLOW_SRC).expect("render hollow actorStyle");

    assert!(
        svg.contains("data-uml-actor-style=\"hollow\""),
        "hollow actorStyle should mark the alternate glyph; svg={svg}"
    );
    assert!(
        svg.contains("fill=\"none\""),
        "hollow actorStyle should render an outlined glyph; svg={svg}"
    );
}

#[test]
fn actor_style_is_preserved_in_family_style() {
    let document = puml::parser::parse(ACTOR_STYLE_AWESOME_SRC).expect("parse actorStyle");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize actorStyle")
    else {
        panic!("usecase diagram should normalize as Family");
    };

    let Some(FamilyStyle::Class(style)) = model.family_style else {
        panic!("usecase family should carry ClassStyle");
    };

    assert_eq!(style.actor_style, ActorStyle::Awesome);
    assert!(
        model.warnings.is_empty(),
        "supported actorStyle should not warn: {:?}",
        model.warnings
    );
}

#[test]
fn actor_style_invalid_value_warns() {
    let src = r##"@startuml
skinparam actorStyle neon
(Login)
actor User
@enduml
"##;
    let document = puml::parser::parse(src).expect("parse invalid actorStyle");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize invalid actorStyle")
    else {
        panic!("usecase diagram should normalize as Family");
    };

    assert!(
        model
            .warnings
            .iter()
            .any(|warning| warning.message.contains("W_SKINPARAM_UNSUPPORTED_VALUE")),
        "invalid actorStyle value should produce unsupported-value warning: {:?}",
        model.warnings
    );
}
