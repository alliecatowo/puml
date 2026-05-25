/// Chapter 7 component diagram parity tests.
///
/// Covers:
///   7.8 / 7.9  `skinparam componentStyle uml2` (default — badges on left)
///   7.9        `skinparam componentStyle uml1`   (badges in top-right corner)
///   7.14       `skinparam componentStyle rectangle` (plain rect, no badges, no «component»)
///   7.15       `hide @unlinked` / `remove @unlinked` (filter orphan nodes)
///   7.16       `hide` / `remove` / `restore $tag` (component tags)
use puml::model::{FamilyStyle, NormalizedDocument};
use puml::{normalize_family, parse, render_source_to_svg};

// ── helpers ───────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    render_source_to_svg(src).expect("render should succeed")
}

fn attr_value_in_tag(haystack: &str, marker: &str, attr: &str) -> i32 {
    let marker_idx = haystack.find(marker).expect("marker should exist");
    let tag_start = haystack[..=marker_idx]
        .rfind('<')
        .expect("tag start should exist");
    let tag_end = haystack[marker_idx..]
        .find('>')
        .map(|idx| marker_idx + idx)
        .expect("tag end should exist");
    let tag = &haystack[tag_start..=tag_end];
    let needle = format!("{attr}=\"");
    let attr_start = tag.find(&needle).expect("attribute should exist") + needle.len();
    let rest = &tag[attr_start..];
    let end = rest.find('"').expect("attribute should terminate");
    rest[..end]
        .parse::<i32>()
        .expect("attribute should parse as i32")
}

// ── 7.8 / 7.9  componentStyle uml2 (default) ─────────────────────────────────

#[test]
fn component_style_uml2_is_default() {
    let src = "\
@startuml
[WebServer]
@enduml
";
    let svg = render_svg(src);
    // Default (uml2): badges appear on the LEFT side; the rect carries
    // data-component-style=\"uml2\" (or no attribute — both acceptable); no
    // \"rectangle\" attribute.
    assert!(
        !svg.contains("data-component-style=\"rectangle\""),
        "uml2 default should NOT use rectangle style"
    );
    assert!(
        !svg.contains("data-component-style=\"uml1\""),
        "uml2 default should NOT use uml1 style"
    );
    // The «component» stereotype text should be present.
    assert!(
        svg.contains("component"),
        "uml2 default should show «component» stereotype"
    );
}

// ── 7.9  componentStyle uml1 ─────────────────────────────────────────────────

#[test]
fn component_style_uml1_sets_badge_attribute() {
    let src = "\
@startuml
skinparam componentStyle uml1
[WebServer]
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains("data-component-style=\"uml1\""),
        "uml1 style should set data-component-style=\"uml1\" on component rect; got: {}",
        &svg[..svg.len().min(400)]
    );
}

#[test]
fn component_style_uml1_badges_are_in_top_right() {
    let src = "\
@startuml
skinparam componentStyle uml1
[WebServer]
@enduml
";
    let svg = render_svg(src);
    // The SVG should have two small badge rects; since uml1 puts them in the
    // top-right, we only check the attribute presence (pixel-position testing
    // is done via visual PNG audit).
    assert!(
        svg.contains("data-component-style=\"uml1\""),
        "uml1 style attribute must be present"
    );
    // The «component» stereotype text should still appear.
    assert!(
        svg.contains("component"),
        "uml1 style should still show «component» stereotype"
    );
}

// ── 7.14  componentStyle rectangle ───────────────────────────────────────────

#[test]
fn component_style_rectangle_sets_attribute() {
    let src = "\
@startuml
skinparam componentStyle rectangle
[WebServer]
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains("data-component-style=\"rectangle\""),
        "rectangle style should set data-component-style=\"rectangle\"; got: {}",
        &svg[..svg.len().min(400)]
    );
}

#[test]
fn component_style_rectangle_hides_stereotype() {
    let src = "\
@startuml
skinparam componentStyle rectangle
[WebServer]
@enduml
";
    let svg = render_svg(src);
    // Rectangle mode: the «component» stereotype text must NOT appear.
    assert!(
        !svg.contains("«component»") && !svg.contains("&laquo;component&raquo;"),
        "rectangle style should suppress «component» stereotype; got SVG containing it"
    );
}

#[test]
fn component_style_rectangle_no_badges() {
    let src = "\
@startuml
skinparam componentStyle rectangle
[WebServer]
@enduml
";
    let svg = render_svg(src);
    // Rectangle mode should have no component-badge small rects (they are
    // identified by having class=\"component-badge\" or similar).  We
    // specifically verify there is no uml1/uml2 badge attribute.
    assert!(
        !svg.contains("data-component-style=\"uml1\""),
        "rectangle style should not produce uml1 badges"
    );
    assert!(
        !svg.contains("data-component-style=\"uml2\""),
        "rectangle style should not produce uml2 badges"
    );
}

#[test]
fn component_diagram_style_block_component_colors_override_skinparam() {
    let src = include_str!("fixtures/styling/valid_style_block_component.puml");
    let document = parse(src).expect("component style block should parse");
    let model = normalize_family(document).expect("component style block should normalize");

    let NormalizedDocument::Family(family) = model else {
        panic!("component diagram should normalize as a family document");
    };
    let Some(FamilyStyle::Component(style)) = family.family_style else {
        panic!("component diagram should carry component style");
    };

    assert_eq!(style.background_color, "#dbeafe");
    assert_eq!(style.border_color, "#2563eb");
    assert_eq!(style.font_color, "#7c2d12");
}

#[test]
fn component_diagram_style_block_component_colors_reach_svg() {
    let src = include_str!("fixtures/styling/valid_style_block_component.puml");
    let svg = render_svg(src);

    assert!(
        svg.contains("#dbeafe"),
        "style block component BackgroundColor should reach SVG: {svg}"
    );
    assert!(
        svg.contains("#2563eb"),
        "style block component BorderColor should reach SVG"
    );
    assert!(
        svg.contains("#7c2d12"),
        "style block component FontColor should reach SVG"
    );
}

#[test]
fn component_inline_visual_style_reaches_shape_and_label() {
    let src = "\
@startuml
component \"Styled API\" as api #back:HoneyDew;line:DarkGreen;line.dashed;line.bold;text:DarkBlue
[Worker] #back:Lavender;line:Purple;text:Indigo
api --> Worker
@enduml
";
    let svg = render_svg(src);

    assert!(
        svg.contains("fill=\"#f0fff0\""),
        "component back: color should style the component fill; svg={svg}"
    );
    assert!(
        svg.contains("stroke=\"#006400\""),
        "component line: color should style the component border; svg={svg}"
    );
    assert!(
        svg.contains("stroke-dasharray=\"5 3\""),
        "component line.dashed should style the component border; svg={svg}"
    );
    assert!(
        svg.contains("stroke-width=\"3\""),
        "component line.bold should thicken the component border; svg={svg}"
    );
    assert!(
        svg.contains("fill=\"#00008b\">Styled API</text>"),
        "component text: color should style the component label; svg={svg}"
    );
    assert!(
        svg.contains("fill=\"#e6e6fa\"") && svg.contains("stroke=\"#800080\""),
        "bracket shorthand inline style should reach component SVG; svg={svg}"
    );
}

// ── 7.15  hide @unlinked ──────────────────────────────────────────────────────

#[test]
fn hide_unlinked_removes_orphan_nodes() {
    let src = "\
@startuml
hide @unlinked

[WebServer] --> [Database]
[UnusedComponent]
@enduml
";
    let svg = render_svg(src);
    // Linked nodes should appear.
    assert!(
        svg.contains("WebServer"),
        "WebServer (linked) should remain after hide @unlinked"
    );
    assert!(
        svg.contains("Database"),
        "Database (linked) should remain after hide @unlinked"
    );
    // Orphan should be removed.
    assert!(
        !svg.contains("UnusedComponent"),
        "UnusedComponent (orphan) should be removed by hide @unlinked"
    );
}

#[test]
fn remove_unlinked_removes_orphan_nodes() {
    let src = "\
@startuml
remove @unlinked

[WebServer] --> [Database]
[UnusedComponent]
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains("WebServer"),
        "WebServer (linked) should remain after remove @unlinked"
    );
    assert!(
        svg.contains("Database"),
        "Database (linked) should remain after remove @unlinked"
    );
    assert!(
        !svg.contains("UnusedComponent"),
        "UnusedComponent (orphan) should be removed by remove @unlinked"
    );
}

#[test]
fn hide_unlinked_keeps_all_when_all_linked() {
    let src = "\
@startuml
hide @unlinked

[A] --> [B]
[B] --> [C]
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains(">A<") || svg.contains(">A "),
        "A should be present"
    );
    assert!(
        svg.contains(">B<") || svg.contains(">B "),
        "B should be present"
    );
    assert!(
        svg.contains(">C<") || svg.contains(">C "),
        "C should be present"
    );
}

#[test]
fn hide_unlinked_does_not_affect_sequence_diagrams() {
    // hide footbox must still work in sequence diagrams and must not be
    // confused with hide @unlinked.
    let src = "\
@startuml
hide footbox
A -> B : hello
@enduml
";
    let svg = render_svg(src);
    // The diagram should render successfully and include the message.
    assert!(
        svg.contains("hello"),
        "sequence message should still render"
    );
}

// ── 7.16  hide/remove/restore $tag ───────────────────────────────────────────

#[test]
fn hide_component_tag_removes_tagged_nodes_and_edges() {
    let src = "\
@startuml
[Frontend] $public
[Backend] $internal
[Frontend] --> [Backend] : calls
hide $internal
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains("Frontend"),
        "untagged-visible node should remain"
    );
    assert!(
        !svg.contains("Backend"),
        "node tagged $internal should be hidden"
    );
    assert!(
        !svg.contains("calls"),
        "relations touching hidden tagged nodes should also be hidden"
    );
}

#[test]
fn remove_component_tag_removes_all_matching_nodes() {
    let src = "\
@startuml
component [Gateway] $edge
component [Worker] $internal
remove $internal
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains("Gateway"),
        "non-matching tagged node should remain"
    );
    assert!(
        !svg.contains("Worker"),
        "node tagged $internal should be removed"
    );
}

#[test]
fn restore_component_tag_after_hide_all_keeps_tagged_nodes() {
    let src = "\
@startuml
component [Gateway] $edge
component [Worker] $internal
hide *
restore $edge
@enduml
";
    let svg = render_svg(src);
    assert!(svg.contains("Gateway"), "restored tag should render");
    assert!(
        !svg.contains("Worker"),
        "non-restored tag should stay hidden after hide *"
    );
}

#[test]
fn dollar_named_component_is_not_treated_as_tag_without_tag_marker() {
    let src = "\
@startuml
component [$C1]
hide $C1
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains("$C1"),
        "a bracketed component named $C1 should not be hidden as a tag"
    );
}

#[test]
fn component_tags_do_not_render_as_member_text() {
    let src = "\
@startuml
[Gateway] $edge
@enduml
";
    let svg = render_svg(src);
    assert!(svg.contains("Gateway"), "tagged component should render");
    assert!(
        !svg.contains("$edge"),
        "component tag metadata should not render as visible text"
    );
}

#[test]
fn component_note_is_visible_and_connected() {
    let src = "\
@startuml
[API]
[DB]
API --> DB : query
note right of API: public facade
note right on link: encrypted
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains("public facade"),
        "component note text should render as a note node"
    );
    assert!(
        svg.contains("encrypted"),
        "on-link note text should render as a note node"
    );
    assert!(
        svg.contains("data-uml-from=\"API\"") && svg.contains("data-uml-arrow=\"..\""),
        "note attachment edge should connect API to a note with dotted relation"
    );
    assert!(
        svg.contains("data-uml-from=\"DB\"") && svg.matches("data-uml-arrow=\"..\"").count() >= 2,
        "on-link note should attach to the most recent relation target with dotted relation"
    );
}

#[test]
fn component_multiline_bracket_description_renders_without_parse_error() {
    let src = "\
@startuml
component comp1 [
Line 1
Line 2
]
@enduml
";
    let svg = render_svg(src);
    assert!(svg.contains("comp1"), "component name should render");
    assert!(
        svg.contains("Line 1"),
        "first multiline description row should render"
    );
    assert!(
        svg.contains("Line 2"),
        "second multiline description row should render"
    );
    assert!(
        !svg.contains("Line 1Line 2"),
        "multiline bracket description should preserve line breaks"
    );
}

#[test]
fn component_group_ports_attach_to_container_boundary() {
    let src = "\
@startuml
component Gateway {
  portin HTTPS
  portout Events
  component Worker
}
HTTPS --> Worker
Worker --> Events
@enduml
";
    let svg = render_svg(src);
    let group_marker = "class=\"uml-group-frame\" data-uml-group=\"Gateway\"";
    let group_x = attr_value_in_tag(&svg, group_marker, "x");
    let group_w = attr_value_in_tag(&svg, group_marker, "width");
    let in_x = attr_value_in_tag(&svg, "data-uml-port-direction=\"in\"", "x");
    let out_x = attr_value_in_tag(&svg, "data-uml-port-direction=\"out\"", "x");

    assert_eq!(in_x + 12, group_x, "portin should straddle left boundary");
    assert_eq!(
        out_x + 12,
        group_x + group_w,
        "portout should straddle right boundary"
    );
    assert!(
        !svg.contains("&lt;&lt;portin&gt;&gt;") && !svg.contains("&lt;&lt;portout&gt;&gt;"),
        "port direction metadata should not render as user stereotypes"
    );
}

#[test]
fn component_sprite_stereotype_renders_icon() {
    let src = "\
@startuml
sprite $businessProcess [4x4/16] {
0FF0
F00F
F00F
0FF0
}
rectangle \"Order flow\" <<$businessProcess>>
@enduml
";
    let svg = render_svg(src);
    assert!(
        svg.contains("data-sprite=\"businessProcess\""),
        "sprite stereotype should render as a sprite glyph"
    );
    assert!(
        !svg.contains("&lt;&lt;$businessProcess&gt;&gt;"),
        "sprite stereotype marker should not render as escaped text"
    );
}
