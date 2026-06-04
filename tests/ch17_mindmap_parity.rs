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

const THEME_SRC: &str = r##"@startmindmap
!theme vibrant
* Theme root
** Themed branch
*** Themed leaf
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

/// #1467 — PlantUML layout parity: without explicit `left side` markers, all
/// depth-1 branches default to the right side (vertical-stack mindmap), matching
/// (#1538) PlantUML parity: without explicit `left side` markers the renderer
/// auto-balances depth-1 branches using even→right, odd→left heuristic.
#[test]
fn mindmap_default_layout_auto_balances_branches() {
    let src = "@startmindmap\n\
               * Root\n\
               ** Frontend\n\
               ** Backend\n\
               ** DevOps\n\
               @endmindmap\n";
    let svg = puml::render_source_to_svg(src).expect("mindmap should render");
    // Auto-balance: even-indexed → right, odd-indexed → left.
    // Frontend(0)=right, Backend(1)=left, DevOps(2)=right.
    assert!(
        svg.contains("data-mindmap-side=\"left\""),
        "auto-balance should put odd-indexed depth-1 branches on the left (#1538): {svg}"
    );
    assert!(
        svg.contains("data-mindmap-side=\"right\""),
        "right-side branches should still be present: {svg}"
    );
}

/// #1467 — explicit `left side` markers still opt into the symmetric splay
/// layout, preserving the manual escape hatch for users who want the previous
/// auto-balance look.
#[test]
fn mindmap_explicit_left_side_still_splits_layout() {
    let src = "@startmindmap\n\
               * Root\n\
               ** Right1\n\
               left side\n\
               ** Left1\n\
               @endmindmap\n";
    let svg = puml::render_source_to_svg(src).expect("mindmap should render");
    assert!(
        svg.contains("data-mindmap-side=\"left\""),
        "explicit `left side` marker must place at least one branch on the left: {svg}"
    );
    assert!(
        svg.contains("data-mindmap-side=\"right\""),
        "right-side branches should also be present: {svg}"
    );
}

#[test]
fn mindmap_theme_preset_applies_to_existing_depth_style_hooks() {
    let document = puml::parser::parse(THEME_SRC).expect("parse themed mindmap");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize themed mindmap")
    else {
        panic!("mindmap should normalize as family document");
    };

    let Some(FamilyStyle::MindMap(style)) = &model.family_style else {
        panic!("theme preset should be carried as mindmap family_style");
    };
    let depth_one = style
        .depth_styles
        .get(&1)
        .expect("theme should seed depth(1) style");
    assert_eq!(depth_one.background_color.as_deref(), Some("#ede9fe"));
    assert_eq!(depth_one.font_color.as_deref(), Some("#7c3aed"));
    assert_eq!(depth_one.border_color.as_deref(), Some("#7c3aed"));

    let svg = puml::render_source_to_svg(THEME_SRC).expect("render themed mindmap");
    assert!(
        svg.contains("data-mindmap-depth=\"1\"")
            && svg.contains("fill=\"#ede9fe\"")
            && svg.contains("stroke=\"#7c3aed\""),
        "depth(1) node should use themed colors: {svg}"
    );
}
