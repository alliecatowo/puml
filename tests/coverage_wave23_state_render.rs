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

    // After the region-aware naming fix, pseudo-states include a region suffix
    // (e.g. "__r0" for region 0) so that concurrent regions under the same parent
    // produce distinct synthetic names and don't overwrite each other's anchors.
    assert!(
        svg.contains("data-state-from=\"[*]__in__Parent__r0\" data-state-to=\"Child\""),
        "internal start pseudo-state should be scoped to composite with region suffix"
    );
    assert!(
        svg.contains("data-state-from=\"Child\" data-state-to=\"[*]__end__Parent__r0\""),
        "internal end pseudo-state should be scoped to composite with region suffix"
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
fn concurrent_state_example_renders_horizontal_divider_and_connected_outer_transitions() {
    let src = include_str!("../docs/examples/state/03_concurrent.puml");
    let svg = render_source_to_svg(src).expect("concurrent state example should render");
    let doc = SvgDoc::parse(&svg);

    let processing = rect_containing_text(&doc, "Processing");
    let parsing = rect_containing_text(&doc, "Parsing");
    let validating = rect_containing_text(&doc, "Validating");
    let logging = rect_containing_text(&doc, "Logging");
    let auditing = rect_containing_text(&doc, "Auditing");

    // Within each region the children share approximately the same x position
    // (single column, top-to-bottom stacking as per PlantUML 1.2026.5 / UML 2.x).
    assert!(
        (parsing.x - validating.x).abs() <= 20.0,
        "first concurrent region should stay in a single column"
    );
    assert!(
        (logging.x - auditing.x).abs() <= 20.0,
        "second concurrent region should stay in a single column"
    );
    // Region 1 (Logging) must be below region 0 (Parsing/Validating) — top-to-bottom layout.
    assert!(
        logging.y >= validating.bottom() + 4.0,
        "concurrent regions should stack top-to-bottom: Logging (y={}) should be below Validating (bottom={})",
        logging.y,
        validating.bottom()
    );

    // Horizontal dashed divider line: y1 == y2, spanning the inner width.
    let divider = doc
        .elements("line")
        .into_iter()
        .find(|line| {
            line.attribute("stroke-dasharray") == Some("5 3")
                && (f64_attr(*line, "y1") - f64_attr(*line, "y2")).abs() < 1.0
                && (f64_attr(*line, "x1") - f64_attr(*line, "x2")).abs() > 10.0
                && f64_attr(*line, "y1") > processing.y
                && f64_attr(*line, "y1") < processing.bottom()
        })
        .expect("expected horizontal dashed divider inside concurrent composite");
    let divider_y = f64_attr(divider, "y1");
    assert!(
        divider_y > validating.bottom() && divider_y < logging.y,
        "divider should separate the two concurrent regions (divider_y={divider_y}, validating.bottom={}, logging.y={})",
        validating.bottom(),
        logging.y
    );

    // State transitions are now emitted as <path> elements (orthogonal routing).
    // We look up the path by its data-state-from/to attributes and extract the
    // start (x1,y1) and end (x2,y2) coordinates by parsing the first and last
    // coordinate pairs in the SVG path `d` attribute.
    fn path_coords(d: &str) -> (f64, f64, f64, f64) {
        // Collect all numeric tokens (may be "M x y L x y ..." or "M x y L x1 y1 L x2 y2 L x3 y3")
        let nums: Vec<f64> = d
            .split_ascii_whitespace()
            .filter_map(|tok| tok.parse::<f64>().ok())
            .collect();
        assert!(
            nums.len() >= 4,
            "expected at least two coordinate pairs in path d={d:?}"
        );
        let (x1, y1) = (nums[0], nums[1]);
        let (x2, y2) = (nums[nums.len() - 2], nums[nums.len() - 1]);
        (x1, y1, x2, y2)
    }

    let start_transition = doc
        .elements_with_attr("path", "data-state-from", "[*]")
        .into_iter()
        .find(|p| p.attribute("data-state-to") == Some("Processing"))
        .expect("expected outer initial transition into Processing");
    let end_transition = doc
        .elements_with_attr("path", "data-state-from", "Processing")
        .into_iter()
        .find(|p| {
            p.attribute("data-state-to")
                .is_some_and(|target| target == "[*]" || target.starts_with("[*]__end"))
        })
        .expect("expected outer exit transition from Processing");

    let start_d = start_transition
        .attribute("d")
        .expect("start transition path should have d attribute");
    let end_d = end_transition
        .attribute("d")
        .expect("end transition path should have d attribute");
    let (_, _, st_x2, st_y2) = path_coords(start_d);
    let (et_x1, et_y1, _, _) = path_coords(end_d);

    assert!(
        st_x2 >= processing.x && st_x2 <= processing.right(),
        "initial transition should terminate on the composite boundary"
    );
    assert!(
        st_y2 <= processing.y + 1.0,
        "initial transition should connect to the top edge of the composite"
    );
    assert!(
        et_x1 >= processing.x && et_x1 <= processing.right(),
        "exit transition should originate on the composite boundary"
    );
    assert!(
        et_y1 >= processing.bottom() - 1.0,
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
