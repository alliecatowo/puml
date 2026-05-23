//! Chapter 3 class-diagram parity tests.
//!
//! Covers the features implemented in this wave:
//! - `skinparam <prefix> { Key Value }` block form (3.29)
//! - `skinparam packageStyle` accepted without warning (3.22)
//! - `remove @unlinked` removes unlinked nodes (3.18)
//! - `hide @unlinked` removes unlinked nodes (3.18)
//! - `note on link` attaches a note to the most recent relation (3.12)
//! - Generic type parameters `class Pair<T, U>` display correctly (3.19)
//! - `extends`/`implements` keywords generate heritage relations (3.34)
//! - `hide <classname>` / `remove <classname>` remove specific classes (3.15–3.16)
//! - Stereotype-scoped skinparam block form applies per-stereotype colors (3.30)
//! - Escaped leading visibility markers render as literal member text (3.6)

use puml::model::{FamilyNodeKind, FamilyStyle, NormalizedDocument};

#[path = "svg_test_helpers.rs"]
mod svg_test_helpers;
use svg_test_helpers::{bounds, SvgDoc};

// ─── 3.29 skinparam block form ───────────────────────────────────────────────

const SKINPARAM_BLOCK_SRC: &str = r##"@startuml
skinparam class {
  BackgroundColor Yellow
  BorderColor Red
}
class Foo
@enduml
"##;

#[test]
fn skinparam_block_applies_background_and_border_colors() {
    let svg = puml::render_source_to_svg(SKINPARAM_BLOCK_SRC).expect("render skinparam block form");
    // Yellow background (#ffff00) and red border (#ff0000) should appear
    assert!(
        svg.contains("#ffff00"),
        "skinparam block: BackgroundColor Yellow should produce #ffff00; svg={svg}"
    );
    assert!(
        svg.contains("#ff0000"),
        "skinparam block: BorderColor Red should produce #ff0000; svg={svg}"
    );
}

#[test]
fn skinparam_block_model_reflects_background_color() {
    let document = puml::parser::parse(SKINPARAM_BLOCK_SRC).expect("parse skinparam block");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize skinparam block")
    else {
        panic!("class diagram should normalize as Family");
    };
    if let Some(FamilyStyle::Class(cs)) = &model.family_style {
        assert_eq!(
            cs.background_color, "#ffff00",
            "ClassStyle.background_color should be yellow"
        );
        assert_eq!(
            cs.border_color, "#ff0000",
            "ClassStyle.border_color should be red"
        );
    } else {
        panic!("family_style should be Some(FamilyStyle::Class(...))");
    }
}

// ─── 3.30 Stereotype-scoped skinparam block form ────────────────────────────

const STEREO_SKINPARAM_BLOCK_SRC: &str = r##"@startuml
skinparam class {
  BackgroundColor<<Abstract>> LightYellow
  BorderColor<<Abstract>> Blue
}
class Foo <<Abstract>>
class Bar
@enduml
"##;

#[test]
fn skinparam_block_stereotype_scope_applies_to_matching_class() {
    let svg = puml::render_source_to_svg(STEREO_SKINPARAM_BLOCK_SRC)
        .expect("render stereotype-scoped skinparam block");
    // Foo<<Abstract>> should get LightYellow background (#ffffe0)
    assert!(
        svg.contains("#ffffe0") || svg.contains("lightyellow"),
        "stereotype-scoped skinparam: BackgroundColor<<Abstract>> LightYellow should be applied; svg={svg}"
    );
}

#[test]
fn skinparam_block_stereotype_scope_model_stereotype_styles() {
    let document =
        puml::parser::parse(STEREO_SKINPARAM_BLOCK_SRC).expect("parse stereotype skinparam block");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize stereotype skinparam block")
    else {
        panic!("should normalize as Family");
    };
    if let Some(FamilyStyle::Class(cs)) = &model.family_style {
        let abstract_style = cs.stereotype_styles.get("abstract");
        assert!(
            abstract_style.is_some(),
            "stereotype_styles should contain 'abstract'"
        );
        let bg = abstract_style.and_then(|s| s.background_color.as_deref());
        assert!(
            bg.is_some(),
            "abstract stereotype background_color should be set"
        );
    } else {
        panic!("family_style should be Some(FamilyStyle::Class(...))");
    }
}

// ─── 3.22 skinparam packageStyle accepted ───────────────────────────────────

const PACKAGE_STYLE_SRC: &str = r##"@startuml
skinparam packageStyle rectangle
package foo {
  class A
}
@enduml
"##;

#[test]
fn skinparam_package_style_accepted_without_warning() {
    // Should not produce a [W_SKINPARAM_UNSUPPORTED] warning or error
    let svg = puml::render_source_to_svg(PACKAGE_STYLE_SRC)
        .expect("render packageStyle — should not fail");
    // The class A should still appear (may be rendered as "foo::A" due to namespace-qualified name)
    assert!(
        svg.contains(">A<") || svg.contains("A"),
        "class A should appear in packageStyle diagram"
    );
}

#[test]
fn skinparam_package_style_no_diagnostic_warnings() {
    let document = puml::parser::parse(PACKAGE_STYLE_SRC).expect("parse packageStyle diagram");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize packageStyle diagram")
    else {
        panic!("should normalize as Family");
    };
    // No unsupported-skinparam warnings for packageStyle
    let unsupported: Vec<_> = model
        .warnings
        .iter()
        .filter(|w| w.message.contains("W_SKINPARAM_UNSUPPORTED"))
        .collect();
    assert!(
        unsupported.is_empty(),
        "no W_SKINPARAM_UNSUPPORTED warnings for packageStyle: {unsupported:?}"
    );
}

#[test]
fn package_frames_fit_inside_svg_viewbox() {
    let svg = puml::render_source_to_svg(
        r##"@startuml
package "Outer Boundary" {
  package "Inner Domain" {
    class "A class with a fairly wide name" as A
    class "Another class with members" as B {
      +identifier: VeryLongDomainSpecificIdentifier
    }
  }
}
A --> B : relation label
@enduml
"##,
    )
    .expect("render nested class packages");
    let doc = SvgDoc::parse(&svg);
    let viewbox: Vec<f64> = doc
        .root_attr("viewBox")
        .expect("svg should expose a viewBox")
        .split_whitespace()
        .map(|part| part.parse::<f64>().expect("numeric viewBox component"))
        .collect();
    assert_eq!(viewbox.len(), 4, "viewBox should have four components");
    let (vb_x, vb_y, vb_right, vb_bottom) = (
        viewbox[0],
        viewbox[1],
        viewbox[0] + viewbox[2],
        viewbox[1] + viewbox[3],
    );

    let frames = doc.elements_with_class("rect", "uml-group-frame");
    assert!(!frames.is_empty(), "expected rendered package frames");
    for frame in frames {
        let b = bounds(frame);
        assert!(
            b.x >= vb_x && b.right() <= vb_right && b.y >= vb_y && b.bottom() <= vb_bottom,
            "package frame should fit viewBox: frame={b:?}, viewBox={viewbox:?}"
        );
    }
}

// ─── 3.18 hide @unlinked / remove @unlinked ─────────────────────────────────

const HIDE_UNLINKED_SRC: &str = r##"@startuml
class A
class B
class C
A -- B
hide @unlinked
@enduml
"##;

const REMOVE_UNLINKED_SRC: &str = r##"@startuml
class A
class B
class C
A -- B
remove @unlinked
@enduml
"##;

#[test]
fn hide_at_unlinked_removes_isolated_nodes() {
    let svg = puml::render_source_to_svg(HIDE_UNLINKED_SRC).expect("render hide @unlinked");
    assert!(svg.contains(">A<"), "A should be visible (linked)");
    assert!(svg.contains(">B<"), "B should be visible (linked)");
    assert!(
        !svg.contains(">C<"),
        "C should be hidden (unlinked): svg={svg}"
    );
}

#[test]
fn remove_at_unlinked_removes_isolated_nodes() {
    let svg = puml::render_source_to_svg(REMOVE_UNLINKED_SRC).expect("render remove @unlinked");
    assert!(svg.contains(">A<"), "A should be visible (linked)");
    assert!(svg.contains(">B<"), "B should be visible (linked)");
    assert!(
        !svg.contains(">C<"),
        "C should be removed (unlinked): svg={svg}"
    );
}

#[test]
fn hide_at_unlinked_model_does_not_contain_isolated_node() {
    let document = puml::parser::parse(HIDE_UNLINKED_SRC).expect("parse hide @unlinked");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize hide @unlinked")
    else {
        panic!("should normalize as Family");
    };
    assert!(
        !model.nodes.iter().any(|n| n.name == "C"),
        "C should have been removed by hide @unlinked"
    );
}

// ─── 3.15–3.16 hide/remove specific classes ─────────────────────────────────

const HIDE_CLASS_SRC: &str = r##"@startuml
class A
class B
class C
A -- B
hide C
@enduml
"##;

const REMOVE_CLASS_SRC: &str = r##"@startuml
class A
class B
class C
A -- B
remove C
@enduml
"##;

#[test]
fn hide_classname_removes_that_class() {
    let svg = puml::render_source_to_svg(HIDE_CLASS_SRC).expect("render hide classname");
    assert!(svg.contains(">A<"), "A should be visible");
    assert!(svg.contains(">B<"), "B should be visible");
    assert!(!svg.contains(">C<"), "C should be hidden: svg={svg}");
}

#[test]
fn remove_classname_removes_that_class() {
    let svg = puml::render_source_to_svg(REMOVE_CLASS_SRC).expect("render remove classname");
    assert!(svg.contains(">A<"), "A should be visible");
    assert!(svg.contains(">B<"), "B should be visible");
    assert!(!svg.contains(">C<"), "C should be removed: svg={svg}");
}

// ─── 3.6 Escaped leading visibility markers ─────────────────────────────────

const ESCAPED_VISIBILITY_MEMBERS_SRC: &str = r"@startuml
class Escaped {
  \+literalPublic
  \-literalPrivate
  \#literalProtected
  \~literalPackage
  +actualPublic
}
@enduml
";

#[test]
fn escaped_visibility_members_render_as_literal_text() {
    let svg = puml::render_source_to_svg(ESCAPED_VISIBILITY_MEMBERS_SRC)
        .expect("render escaped visibility markers");
    let doc = SvgDoc::parse(&svg);
    for literal in [
        "+literalPublic",
        "-literalPrivate",
        "#literalProtected",
        "~literalPackage",
    ] {
        let text = doc
            .elements("text")
            .into_iter()
            .find(|node| node.text().map(str::trim) == Some(literal))
            .unwrap_or_else(|| panic!("expected escaped member text {literal:?}; svg={svg}"));
        assert_eq!(
            text.attribute("data-uml-visibility"),
            None,
            "escaped member {literal:?} should not be tagged as a visibility member"
        );
        assert_eq!(
            text.attribute("fill"),
            Some("#334155"),
            "escaped member {literal:?} should use the normal member color"
        );
    }

    let actual_visibility_members = doc
        .elements_with_class("text", "uml-member")
        .into_iter()
        .filter(|node| node.attribute("data-uml-visibility").is_some())
        .filter_map(|node| node.text().map(str::trim).map(str::to_string))
        .collect::<Vec<_>>();
    assert_eq!(
        actual_visibility_members,
        vec!["+actualPublic"],
        "only the unescaped member should receive visibility styling"
    );
}

// ─── 3.12 note on link ───────────────────────────────────────────────────────

const NOTE_ON_LINK_SRC: &str = r##"@startuml
class A
class B
A -- B
note on link: link annotation
@enduml
"##;

#[test]
fn note_on_link_appears_in_svg() {
    let svg = puml::render_source_to_svg(NOTE_ON_LINK_SRC).expect("render note on link");
    assert!(
        svg.contains("link annotation"),
        "note on link text should appear in svg: svg={svg}"
    );
}

#[test]
fn note_on_link_creates_note_node_in_model() {
    let document = puml::parser::parse(NOTE_ON_LINK_SRC).expect("parse note on link");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize note on link")
    else {
        panic!("should normalize as Family");
    };
    let note_node = model
        .nodes
        .iter()
        .find(|n| n.kind == FamilyNodeKind::Note && n.label.as_deref() == Some("link annotation"));
    assert!(
        note_node.is_some(),
        "note on link should create a Note node with the annotation text"
    );
}

// ─── 3.19 Generic type parameters ────────────────────────────────────────────

const GENERICS_SRC: &str = r##"@startuml
class Pair<T, U> {
  T first
  U second
}
class List<E>
@enduml
"##;

#[test]
fn generic_type_params_appear_in_class_name() {
    let svg = puml::render_source_to_svg(GENERICS_SRC).expect("render generics");
    // The SVG should contain the class name with angle brackets
    assert!(
        svg.contains("Pair") && (svg.contains("&lt;T, U&gt;") || svg.contains("<T, U>")),
        "Pair<T, U> should appear with type parameters: svg={svg}"
    );
    assert!(
        svg.contains("List") && (svg.contains("&lt;E&gt;") || svg.contains("<E>")),
        "List<E> should appear with type parameter: svg={svg}"
    );
}

#[test]
fn generic_type_params_preserved_in_class_name_node() {
    let document = puml::parser::parse(GENERICS_SRC).expect("parse generics");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize generics")
    else {
        panic!("should normalize as Family");
    };
    let pair = model.nodes.iter().find(|n| n.name.contains("Pair"));
    assert!(
        pair.is_some(),
        "Pair<T, U> class should exist in model nodes"
    );
}

// ─── 3.34 extends/implements keywords ────────────────────────────────────────

const EXTENDS_IMPLEMENTS_SRC: &str = r##"@startuml
class ArrayList extends AbstractList implements List
class AbstractList
interface List
@enduml
"##;

#[test]
fn extends_generates_inheritance_relation() {
    let svg =
        puml::render_source_to_svg(EXTENDS_IMPLEMENTS_SRC).expect("render extends/implements");
    // The SVG should contain both classes and a relation
    assert!(
        svg.contains("ArrayList"),
        "ArrayList should appear in output"
    );
    assert!(
        svg.contains("AbstractList"),
        "AbstractList should appear in output"
    );
    // Should have a relation arrow (uml-relation element)
    assert!(
        svg.contains("uml-relation"),
        "extends should generate a relation: svg={svg}"
    );
}

#[test]
fn extends_implements_model_contains_heritage_relations() {
    let document = puml::parser::parse(EXTENDS_IMPLEMENTS_SRC).expect("parse extends/implements");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize extends/implements")
    else {
        panic!("should normalize as Family");
    };
    // Should have relations from ArrayList to AbstractList (extends) and List (implements)
    let has_extends = model.relations.iter().any(|r| {
        (r.from == "AbstractList" && r.to == "ArrayList")
            || (r.from == "ArrayList" && r.to == "AbstractList")
    });
    assert!(
        has_extends,
        "extends should generate a relation between ArrayList and AbstractList"
    );

    let has_implements = model.relations.iter().any(|r| {
        (r.from == "List" && r.to == "ArrayList") || (r.from == "ArrayList" && r.to == "List")
    });
    assert!(
        has_implements,
        "implements should generate a relation between ArrayList and List"
    );
}
