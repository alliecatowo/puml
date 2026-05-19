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

#[test]
fn concurrent_state_example_renders_vertical_divider_and_connected_outer_transitions() {
    let src = include_str!("../docs/examples/state/03_concurrent.puml");
    let svg = render_source_to_svg(src).expect("concurrent state example should render");
    let doc = SvgDoc::parse(&svg);

    let processing = rect_containing_text(&doc, "Processing");
    let parsing = rect_containing_text(&doc, "Parsing");
    let validating = rect_containing_text(&doc, "Validating");
    let logging = rect_containing_text(&doc, "Logging");
    let auditing = rect_containing_text(&doc, "Auditing");

    assert!(
        (parsing.x - validating.x).abs() <= 1.0,
        "first concurrent region should stay in a single column"
    );
    assert!(
        (logging.x - auditing.x).abs() <= 1.0,
        "second concurrent region should stay in a single column"
    );
    assert!(
        logging.x >= parsing.right() + 8.0,
        "concurrent regions should render side by side"
    );

    let divider = doc
        .elements("line")
        .into_iter()
        .find(|line| {
            line.attribute("stroke-dasharray") == Some("5 3")
                && f64_attr(*line, "x1") == f64_attr(*line, "x2")
                && f64_attr(*line, "x1") > processing.x
                && f64_attr(*line, "x1") < processing.right()
                && f64_attr(*line, "y1") >= processing.y
                && f64_attr(*line, "y2") <= processing.bottom() + 8.0
        })
        .expect("expected vertical dashed divider inside concurrent composite");
    let divider_x = f64_attr(divider, "x1");
    assert!(
        divider_x > parsing.right() && divider_x < logging.x,
        "divider should separate the two concurrent regions"
    );

    let start_transition = doc
        .elements_with_attr("line", "data-state-from", "[*]")
        .into_iter()
        .find(|line| line.attribute("data-state-to") == Some("Processing"))
        .expect("expected outer initial transition into Processing");
    let end_transition = doc
        .elements_with_attr("line", "data-state-from", "Processing")
        .into_iter()
        .find(|line| {
            line.attribute("data-state-to")
                .is_some_and(|target| target == "[*]" || target.starts_with("[*]__end"))
        })
        .expect("expected outer exit transition from Processing");

    assert!(
        f64_attr(start_transition, "x2") >= processing.x
            && f64_attr(start_transition, "x2") <= processing.right(),
        "initial transition should terminate on the composite boundary"
    );
    assert!(
        f64_attr(start_transition, "y2") <= processing.y + 1.0,
        "initial transition should connect to the top edge of the composite"
    );
    assert!(
        f64_attr(end_transition, "x1") >= processing.x
            && f64_attr(end_transition, "x1") <= processing.right(),
        "exit transition should originate on the composite boundary"
    );
    assert!(
        f64_attr(end_transition, "y1") >= processing.bottom() - 1.0,
        "exit transition should leave from the bottom edge of the composite"
    );
}

/// Regression guard for #476: fork/join/choice diagram must render
/// all nodes and labeled transitions without crossings or orphaned shapes.
#[test]
fn fork_join_choice_renders_all_nodes_and_labels() {
    let src = r#"@startuml
state fork1 <<fork>>
state join1 <<join>>
state choice1 <<choice>>
[*] --> fork1
fork1 --> TaskA
fork1 --> TaskB
TaskA --> join1
TaskB --> join1
join1 --> choice1
choice1 --> Success : ok
choice1 --> Failure : error
@enduml"#;

    let svg = render_source_to_svg(src).expect("fork/join/choice svg should render");
    let doc = SvgDoc::parse(&svg);

    // All state nodes must be present
    for label in ["TaskA", "TaskB", "Success", "Failure"] {
        assert!(
            !doc.texts_containing(label).is_empty(),
            "expected state label {label:?} in SVG"
        );
    }

    // Transition labels must be present
    for label in ["ok", "error"] {
        assert!(
            svg.contains(label),
            "expected transition label {label:?} in SVG"
        );
    }

    // Fork and join bars must be rendered (data-state-kind attributes)
    assert!(
        svg.contains("data-state-kind=\"fork\""),
        "fork node must appear in SVG"
    );
    assert!(
        svg.contains("data-state-kind=\"join\""),
        "join node must appear in SVG"
    );
    assert!(
        svg.contains("data-state-kind=\"choice\""),
        "choice node must appear in SVG"
    );
}

/// Regression guard for #472: intra-composite transitions (within concurrent
/// regions) must appear above the composite background rect.
#[test]
fn concurrent_state_intra_region_transitions_are_visible() {
    let src = r#"@startuml
state Processing {
  state "Parsing" as Parse
  state "Validating" as Validate
  Parse --> Validate : parsed
  ||
  state "Logging" as Log
  state "Auditing" as Audit
  Log --> Audit : logged
}
[*] --> Processing : start
Processing --> [*] : done
@enduml"#;

    let svg = render_source_to_svg(src).expect("concurrent state svg should render");

    // Intra-region transitions must be emitted in the SVG
    assert!(
        svg.contains("data-state-from=\"Parse\" data-state-to=\"Validate\""),
        "Parse→Validate transition must appear in SVG above composite background"
    );
    assert!(
        svg.contains("data-state-from=\"Log\" data-state-to=\"Audit\""),
        "Log→Audit transition must appear in SVG above composite background"
    );

    // Dashed divider must separate the concurrent regions
    assert!(
        svg.contains("stroke-dasharray"),
        "dashed concurrent-region divider must appear in SVG"
    );

    // Outer transition labels must be present
    for label in ["start", "done"] {
        assert!(
            svg.contains(label),
            "expected outer transition label {label:?} in SVG"
        );
    }
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
        .filter(|rect| {
            rect.width > 0.0
                && rect.height > 0.0
                && x >= rect.x
                && x <= rect.right()
                && y >= rect.y
                && y <= rect.bottom()
        })
        .min_by(|a, b| {
            let area_a = a.width * a.height;
            let area_b = b.width * b.height;
            area_a
                .partial_cmp(&area_b)
                .expect("rect areas should be comparable")
        })
        .unwrap_or_else(|| panic!("expected rect containing node label {label:?}"))
}
