mod svg_test_helpers;

use puml::render_source_to_svg;
use svg_test_helpers::{bounds, f64_attr, SvgDoc};

#[test]
fn composite_state_scopes_internal_start_and_end_pseudostates() {
    let src = r#"@startuml
state Parent {
  [*] --> Child
  Child --> [*]
}
[*] --> Parent
@enduml"#;

    let svg = render_source_to_svg(src).expect("state svg should render");

    assert!(
        svg.contains("data-state-from=\"[*]__in__Parent\" data-state-to=\"Child\""),
        "internal start pseudo-state should be scoped to composite"
    );
    assert!(
        svg.contains("data-state-from=\"Child\" data-state-to=\"[*]__end__Parent\""),
        "internal end pseudo-state should be scoped to composite"
    );
    assert!(
        svg.contains("data-state-from=\"[*]\" data-state-to=\"Parent\""),
        "outer transition should keep global pseudo-state"
    );
}

#[test]
fn composite_state_keeps_implicit_substates_inside_enclosing_box() {
    let src = r#"
state Parent {
  [*] --> Child
  Child --> Done
  Done --> [*]
}
[*] --> Parent
"#;

    let svg = render_source_to_svg(src).expect("state svg should render");
    let doc = SvgDoc::parse(&svg);

    let parent_rect = rect_containing_text(&doc, "Parent");
    let child_rect = rect_containing_text(&doc, "Child");
    let done_rect = rect_containing_text(&doc, "Done");

    for (label, rect) in [("Child", child_rect), ("Done", done_rect)] {
        assert!(
            rect.x >= parent_rect.x
                && rect.y >= parent_rect.y
                && rect.right() <= parent_rect.right()
                && rect.bottom() <= parent_rect.bottom(),
            "{label} should render inside its composite parent"
        );
        assert_eq!(
            doc.texts_containing(label).len(),
            1,
            "{label} should not render as an orphaned duplicate"
        );
    }
}

#[test]
fn architecture_state_transition_labels_clear_state_boxes() {
    let src = include_str!("../docs/diagrams/diagram-family-lifecycle.puml");
    let svg = render_source_to_svg(src).expect("architecture state svg should render");
    let doc = SvgDoc::parse(&svg);

    let guarded_rects = [
        rect_containing_text(&doc, "Source"),
        rect_containing_text(&doc, "Tokenized"),
        rect_containing_text(&doc, "Parsed"),
        rect_containing_text(&doc, "Normalized"),
        rect_containing_text(&doc, "Diagnostics"),
    ];

    for label in [
        "Preprocessor expands includes and macros",
        "Parser builds AST",
        "Normalizer resolves family and layout",
        "Theme Engine applies styles",
        "Renderer computes geometry and emits nodes",
        "lex or parse error",
        "semantic error",
    ] {
        let text = doc.first_with_attr("text", "data-state-label", label);
        let x = f64_attr(text, "x");
        let y = bounds(text).y;

        for rect in guarded_rects {
            assert!(
                x < rect.x - 12.0
                    || x > rect.right() + 12.0
                    || y < rect.y - 12.0
                    || y > rect.bottom() + 12.0,
                "label {label:?} should clear state rect {rect:?}"
            );
        }
    }

    let parse_error = doc.first_with_attr("text", "data-state-label", "lex or parse error");
    let semantic_error = doc.first_with_attr("text", "data-state-label", "semantic error");
    let dx = (f64_attr(parse_error, "x") - f64_attr(semantic_error, "x")).abs();
    let dy = (bounds(parse_error).y - bounds(semantic_error).y).abs();
    assert!(
        dx >= 8.0 || dy >= 14.0,
        "fan-in labels should not stack on the same anchor"
    );
}

fn rect_containing_text(doc: &SvgDoc<'_>, label: &str) -> svg_test_helpers::Bounds {
    let text = doc
        .texts_containing(label)
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("expected visible <text> node containing {label:?}"));
    let x = f64_attr(text, "x");
    let y = f64_attr(text, "y");
    doc.elements("rect")
        .into_iter()
        .filter(|node| node.attribute("x").is_some() && node.attribute("y").is_some())
        .map(bounds)
        .find(|rect| {
            rect.width > 0.0
                && rect.height > 0.0
                && x >= rect.x
                && x <= rect.right()
                && y >= rect.y
                && y <= rect.bottom()
        })
        .unwrap_or_else(|| panic!("expected rect containing node label {label:?}"))
}
