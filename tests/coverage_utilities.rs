use puml::diagnostic::{render_caret_line, Diagnostic, Severity};
use puml::scene::TextOverflowPolicy;
use puml::source::{Source, Span};
use puml::theme::Theme;
use puml::{
    extract_markdown_diagrams, parse_with_pipeline_options, render_source_to_svg_for_family,
    CompatMode, DeterminismMode, DiagramFamily, FrontendSelection, ParsePipelineOptions,
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
