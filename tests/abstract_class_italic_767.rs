/// Regression tests for #767 — abstract class names and interface names must
/// render in italic text per PlantUML / UML convention.

fn svg_for(src: &str) -> String {
    puml::render_source_to_svg(src).expect("svg should render")
}

/// Return true if the SVG `<text>` element containing `content` carries
/// `font-style="italic"`.
fn has_italic(svg: &str, content: &str) -> bool {
    let idx = match svg.find(content) {
        Some(i) => i,
        None => panic!("content {:?} not found in SVG", content),
    };
    // Walk backwards to the opening `<text` tag.
    let before = &svg[..idx];
    let tag_start = before.rfind("<text").expect("enclosing <text> tag");
    let tag_end = svg[tag_start..]
        .find('>')
        .map(|i| tag_start + i)
        .expect("tag close");
    let tag = &svg[tag_start..=tag_end];
    tag.contains("font-style=\"italic\"")
}

// ── abstract class name ───────────────────────────────────────────────────────

#[test]
fn abstract_class_name_is_italic() {
    let svg = svg_for("@startuml\nabstract class Shape {\n  area(): Float\n}\n@enduml\n");
    assert!(
        has_italic(&svg, ">Shape</text>"),
        "abstract class name should render in italic"
    );
}

// ── interface name ────────────────────────────────────────────────────────────

#[test]
fn interface_name_is_italic() {
    // interface keyword requires a class-diagram context (at least one other class element)
    let svg =
        svg_for("@startuml\nclass Concrete\ninterface Drawable {\n  draw(): void\n}\n@enduml\n");
    assert!(
        has_italic(&svg, ">Drawable</text>"),
        "interface name should render in italic"
    );
}

// ── interface members are implicitly abstract (italic) ────────────────────────

#[test]
fn interface_member_is_italic() {
    // interface keyword requires a class-diagram context (at least one other class element)
    let svg =
        svg_for("@startuml\nclass Concrete\ninterface Drawable {\n  draw(): void\n}\n@enduml\n");
    assert!(
        has_italic(&svg, ">draw(): void</text>"),
        "interface member should render in italic (implicitly abstract)"
    );
}

// ── regular class name is NOT italic ─────────────────────────────────────────

#[test]
fn regular_class_name_is_not_italic() {
    let svg = svg_for("@startuml\nclass Concrete {\n  value: int\n}\n@enduml\n");
    assert!(
        !has_italic(&svg, ">Concrete</text>"),
        "regular class name should NOT render in italic"
    );
}

// ── {abstract} member modifier makes individual members italic ────────────────

#[test]
fn abstract_modifier_member_is_italic() {
    let svg =
        svg_for("@startuml\nabstract class Shape {\n  {abstract} area(): Float\n}\n@enduml\n");
    assert!(
        has_italic(&svg, ">area(): Float</text>"),
        "member with {{abstract}} modifier should render in italic"
    );
}
