use puml::model::{FamilyNodeKind, NormalizedDocument};

const CH04_DIAMOND_SRC: &str = r##"@startuml
object o1
object o2
diamond dia
o1 --> dia
o2 --> dia
@enduml
"##;

const CH04_MAP_SRC: &str = r##"@startuml
map CapitalCity {
  UK => London
  USA => Washington
}
object NewYork
NewYork --> CapitalCity::USA
@enduml
"##;

const CH04_PERT_SRC: &str = r##"@startuml
left to right direction
map Kick.Off {
}
map task.1 {
  Start => End
}
Kick.Off --> task.1 : Label 1
@enduml
"##;

const CH04_PARITY_SRC: &str = include_str!("../docs/examples/object/05_ch04_parity.puml");

fn normalize_object(src: &str) -> puml::model::FamilyDocument {
    let document = puml::parser::parse(src).expect("parse ch04 object slice");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize ch04 object slice")
    else {
        panic!("object diagram should normalize as family");
    };
    model
}

#[test]
fn ch04_diamond_parses_as_hub_kind() {
    let model = normalize_object(CH04_DIAMOND_SRC);
    let dia = model
        .nodes
        .iter()
        .find(|n| n.name == "dia")
        .expect("diamond hub node");
    assert_eq!(dia.kind, FamilyNodeKind::Diamond);
    assert!(
        !model
            .nodes
            .iter()
            .any(|n| { n.name == "dia" && n.members.iter().any(|m| m.text == "<<diamond>>") }),
        "diamond hub must not be a stereotyped class box"
    );
}

#[test]
fn ch04_map_parses_arrow_rows_and_qualified_relation() {
    let model = normalize_object(CH04_MAP_SRC);
    let map = model
        .nodes
        .iter()
        .find(|n| n.name == "CapitalCity")
        .expect("map node");
    assert_eq!(map.kind, FamilyNodeKind::Map);

    let rows: Vec<_> = map
        .members
        .iter()
        .filter_map(|m| m.text.strip_prefix("\x1fmap:"))
        .collect();
    assert_eq!(rows.len(), 2, "expected two key=>value rows");
    assert!(rows.iter().any(|r| r.starts_with("UK\x1f")));
    assert!(rows.iter().any(|r| r.starts_with("USA\x1f")));

    let rel = model
        .relations
        .iter()
        .find(|r| r.label.is_none() && r.to.contains("CapitalCity"))
        .or_else(|| model.relations.first())
        .expect("relation into map");
    assert!(
        rel.to.contains("::USA") || rel.to.contains("CapitalCity::USA"),
        "qualified map endpoint should be preserved: {:?}",
        rel.to
    );
}

#[test]
fn ch04_pert_map_chain_renders() {
    let svg = puml::render_source_to_svg(CH04_PERT_SRC).expect("render PERT map chain");
    assert!(svg.contains("data-uml-kind=\"map\""));
    assert!(svg.contains("Kick.Off"));
    assert!(svg.contains("task.1"));
    assert!(svg.contains("Label 1"));
}

#[test]
fn ch04_render_emits_diamond_hub_and_map_table() {
    let diamond_svg =
        puml::render_source_to_svg(CH04_DIAMOND_SRC).expect("render diamond hub slice");
    assert!(diamond_svg.contains("uml-diamond-hub"));
    assert!(diamond_svg.contains("data-uml-kind=\"diamond\""));

    let map_svg = puml::render_source_to_svg(CH04_MAP_SRC).expect("render map table slice");
    assert!(map_svg.contains("data-uml-kind=\"map\""));
    assert!(map_svg.contains("uml-map-key"));
    assert!(map_svg.contains("uml-map-value"));
    assert!(map_svg.contains("data-uml-map-row=\"USA\""));
    assert!(map_svg.contains("London"));
    assert!(map_svg.contains("Washington"));
}

#[test]
fn ch04_hide_attributes_strips_object_fields() {
    let src = r##"@startuml
hide attributes
object user {
  name = "Dummy"
  id = 123
}
@enduml
"##;
    let model = normalize_object(src);
    let user = model
        .nodes
        .iter()
        .find(|n| n.name == "user")
        .expect("user node");
    assert!(
        user.members.is_empty(),
        "hide attributes should remove object field rows"
    );
}

#[test]
fn ch04_fixture_renders_combined_parity_slice() {
    let svg = puml::render_source_to_svg(CH04_PARITY_SRC).expect("render ch04 parity fixture");
    assert!(svg.contains("uml-diamond-hub"));
    assert!(svg.contains("data-uml-kind=\"map\""));
    assert!(svg.contains("CapitalCity"));
    assert!(svg.contains("data-uml-map-row=\"USA\""));
}
