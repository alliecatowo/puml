use puml::diagnostic::{render_caret_line, Diagnostic, Severity};
use puml::scene::TextOverflowPolicy;
use puml::source::{Source, Span};
use puml::theme::Theme;
use puml::{
    detect_diagram_family, extract_markdown_diagrams, parse_with_pipeline_options,
    render_source_to_svg, render_source_to_svg_for_family, CompatMode, DeterminismMode,
    DiagramFamily, FrontendSelection, ParsePipelineOptions,
};

#[test]
fn span_len_and_empty_handle_inverted_and_equal_bounds() {
    assert_eq!(Span::new(2, 6).len(), 4);
    assert_eq!(Span::new(5, 2).len(), 0);
    assert!(Span::new(3, 3).is_empty());
    assert!(Span::new(4, 1).is_empty());
    assert!(!Span::new(1, 4).is_empty());
}

#[test]
fn source_slice_clamps_bounds_without_panicking() {
    let src = Source::new("abcdef");
    assert_eq!(src.as_str(), "abcdef");
    assert_eq!(src.slice(Span::new(1, 4)), "bcd");
    assert_eq!(src.slice(Span::new(4, 99)), "ef");
    assert_eq!(src.slice(Span::new(99, 99)), "");
}

#[test]
fn diagnostic_line_col_tracks_lines_and_unicode_columns() {
    let src = "alpha\nµbeta\nfinal";
    let start = src.find('µ').expect("expected micro sign");

    let d = Diagnostic::error("bad token").with_span(Span::new(start, start + 'µ'.len_utf8()));

    assert_eq!(d.line_col(src), Some((2, 1)));

    let beta_offset = src.find("beta").expect("expected beta");
    let d2 = Diagnostic::warning("warn").with_span(Span::new(beta_offset, beta_offset + 1));
    assert_eq!(d2.line_col(src), Some((2, 2)));
}

#[test]
fn diagnostic_without_span_reports_plain_message() {
    let d = Diagnostic::warning("heads up");
    assert_eq!(d.render_with_source("ignored"), "heads up");
    assert_eq!(d.line_col("ignored"), None);
    assert!(matches!(d.severity, Severity::Warning));
}

#[test]
fn render_with_source_includes_location_line_and_caret() {
    let src = "first\nsecond line\nthird";
    let start = src.find("second").expect("expected second");
    let end = start + "second".len();
    let d = Diagnostic::error("problem").with_span(Span::new(start, end));

    let rendered = d.render_with_source(src);
    assert!(rendered.contains("problem at line 2, column 1"));
    assert!(rendered.contains("second line\n^^^^^^"));
}

#[test]
fn render_caret_line_marks_at_least_one_char_for_empty_or_oversized_span() {
    let src = "abc\ndef";

    let empty = render_caret_line(src, Span::new(1, 1));
    assert_eq!(empty, "abc\n ^");

    let oversized = render_caret_line(src, Span::new(4, 100));
    assert_eq!(oversized, "def\n^^^");

    let beyond_end = render_caret_line(src, Span::new(100, 100));
    assert_eq!(beyond_end, "def\n   ^");
}

#[test]
fn theme_new_and_default_enable_footbox_and_empty_skinparams() {
    let fresh = Theme::new();
    assert!(fresh.footbox_visible);
    assert!(fresh.skinparams.is_empty());
    assert_eq!(fresh.text_overflow_policy, TextOverflowPolicy::WrapAndGrow);

    let defaulted = Theme::default();
    assert!(!defaulted.footbox_visible);
    assert!(defaulted.skinparams.is_empty());
    assert_eq!(
        defaulted.text_overflow_policy,
        TextOverflowPolicy::WrapAndGrow
    );
}

#[test]
fn diagram_family_as_str_covers_all_variants() {
    assert_eq!(DiagramFamily::Sequence.as_str(), "sequence");
    assert_eq!(DiagramFamily::Class.as_str(), "class");
    assert_eq!(DiagramFamily::Gantt.as_str(), "gantt");
    assert_eq!(DiagramFamily::Chronology.as_str(), "chronology");
    assert_eq!(DiagramFamily::State.as_str(), "state");
    assert_eq!(DiagramFamily::Activity.as_str(), "activity");
    assert_eq!(DiagramFamily::Component.as_str(), "component");
    assert_eq!(DiagramFamily::Deployment.as_str(), "deployment");
    assert_eq!(DiagramFamily::UseCase.as_str(), "usecase");
    assert_eq!(DiagramFamily::Object.as_str(), "object");
    assert_eq!(DiagramFamily::Salt.as_str(), "salt");
    assert_eq!(DiagramFamily::MindMap.as_str(), "mindmap");
    assert_eq!(DiagramFamily::Wbs.as_str(), "wbs");
    assert_eq!(DiagramFamily::Gantt.as_str(), "gantt");
    assert_eq!(DiagramFamily::Chronology.as_str(), "chronology");
    assert_eq!(DiagramFamily::Unknown.as_str(), "unknown");
}

#[test]
fn render_for_class_family_returns_stub_svg() {
    let src = "@startuml\nclass User\n@enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Class).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("User"));
    assert!(svg.contains("<rect"));
}

#[test]
fn render_for_salt_family_returns_stub_svg() {
    let src = "@startsalt\nwidget submit_button\n@endsalt\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Salt).unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("submit_button"));
}

#[test]
fn class_family_scope_and_hide_controls_affect_stub_output() {
    let src = "@startuml\nhide stereotype\nhide circle\nhide empty members\npackage Domain {\nnamespace Core {\nclass User {\n  <<Entity>>\n  ()\n  --\n  +id: UUID\n}\n}\n}\n@enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Class).unwrap();
    // The `class` keyword is not part of the display label — only the identifier appears.
    assert!(svg.contains("Domain::Core::User"));
    assert!(!svg.contains("class Domain::Core::User"));
    assert!(svg.contains("+id: UUID"));
    assert!(!svg.contains("&lt;&lt;Entity&gt;&gt;"));
    assert!(!svg.contains("()"));
}

#[test]
fn render_for_timeline_families_returns_timeline_preview() {
    let gantt = render_source_to_svg_for_family(
        "@startgantt\n[Build]\n[Build] starts 2026-04-01\n@endgantt\n",
        DiagramFamily::Gantt,
    )
    .unwrap();
    assert!(gantt.contains("<svg"));

    let chronology = render_source_to_svg_for_family(
        "@startchronology\nLaunch happens on 2026-05-15\n@endchronology\n",
        DiagramFamily::Chronology,
    )
    .unwrap();
    assert!(chronology.contains("<svg"));
}

#[test]
fn render_for_mismatched_family_reports_deterministic_error() {
    let src = "@startuml\nclass User\n@enduml\n";
    let err = render_source_to_svg_for_family(src, DiagramFamily::Sequence).unwrap_err();
    assert!(err.message.contains("E_FAMILY_MISMATCH"));
}

#[test]
fn render_for_unsupported_families_reports_specific_codes() {
    // MindMap and WBS are now implemented; only Unknown should error.
    let cases = [(
        "@startuml\nfoo bar\n@enduml\n",
        DiagramFamily::Unknown,
        "E_RENDER_FAMILY_UNSUPPORTED",
    )];

    for (src, family, code) in cases {
        let err = render_source_to_svg_for_family(src, family).expect_err("unsupported family");
        assert!(err.message.contains(code), "missing code {code}");
    }

    // MindMap and WBS now render successfully.
    let mindmap_svg = render_source_to_svg_for_family(
        "@startmindmap\n* Root\n@endmindmap\n",
        DiagramFamily::MindMap,
    )
    .expect("mindmap should render");
    assert!(
        mindmap_svg.contains("<svg"),
        "expected SVG output from mindmap"
    );

    let wbs_svg =
        render_source_to_svg_for_family("@startwbs\n* Scope\n@endwbs\n", DiagramFamily::Wbs)
            .expect("wbs should render");
    assert!(wbs_svg.contains("<svg"), "expected SVG output from wbs");
}

#[test]
fn render_for_implemented_families_produces_svg() {
    let cases = [
        (
            "@startuml\ncomponent API\n@enduml\n",
            DiagramFamily::Component,
        ),
        ("@startuml\nnode web\n@enduml\n", DiagramFamily::Deployment),
        ("@startuml\nstate Running\n@enduml\n", DiagramFamily::State),
        (
            "@startuml\nstart\n:work;\nstop\n@enduml\n",
            DiagramFamily::Activity,
        ),
        ("@startuml\nclock clk\n@enduml\n", DiagramFamily::Timing),
    ];

    for (src, family) in cases {
        let svg = render_source_to_svg_for_family(src, family)
            .unwrap_or_else(|e| panic!("render failed for {}: {}", family.as_str(), e.message));
        assert!(
            svg.contains("<svg") && svg.contains("</svg>"),
            "expected svg envelope for {}",
            family.as_str()
        );
    }
}

#[test]
fn extract_markdown_diagrams_supports_tilde_fences_and_ignores_deep_indentation() {
    let src = concat!(
        "    ```puml\n", // 4-space indentation should be ignored as fence opener
        "@startuml\n",
        "A -> B: ignored\n",
        "@enduml\n",
        "    ```\n",
        "~~~mermaid\n",
        "sequenceDiagram\n",
        "Alice->Bob: hi\n",
        "~~~\n"
    );
    let diagrams = extract_markdown_diagrams(src);
    assert_eq!(
        diagrams.len(),
        1,
        "expected only the tilde fence to be ingested"
    );
    assert_eq!(diagrams[0].fence_frontend, FrontendSelection::Mermaid);
    assert!(diagrams[0].source.contains("Alice->Bob: hi"));
}

#[test]
fn mermaid_pipeline_supports_short_arrows_and_rejects_empty_declaration() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };

    let supported = "sequenceDiagram\nAlice->Bob: hello\nAlice-->Bob: world\n";
    parse_with_pipeline_options(supported, &options)
        .expect("short mermaid arrows should adapt successfully");

    let unsupported = "sequenceDiagram\nparticipant\n";
    let err = parse_with_pipeline_options(unsupported, &options).unwrap_err();
    assert!(err.message.contains("E_MERMAID_CONSTRUCT_UNSUPPORTED"));
}

#[test]
fn mermaid_flowchart_style_and_picouml_multitarget_notes_adapt() {
    let mermaid = "flowchart LR
classDef hot fill:#fef3c7,stroke:#92400e
A[API]:::hot -->|calls| B[(Database)]
style B fill:#dbeafe,stroke:#1d4ed8
class A hot
";
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };
    let parsed = parse_with_pipeline_options(mermaid, &options)
        .expect("mermaid flowchart style/class syntax should adapt");
    assert!(
        parsed.statements.len() >= 3,
        "adapted flowchart should emit component statements and relations"
    );

    let picouml = "@startpicouml
A => B : request
note A,B : shared context
@endpicouml
";
    let pico_options = ParsePipelineOptions {
        frontend: FrontendSelection::Picouml,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };
    parse_with_pipeline_options(picouml, &pico_options)
        .expect("picouml multi-target shorthand note should adapt");
}

#[test]
fn mermaid_pipeline_supports_cross_and_open_sequence_arrows() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };

    let src = "sequenceDiagram\nAlice-xBob: lost\nBob--xAlice: dotted lost\nAlice-)Bob: async open\nBob--)Alice: dotted async open\n";
    let document =
        parse_with_pipeline_options(src, &options).expect("mermaid cross/open arrows should adapt");
    let model = puml::normalize(document).expect("normalize");
    let arrows = model
        .events
        .iter()
        .filter_map(|event| match &event.kind {
            puml::model::SequenceEventKind::Message { arrow, .. } => Some(arrow.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(arrows, vec!["->x", "-->x", "->>", "-->>"]);
}

#[test]
fn picouml_pipeline_supports_reverse_custom_arrows() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Picouml,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };

    let src = "@startpicouml\nAlice <= Bob : reply\nBob <~ Carol : signal\n@endpicouml\n";
    let document =
        parse_with_pipeline_options(src, &options).expect("picouml reverse arrows should adapt");
    let model = puml::normalize(document).expect("normalize");
    let messages = model
        .events
        .iter()
        .filter_map(|event| match &event.kind {
            puml::model::SequenceEventKind::Message {
                from, to, label, ..
            } => Some((from.as_str(), to.as_str(), label.as_deref().unwrap_or(""))),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        messages,
        vec![
            ("Bob", "Alice", "reply <<sync>>"),
            ("Carol", "Bob", "signal <<async>>")
        ]
    );
}

#[test]
fn library_detect_diagram_family_and_single_svg_contracts_are_deterministic() {
    let sequence = "@startuml\nA -> B: hi\n@enduml\n";
    let component = "@startuml\ncomponent API\n@enduml\n";
    let mindmap = "@startmindmap\n* Root\n@endmindmap\n";
    let wbs = "@startwbs\n* Scope\n@endwbs\n";
    let gantt = "@startgantt\n[Build]\n@endgantt\n";
    let chronology = "@startchronology\nLaunch happens on 2026-05-16\n@endchronology\n";

    assert_eq!(
        detect_diagram_family(sequence).expect("sequence family"),
        DiagramFamily::Sequence
    );
    assert_eq!(
        detect_diagram_family(component).expect("component family"),
        DiagramFamily::Component
    );
    assert_eq!(
        detect_diagram_family(mindmap).expect("mindmap family"),
        DiagramFamily::MindMap
    );
    assert_eq!(
        detect_diagram_family(wbs).expect("wbs family"),
        DiagramFamily::Wbs
    );
    assert_eq!(
        detect_diagram_family(gantt).expect("gantt family"),
        DiagramFamily::Gantt
    );
    assert_eq!(
        detect_diagram_family(chronology).expect("chronology family"),
        DiagramFamily::Chronology
    );
    assert_eq!(
        detect_diagram_family("@startsalt\nwidget\n@endsalt\n").expect("salt family"),
        DiagramFamily::Salt
    );

    let multipage = "@startuml\nA -> B: one\nnewpage\nB -> A: two\n@enduml\n";
    let err = render_source_to_svg(multipage).expect_err("single-page API should reject multipage");
    assert!(err
        .message
        .contains("multiple pages detected; use render_source_to_svgs or --multi"));
}

#[test]
fn picouml_pipeline_selection_routes_deterministically_in_library_api() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Picouml,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };
    parse_with_pipeline_options("@startpicouml\nA -> B\n@endpicouml\n", &options)
        .expect("picouml should route via shared model parser");
}

#[test]
fn picouml_pipeline_rejects_mixed_marker_forms_deterministically() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Picouml,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };
    let err = parse_with_pipeline_options("@startpicouml\nA -> B\n@enduml\n", &options)
        .expect_err("mixed picouml/uml markers should be rejected");
    assert!(err.message.contains("E_PICOUML_MARKER_MIXED"));
}

#[test]
fn render_source_to_svg_for_family_rejects_multipage_sequence_input() {
    let src = "@startuml\nA -> B: one\nnewpage\nB -> A: two\n@enduml\n";
    let err = render_source_to_svg_for_family(src, DiagramFamily::Sequence)
        .expect_err("single-page family API should reject multipage sequence");
    assert!(err
        .message
        .contains("multiple pages detected; use render_source_to_svgs or --multi"));
}

#[test]
fn mermaid_pipeline_supports_notes_lifecycle_and_inline_comments() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };
    let src =
        "sequenceDiagram\nA->>B: hi %% comment\nactivate B\nNote over A,B: synced\nautonumber\n";
    parse_with_pipeline_options(src, &options).expect("expanded mermaid subset should adapt");
}

#[test]
fn mermaid_pipeline_accepts_supported_block_constructs() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };
    let src = "sequenceDiagram\nloop retry\nA->>B: hi\nend\n";
    let doc = parse_with_pipeline_options(src, &options)
        .expect("loop/end mermaid blocks should adapt to plantuml groups");
    assert!(
        !doc.statements.is_empty(),
        "expected statements for supported mermaid block construct"
    );
}

#[test]
fn mermaid_pipeline_supports_note_sides_and_destroy_lifecycle() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };
    let src = "sequenceDiagram\nNote left of A: left\ndestroy A\nNote right of A: right\n";
    parse_with_pipeline_options(src, &options)
        .expect("left/right notes and destroy should adapt successfully");
}

#[test]
fn mermaid_pipeline_accepts_create_and_link_constructs() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };

    let create = "sequenceDiagram\ncreate A\n";
    parse_with_pipeline_options(create, &options)
        .expect("`create X` should adapt to plantuml `create X`");

    let create_participant = "sequenceDiagram\ncreate participant Worker\n";
    parse_with_pipeline_options(create_participant, &options)
        .expect("`create participant X` should adapt to plantuml `create X`");

    let link = "sequenceDiagram\nlink A: https://example.test\n";
    parse_with_pipeline_options(link, &options)
        .expect("`link` lines should adapt to a benign plantuml comment placeholder");
}

#[test]
fn mermaid_pipeline_reports_empty_and_generic_construct_errors() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };

    let empty_err = parse_with_pipeline_options("%% comment only\n", &options).unwrap_err();
    assert!(empty_err.message.contains("E_MERMAID_EMPTY"));

    let unsupported_generic = "sequenceDiagram\ntitle   \n";
    let generic_err = parse_with_pipeline_options(unsupported_generic, &options).unwrap_err();
    assert!(generic_err
        .message
        .contains("E_MERMAID_CONSTRUCT_UNSUPPORTED"));
}

use puml::sprites::{
    builtin_sprite, encode_pixels, normalize_sprite_name, parse_hex_grid_sprite,
    parse_packed_sprite, parse_sprite_header_spec, parse_sprite_ref_at, parse_svg_sprite,
    render_sprite, SpriteKind,
};
use puml::theme::{
    apply_monochrome_to_activity_style, apply_monochrome_to_chart_style,
    apply_monochrome_to_class_style, apply_monochrome_to_component_style,
    apply_monochrome_to_sequence_style, apply_monochrome_to_state_style,
    apply_monochrome_to_timing_style, chart_style_from_sequence_theme, classify_class_skinparam,
    classify_component_skinparam, classify_sequence_skinparam, component_style_from_sequence_theme,
    resolve_sequence_theme_preset, timing_style_from_sequence_theme, ActivitySkinParamValue,
    ActivityStyle, ActorStyle, ChartSkinParamValue, ChartStyle, ClassSkinParamValue, ClassStyle,
    ComponentSkinParamValue, ComponentStyle, ComponentStyleMode, GenericSkinParamValue,
    GroupHeaderFontStyle, MessageAlign, MonochromeMode, SequenceSkinParamSupport,
    SequenceSkinParamValue, SequenceStyle, SkinParamSupport, StateStyle, TextAlignment,
    TimingStyle, LOCAL_SEQUENCE_THEME_CATALOG,
};

fn mono_pixels(def: &puml::sprites::SpriteDefinition) -> &[u8] {
    match &def.kind {
        SpriteKind::Monochrome { pixels } => pixels,
        SpriteKind::Svg { .. } => panic!("expected monochrome sprite"),
    }
}

#[test]
fn sprite_refs_parse_scale_color_clamps_and_rejects_empty_names() {
    assert_eq!(normalize_sprite_name("  \"$server\"  "), "$server");

    let (scaled, consumed) = parse_sprite_ref_at("<$server*2.5> tail").expect("sprite ref");
    assert_eq!(scaled.name, "server");
    assert_eq!(scaled.scale, 2.5);
    assert_eq!(scaled.color, None);
    assert_eq!(consumed, "<$server*2.5>".len());

    let (param_ref, _) = parse_sprite_ref_at("<$server{scale=999, colour=#AABBCC, ignored=value}>")
        .expect("param sprite ref");
    assert_eq!(param_ref.scale, 32.0);
    assert_eq!(param_ref.color.as_deref(), Some("#AABBCC"));

    let (fallback_scale, _) =
        parse_sprite_ref_at("<$server,not-a-scale,color=orange>").expect("comma sprite ref");
    assert_eq!(fallback_scale.scale, 1.0);
    assert_eq!(fallback_scale.color.as_deref(), Some("orange"));

    assert!(parse_sprite_ref_at("<$>").is_none());
    assert!(parse_sprite_ref_at("plain <$server>").is_none());
    assert!(parse_sprite_ref_at("<$server{scale=2>").is_none());
}

#[test]
fn hex_grid_sprites_validate_dimensions_and_scale_gray_levels() {
    let rows = vec!["0F".to_string(), "84".to_string()];
    let sprite = parse_hex_grid_sprite("$grid", Some(2), Some(2), 8, &rows).expect("hex grid");

    assert_eq!(sprite.name, "grid");
    assert_eq!(sprite.width, 2);
    assert_eq!(sprite.height, 2);
    assert_eq!(sprite.gray_levels, 8);
    assert_eq!(mono_pixels(&sprite), &[0, 7, 4, 2]);

    assert!(parse_hex_grid_sprite("empty", None, None, 16, &[]).is_err());
    assert!(parse_hex_grid_sprite("zero", Some(0), Some(1), 16, &["0".into()]).is_err());
    assert!(parse_hex_grid_sprite("height", Some(1), Some(2), 16, &["0".into()]).is_err());
    assert!(parse_hex_grid_sprite("width", Some(2), Some(1), 16, &["0".into()]).is_err());
    assert!(parse_hex_grid_sprite("bad", Some(1), Some(1), 16, &["G".into()]).is_err());
}

#[test]
fn packed_sprite_round_trips_uncompressed_and_compressed_depths() {
    let pixels = [0, 3, 6, 9, 12, 15];

    let encoded_4 = encode_pixels("packed", 2, 3, 4, false, &pixels).expect("encode 4-level");
    assert!(encoded_4.starts_with("sprite $packed [2x3/4] {"));
    let payload_4 = encoded_4
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            (!trimmed.starts_with("sprite") && trimmed != "}").then_some(trimmed)
        })
        .collect::<Vec<_>>()
        .join("");
    let decoded_4 =
        parse_packed_sprite("$packed", 2, 3, 4, false, &payload_4).expect("decode 4-level");
    assert_eq!(mono_pixels(&decoded_4), &[0, 0, 1, 2, 3, 3]);

    let encoded_8 = encode_pixels("packed", 3, 2, 8, false, &pixels).expect("encode 8-level");
    let payload_8 = encoded_8
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            (!trimmed.starts_with("sprite") && trimmed != "}").then_some(trimmed)
        })
        .collect::<Vec<_>>()
        .join("");
    let decoded_8 =
        parse_packed_sprite("$packed", 3, 2, 8, false, &payload_8).expect("decode 8-level");
    assert_eq!(mono_pixels(&decoded_8), &[0, 1, 3, 4, 6, 7]);

    let encoded_z = encode_pixels("zip", 3, 2, 16, true, &pixels).expect("encode compressed");
    let payload_z = encoded_z
        .split_whitespace()
        .last()
        .expect("compressed payload");
    let decoded_z = parse_packed_sprite("$zip", 3, 2, 16, true, payload_z).expect("decode z");
    assert_eq!(mono_pixels(&decoded_z), pixels);

    assert!(encode_pixels("bad", 2, 2, 16, false, &[1, 2, 3]).is_err());
    assert!(encode_pixels("bad", 1, 1, 5, false, &[1]).is_err());
    assert!(parse_packed_sprite("bad", 0, 1, 16, false, "0").is_err());
    assert!(parse_packed_sprite("bad", 2, 2, 8, false, "0").is_err());
    assert!(parse_packed_sprite("bad", 1, 1, 8, false, "?").is_err());
    assert!(parse_packed_sprite("bad", 1, 1, 16, true, "?").is_err());
}

#[test]
fn sprite_headers_svg_dimensions_and_rendering_cover_fallbacks() {
    assert_eq!(
        parse_sprite_header_spec("[2x3/16]"),
        Some((2, 3, 16, false))
    );
    assert_eq!(parse_sprite_header_spec("[2X3/8z]"), Some((2, 3, 8, true)));
    assert_eq!(parse_sprite_header_spec("[2x3/4Z]"), Some((2, 3, 4, true)));
    assert_eq!(parse_sprite_header_spec("[2x3/5]"), None);
    assert_eq!(parse_sprite_header_spec("2x3/16"), None);

    let svg = parse_svg_sprite(
        "$svg",
        "<svg width=\"2.2px\" height=\"3.1px\"><path d=\"M0 0\"/></svg>",
    )
    .expect("svg dimensions");
    assert_eq!((svg.width, svg.height), (3, 4));

    let viewbox = parse_svg_sprite("view", "<svg viewBox=\"0 0 4.2 5.8\"></svg>").expect("viewBox");
    assert_eq!((viewbox.width, viewbox.height), (5, 6));

    let fallback = parse_svg_sprite("fallback", "<svg><path/></svg>").expect("default dims");
    assert_eq!((fallback.width, fallback.height), (16, 16));
    assert!(parse_svg_sprite("$", "<svg/>").is_err());

    let builtin = builtin_sprite("a&b", "jar:demo");
    let rendered_default = render_sprite(
        &builtin,
        1.25,
        2.5,
        &puml::sprites::SpriteRef {
            name: "a&b".to_string(),
            scale: 0.5,
            color: None,
        },
    );
    assert!(rendered_default.contains("data-sprite=\"a&amp;b\""));
    assert!(rendered_default.contains("translate(1.25,2.50) scale(0.500)"));
    assert!(rendered_default.contains("fill=\"#111827\""));
    assert!(rendered_default.contains("fill-opacity=\"1.000\""));

    let rendered_svg = render_sprite(
        &svg,
        0.0,
        0.0,
        &puml::sprites::SpriteRef {
            name: "svg".to_string(),
            scale: 3.0,
            color: Some("#123456".to_string()),
        },
    );
    assert!(rendered_svg.contains("puml-sprite-svg"));
    assert!(rendered_svg.contains("<path d=\"M0 0\"/>"));
    assert!(rendered_svg.contains("scale(3.000)"));
}

#[test]
fn every_local_theme_resolves_and_family_fallback_styles_are_stable() {
    for name in LOCAL_SEQUENCE_THEME_CATALOG {
        let preset = resolve_sequence_theme_preset(name).expect("catalog theme should resolve");
        assert_eq!(preset.name, *name);
        assert!(!preset.style.arrow_color.is_empty());
    }

    assert!(resolve_sequence_theme_preset("")
        .unwrap_err()
        .contains("missing theme name"));
    assert!(resolve_sequence_theme_preset("plain from remote")
        .unwrap_err()
        .contains("unsupported !theme source"));
    assert!(resolve_sequence_theme_preset("plain extra")
        .unwrap_err()
        .contains("malformed !theme syntax"));
    assert!(resolve_sequence_theme_preset("missing-theme")
        .unwrap_err()
        .contains("available local themes"));

    let default_sequence = SequenceStyle::default();
    assert_eq!(
        timing_style_from_sequence_theme(&default_sequence).background_color,
        "#ffffff"
    );
    assert_eq!(
        chart_style_from_sequence_theme(&default_sequence).background_color,
        "#ffffff"
    );

    let themed = SequenceStyle {
        background_color: Some("#101820".to_string()),
        ..SequenceStyle::default()
    };
    assert_eq!(
        timing_style_from_sequence_theme(&themed).background_color,
        "#101820"
    );
    assert_eq!(
        chart_style_from_sequence_theme(&themed).background_color,
        "#101820"
    );
    assert_eq!(
        component_style_from_sequence_theme(&themed).component_style_mode,
        ComponentStyleMode::Uml2
    );
}

#[test]
fn sequence_skinparam_branches_cover_values_noops_and_invalid_inputs() {
    assert_eq!(
        classify_sequence_skinparam("DefaultTextAlignment", "right"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::DefaultTextAlignment(
            TextAlignment::Right
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("SequenceMessageAlign", "reverse-direction"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::MessageAlign(
            MessageAlign::Right
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("SequenceReferenceAlign", "direction"),
        SequenceSkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_sequence_skinparam("SequenceGroupHeaderFontStyle", "italic"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::GroupHeaderFontStyle(
            GroupHeaderFontStyle::Italic
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("lifelineStrategy", "solid"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::LifelineNoSolid(
            false
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("monochrome", "reverse"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Monochrome(
            MonochromeMode::Reverse
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("monochrome", "off"),
        SequenceSkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_sequence_skinparam("handwritten", "yes"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::Handwritten(true))
    );
    assert_eq!(
        classify_sequence_skinparam("defaultFontName", "  "),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("defaultFontSize", "large"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("backgroundColor", "#abcd"),
        SequenceSkinParamSupport::SupportedWithValue(SequenceSkinParamValue::BackgroundColor(
            "#abcd".to_string()
        ))
    );
    assert_eq!(
        classify_sequence_skinparam("SequenceReferenceAlign", "diagonal"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("roundCorner", "round"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("shadowing", "maybe"),
        SequenceSkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_sequence_skinparam("unknownSequenceParam", "x"),
        SequenceSkinParamSupport::UnsupportedKey
    );
}

#[test]
fn class_and_component_skinparam_selectors_cover_scoped_and_fallback_branches() {
    assert_eq!(
        classify_class_skinparam("ClassBackgroundColor<<Entity>>", "#abc"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::StereotypeBackgroundColor(
            "entity".to_string(),
            "#abc".to_string()
        ))
    );
    assert_eq!(
        classify_class_skinparam("ClassBorderColor<<Entity>>", "navy"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::StereotypeBorderColor(
            "entity".to_string(),
            "#000080".to_string()
        ))
    );
    assert_eq!(
        classify_class_skinparam("ClassHeaderBackgroundColor<<Entity>>", "white"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::StereotypeHeaderBackgroundColor(
            "entity".to_string(),
            "#ffffff".to_string()
        ))
    );
    assert_eq!(
        classify_class_skinparam("ClassFontColor<<Entity>>", "black"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::StereotypeFontColor(
            "entity".to_string(),
            "#000000".to_string()
        ))
    );
    assert_eq!(
        classify_class_skinparam("shadowing<<Entity>>", "off"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_class_skinparam("ClassFontSize<<Entity>>", "12"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_class_skinparam("ClassFontColor<<Entity>>", "bad value"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_class_skinparam("notAClassParam<<Entity>>", "red"),
        SkinParamSupport::UnsupportedKey
    );
    assert_eq!(
        classify_class_skinparam("ActorStyle", "awesome"),
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::ActorStyle(ActorStyle::Awesome))
    );
    assert_eq!(
        classify_class_skinparam("ActorStyle", "stick"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_class_skinparam("FontName", ""),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_class_skinparam("Monochrome", "false"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_class_skinparam("Handwritten", "maybe"),
        SkinParamSupport::UnsupportedValue
    );

    assert_eq!(
        classify_component_skinparam("componentStyle", ""),
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::StyleMode(
            ComponentStyleMode::Uml2
        ))
    );
    assert_eq!(
        classify_component_skinparam("componentStyle", "rectangle"),
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::StyleMode(
            ComponentStyleMode::Rectangle
        ))
    );
    assert_eq!(
        classify_component_skinparam("componentStyle", "circle"),
        SkinParamSupport::UnsupportedValue
    );
    assert_eq!(
        classify_component_skinparam("PortColor", "teal"),
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::InterfaceColor(
            "#008080".to_string()
        ))
    );
    assert_eq!(
        classify_component_skinparam("PackageFontName", "Courier"),
        SkinParamSupport::SupportedNoop
    );
}

#[test]
fn monochrome_application_updates_all_family_styles() {
    let mut sequence = SequenceStyle {
        shadowing: true,
        ..SequenceStyle::default()
    };
    apply_monochrome_to_sequence_style(&mut sequence, MonochromeMode::Reverse);
    assert_eq!(sequence.arrow_color, "#ffffff");
    assert_eq!(sequence.background_color.as_deref(), Some("#000000"));
    assert!(!sequence.shadowing);

    let mut class_style = ClassStyle {
        stereotype_styles: [(
            "entity".to_string(),
            puml::theme::ClassStereotypeStyle {
                background_color: Some("#abc".to_string()),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..ClassStyle::default()
    };
    apply_monochrome_to_class_style(&mut class_style, MonochromeMode::True);
    assert_eq!(class_style.background_color, "#ffffff");
    assert!(class_style.stereotype_styles.is_empty());

    let mut state = StateStyle::default();
    apply_monochrome_to_state_style(&mut state, MonochromeMode::Reverse);
    assert_eq!(state.background_color, "#000000");
    assert_eq!(state.start_color, "#ffffff");

    let mut component = ComponentStyle::default();
    apply_monochrome_to_component_style(&mut component, MonochromeMode::True);
    assert_eq!(component.interface_color, "#ffffff");

    let mut activity = ActivityStyle::default();
    apply_monochrome_to_activity_style(&mut activity, MonochromeMode::Reverse);
    assert_eq!(activity.diamond_color, "#000000");

    let mut timing = TimingStyle::default();
    apply_monochrome_to_timing_style(&mut timing, MonochromeMode::True);
    assert_eq!(timing.grid_color, "#000000");

    let mut chart = ChartStyle::default();
    apply_monochrome_to_chart_style(&mut chart, MonochromeMode::Reverse);
    assert_eq!(chart.pie_border_color, "#ffffff");
}

#[test]
fn generic_skinparam_invalid_values_are_reported_for_family_helpers() {
    use puml::theme::{
        classify_activity_skinparam, classify_archimate_skinparam, classify_chart_skinparam,
        classify_ditaa_skinparam, classify_gantt_skinparam, classify_mindmap_skinparam,
        classify_nwdiag_skinparam, classify_salt_skinparam, classify_sdl_skinparam,
        classify_timeline_skinparam, classify_wbs_skinparam,
    };

    assert_eq!(
        classify_activity_skinparam("ActivityDiamondColor", "#123456"),
        SkinParamSupport::SupportedWithValue(ActivitySkinParamValue::DiamondBackgroundColor(
            "#123456".to_string()
        ))
    );
    assert_eq!(
        classify_activity_skinparam("ActivityFontSize", "12"),
        SkinParamSupport::SupportedNoop
    );
    assert_eq!(
        classify_chart_skinparam("ChartPieBorderColor", "#123456"),
        SkinParamSupport::SupportedWithValue(ChartSkinParamValue::PieBorderColor(
            "#123456".to_string()
        ))
    );
    assert_eq!(
        classify_chart_skinparam("LegendFontSize", "12"),
        SkinParamSupport::SupportedNoop
    );

    let invalid_generic = [
        classify_gantt_skinparam("FontSize", "large"),
        classify_mindmap_skinparam("NodeFontSize", "large"),
        classify_wbs_skinparam("FontSize", "large"),
        classify_timeline_skinparam("FontSize", "large"),
        classify_nwdiag_skinparam("FontSize", "large"),
        classify_archimate_skinparam("FontSize", "large"),
        classify_sdl_skinparam("FontSize", "large"),
        classify_ditaa_skinparam("FontSize", "large"),
        classify_salt_skinparam("FontSize", "large"),
    ];
    assert!(invalid_generic
        .iter()
        .all(|support| matches!(support, SkinParamSupport::UnsupportedValue)));

    assert_eq!(
        classify_gantt_skinparam("BorderColor", "bad value"),
        SkinParamSupport::<GenericSkinParamValue>::UnsupportedValue
    );
}
