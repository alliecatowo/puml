/// Chapter 7 component diagram parity tests.
///
/// Covers:
///   7.8 / 7.9  `skinparam componentStyle uml2` (default — badges on left)
///   7.9        `skinparam componentStyle uml1`   (badges in top-right corner)
///   7.14       `skinparam componentStyle rectangle` (plain rect, no badges, no «component»)
///   7.15       `hide @unlinked` / `remove @unlinked` (filter orphan nodes)
///   7.16       `hide` / `remove` / `restore $tag` (component tags)
use puml::render_source_to_svg;

// ── helpers ───────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    render_source_to_svg(src).expect("render should succeed")
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
