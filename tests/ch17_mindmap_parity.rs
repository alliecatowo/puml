use puml::model::{FamilyNodeKind, FamilyStyle, NormalizedDocument};

const MAX_WIDTH_SRC: &str = r##"@startmindmap
skinparam MaximumWidth 100
* Short
** This is a deliberately long child label that should wrap across multiple lines
@endmindmap
"##;

const CREOLE_SRC: &str = r##"@startmindmap
* **Bold root**
** //Italic branch//
*** <color:blue>Blue leaf</color>
@endmindmap
"##;

const MULTILINE_SRC: &str = r##"@startmindmap
* Root
**:Line one
Line two
;
@endmindmap
"##;

const DEPTH_STYLE_SRC: &str = r##"@startmindmap
<style>
mindmapDiagram {
  :depth(1) {
    BackGroundColor white
    FontColor darkgreen
    LineColor red
  }
}
</style>
* Root
** Styled child
*** Default grandchild
@endmindmap
"##;

#[test]
fn mindmap_maximum_width_skinparam_wraps_long_labels() {
    let document = puml::parser::parse(MAX_WIDTH_SRC).expect("parse mindmap MaximumWidth");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize mindmap MaximumWidth")
    else {
        panic!("mindmap should normalize as family document");
    };
    assert_eq!(model.maximum_width, Some(100));

    assert!(
        model
            .nodes
            .iter()
            .any(|node| node.name.contains("deliberately long")),
        "long child node should survive normalization"
    );

    let svg = puml::render_source_to_svg(MAX_WIDTH_SRC).expect("render mindmap MaximumWidth");
    assert!(svg.contains("mindmap-node"));
    assert!(
        svg.matches("<tspan").count() >= 2,
        "rendered SVG should contain wrapped tspans: {svg}"
    );
}

#[test]
fn mindmap_creole_markup_renders_bold_italic_and_color() {
    let svg = puml::render_source_to_svg(CREOLE_SRC).expect("render mindmap creole");
    assert!(
        svg.contains("font-weight=\"bold\"") || svg.contains("font-weight='bold'"),
        "expected bold creole markup in SVG"
    );
    assert!(
        svg.contains("font-style=\"italic\"") || svg.contains("font-style='italic'"),
        "expected italic creole markup in SVG"
    );
    assert!(
        svg.contains("fill=\"#0000ff\"") || svg.contains("fill=\"blue\""),
        "expected blue color creole markup in SVG"
    );
    assert!(svg.contains(">Bold root<") || svg.contains("Bold root"));
}

#[test]
fn mindmap_multiline_colon_semicolon_block_parses() {
    let document = puml::parser::parse(MULTILINE_SRC).expect("parse multiline mindmap");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize multiline mindmap")
    else {
        panic!("mindmap should normalize as family document");
    };

    let block = model
        .nodes
        .iter()
        .find(|node| node.name.contains("Line one"))
        .expect("multiline node");
    assert_eq!(block.kind, FamilyNodeKind::MindMap);
    assert!(
        block.name.contains('\n'),
        "multiline body should contain newline"
    );
    assert!(block.name.contains("Line two"));

    let svg = puml::render_source_to_svg(MULTILINE_SRC).expect("render multiline mindmap");
    assert!(svg.contains("Line one"));
    assert!(svg.contains("Line two"));
}

#[test]
fn mindmap_depth_style_block_applies_depth_specific_colors() {
    let document = puml::parser::parse(DEPTH_STYLE_SRC).expect("parse mindmap depth style");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize mindmap depth style")
    else {
        panic!("mindmap should normalize as family document");
    };

    let Some(FamilyStyle::MindMap(style)) = &model.family_style else {
        panic!("mindmap depth style should be carried in family_style");
    };
    let depth_one = style
        .depth_styles
        .get(&1)
        .expect("depth(1) style should be captured");
    assert_eq!(depth_one.background_color.as_deref(), Some("white"));
    assert_eq!(depth_one.font_color.as_deref(), Some("darkgreen"));
    assert_eq!(depth_one.border_color.as_deref(), Some("red"));

    let svg = puml::render_source_to_svg(DEPTH_STYLE_SRC).expect("render mindmap depth style");
    assert!(
        svg.contains("class=\"mindmap-node mindmap-depth-1")
            && svg.contains("data-mindmap-depth=\"1\"")
            && svg.contains("fill=\"white\"")
            && svg.contains("stroke=\"red\""),
        "depth(1) node should use style colors: {svg}"
    );
    assert!(
        svg.contains("fill=\"darkgreen\""),
        "depth(1) label should use styled font color: {svg}"
    );
}
