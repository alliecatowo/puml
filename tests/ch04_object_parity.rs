//! Chapter 4 object-diagram parity tests.

use puml::model::{FamilyNodeKind, NormalizedDocument};

const DIAMOND_HUB_SRC: &str = r##"@startuml
object Customer
object Seller
diamond Sale
object Fulfillment
Customer --> Sale
Seller --> Sale
Sale --> Fulfillment
@enduml
"##;

const MAP_ROWS_SRC: &str = r##"@startuml
object London
object Washington
object Berlin
object NewYork
map CapitalCity {
  UK *-> London
  USA *--> Washington
  Germany *---> Berlin
}
NewYork --> CapitalCity::USA
@enduml
"##;

const MEMBER_ANCHOR_SRC: &str = r##"@startuml
object Account {
  id = 42
  name = "A"
}
object Audit
Audit --> Account::id
@enduml
"##;

const TYPED_OBJECT_SRC: &str = r##"@startuml
object Alice : Person
object Bob : Person
Alice --> Bob : knows
@enduml
"##;

#[test]
fn object_ch04_normalizes_diamond_map_and_qualified_relation() {
    let document = puml::parser::parse(DIAMOND_HUB_SRC).expect("parse ch04 diamond hub");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize ch04 diamond hub")
    else {
        panic!("object diagram should normalize as Family");
    };
    assert!(
        model
            .nodes
            .iter()
            .any(|node| node.name == "Sale" && node.kind == FamilyNodeKind::Diamond),
        "diamond declaration should normalize as a diamond node"
    );

    let document = puml::parser::parse(MAP_ROWS_SRC).expect("parse ch04 map rows");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize ch04 map rows")
    else {
        panic!("object diagram should normalize as Family");
    };
    assert!(
        model
            .nodes
            .iter()
            .any(|node| node.name == "CapitalCity" && node.kind == FamilyNodeKind::Map),
        "map declaration should normalize as a map node"
    );
    assert!(
        model
            .relations
            .iter()
            .any(|rel| rel.from == "NewYork" && rel.to == "CapitalCity::USA"),
        "qualified map-row endpoint should be preserved on the relation"
    );
    assert!(
        model.relations.iter().any(|rel| {
            rel.from == "CapitalCity::USA" && rel.to == "Washington" && rel.arrow == "*-->"
        }),
        "map row link syntax should normalize as a relation from the qualified row"
    );
    assert!(
        model
            .relations
            .iter()
            .any(|rel| rel.from == "CapitalCity::UK" && rel.to == "London" && rel.arrow == "*->"),
        "short map row link syntax should normalize as a relation from the qualified row"
    );
}

#[test]
fn object_ch04_normalizes_typed_instance_label_without_losing_identity() {
    let document = puml::parser::parse(TYPED_OBJECT_SRC).expect("parse typed object instances");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize typed object instances")
    else {
        panic!("object diagram should normalize as Family");
    };

    assert!(
        model.nodes.iter().any(|node| {
            node.name == "Alice" && node.label.as_deref() == Some("Alice : Person")
        }),
        "typed object should keep `Alice` as the relation id and use the typed label"
    );
    assert!(
        model.relations.iter().any(|rel| rel.from == "Alice"
            && rel.to == "Bob"
            && rel.label.as_deref() == Some("knows")),
        "relation endpoints should still resolve by instance name"
    );
}

#[test]
fn object_ch04_renders_diamond_and_two_column_map_rows() {
    let svg = puml::render_source_to_svg(DIAMOND_HUB_SRC).expect("render ch04 diamond hub");
    assert!(
        svg.contains("class=\"uml-node uml-diamond\""),
        "diamond hub should render as a polygon node: {svg}"
    );

    let svg = puml::render_source_to_svg(MAP_ROWS_SRC).expect("render ch04 map rows");
    assert!(
        svg.contains("class=\"uml-map-divider\""),
        "map should render a two-column divider: {svg}"
    );
    assert!(
        svg.contains("class=\"uml-map-key\" data-uml-anchor=\"CapitalCity::USA\""),
        "map row should publish the qualified USA anchor: {svg}"
    );
    assert!(
        svg.contains("class=\"uml-map-value\"") && svg.contains(">Washington<"),
        "map row relation syntax should render its target in the value column: {svg}"
    );
}

#[test]
fn object_ch04_renders_typed_instances_underlined_with_relation() {
    let svg = puml::render_source_to_svg(TYPED_OBJECT_SRC).expect("render typed object instances");
    assert!(
        svg.contains(">Alice : Person</text>") && svg.contains(">Bob : Person</text>"),
        "typed object labels should render in the object header: {svg}"
    );
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "typed object headers should be underlined: {svg}"
    );
    assert!(
        svg.contains("data-uml-from=\"Alice\" data-uml-to=\"Bob\""),
        "typed object relation should render between instance ids: {svg}"
    );
}

#[test]
fn qualified_map_link_anchors_to_the_named_row() {
    let svg = puml::render_source_to_svg(MAP_ROWS_SRC).expect("render qualified map link");
    let row_tag = svg_text_tag_with(&svg, "data-uml-anchor=\"CapitalCity::USA\"")
        .expect("USA map row text tag");
    let row_y = svg_attr_i32(row_tag, "y");
    let rel_tag = svg_relation_tag_with(&svg, "data-uml-to=\"CapitalCity::USA\"")
        .expect("qualified relation");
    let rel_y = svg_relation_end_y(rel_tag);

    assert_eq!(
        rel_y, row_y,
        "qualified link should terminate at the USA row center"
    );
    assert!(
        svg.contains("data-uml-from=\"CapitalCity::USA\" data-uml-to=\"Washington\""),
        "map row link syntax should render a relation from the USA row to Washington"
    );
}

#[test]
fn qualified_map_row_ports_enter_and_exit_horizontally() {
    let svg = puml::render_source_to_svg(MAP_ROWS_SRC).expect("render qualified map row ports");

    let usa_target = svg_relation_tag_with(&svg, "data-uml-to=\"CapitalCity::USA\"")
        .expect("external qualified relation");
    assert!(
        relation_final_segment_is_horizontal(usa_target),
        "external relation should enter the USA row horizontally: {usa_target}"
    );

    for row in ["UK", "USA", "Germany"] {
        let needle = format!("data-uml-from=\"CapitalCity::{row}\"");
        let rel_tag = svg_relation_tag_with(&svg, &needle)
            .unwrap_or_else(|| panic!("map-body relation for {row}"));
        assert!(
            relation_initial_segment_is_horizontal(rel_tag),
            "map-body relation should leave the {row} row horizontally: {rel_tag}"
        );
    }

    for object in ["London", "Washington", "Berlin"] {
        let needle = format!("data-uml-to=\"{object}\"");
        let rel_tag =
            svg_relation_tag_with(&svg, &needle).unwrap_or_else(|| panic!("relation to {object}"));
        assert!(
            relation_final_segment_is_vertical(rel_tag),
            "map-body relation should enter {object} through a visible top/bottom port: {rel_tag}"
        );
    }
}

#[test]
fn qualified_member_row_target_enters_horizontally() {
    let svg = puml::render_source_to_svg(MEMBER_ANCHOR_SRC).expect("render member row anchor");
    let rel_tag = svg_relation_tag_with(&svg, "data-uml-to=\"Account::id\"")
        .expect("member-qualified relation");

    assert!(
        relation_final_segment_is_horizontal(rel_tag),
        "member-qualified relation should enter the member row horizontally: {rel_tag}"
    );
}

fn svg_text_tag_with<'a>(svg: &'a str, needle: &str) -> Option<&'a str> {
    let idx = svg.find(needle)?;
    let start = svg[..idx].rfind("<text ")?;
    let end = svg[idx..].find('>')?;
    Some(&svg[start..idx + end + 1])
}

fn svg_relation_tag_with<'a>(svg: &'a str, needle: &str) -> Option<&'a str> {
    let idx = svg.find(needle)?;
    let line_start = svg[..idx].rfind("<line ");
    let polyline_start = svg[..idx].rfind("<polyline ");
    let start = line_start.into_iter().chain(polyline_start).max()?;
    let end = svg[idx..].find("/>")?;
    Some(&svg[start..idx + end + 2])
}

fn svg_relation_end_y(tag: &str) -> i32 {
    if tag.starts_with("<line ") {
        return svg_attr_i32(tag, "y2");
    }
    svg_relation_points(tag)
        .last()
        .map(|(_, y)| *y)
        .unwrap_or_else(|| panic!("empty points attr in {tag}"))
}

fn relation_initial_segment_is_horizontal(tag: &str) -> bool {
    let points = svg_relation_points(tag);
    points
        .windows(2)
        .next()
        .is_some_and(|segment| segment[0].1 == segment[1].1)
}

fn relation_final_segment_is_horizontal(tag: &str) -> bool {
    let points = svg_relation_points(tag);
    points
        .windows(2)
        .last()
        .is_some_and(|segment| segment[0].1 == segment[1].1)
}

fn relation_final_segment_is_vertical(tag: &str) -> bool {
    let points = svg_relation_points(tag);
    points
        .windows(2)
        .last()
        .is_some_and(|segment| segment[0].0 == segment[1].0)
}

fn svg_relation_points(tag: &str) -> Vec<(i32, i32)> {
    if tag.starts_with("<line ") {
        return vec![
            (svg_attr_i32(tag, "x1"), svg_attr_i32(tag, "y1")),
            (svg_attr_i32(tag, "x2"), svg_attr_i32(tag, "y2")),
        ];
    }
    tag.split("points=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_else(|| panic!("missing points attr in {tag}"))
        .split_whitespace()
        .map(|point| {
            let mut coords = point.split(',');
            let x = coords
                .next()
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or_else(|| panic!("missing point x in {tag}"));
            let y = coords
                .next()
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or_else(|| panic!("missing point y in {tag}"));
            (x, y)
        })
        .collect()
}

fn svg_attr_i32(tag: &str, attr: &str) -> i32 {
    let prefix = format!("{attr}=\"");
    tag.split(&prefix)
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or_else(|| panic!("missing integer attr {attr} in {tag}"))
}
