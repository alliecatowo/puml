use super::support::*;
use super::*;

#[test]
fn mindmap_palette_fixture_renders_svg() {
    let svg = render_source_to_svg(
        &fs::read_to_string(fixture("families/valid_mindmap_palette.puml")).unwrap(),
    )
    .expect("mindmap should render");
    assert!(svg.starts_with("<svg"), "expected SVG");
    assert!(svg.contains("Root"), "expected Root node in mindmap SVG");
}

#[test]
fn wbs_progress_fixture_renders_svg_with_progress_bar() {
    let svg = render_source_to_svg(
        &fs::read_to_string(fixture("families/valid_wbs_progress.puml")).unwrap(),
    )
    .expect("wbs should render");
    assert!(svg.starts_with("<svg"), "expected SVG");
    assert!(svg.contains("Project"), "expected Project root");
    // Progress bar should be present (blue rect)
    assert!(svg.contains("#3b82f6"), "expected progress bar fill color");
}

#[test]
fn wbs_checked_unchecked_nodes_render_svg() {
    let src = "@startwbs\n* Scope\n** Done [x]\n** Pending [ ]\n@endwbs\n";
    let svg = render_source_to_svg(src).expect("wbs checkbox should render");
    assert!(svg.starts_with("<svg"), "expected SVG");
    assert!(svg.contains("#16a34a"), "expected checked green color");
}

#[test]
fn mindmap_left_side_mode_accepted_in_check() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_mindmap_palette.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn mindmap_plus_minus_prefix_side_assignment() {
    let src = "@startmindmap\n* Root\n+** Right\n-** Left\n@endmindmap\n";
    let svg = render_source_to_svg(src).expect("mindmap +/- prefix should render");
    assert!(svg.starts_with("<svg"), "expected SVG");
    assert!(svg.contains("Right"), "expected Right node");
    assert!(svg.contains("Left"), "expected Left node");
}

#[test]
fn mindmap_orientation_directive_check_passes() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("families/valid_mindmap_orientation.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn picouml_full_constructs_renders_nonempty_svg() {
    let src = fs::read_to_string(fixture("picouml/valid_full_constructs.puml")).unwrap();
    use puml::{parse_with_pipeline_options, FrontendSelection, ParsePipelineOptions};
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Picouml,
        ..ParsePipelineOptions::default()
    };
    let _doc = parse_with_pipeline_options(&src, &options)
        .expect("picouml full constructs must parse via picouml adapter");
}

// ── Issue #103: JSON projection into UML contexts ────────────────────────────

#[test]
fn json_projection_fixture_passes_check() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_json_projection.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn json_projection_render_contains_alias_and_keys() {
    let src = fs::read_to_string(fixture("families/valid_json_projection.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("json projection should render");
    assert!(svg.starts_with("<svg"), "output must be SVG");
    assert!(!svg.is_empty(), "SVG must be non-empty");
    assert!(svg.contains("$user"), "SVG must contain the alias header");
    assert!(svg.contains("name"), "SVG must contain the 'name' key");
}

#[test]
fn json_projection_inline_parse_roundtrip() {
    let src = "@startuml\njson $cfg { \"key\": \"val\" }\n@enduml\n";
    let svg = render_source_to_svg(src).expect("inline json projection should render");
    assert!(svg.contains("$cfg"), "SVG must contain alias '$cfg'");
    assert!(svg.contains("key"), "SVG must contain key 'key'");
}

#[test]
fn json_projection_renders_nested_tree_rows_and_connectors() {
    let src = "@startuml\njson $cfg { \"user\": { \"name\": \"Ada\" }, \"roles\": [\"admin\"] }\n@enduml\n";
    let svg = render_source_to_svg(src).expect("nested json projection should render");
    assert!(
        svg.contains("data-uml-projection-row-label=\"user\""),
        "SVG must contain a parent object row"
    );
    assert!(
        svg.contains("data-uml-projection-row-label=\"name: Ada\""),
        "SVG must contain a nested child row"
    );
    assert!(
        svg.contains("data-uml-projection-row-label=\"roles\""),
        "SVG must contain an array parent row"
    );
    assert!(
        svg.contains("data-uml-projection-row-label=\"[0]: admin\""),
        "SVG must contain an array child row"
    );
    assert!(
        svg.contains("data-uml-projection-row-depth=\"1\""),
        "SVG must include nested projection row depth metadata"
    );
    assert!(
        svg.contains("class=\"uml-projection-connector\""),
        "SVG must include projection connector lines"
    );
}

#[test]
fn json_projection_accepts_partial_rows_and_quoted_braces() {
    let src =
        "@startuml\njson $cfg {\n  \"name\": \"Ada\"\n  \"template\": \"{literal}\"\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("partial JSON projection rows should render");
    assert!(svg.contains("$cfg"), "SVG must contain alias '$cfg'");
    assert!(
        svg.contains("name: Ada"),
        "SVG must contain partial row key"
    );
    assert!(
        svg.contains("template: {literal}"),
        "quoted braces must not close the projection"
    );
}

#[test]
fn yaml_projection_accepts_partial_rows_and_quoted_braces() {
    let src = "@startuml\nyaml $cfg {\n  name: Ada\n  template: \"{literal}\"\n}\n@enduml\n";
    let svg = render_source_to_svg(src).expect("partial YAML projection rows should render");
    assert!(svg.contains("$cfg"), "SVG must contain alias '$cfg'");
    assert!(
        svg.contains("name: Ada"),
        "SVG must contain partial YAML row key"
    );
    assert!(
        svg.contains("template: {literal}"),
        "quoted braces must not close the YAML projection"
    );
}

#[test]
fn yaml_projection_render_contains_alias_and_keys() {
    let src = fs::read_to_string(fixture("families/valid_yaml_projection.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("yaml projection should render");
    assert!(svg.contains("$cfg"), "SVG must contain alias '$cfg'");
    assert!(svg.contains("name"), "SVG must contain YAML key 'name'");
}

#[test]
fn mindmap_caption_and_legend_render_in_svg() {
    let src =
        "@startmindmap\ntitle My Map\ncaption A test diagram\nlegend\nsome legend\nend legend\n* Root\n** Child\n@endmindmap\n";
    let svg = render_source_to_svg(src).expect("mindmap with caption/legend should render");
    assert!(svg.contains("A test diagram"), "expected caption text");
}

#[test]
fn hide_unlinked_removes_unreferenced_participant_from_svg() {
    let src = "@startuml\nhide unlinked\nparticipant Alice\nparticipant Bob\nparticipant Unused\nAlice -> Bob: hello\n@enduml\n";
    let svg = render_source_to_svg(src).expect("hide unlinked diagram should render");
    assert!(svg.contains("Alice"), "expected Alice in rendered SVG");
    assert!(svg.contains("Bob"), "expected Bob in rendered SVG");
    assert!(
        !svg.contains("Unused"),
        "Unused should be filtered by hide unlinked"
    );
}

#[test]
fn hide_unlinked_records_hidden_participants_without_warning() {
    let src = "@startuml\nhide unlinked\nparticipant Alice\nparticipant Bob\nparticipant Unused\nAlice -> Bob: hello\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let ids = model
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["Alice", "Bob"]);
    assert_eq!(model.hidden_participants, vec!["Unused"]);
    assert!(
        model.warnings.is_empty(),
        "hide unlinked should not warn when it filters participants"
    );
}

#[test]
fn hide_unlinked_preserves_explicit_participants_used_by_messages() {
    let src = "@startuml\nhide unlinked\nparticipant User\nparticipant UI\nparticipant Controller\nparticipant Jobs\nUser -> UI: open\nUI -> Controller: dispatch\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let ids = model
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["User", "UI", "Controller"]);
    assert_eq!(model.hidden_participants, vec!["Jobs"]);

    let svg = render_source_to_svg(src).expect("hide unlinked participants should render");
    assert!(svg.contains("User"));
    assert!(svg.contains("UI"));
    assert!(svg.contains("Controller"));
    assert!(!svg.contains("Jobs"));
}

#[test]
fn hide_unlinked_keeps_note_targets_inside_groups() {
    let src = fs::read_to_string(fixture("styling/valid_hide_unlinked_notes_groups.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let ids = model
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["Alice", "Bob", "Carol"]);
    assert_eq!(model.hidden_participants, vec!["Unused"]);

    let svg = render_source_to_svg(&src).expect("hide unlinked note/group should render");
    assert!(svg.contains("Carol"));
    assert!(!svg.contains("Unused"));
}

#[test]
fn sequence_without_hide_unlinked_keeps_declared_unused_participants() {
    let src = "@startuml\nparticipant Alice\nparticipant Bob\nparticipant Unused\nAlice -> Bob: hello\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let ids = model
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["Alice", "Bob", "Unused"]);
    assert!(!model.hide_unlinked);
    assert!(model.hidden_participants.is_empty());
}

#[test]
fn hide_unlinked_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_hide_unlinked.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn extended_skinparams_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_skinparam_extended.puml")])
        .assert()
        .success();
}

#[test]
fn salt_login_form_fixture_renders_svg() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_salt_login_form.puml")])
        .assert()
        .success();
}

#[test]
fn salt_wireframe_grid_renders_button_and_input() {
    let src = "@startsalt\n{\nName: | \"Enter name\"\n[OK]  | [Cancel]\n}\n@endsalt\n";
    let svg = render_source_to_svg(src).expect("salt grid should render");
    assert!(
        svg.contains("Enter name"),
        "expected input placeholder in SVG"
    );
    assert!(svg.contains("OK"), "expected button label in SVG");
}

#[test]
fn salt_basic_widgets_use_intrinsic_plantuml_like_geometry() {
    let svg = render_source_to_svg(include_str!(
        "../fixtures/families/valid_salt_widget_fidelity.puml"
    ))
    .expect("salt fidelity fixture should render");

    assert!(svg.contains("width=\"257\" height=\"172\""));
    assert!(svg.contains("data-salt-style=\"canvas\""));
    assert!(
        !svg.contains("data-salt-style=\"panel\""),
        "plain Salt forms should not get a table panel"
    );
    assert!(
        !svg.contains("stroke=\"#ccc\""),
        "plain Salt forms should not get generic table grid lines"
    );
    assert!(svg.contains("data-salt-widget=\"input\""));
    assert!(svg.contains("<line x1=\"109\" y1=\"22\" x2=\"243\" y2=\"22\""));
    assert!(svg.contains("data-salt-widget=\"button\""));
    assert!(svg.contains("stroke-width=\"2.5\" rx=\"5\" ry=\"5\""));
    assert!(svg.contains("data-salt-widget=\"combo\""));
    assert!(svg.contains("data-salt-widget=\"checkbox\""));
    assert!(svg.contains("<polygon points=\"15,75 18,78 25,69 18,76\""));
    assert!(svg.contains("data-salt-widget=\"radio\""));
}

#[test]
fn salt_tab_strip_renders_as_visual_tab_widgets_not_literal_text() {
    // Regression test for #719 — {/ Tab1 | Tab2 | Tab3 } inside a {+ bordered
    // container was being emitted as literal text instead of a tab-strip widget.
    let src = "@startsalt\n{\n{/ Tab1 | Tab2 | Tab3 }\nContent here\n[OK]\n}\n@endsalt\n";
    let svg = render_source_to_svg(src).expect("salt tab strip should render");
    // The tab bar widget must appear, not bare literal text.
    assert!(
        svg.contains("data-salt-widget=\"tab\""),
        "expected tab widget elements in SVG; literal text was rendered instead"
    );
    assert!(svg.contains("Tab1"), "expected Tab1 label in SVG");
    assert!(svg.contains("Tab2"), "expected Tab2 label in SVG");
    assert!(svg.contains("Tab3"), "expected Tab3 label in SVG");
    // Active tab (index 0) must carry the bold attribute.
    assert!(
        svg.contains("data-salt-tab-active=\"true\""),
        "expected first tab to be marked active"
    );
    // The literal brace-slash syntax must NOT appear as text content.
    assert!(
        !svg.contains("{/ Tab1"),
        "literal '{{/ Tab1' must not appear as text — tab strip was not decoded"
    );
}

#[test]
fn salt_tab_strip_inside_bordered_container_renders_correctly() {
    // The {+ bordered box followed by {/ tab bar must decode the tab bar even
    // while in_text_area state is active.
    let src =
        "@startsalt\n{+\n  {/ First | **Second** | Third }\n  Body text\n  [Cancel]\n}\n@endsalt\n";
    let svg = render_source_to_svg(src).expect("salt nested tab strip should render");
    assert!(
        svg.contains("data-salt-widget=\"tab\""),
        "tab widget must render inside {{+ container"
    );
    // **Second** marks the active tab (index 1).
    assert!(svg.contains("Second"), "Second tab label must appear");
    assert!(
        !svg.contains("{/ First"),
        "literal brace-slash must not leak into output"
    );
}

#[test]
fn chart_area_and_scatter_render_paths_are_supported() {
    let area = "@startchart area\ntitle Area\nA: 4\nB: 7\nC: 3\n@endchart\n";
    let area_svg = render_source_to_svg(area).expect("area chart should render");
    assert!(
        area_svg.contains("<polygon"),
        "area chart should include fill polygon"
    );

    let scatter = "@startchart scatter\ntitle Scatter\nA: 2\nB: 8\nC: 5\n@endchart\n";
    let scatter_svg = render_source_to_svg(scatter).expect("scatter chart should render");
    assert!(
        scatter_svg.matches("<circle").count() >= 3,
        "scatter chart should render point circles"
    );
}

#[test]
fn specialized_chart_render_routes_preprocess_define_like_check_mode() {
    let src = "@startchart\n!define ROW Q1 : 42\nROW\n@endchart\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let cli_svg = Command::cargo_bin("puml")
        .expect("binary")
        .arg("-")
        .write_stdin(src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cli_svg = String::from_utf8(cli_svg).expect("CLI SVG should be UTF-8");
    let lib_svg = render_source_to_svg(src).expect("library chart render should succeed");

    assert_eq!(cli_svg.trim_end(), lib_svg);
    assert!(lib_svg.contains(">Q1<"));
    assert!(lib_svg.contains(">42<"));
    assert!(!lib_svg.contains("!define ROW Q1"));
}

#[test]
fn specialized_regex_render_routes_preprocess_define_like_check_mode() {
    let src = "@startregex\n!define WORD ^foo$\nWORD\n@endregex\n";

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let cli_svg = Command::cargo_bin("puml")
        .expect("binary")
        .arg("-")
        .write_stdin(src)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let cli_svg = String::from_utf8(cli_svg).expect("CLI SVG should be UTF-8");
    let lib_svg = render_source_to_svg(src).expect("library regex render should succeed");

    assert_eq!(cli_svg.trim_end(), lib_svg);
    assert!(lib_svg.contains("foo"));
    assert!(!lib_svg.contains("!define WORD"));
}

#[test]
fn non_uml_advanced_chart_annotations_and_style_directives_render() {
    let src = "@startchart bar\n\
title Revenue\n\
skinparam backgroundColor #f8fafc\n\
skinparam axisColor #334155\n\
palette #0ea5e9 #f97316\n\
caption Forecast confidence\n\
\"Q1\" : 10\n\
\"Q2\" : 18\n\
annotation \"Q2\" : peak quarter\n\
@endchart\n";
    let svg = render_source_to_svg(src).expect("styled annotated chart should render");
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("stroke=\"#334155\""));
    assert!(svg.contains("data-chart-palette=\"#0ea5e9 #f97316\""));
    assert!(svg.contains("data-chart-annotation=\"Q2\""));
    assert!(svg.contains("peak quarter"));
    assert!(svg.contains("data-chart-caption=\"true\""));
}

#[test]
fn non_uml_advanced_regex_localized_descriptive_labels_render() {
    let src = "@startregex\nlocale fr\n^\\d+\\s\\w+$\n@endregex\n";
    let svg = render_source_to_svg(src).expect("localized regex should render");
    assert!(svg.contains("chiffre"));
    assert!(svg.contains("espace"));
    assert!(svg.contains("mot"));
    assert!(svg.contains("debut"));
    assert!(svg.contains("fin"));
}

#[test]
fn regex_exact_and_ranged_quantifiers_render_as_supported_repeats() {
    let src = "@startregex\n^\\d{3}-[A-Z]{2,5}(foo|bar){1,}$\n@endregex\n";
    let svg = render_source_to_svg(src).expect("regex counted quantifiers should render");
    assert!(svg.contains("\\d{3}"), "expected exact count repeat");
    assert!(svg.contains("[A-Z]{2,5}"), "expected bounded range repeat");
    assert!(
        svg.contains("(alt(&#39;foo&#39;|&#39;bar&#39;)){1,}"),
        "expected open-ended range repeat on group"
    );
    assert!(
        !svg.contains("?{3}?"),
        "counted quantifiers should not render as unsupported tokens"
    );
}

#[test]
fn ebnf_exact_and_ranged_quantifiers_render_as_supported_repeats() {
    let src = "@startebnf\nidentifier = letter{1,} , digit{2} , [ \"-\" ]{0,1} ;\n@endebnf\n";
    let svg = render_source_to_svg(src).expect("ebnf counted quantifiers should render");
    assert!(
        svg.contains("letter{1,}"),
        "expected open-ended EBNF repeat"
    );
    assert!(svg.contains("digit{2}"), "expected exact EBNF repeat");
    assert!(
        svg.contains("{0,1}"),
        "expected counted repeat on optional group"
    );
    assert!(
        !svg.contains("?{2}?"),
        "counted EBNF repeats should not render as unsupported tokens"
    );
}

#[test]
fn chart_plantuml_style_named_subtypes_and_colon_points_render() {
    let cases = [
        (
            "@startchart\nbar chart\nQ1 : 42\nQ2 : 58\n@endchart\n",
            "data-chart-type=\"bar\"",
        ),
        (
            "@startchart\nline chart\nJan : 10\nFeb : 15\n@endchart\n",
            "data-chart-type=\"line\"",
        ),
        (
            "@startchart\npie chart\nFrontend : 35\nBackend : 65\n@endchart\n",
            "data-chart-type=\"pie\"",
        ),
    ];
    for (src, marker) in cases {
        let svg = render_source_to_svg(src).expect("chart should render");
        assert!(svg.contains(marker), "expected subtype marker `{marker}`");
        assert!(
            svg.contains("42")
                || svg.contains("10")
                || svg.contains("Frontend")
                || svg.contains("Backend"),
            "expected colon-delimited chart data to render"
        );
    }
}

#[test]
fn non_uml_advanced_ebnf_style_and_rule_notes_render() {
    let src = "@startebnf\n\
style terminal #ecfeff\n\
style nonterminal #eef2ff\n\
note expr : entry point\n\
expr ::= term , { \"+\" , term } ;\n\
term = \"id\" ;\n\
@endebnf\n";
    let svg = render_source_to_svg(src).expect("styled ebnf should render");
    assert!(svg.contains("data-ebnf-style=\"customizable\""));
    assert!(svg.contains("fill=\"#ecfeff\""));
    assert!(svg.contains("fill=\"#eef2ff\""));
    assert!(svg.contains("data-ebnf-note-for=\"expr\""));
    assert!(svg.contains("entry point"));
}

#[test]
fn sdl_state_stereotypes_render_special_shapes() {
    let src = "@startsdl\nstate Start <<start>>\nstate Decide <<decision>>\nstate End <<end>>\nstate Start -> Decide : next\nstate Decide -> End : done\n@endsdl\n";
    let svg = render_source_to_svg(src).expect("sdl stereotypes should render");
    assert!(
        svg.contains("<polygon points="),
        "decision state should render diamond polygon"
    );
    assert!(
        svg.matches("<circle").count() >= 2,
        "start/end should render circular markers"
    );
}

#[test]
fn ditaa_scale_and_transparent_options_are_applied() {
    let baseline = "@startditaa\n+---+\n|A  |\n+---+\n@endditaa\n";
    let baseline_svg = render_source_to_svg(baseline).expect("baseline ditaa should render");
    let src = "@startditaa scale=2 transparent=true\n+---+\n|A  |\n+---+\n@endditaa\n";
    let svg = render_source_to_svg(src).expect("ditaa options should render");
    let baseline_w = extract_svg_width_attr(&baseline_svg).unwrap_or(0);
    let scaled_w = extract_svg_width_attr(&svg).unwrap_or(0);
    assert!(
        scaled_w > baseline_w,
        "scaled ditaa should produce wider canvas: baseline={baseline_w}, scaled={scaled_w}"
    );
    assert!(
        !svg.contains("fill=\"white\""),
        "transparent=true should omit white background"
    );
}

#[test]
fn non_uml_advanced_ditaa_diagonal_connectors_shadow_and_background_options_render() {
    let src = "@startditaa shadow=true background=#f8fafc\n\
+---+   +---+\n\
| A | / | B |\n\
+---+   +---+\n\
@endditaa\n";
    let svg = render_source_to_svg(src).expect("ditaa diagonal connector should render");
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("id=\"ditaa-shadow\""));
    assert!(svg.contains("filter=\"url(#ditaa-shadow)\""));
    assert!(svg.contains("<line"));
}

#[test]
fn non_uml_advanced_math_scripts_accents_and_fences_render() {
    let src = "@startmath\n\\left( \\hat{x}_{i}^{2} + \\vec{v} \\right) = \\sqrt{\\frac{a_1^2}{b}}\n@endmath\n";
    let svg = render_source_to_svg(src).expect("advanced math expression should render");
    assert!(svg.contains("id=\"math-arrow\""));
    assert!(svg.contains("<path d=\"M"));
    assert!(svg.contains("marker-end=\"url(#math-arrow)\""));
    assert!(svg.contains("&quot;") || svg.contains("("));
    assert!(svg.contains("<line"));
}

#[test]
fn non_uml_advanced_math_matrix_symbols_and_text_constructs_render() {
    let src = "@startlatex\n\\begin{bmatrix} \\alpha & \\beta \\\\ \\sum_{i=0}^{n} i & \\int_{0}^{\\infty} e^{-x} dx \\end{bmatrix} \\Rightarrow \\text{ok} \\subseteq \\mathbb{R}\n@endlatex\n";
    let svg = render_source_to_svg(src).expect("matrix math expression should render");
    assert!(svg.contains("data-math-env=\"bmatrix\""));
    assert!(svg.contains("α"));
    assert!(svg.contains("β"));
    assert!(svg.contains("∑"));
    assert!(svg.contains("∫"));
    assert!(svg.contains("∞"));
    assert!(svg.contains("⇒"));
    assert!(svg.contains("⊆"));
    assert!(svg.contains(">ok<"));
}

#[test]
fn non_uml_math_svg_quotes_multi_word_font_family_fallbacks() {
    let src = "@startmath\n\\alpha + \\int_{0}^{\\infty} x\n@endmath\n";
    let svg = render_source_to_svg(src).expect("math SVG should render");
    assert!(
        svg.contains("font-family=\"'Noto Sans Math','STIX Two Math','Cambria Math','Latin Modern Math','DejaVu Serif','Times New Roman',serif\""),
        "math SVG should quote multi-word font family fallbacks for external rasterizers"
    );
    assert!(
        !svg.contains("font-family=\"Noto Sans Math,STIX Two Math,serif\""),
        "legacy unquoted multi-word font stack should not be emitted"
    );
}

#[test]
fn non_uml_advanced_ditaa_junctions_and_diagonal_arrowheads_render() {
    let src = "@startditaa\n\
+---+   +---+\n\
| A |---+-->B\n\
+---+  /    \\\n\
      <      v\n\
@endditaa\n";
    let svg = render_source_to_svg(src).expect("ditaa junctions and diagonal heads should render");
    assert!(svg.contains("marker-end=\"url(#da)\""));
    assert!(svg.contains("marker-start=\"url(#dah)\""));
    assert!(
        svg.matches("<line").count() >= 3,
        "junction and diagonal connectors should emit several line segments"
    );
}

#[test]
fn specialized_renderer_wave_ditaa_shape_tags_and_junction_metadata_render() {
    let src = "@startditaa\n\
+------+   +------+\n\
| cAAA |---+-->{io}\n\
| {c}  |   | {s} |\n\
+------+   +------+\n\
    |          ^\n\
    +----------+\n\
\n\
+------+\n\
| {s}  |\n\
+------+\n\
@endditaa\n";
    let svg = render_source_to_svg(src).expect("ditaa shape tags should render");
    assert!(svg.contains("data-ditaa-shape=\"choice\""));
    assert!(svg.contains("data-ditaa-shape=\"storage\""));
    assert!(svg.contains("data-ditaa-junction=\"true\""));
    assert!(svg.contains("data-ditaa-arrow-end=\"true\""));
}

#[test]
fn specialized_renderer_wave_math_binom_cases_and_construct_metadata_render() {
    let src = "@startlatex\n\\binom{n}{k} + \\begin{cases} x^2 & x \\geq 0 \\\\ -x & x < 0 \\end{cases}\n@endlatex\n";
    let svg = render_source_to_svg(src).expect("math binom/cases should render");
    assert!(svg.contains("data-math-construct=\"binom\""));
    assert!(svg.contains("data-math-env=\"cases\""));
    assert!(svg.contains("≥"));
    assert!(svg.contains(">n<"));
    assert!(svg.contains(">k<"));
}

#[test]
fn specialized_renderer_wave_regex_unicode_lazy_and_repeat_metadata_render() {
    let src = "@startregex\nlang es\n^\\p{Lu}+?\\P{Nd}{2,4}?$\n@endregex\n";
    let svg = render_source_to_svg(src).expect("regex unicode classes should render");
    assert!(svg.contains("data-regex-locale=\"es\""));
    assert!(svg.contains("unicode uppercase letter"));
    assert!(svg.contains("not unicode decimal digit"));
    assert!(svg.contains("data-rail-repeat-label=\"2 to 4\""));
    assert!(svg.contains("class=\"regex-token regex-charclass\""));
}

#[test]
fn specialized_renderer_wave_ebnf_special_sequences_style_block_and_prefix_repeat_render() {
    let src = "@startebnf\n\
<style>\n\
element { ebnf { BackgroundColor #ecfeff FontColor #155e75 LineColor #0891b2 } }\n\
</style>\n\
title Tokens\n\
token = ? unicode category ? | 4 * \"x\" | [ name ];\n\
@endebnf\n";
    let svg = render_source_to_svg(src).expect("ebnf special sequence should render");
    assert!(svg.contains("data-ebnf-special=\"unicode category\""));
    assert!(svg.contains("? unicode category ?"));
    assert!(svg.contains("data-rail-repeat-label=\"exactly 4\""));
    assert!(svg.contains("class=\"ebnf-token ebnf-optional\""));
}
