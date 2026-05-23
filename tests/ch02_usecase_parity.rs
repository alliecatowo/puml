//! Chapter 2 use-case parity tests.
//!
//! Covers the focused slice implemented here:
//! - `skinparam actorStyle awesome|hollow` changes use-case actor glyphs (2.3).
//! - Business use-case and actor variants parse/normalize/render distinctly (2.15).

use puml::model::FamilyNodeKind;
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

const BUSINESS_VARIANTS_SRC: &str = r##"@startuml
left to right direction
(Checkout)/ as UC1
usecase/ "Approve refund" as UC2 #AliceBlue
:Customer:/ as Customer
actor/ :Sales Manager: as Manager
Customer --> UC1
Manager --> UC2
@enduml
"##;

const INLINE_ELEMENT_STYLE_SRC: &str = r##"@startuml
left to right direction
actor Shopper #back:Wheat;line:DarkGreen;line.bold;text:DarkBlue
usecase "Styled checkout" as Checkout #pink;line:red;line.dashed;line.bold;text:blue
Shopper --> Checkout
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

#[test]
fn business_variants_are_normalized_to_distinct_kinds() {
    let document = puml::parser::parse(BUSINESS_VARIANTS_SRC).expect("parse business variants");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize business variants")
    else {
        panic!("business usecase diagram should normalize as Family");
    };

    assert!(
        model
            .nodes
            .iter()
            .any(|node| node.name == "Checkout" && node.kind == FamilyNodeKind::BusinessUseCase),
        "trailing slash usecase should normalize as BusinessUseCase: {:?}",
        model.nodes
    );
    assert!(
        model.nodes.iter().any(|node| {
            node.alias.as_deref() == Some("Manager") && node.kind == FamilyNodeKind::BusinessActor
        }),
        "actor/ declaration should normalize as BusinessActor: {:?}",
        model.nodes
    );
    assert!(
        model.nodes.iter().any(|node| {
            node.alias.as_deref() == Some("Customer") && node.kind == FamilyNodeKind::BusinessActor
        }),
        "colon actor trailing slash should normalize as BusinessActor: {:?}",
        model.nodes
    );
}

#[test]
fn business_variants_render_business_shapes() {
    let svg = puml::render_source_to_svg(BUSINESS_VARIANTS_SRC).expect("render business variants");

    assert!(
        svg.contains("class=\"uml-business-usecase\""),
        "business usecases should render as rounded rectangles; svg={svg}"
    );
    assert!(
        svg.contains("class=\"uml-business-actor\""),
        "business actors should render a distinct boxed actor glyph; svg={svg}"
    );
    assert!(
        svg.contains("fill=\"#f0f8ff\""),
        "business usecase should preserve inline fill color; svg={svg}"
    );
}

#[test]
fn inline_element_style_renders_usecase_node_style() {
    let svg = puml::render_source_to_svg(INLINE_ELEMENT_STYLE_SRC)
        .expect("render inline usecase node style");

    assert!(
        svg.contains("<ellipse")
            && svg.contains("fill=\"#ffc0cb\"")
            && svg.contains("stroke=\"#ff0000\""),
        "usecase inline fill and line color should reach ellipse geometry; svg={svg}"
    );
    assert!(
        svg.contains("stroke-dasharray=\"5 3\""),
        "usecase line.dashed should render dashed node border; svg={svg}"
    );
    assert!(
        svg.contains("stroke-width=\"3\""),
        "usecase line.bold should render a thicker node border; svg={svg}"
    );
    assert!(
        svg.contains("fill=\"#0000ff\">Styled checkout</text>"),
        "usecase text: color should style the label; svg={svg}"
    );
}

#[test]
fn inline_element_style_renders_actor_node_style() {
    let svg =
        puml::render_source_to_svg(INLINE_ELEMENT_STYLE_SRC).expect("render inline actor style");

    assert!(
        svg.contains("stroke=\"#006400\""),
        "actor line: color should style the actor glyph stroke; svg={svg}"
    );
    assert!(
        svg.contains("fill=\"#00008b\">Shopper</text>"),
        "actor text: color should style the actor label; svg={svg}"
    );
}
