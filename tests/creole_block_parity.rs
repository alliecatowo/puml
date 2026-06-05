//! Parity tests for Creole block-level markup: lists, headings, horizontal
//! rules, and tilde escape (wave-9 batch E).
//!
//! These tests call the public tokenizer and SVG renderer; they do not depend
//! on any diagram-family plumbing.

use puml::creole::{render_creole_line_to_tspans, render_creole_to_svg_tspans, tokenize_creole};

// ---------------------------------------------------------------------------
// Bullet lists
// ---------------------------------------------------------------------------

/// `* item` lines produce an indented prefix span followed by the item text.
/// Depth-1 bullets use "• " (U+2022) as the prefix (#1554).
#[test]
fn creole_bullet_list_renders_indented_items() {
    let lines = tokenize_creole("* First item\n* Second item");

    // Each line must start with the bullet prefix span.
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0][0].text, "\u{2022} ", "depth-1 bullet prefix"); // • BULLET
    assert_eq!(lines[0][1].text, "First item");
    assert_eq!(
        lines[1][0].text, "\u{2022} ",
        "second depth-1 bullet prefix"
    ); // • BULLET
    assert_eq!(lines[1][1].text, "Second item");

    // The SVG output must contain the item text.
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("First item"), "bullet text in SVG");
    assert!(svg.contains("Second item"), "second bullet text in SVG");
}

// ---------------------------------------------------------------------------
// Numbered lists
// ---------------------------------------------------------------------------

/// `# item` lines produce a numbered prefix ("1. ") followed by the item text.
#[test]
fn creole_numbered_list_renders_sequence() {
    let lines = tokenize_creole("# Alpha\n# Beta\n# Gamma");

    assert_eq!(lines.len(), 3);
    for line in &lines {
        assert_eq!(line[0].text, "1. ", "numbered prefix");
    }
    assert_eq!(lines[0][1].text, "Alpha");
    assert_eq!(lines[1][1].text, "Beta");
    assert_eq!(lines[2][1].text, "Gamma");

    // Verify SVG contains the text.
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("Alpha"));
    assert!(svg.contains("Beta"));
    assert!(svg.contains("Gamma"));
}

// ---------------------------------------------------------------------------
// Headings
// ---------------------------------------------------------------------------

/// `= H1 =` / `== H2 ==` / `=== H3 ===` must produce progressively smaller
/// font sizes (h1 > h2 > h3) and always set bold.
#[test]
fn creole_heading_increases_font_size() {
    let lines = tokenize_creole("= Heading One =\n== Heading Two ==\n=== Heading Three ===");

    assert_eq!(lines.len(), 3);

    let h1 = &lines[0];
    let h2 = &lines[1];
    let h3 = &lines[2];

    // Text content is extracted without the surrounding `=` markers.
    assert_eq!(h1[0].text, "Heading One");
    assert_eq!(h2[0].text, "Heading Two");
    assert_eq!(h3[0].text, "Heading Three");

    // All headings must be bold.
    assert!(h1[0].bold, "h1 must be bold");
    assert!(h2[0].bold, "h2 must be bold");
    assert!(h3[0].bold, "h3 must be bold");

    // Font sizes must decrease with depth.
    let s1 = h1[0].size.expect("h1 must have a font size");
    let s2 = h2[0].size.expect("h2 must have a font size");
    let s3 = h3[0].size.expect("h3 must have a font size");

    assert!(s1 > s2, "h1 ({s1}) must be larger than h2 ({s2})");
    assert!(s2 > s3, "h2 ({s2}) must be larger than h3 ({s3})");

    // Rough check against the 1.5× / 1.3× / 1.15× spec targets (base ≈ 16).
    assert!(s1 >= 22, "h1 size {s1} should be ≥22 (1.5× base)");
    assert!(s2 >= 19, "h2 size {s2} should be ≥19 (1.3× base)");
    assert!(s3 >= 17, "h3 size {s3} should be ≥17 (1.15× base)");

    // SVG output must include the heading text.
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("Heading One"));
    assert!(svg.contains("Heading Two"));
    assert!(svg.contains("Heading Three"));
    assert!(
        svg.contains("font-weight=\"bold\""),
        "headings must be bold in SVG"
    );
}

// ---------------------------------------------------------------------------
// Horizontal rules
// ---------------------------------------------------------------------------

/// `----`, `====`, and `____` alone on a line must produce a span with
/// `is_hr = true`, and the SVG renderer must emit an SVG `<line>` element.
#[test]
fn creole_horizontal_rule_renders_line() {
    for rule_src in ["----", "====", "____"] {
        let lines = tokenize_creole(rule_src);
        assert_eq!(lines.len(), 1, "single line for {rule_src}");
        assert_eq!(lines[0].len(), 1, "single span for {rule_src}");

        let span = &lines[0][0];
        assert!(
            span.is_hr,
            "span.is_hr must be true for {rule_src}; got: {span:?}"
        );

        // The SVG renderer must produce a <line> element (not just a tspan).
        let svg = render_creole_line_to_tspans(&lines[0], 0, "black");
        assert!(
            svg.contains("<line "),
            "SVG must contain <line> for {rule_src}; got: {svg}"
        );
        assert!(
            svg.contains("stroke"),
            "SVG <line> must have a stroke attribute for {rule_src}"
        );
    }
}

// ---------------------------------------------------------------------------
// Tilde escape
// ---------------------------------------------------------------------------

/// `~X` where X is a Creole metacharacter emits X literally and suppresses
/// markup processing for that character.
#[test]
fn creole_tilde_escapes_metacharacter() {
    // ~** should emit ** literally without activating bold.
    let line_bold = tokenize_creole("~**not bold** literal");
    assert_eq!(line_bold.len(), 1);
    // The leading "**" must appear in the text, not as markup.
    let combined: String = line_bold[0].iter().map(|s| s.text.as_str()).collect();
    assert!(
        combined.contains("**"),
        "escaped ** must appear literally; got: {combined}"
    );
    // No span should be bold.
    for span in &line_bold[0] {
        assert!(
            !span.bold,
            "no span should be bold after tilde escape; span: {span:?}"
        );
    }

    // ~// should emit // literally without activating italic.
    let line_italic = tokenize_creole("~//not italic");
    let combined_i: String = line_italic[0].iter().map(|s| s.text.as_str()).collect();
    assert!(
        combined_i.contains("//"),
        "escaped // must appear literally; got: {combined_i}"
    );
    for span in &line_italic[0] {
        assert!(!span.italic, "no span should be italic after tilde escape");
    }
}

// ---------------------------------------------------------------------------
// Nested bullets (indent depth)
// ---------------------------------------------------------------------------

/// `**` (depth 2) and `***` (depth 3) bullets must produce indented prefixes
/// that are wider than their shallower counterparts.
#[test]
fn creole_nested_bullet_increases_indent() {
    let lines = tokenize_creole("* depth one\n** depth two\n*** depth three");

    assert_eq!(lines.len(), 3);

    let prefix_d1 = &lines[0][0].text;
    let prefix_d2 = &lines[1][0].text;
    let prefix_d3 = &lines[2][0].text;

    // Each deeper level must have a longer (more indented) prefix.
    assert!(
        prefix_d2.len() > prefix_d1.len(),
        "depth-2 prefix ({prefix_d2:?}) must be longer than depth-1 ({prefix_d1:?})"
    );
    assert!(
        prefix_d3.len() > prefix_d2.len(),
        "depth-3 prefix ({prefix_d3:?}) must be longer than depth-2 ({prefix_d2:?})"
    );

    // Item texts must survive unchanged.
    assert_eq!(lines[0][1].text, "depth one");
    assert_eq!(lines[1][1].text, "depth two");
    assert_eq!(lines[2][1].text, "depth three");

    // No text span should be bold (the `*` markers must not leak into content).
    for line in &lines {
        for span in line.iter().skip(1) {
            assert!(!span.bold, "item text must not be bold; got: {span:?}");
        }
    }
}
