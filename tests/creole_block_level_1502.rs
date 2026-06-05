//! Creole block-level markup parity tests for issue #1502.
//!
//! Verifies that PUML's Creole engine handles all five Phase-A block constructs:
//! headings, bullet/numbered lists, inline code (monospace), horizontal rules,
//! and basic pipe tables. Also covers the new definition-list construct
//! (`; Term : Definition`) added in this PR.
//!
//! These tests exercise only the public tokenizer and SVG renderer; they do not
//! depend on any diagram-family plumbing.

use puml::creole::{render_creole_line_to_tspans, render_creole_to_svg_tspans, tokenize_creole};

// ---------------------------------------------------------------------------
// Headings
// ---------------------------------------------------------------------------

/// `= H1 =` heading produces a single bold span with the largest font size.
#[test]
fn heading_h1_is_bold_and_largest() {
    let lines = tokenize_creole("= Main Title =");
    assert_eq!(lines.len(), 1);
    let span = &lines[0][0];
    assert_eq!(span.text, "Main Title");
    assert!(span.bold, "h1 must be bold");
    let size = span.size.expect("h1 must have font-size");
    assert!(size >= 22, "h1 font-size {size} should be >= 22");
}

/// `== H2 ==` heading is bold with a font size between h1 and h3.
#[test]
fn heading_h2_is_bold_and_intermediate() {
    let lines_h1 = tokenize_creole("= Big =");
    let lines_h2 = tokenize_creole("== Medium ==");
    let lines_h3 = tokenize_creole("=== Small ===");

    let s1 = lines_h1[0][0].size.unwrap();
    let s2 = lines_h2[0][0].size.unwrap();
    let s3 = lines_h3[0][0].size.unwrap();

    assert!(s1 > s2, "h1 ({s1}) > h2 ({s2})");
    assert!(s2 > s3, "h2 ({s2}) > h3 ({s3})");
    assert!(lines_h2[0][0].bold, "h2 must be bold");
}

/// `=== H3 ===` strips the surrounding `=` markers and text is bold.
#[test]
fn heading_h3_strips_markers_and_is_bold() {
    let lines = tokenize_creole("=== Section Name ===");
    assert_eq!(lines[0][0].text, "Section Name");
    assert!(lines[0][0].bold);
}

/// Headings without closing `=` markers are also accepted (PlantUML style).
#[test]
fn heading_without_closing_markers() {
    let lines = tokenize_creole("= No Closing");
    assert_eq!(lines[0][0].text, "No Closing");
    assert!(lines[0][0].bold);
    assert!(lines[0][0].size.is_some());
}

/// Heading SVG output contains font-weight bold and the heading text.
#[test]
fn heading_svg_contains_bold_and_text() {
    let lines = tokenize_creole("= Chapter One =");
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("font-weight=\"bold\""), "SVG must be bold");
    assert!(svg.contains("Chapter One"), "SVG must contain heading text");
}

// ---------------------------------------------------------------------------
// Bullet lists
// ---------------------------------------------------------------------------

/// Depth-1 `* item` uses the Unicode BULLET glyph (•) as the prefix (#1554).
#[test]
fn bullet_list_depth1_prefix() {
    let lines = tokenize_creole("* Alpha");
    assert_eq!(lines[0][0].text, "\u{2022} "); // • BULLET
    assert_eq!(lines[0][1].text, "Alpha");
}

/// Depth-2 `** item` prefix is longer than depth-1 prefix.
#[test]
fn bullet_list_depth2_longer_prefix() {
    let lines = tokenize_creole("* outer\n** inner");
    let p1 = &lines[0][0].text;
    let p2 = &lines[1][0].text;
    assert!(
        p2.len() > p1.len(),
        "depth-2 prefix ({p2:?}) must be longer than depth-1 ({p1:?})"
    );
}

/// Bullet list items must NOT be rendered as bold (the `*` must not bleed).
#[test]
fn bullet_list_item_not_bold() {
    let lines = tokenize_creole("* plain item\n** also plain");
    for line in &lines {
        for span in line.iter().skip(1) {
            assert!(!span.bold, "item text must not be bold; span: {span:?}");
        }
    }
}

/// Bullet list SVG output contains the item text.
#[test]
fn bullet_list_svg_contains_item_text() {
    let lines = tokenize_creole("* First\n* Second");
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("First"));
    assert!(svg.contains("Second"));
}

// ---------------------------------------------------------------------------
// Numbered lists
// ---------------------------------------------------------------------------

/// `# item` produces a numbered prefix `1. `.
#[test]
fn numbered_list_depth1_prefix() {
    let lines = tokenize_creole("# Step One");
    assert_eq!(lines[0][0].text, "1. ");
    assert_eq!(lines[0][1].text, "Step One");
}

/// Nested numbered lists (`##`) produce a longer indented prefix.
#[test]
fn numbered_list_depth2_longer_prefix() {
    let lines = tokenize_creole("# top\n## nested");
    let p1 = &lines[0][0].text;
    let p2 = &lines[1][0].text;
    assert!(
        p2.len() > p1.len(),
        "depth-2 numbered prefix ({p2:?}) must be longer than depth-1 ({p1:?})"
    );
}

/// Numbered list SVG output contains the item text.
#[test]
fn numbered_list_svg_contains_text() {
    let lines = tokenize_creole("# Alpha\n# Beta");
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("Alpha"));
    assert!(svg.contains("Beta"));
}

// ---------------------------------------------------------------------------
// Inline code / monospace
// ---------------------------------------------------------------------------

/// `""code""` renders as monospace.
#[test]
fn creole_double_quote_monospace() {
    let lines = tokenize_creole("\"\"inline code\"\"");
    assert!(lines[0][0].mono, "double-quote mono must be monospace");
    assert_eq!(lines[0][0].text, "inline code");
}

/// `<code>...</code>` tag renders as verbatim monospace.
#[test]
fn html_code_tag_is_verbatim_mono() {
    let lines = tokenize_creole("<code>**not bold** raw</code>");
    assert!(lines[0][0].mono, "<code> content must be mono");
    assert!(!lines[0][0].bold, "<code> content must NOT be bold");
    assert_eq!(lines[0][0].text, "**not bold** raw");
}

/// `<code>` block SVG output uses monospace font-family.
#[test]
fn html_code_tag_svg_uses_monospace_font() {
    let lines = tokenize_creole("<code>example</code>");
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("font-family=\"monospace\""));
    assert!(svg.contains("example"));
}

// ---------------------------------------------------------------------------
// Horizontal rules
// ---------------------------------------------------------------------------

/// `----` alone on a line produces an `is_hr` sentinel span.
#[test]
fn hr_dash_produces_is_hr_sentinel() {
    let lines = tokenize_creole("----");
    assert_eq!(lines[0].len(), 1);
    assert!(lines[0][0].is_hr, "---- must produce is_hr span");
}

/// `====` alone on a line also produces `is_hr`.
#[test]
fn hr_equals_produces_is_hr_sentinel() {
    let lines = tokenize_creole("====");
    assert!(lines[0][0].is_hr, "==== must produce is_hr span");
}

/// `____` alone on a line also produces `is_hr`.
#[test]
fn hr_underline_produces_is_hr_sentinel() {
    let lines = tokenize_creole("____");
    assert!(lines[0][0].is_hr, "____ must produce is_hr span");
}

/// The SVG renderer emits an SVG `<line>` element for `is_hr` spans.
#[test]
fn hr_svg_emits_line_element() {
    let lines = tokenize_creole("----");
    let svg = render_creole_line_to_tspans(&lines[0], 0, "black");
    assert!(
        svg.contains("<line "),
        "SVG must contain <line> for ----; got: {svg}"
    );
    assert!(svg.contains("stroke"), "SVG <line> must have stroke attr");
}

/// `.. Section ..` titled rule renders as text (not as is_hr).
#[test]
fn titled_rule_renders_as_text() {
    let lines = tokenize_creole(".. My Section ..");
    assert!(
        !lines[0][0].is_hr,
        "titled rule must not produce is_hr span"
    );
    assert!(
        lines[0][0].text.contains("My Section"),
        "titled rule must contain the section title"
    );
}

// ---------------------------------------------------------------------------
// Basic pipe tables
// ---------------------------------------------------------------------------

/// A pipe table row produces spans in monospace.
#[test]
fn table_cells_are_monospace() {
    let lines = tokenize_creole("| alpha | beta |");
    for span in &lines[0] {
        if !span.text.trim().is_empty() && span.text != " | " {
            assert!(span.mono, "table cell span must be mono; span: {span:?}");
        }
    }
}

/// Header cells (`|= Header |`) are bold and monospace.
#[test]
fn table_header_cells_are_bold() {
    let lines = tokenize_creole("|= Name |= Value |");
    let bold_spans: Vec<_> = lines[0].iter().filter(|s| s.bold).collect();
    assert!(
        !bold_spans.is_empty(),
        "header cells must be bold; line: {:?}",
        lines[0]
    );
}

/// Cell background colour is carried through.
#[test]
fn table_cell_background_color() {
    let lines = tokenize_creole("|<#FF8080> red |");
    let colored = lines[0].iter().find(|s| s.background.is_some());
    assert!(
        colored.is_some(),
        "table cell with <#color> must have a background set"
    );
    assert_eq!(
        colored.unwrap().background.as_deref(),
        Some("#FF8080"),
        "background color must be #FF8080"
    );
}

/// Row background colour propagates to all cells.
#[test]
fn table_row_background_color() {
    let lines = tokenize_creole("<#yellow>| a | b |");
    for span in lines[0]
        .iter()
        .filter(|s| !s.text.trim().is_empty() && s.text != " | ")
    {
        assert_eq!(
            span.background.as_deref(),
            Some("yellow"),
            "row background must propagate to all cells; span: {span:?}"
        );
    }
}

/// Table SVG output contains the cell text.
#[test]
fn table_svg_contains_cell_text() {
    let lines = tokenize_creole("|= Key | Value |");
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("Key"));
    assert!(svg.contains("Value"));
}

// ---------------------------------------------------------------------------
// Definition lists (new in #1502)
// ---------------------------------------------------------------------------

/// `; Term : Definition` renders term bold and definition in normal weight.
#[test]
fn deflist_term_and_definition() {
    let lines = tokenize_creole("; Name : Alice");
    assert_eq!(lines.len(), 1);

    // First span(s) should be bold (the term).
    let term_span = &lines[0][0];
    assert_eq!(term_span.text, "Name");
    assert!(term_span.bold, "definition list term must be bold");

    // There should be a separator span.
    let separator = lines[0].iter().find(|s| s.text == " : ");
    assert!(
        separator.is_some(),
        "definition list must contain ' : ' separator span"
    );

    // There should be a span containing the definition text.
    let def_span = lines[0].iter().find(|s| s.text.contains("Alice"));
    assert!(
        def_span.is_some(),
        "definition list must contain definition text"
    );
    assert!(!def_span.unwrap().bold, "definition text must NOT be bold");
}

/// `; Term` alone (no `: definition`) renders the term bold.
#[test]
fn deflist_term_only() {
    let lines = tokenize_creole("; SomeTerm");
    assert_eq!(lines.len(), 1);
    assert!(!lines[0].is_empty());
    assert_eq!(lines[0][0].text, "SomeTerm");
    assert!(lines[0][0].bold, "lone definition-list term must be bold");

    // No separator should appear.
    let sep = lines[0].iter().find(|s| s.text == " : ");
    assert!(
        sep.is_none(),
        "term-only line must not have ' : ' separator"
    );
}

/// Definition term may contain inline markup.
#[test]
fn deflist_term_with_inline_markup() {
    let lines = tokenize_creole("; **Bold Term** : description here");

    // The first non-empty content spans should be bold (either from the `**`
    // or from the definition-list logic — both make them bold).
    let first_text_span = lines[0]
        .iter()
        .find(|s| !s.text.is_empty() && s.text != " : ")
        .expect("must have at least one content span");
    assert!(
        first_text_span.bold,
        "bold markup inside term must stay bold"
    );
}

/// Definition list SVG output contains both term and definition.
#[test]
fn deflist_svg_contains_term_and_definition() {
    let lines = tokenize_creole("; Color : Blue");
    let svg = render_creole_to_svg_tspans(&lines, 0, "black");
    assert!(svg.contains("Color"), "SVG must contain term text");
    assert!(svg.contains("Blue"), "SVG must contain definition text");
    assert!(
        svg.contains("font-weight=\"bold\""),
        "SVG must mark term as bold"
    );
}

/// Multiple definition list items work as separate lines.
#[test]
fn deflist_multiple_entries() {
    let lines = tokenize_creole("; Host : example.com\n; Port : 8080\n; Proto : HTTPS");
    assert_eq!(lines.len(), 3);
    for (i, line) in lines.iter().enumerate() {
        let term_span = line.first().expect("each line must have spans");
        assert!(
            term_span.bold,
            "line {i} term must be bold; span: {term_span:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Tree (`|_`)
// ---------------------------------------------------------------------------

/// `|_ child` produces a tree-prefix span in mono followed by the item text.
#[test]
fn tree_line_prefix_is_mono() {
    let lines = tokenize_creole("|_ root item");
    assert_eq!(lines[0][0].text, "`- ");
    assert!(lines[0][0].mono, "tree prefix must be monospace");
    assert_eq!(lines[0][1].text, "root item");
}

// ---------------------------------------------------------------------------
// Tilde escape
// ---------------------------------------------------------------------------

/// `~**text**` emits the `**` literally without activating bold.
#[test]
fn tilde_escape_prevents_bold() {
    let lines = tokenize_creole("~**not bold**");
    let combined: String = lines[0].iter().map(|s| s.text.as_str()).collect();
    assert!(combined.contains("**"), "escaped ** must appear literally");
    for span in &lines[0] {
        assert!(!span.bold, "no span should be bold after tilde escape");
    }
}
