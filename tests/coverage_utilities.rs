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
    assert_eq!(DiagramFamily::State.as_str(), "state");
    assert_eq!(DiagramFamily::Activity.as_str(), "activity");
    assert_eq!(DiagramFamily::Component.as_str(), "component");
    assert_eq!(DiagramFamily::Deployment.as_str(), "deployment");
    assert_eq!(DiagramFamily::UseCase.as_str(), "usecase");
    assert_eq!(DiagramFamily::Object.as_str(), "object");
    assert_eq!(DiagramFamily::Unknown.as_str(), "unknown");
}

#[test]
fn render_for_class_family_returns_stub_svg() {
    let src = "@startuml\nclass User\n@enduml\n";
    let svg = render_source_to_svg_for_family(src, DiagramFamily::Class).unwrap();
    assert!(svg.contains("Bootstrap stub for class diagrams"));
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

    assert_eq!(
        detect_diagram_family(sequence).expect("sequence family"),
        DiagramFamily::Sequence
    );
    assert_eq!(
        detect_diagram_family(component).expect("component family"),
        DiagramFamily::Component
    );

    let multipage = "@startuml\nA -> B: one\nnewpage\nB -> A: two\n@enduml\n";
    let err =
        render_source_to_svg(multipage).expect_err("single-page API should reject multipage");
    assert!(
        err.message
            .contains("multiple pages detected; use render_source_to_svgs or --multi")
    );
}

#[test]
fn picouml_pipeline_selection_fails_deterministically_in_library_api() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Picouml,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
    };
    let err = parse_with_pipeline_options("@startuml\nA -> B\n@enduml\n", &options)
        .expect_err("picouml should be unimplemented");
    assert!(
        err.message
            .contains("frontend 'picouml' is not implemented yet")
    );
}

#[test]
fn render_source_to_svg_for_family_rejects_multipage_sequence_input() {
    let src = "@startuml\nA -> B: one\nnewpage\nB -> A: two\n@enduml\n";
    let err = render_source_to_svg_for_family(src, DiagramFamily::Sequence)
        .expect_err("single-page family API should reject multipage sequence");
    assert!(
        err.message
            .contains("multiple pages detected; use render_source_to_svgs or --multi")
    );
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
fn mermaid_pipeline_reports_specific_code_for_unsupported_blocks() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
    };
    let src = "sequenceDiagram\nloop retry\nA->>B: hi\nend\n";
    let err = parse_with_pipeline_options(src, &options).unwrap_err();
    assert!(err.message.contains("E_MERMAID_BLOCK_UNSUPPORTED"));
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
fn mermaid_pipeline_reports_specific_codes_for_create_and_link_constructs() {
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
    };

    let create = "sequenceDiagram\ncreate A\n";
    let create_err = parse_with_pipeline_options(create, &options).unwrap_err();
    assert!(create_err.message.contains("E_MERMAID_CREATE_UNSUPPORTED"));

    let link = "sequenceDiagram\nlink A: https://example.test\n";
    let link_err = parse_with_pipeline_options(link, &options).unwrap_err();
    assert!(link_err.message.contains("E_MERMAID_LINK_UNSUPPORTED"));
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
    assert!(generic_err.message.contains("E_MERMAID_CONSTRUCT_UNSUPPORTED"));
}
