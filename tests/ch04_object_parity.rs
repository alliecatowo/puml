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
    let points = tag
        .split("points=\"")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_else(|| panic!("missing points attr in {tag}"));
    let last = points
        .split_whitespace()
        .last()
        .unwrap_or_else(|| panic!("empty points attr in {tag}"));
    last.split(',')
        .nth(1)
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or_else(|| panic!("missing final point y in {tag}"))
}

fn svg_attr_i32(tag: &str, attr: &str) -> i32 {
    let prefix = format!("{attr}=\"");
    tag.split(&prefix)
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or_else(|| panic!("missing integer attr {attr} in {tag}"))
}
