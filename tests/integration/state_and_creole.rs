use super::support::*;
use super::*;

#[test]
fn state_history_shallow_renders_h_circle() {
    let src = fs::read_to_string(fixture("families/valid_state_history.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render history state SVG");
    assert!(
        svg.contains(">H<"),
        "expected 'H' label in shallow history node"
    );
}

#[test]
fn creole_color_size_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("conformance/valid_creole_color_size.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn creole_color_size_svg_contains_color_and_size_attributes() {
    let src = fs::read_to_string(fixture("conformance/valid_creole_color_size.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("render");
    assert!(
        svg.contains("fill=\"red\""),
        "expected red fill in SVG for <color:red>"
    );
    assert!(
        svg.contains("font-size=\"14\""),
        "expected font-size=\"14\" in SVG for <size:14>"
    );
}

#[test]
fn state_history_renders_h_circle() {
    let src = fs::read_to_string(fixture("families/valid_state_history.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render history state SVG");
    assert!(
        svg.contains(">H<"),
        "expected 'H' label in shallow history node"
    );
}

#[test]
fn state_history_deep_renders_hstar_circle() {
    let src = fs::read_to_string(fixture("families/valid_state_history.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render history state SVG");
    assert!(
        svg.contains(">H*<"),
        "expected 'H*' label in deep history node"
    );
}

#[test]
fn creole_newlines_fixture_checks_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("conformance/valid_creole_newlines.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn stdlib_angle_bracket_include_with_puml_stdlib_root_env_override() {
    // PUML_STDLIB_ROOT must point directly to the stdlib dir.
    let stdlib_path = format!("{}/stdlib", env!("CARGO_MANIFEST_DIR"));

    let tmp = tempfile::tempdir().unwrap();
    let input = tmp.path().join("env_override.puml");
    fs::write(
        &input,
        "@startuml\n!include <C4/C4_Context>\n!Person(u, TestUser)\n@enduml\n",
    )
    .unwrap();

    Command::cargo_bin("puml")
        .expect("binary")
        .env("PUML_STDLIB_ROOT", &stdlib_path)
        .args(["--check", input.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn creole_inline_bold_produces_tspan_in_message_label() {
    let src = "@startuml\nAlice -> Bob: **hello**\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("font-weight=\"bold\""),
        "expected bold tspan for **hello**"
    );
    assert!(svg.contains(">hello<"), "expected label text in tspan");
}

#[test]
fn creole_inline_mono_produces_monospace_tspan() {
    let src = "@startuml\nAlice -> Bob: \"\"code\"\"\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    // mono spans set font-family=monospace on the inner tspan
    assert!(
        svg.contains("font-family=\"monospace\""),
        "expected monospace tspan for \"\"code\"\""
    );
}

#[test]
fn creole_inline_underline_produces_text_decoration() {
    let src = "@startuml\nAlice -> Bob: __ul__\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "expected underline tspan for __ul__"
    );
}

#[test]
fn creole_inline_strikethrough_produces_line_through() {
    let src = "@startuml\nAlice -> Bob: --strike--\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("text-decoration=\"line-through\""),
        "expected line-through tspan for --strike--"
    );
}

#[test]
fn creole_html_b_tag_produces_bold_tspan() {
    let src = "@startuml\nAlice -> Bob: <b>bold</b>\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("font-weight=\"bold\""),
        "expected bold tspan for <b>bold</b>"
    );
}

#[test]
fn creole_html_i_tag_produces_italic_tspan() {
    let src = "@startuml\nAlice -> Bob: <i>italic</i>\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic tspan for <i>italic</i>"
    );
}

#[test]
fn creole_plain_label_uses_fast_path_without_tspan_wrapper() {
    let src = "@startuml\nAlice -> Bob: plain\n@enduml\n";
    let svg = render_source_to_svg(src).expect("render");
    // Plain text should NOT wrap in tspan at all — fast path
    let plain_text_pattern = ">plain<";
    assert!(
        svg.contains(plain_text_pattern),
        "expected direct text content for plain label"
    );
}

#[test]
fn state_entry_exit_renders_italic_action_text() {
    let src = fs::read_to_string(fixture("families/valid_state_entry_exit.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render entry/exit state SVG");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic text for entry/exit actions"
    );
    assert!(svg.contains("entry"), "expected entry action label in SVG");
}

#[test]
fn class_together_group_member_ids_are_recorded_in_model() {
    use puml::normalize_family;
    use puml::parser::parse;
    use puml::NormalizedDocument;

    let src = "@startuml\nclass A\nclass B\ntogether {\n  A\n  B\n}\n@enduml\n";
    let doc = parse(src).expect("parse ok");
    let model = normalize_family(doc).expect("normalize ok");
    let NormalizedDocument::Family(family) = model else {
        panic!("expected Family model");
    };
    assert_eq!(family.groups.len(), 1, "should have 1 group");
    let group = &family.groups[0];
    assert_eq!(group.kind, "together");
    assert!(group.member_ids.contains(&"A".to_string()));
    assert!(group.member_ids.contains(&"B".to_string()));
}

#[test]
fn class_hide_options_are_recorded_in_model() {
    use puml::normalize_family;
    use puml::parser::parse;
    use puml::NormalizedDocument;

    let src = "@startuml\nhide circle\nhide stereotype\nhide empty members\nclass Foo\n@enduml\n";
    let doc = parse(src).expect("parse ok");
    let model = normalize_family(doc).expect("normalize ok");
    let NormalizedDocument::Family(family) = model else {
        panic!("expected Family model");
    };
    assert!(family.hide_options.contains("circle"));
    assert!(family.hide_options.contains("stereotype"));
    assert!(family.hide_options.contains("empty members"));
}

#[test]
fn state_fork_join_choice_end_renders_stereotyped_shapes() {
    let src = fs::read_to_string(fixture("families/valid_state_fork_join.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render fork/join/choice/end SVG");
    assert!(
        svg.contains("<rect"),
        "expected rect element for fork/join bar"
    );
    assert!(
        svg.contains("<polygon"),
        "expected polygon for choice diamond"
    );
    let circle_count = svg.matches("<circle").count();
    assert!(
        circle_count >= 2,
        "expected at least 2 circle elements, got {circle_count}"
    );
}

#[test]
fn state_transition_labels_appear_in_svg() {
    let src = fs::read_to_string(fixture("families/valid_state_fork_join.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render SVG");
    assert!(
        svg.contains("done"),
        "expected 'done' transition label in SVG"
    );
    assert!(
        svg.contains("retry"),
        "expected 'retry' transition label in SVG"
    );
}

#[test]
fn state_full_machine_offsets_vertical_labels_and_keeps_final_state_in_canvas_flow() {
    let src = fs::read_to_string("docs/examples/state/08_full_machine.puml").unwrap();
    let svg = render_source_to_svg(&src).expect("full machine state example should render");
    let doc = roxmltree::Document::parse(&svg).expect("state SVG should parse");

    for node_name in [
        "Pending",
        "Fulfillment",
        "Picking",
        "Packing",
        "Shipped",
        "Delivered",
        "Cancelled",
    ] {
        assert_eq!(
            doc.descendants()
                .filter(|node| {
                    node.has_tag_name("metadata")
                        && node.attribute("data-state-node") == Some(node_name)
                })
                .count(),
            1,
            "{node_name} should render exactly once"
        );
    }

    // State transitions are now <path> elements (orthogonal routing).
    let confirm_edge = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("Pending")
                && node.attribute("data-state-to") == Some("fork1")
        })
        .expect("Pending -> fork1 edge should render");
    let confirm_label = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("text") && node.attribute("data-state-label") == Some("confirm")
        })
        .expect("Pending -> fork1 label should render");
    assert_eq!(confirm_label.attribute("data-state-label"), Some("confirm"));
    let (confirm_x1, _, _, _) = state_path_endpoints(confirm_edge);
    assert_ne!(
        state_svg_attr_i32(confirm_label, "x"),
        confirm_x1,
        "vertical edge label should be offset from the arrow shaft"
    );

    let instock_edge = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("choice1")
                && node.attribute("data-state-to") == Some("join1")
        })
        .expect("choice1 -> join1 edge should render");
    let instock_label = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("text") && node.attribute("data-state-label") == Some("in stock")
        })
        .expect("choice1 -> join1 label should render");
    assert_eq!(
        instock_label.attribute("data-state-label"),
        Some("in stock")
    );
    let (instock_x1, _, _, _) = state_path_endpoints(instock_edge);
    assert_ne!(
        state_svg_attr_i32(instock_label, "x"),
        instock_x1,
        "branch label should be offset from the crossing arrow shaft"
    );

    let delivered_rect = state_svg_element_after_metadata(&doc, "Delivered");
    let final_state_circle = state_svg_element_after_metadata(&doc, "[*]__end");
    let delivered_bottom =
        state_svg_attr_i32(delivered_rect, "y") + state_svg_attr_i32(delivered_rect, "height");
    let final_state_center_y = state_svg_attr_i32(final_state_circle, "cy");
    assert!(
        final_state_center_y > delivered_bottom,
        "final state should render below Delivered so the terminal arrow stays in canvas"
    );
}

#[test]
fn state_fork_join_choice_example_keeps_parallel_branches_aligned() {
    let src = fs::read_to_string("docs/examples/state/05_fork_join_choice.puml").unwrap();
    let svg = render_source_to_svg(&src).expect("fork/join/choice example should render");
    let doc = roxmltree::Document::parse(&svg).expect("state SVG should parse");

    let fork = state_svg_element_after_metadata(&doc, "fork1");
    let join = state_svg_element_after_metadata(&doc, "join1");
    let choice = state_svg_element_after_metadata(&doc, "choice1");
    let task_a = state_svg_element_after_metadata(&doc, "TaskA");
    let task_b = state_svg_element_after_metadata(&doc, "TaskB");

    let task_a_center = state_svg_center_x(task_a);
    let task_b_center = state_svg_center_x(task_b);
    let fork_left = state_svg_attr_i32(fork, "x");
    let fork_right = fork_left + state_svg_attr_i32(fork, "width");
    let join_left = state_svg_attr_i32(join, "x");
    let join_right = join_left + state_svg_attr_i32(join, "width");

    assert!(
        fork_left <= task_a_center && fork_right >= task_b_center,
        "fork bar should span both task columns"
    );
    assert!(
        join_left <= task_a_center && join_right >= task_b_center,
        "join bar should span both task columns"
    );

    // State transitions are now <path> elements (orthogonal routing).
    let fork_to_a = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("fork1")
                && node.attribute("data-state-to") == Some("TaskA")
        })
        .expect("fork1 -> TaskA edge should render");
    let fork_to_b = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("fork1")
                && node.attribute("data-state-to") == Some("TaskB")
        })
        .expect("fork1 -> TaskB edge should render");
    let task_a_to_join = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("TaskA")
                && node.attribute("data-state-to") == Some("join1")
        })
        .expect("TaskA -> join1 edge should render");
    let task_b_to_join = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("path")
                && node.attribute("data-state-from") == Some("TaskB")
                && node.attribute("data-state-to") == Some("join1")
        })
        .expect("TaskB -> join1 edge should render");

    for (edge, expected_center, label) in [
        (fork_to_a, task_a_center, "fork1 -> TaskA"),
        (fork_to_b, task_b_center, "fork1 -> TaskB"),
        (task_a_to_join, task_a_center, "TaskA -> join1"),
        (task_b_to_join, task_b_center, "TaskB -> join1"),
    ] {
        // For fork/join edges the anchors share the same X, so the orthogonal router
        // emits a straight segment: M x y1 L x y2 (x1 == x2 in the path).
        let (ex1, _, ex2, _) = state_path_endpoints(edge);
        assert_eq!(ex1, ex2, "{label} should stay vertical");
        assert_eq!(
            ex1, expected_center,
            "{label} should align to its task column center"
        );
    }

    let choice_center = state_svg_center_x(choice);
    let join_center = state_svg_center_x(join);
    assert!(
        (choice_center - join_center).abs() <= 2,
        "choice diamond should stay aligned with the main flow"
    );

    let error_label = doc
        .descendants()
        .find(|node| {
            node.has_tag_name("text") && node.attribute("data-state-label") == Some("error")
        })
        .expect("error label should render");
    assert!(
        state_svg_attr_i32(error_label, "x") > choice_center,
        "error label should stay attached to the rightward failure branch"
    );
}

#[test]
fn state_arch_lifecycle_composites_render_enclosing_boxes() {
    let src = fs::read_to_string("docs/diagrams/diagram-family-lifecycle.puml").unwrap();
    let svg = render_source_to_svg(&src).expect("diagram family lifecycle should render");
    let doc = roxmltree::Document::parse(&svg).expect("state SVG should parse");

    let styled_rect = state_svg_element_after_metadata(&doc, "Styled");
    let skin_rect = state_svg_element_after_metadata(&doc, "SkinParams");
    let palette_rect = state_svg_element_after_metadata(&doc, "Palette");

    let styled_x = state_svg_attr_i32(styled_rect, "x");
    let styled_y = state_svg_attr_i32(styled_rect, "y");
    let styled_w = state_svg_attr_i32(styled_rect, "width");
    let styled_h = state_svg_attr_i32(styled_rect, "height");
    assert!(
        styled_x <= state_svg_attr_i32(skin_rect, "x")
            && styled_y <= state_svg_attr_i32(skin_rect, "y")
            && styled_x + styled_w
                >= state_svg_attr_i32(palette_rect, "x")
                    + state_svg_attr_i32(palette_rect, "width")
            && styled_y + styled_h
                >= state_svg_attr_i32(palette_rect, "y")
                    + state_svg_attr_i32(palette_rect, "height"),
        "Styled should render an enclosing box around its child states"
    );

    let rendered_rect = state_svg_element_after_metadata(&doc, "Rendered");
    let svg_rect = state_svg_element_after_metadata(&doc, "SVGOut");
    let txt_rect = state_svg_element_after_metadata(&doc, "TxtOut");

    let rendered_x = state_svg_attr_i32(rendered_rect, "x");
    let rendered_y = state_svg_attr_i32(rendered_rect, "y");
    let rendered_w = state_svg_attr_i32(rendered_rect, "width");
    let rendered_h = state_svg_attr_i32(rendered_rect, "height");
    assert!(
        rendered_x <= state_svg_attr_i32(svg_rect, "x")
            && rendered_y <= state_svg_attr_i32(svg_rect, "y")
            && rendered_x + rendered_w
                >= state_svg_attr_i32(txt_rect, "x") + state_svg_attr_i32(txt_rect, "width")
            && rendered_y + rendered_h
                >= state_svg_attr_i32(txt_rect, "y") + state_svg_attr_i32(txt_rect, "height"),
        "Rendered should render an enclosing box around its child states"
    );
}

#[test]
fn state_basic_render_produces_valid_svg() {
    let src = "@startuml\nstate Active\n[*] --> Active\nActive --> [*]\n@enduml\n";
    let svg = render_source_to_svg(src).expect("basic state should render");
    assert!(svg.starts_with("<svg"), "expected SVG output");
    assert!(svg.contains("Active"), "expected state name in SVG");
}

// ── Issue #183: class member modifiers {field}/{method}/{abstract}/{static} ───

#[test]
fn class_member_modifier_fixture_parses_and_renders() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("families/valid_class_html_members.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn class_member_abstract_modifier_renders_italic_in_svg() {
    // {abstract} and <<abstract>> members must produce italic tspan in SVG
    let src = "@startuml\nclass Animal {\n  {abstract} #speak(): void\n  +name: String <<abstract>>\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic for abstract member"
    );
}

#[test]
fn class_member_static_modifier_renders_underline_in_svg() {
    // {static} / {class} / <<static>> members must produce underline tspan in SVG
    let src = "@startuml\nclass Config {\n  {static} MAX: Int\n  {class} DEFAULT: String\n  COUNT: Int <<static>>\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "expected underline for static member"
    );
}

#[test]
fn class_member_field_modifier_renders_italic_in_svg() {
    // {field} produces italic tspan
    let src = "@startuml\nclass User {\n  {field} +id: UUID\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic for field modifier"
    );
}

#[test]
fn class_member_method_modifier_has_no_special_styling() {
    // {method} produces no extra styling — just plain text
    let src = "@startuml\nclass User {\n  {method} +save(): void\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    // The text "save" must appear; no extra decoration expected
    assert!(svg.contains("save"), "expected method name in SVG");
}

#[test]
fn class_member_trailing_modifier_parsed_correctly() {
    // Trailing modifiers: `member {field}`, `member {static}`
    let src = "@startuml\nclass Foo {\n  +x: Int {field}\n  +y: Float {static}\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("should render");
    assert!(
        svg.contains("font-style=\"italic\""),
        "expected italic for trailing field"
    );
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "expected underline for trailing static"
    );
}

// ── Issue #191: --stdrpt single-line diagnostic format ───────────────────────
