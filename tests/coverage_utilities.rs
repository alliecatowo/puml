use puml::diagnostic::{render_caret_line, Diagnostic, Severity};
use puml::scene::TextOverflowPolicy;
use puml::source::{Source, Span};
use puml::theme::Theme;

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
