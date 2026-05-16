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
    assert_eq!(DiagramFamily::Unknown.as_str(), "unknown");
}

#[test]
fn render_for_class_family_returns_stub_svg() {
    let src = "@startuml\nclass User\n@enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Class).unwrap();
    assert!(svg.contains("Bootstrap stub for class diagrams"));
}

#[test]
fn render_for_salt_family_returns_stub_svg() {
    let src = "@startsalt\nsalt: login form\n@endsalt\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Salt).unwrap();
    assert!(svg.contains("Bootstrap stub for salt diagrams"));
    assert!(svg.contains("widget"));
}

#[test]
fn render_for_mismatched_family_reports_deterministic_error() {
    let src = "@startuml\nclass User\n@enduml\n";
    let err = render_source_to_svg_for_family(src, DiagramFamily::Sequence).unwrap_err();
    assert!(err.message.contains("E_FAMILY_MISMATCH"));
}

#[test]
fn render_for_unsupported_families_reports_specific_codes() {
    let cases = [
        (
            "@startuml\ncomponent API\n@enduml\n",
            DiagramFamily::Component,
            "E_RENDER_COMPONENT_UNSUPPORTED",
        ),
        (
            "@startuml\nnode web\n@enduml\n",
            DiagramFamily::Deployment,
            "E_RENDER_DEPLOYMENT_UNSUPPORTED",
        ),
        (
            "@startuml\nstate Running\n@enduml\n",
            DiagramFamily::State,
            "E_RENDER_STATE_UNSUPPORTED",
        ),
        (
            "@startuml\nstart\n:work;\nstop\n@enduml\n",
            DiagramFamily::Activity,
            "E_RENDER_ACTIVITY_UNSUPPORTED",
        ),
        (
            "@startuml\nclock clk\n@enduml\n",
            DiagramFamily::Timing,
            "E_RENDER_TIMING_UNSUPPORTED",
        ),
        (
            "@startmindmap\n* Root\n@endmindmap\n",
            DiagramFamily::MindMap,
            "E_RENDER_MINDMAP_UNSUPPORTED",
        ),
        (
            "@startwbs\n* Scope\n@endwbs\n",
            DiagramFamily::Wbs,
            "E_RENDER_WBS_UNSUPPORTED",
        ),
        (
            "@startuml\nfoo bar\n@enduml\n",
            DiagramFamily::Unknown,
            "E_RENDER_FAMILY_UNSUPPORTED",
        ),
    ];

    for (src, family, code) in cases {
        let err = render_source_to_svg_for_family(src, family).expect_err("unsupported family");
        assert!(err.message.contains(code), "missing code {code}");
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
    };

    let supported = "sequenceDiagram\nAlice->Bob: hello\nAlice-->Bob: world\n";
    parse_with_pipeline_options(supported, &options)
        .expect("short mermaid arrows should adapt successfully");

    let unsupported = "sequenceDiagram\nparticipant\n";
    let err = parse_with_pipeline_options(unsupported, &options).unwrap_err();
    assert!(err.message.contains("E_MERMAID_CONSTRUCT_UNSUPPORTED"));
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
    };

    let empty_err = parse_with_pipeline_options("%% comment only\n", &options).unwrap_err();
    assert!(empty_err.message.contains("E_MERMAID_EMPTY"));

    let unsupported_generic = "sequenceDiagram\ntitle   \n";
    let generic_err = parse_with_pipeline_options(unsupported_generic, &options).unwrap_err();
    assert!(generic_err
        .message
        .contains("E_MERMAID_CONSTRUCT_UNSUPPORTED"));
}
