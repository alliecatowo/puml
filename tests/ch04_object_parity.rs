use puml::render_source_to_svg;

#[test]
fn object_diamond_node_renders_as_polygon() {
    let src = "@startuml
object A
object B
diamond dia
A --> dia
B --> dia
@enduml
";
    let svg = render_source_to_svg(src).expect("diamond render should succeed");
    assert!(svg.contains("class=\"uml-diamond\""));
    assert!(svg.contains("<polygon"));
}

#[test]
fn map_rows_render_as_two_columns() {
    let src = "@startuml
map CapitalCity {
  UK => London
  USA <=> Washington
}
@enduml
";
    let svg = render_source_to_svg(src).expect("map render should succeed");
    assert!(svg.contains(">UK<"));
    assert!(svg.contains(">London<"));
    assert!(svg.contains(">USA<"));
    assert!(svg.contains(">Washington<"));
    assert!(
        !svg.contains("map\u{bb}") && !svg.contains("&lt;&lt;map&gt;&gt;"),
        "internal map marker should not be rendered"
    );
}

#[test]
fn object_background_skinparam_applies_fill() {
    let src = "@startuml
skinparam objectBackgroundColor LightBlue
object Service
@enduml
";
    let svg = render_source_to_svg(src).expect("object style render should succeed");
    assert!(
        svg.contains("#add8e6"),
        "expected resolved LightBlue fill in object node"
    );
}

#[test]
fn diamond_uses_object_background_skinparam() {
    let src = "@startuml
skinparam objectBackgroundColor LightBlue
diamond dia
@enduml
";
    let svg = render_source_to_svg(src).expect("diamond style render should succeed");
    assert!(svg.contains("class=\"uml-diamond\""));
    assert!(
        svg.contains("fill=\"#add8e6\""),
        "expected resolved object background fill on diamond"
    );
}
