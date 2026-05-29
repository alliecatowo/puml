//! Salt diagram regression tests for issue #1294 — four widget/grammar defects.
//!
//! Each test covers exactly one of the P0 bugs found in the 2026-05-28 visual audit:
//!
//! 1. `widget` keyword must be consumed as a no-op, not rendered as literal text.
//! 2. Menu bar `|` pipe separators must appear in the SVG output between items.
//! 3. Combobox dropdown arrow must point DOWN (▼), not up (▲).
//! 4. `..` and `==` horizontal-rule separators must render as lines, not literal text.

/// Helper: render a salt source string and return the SVG output.
fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("salt source should render without error")
}

// ---------------------------------------------------------------------------
// Bug 1 — `widget` keyword must be a no-op, not emitted as literal text
// ---------------------------------------------------------------------------

/// A bare `widget submit_button` line in a salt diagram must be silently consumed.
/// The string "widget" or "submit_button" must NOT appear as visible label text in
/// the SVG, and the diagram must still render without error.
#[test]
fn salt_widget_keyword_is_silent_no_op() {
    // This is the exact fixture that revealed the bug (valid_salt_bootstrap.puml).
    let src = "@startsalt\nwidget submit_button\n@endsalt\n";
    let svg = render_svg(src);

    // The raw token "widget" must not appear as a rendered text label.
    // It is acceptable for "widget" to appear inside an XML attribute (e.g.
    // data-salt-widget="...") but it must NOT appear as a <text> content node.
    let text_occurrences: Vec<_> = svg.match_indices(">widget").collect();
    assert!(
        text_occurrences.is_empty(),
        "the `widget` keyword must NOT be rendered as visible label text; got: {svg}"
    );

    // The diagram must still produce valid SVG even when `widget` is the only content.
    assert!(
        svg.contains("<svg"),
        "diagram must still render a valid SVG root element; got: {svg}"
    );
}

/// `widget` with mixed case must also be a no-op.
#[test]
fn salt_widget_keyword_case_insensitive_no_op() {
    let src =
        "@startsalt\n{\n| Name | \"Alice\" |\nWidget MyGroup\n| [OK] | [Cancel] |\n}\n@endsalt\n";
    let svg = render_svg(src);

    // Widgets keyword line must not appear as label text in <text> elements.
    assert!(
        !svg.contains(">Widget"),
        "Widget (mixed case) must not be rendered as label text; got: {svg}"
    );
    // The actual button cells must still be present.
    assert!(svg.contains("OK"), "button label 'OK' must still render");
    assert!(
        svg.contains("Cancel"),
        "button label 'Cancel' must still render"
    );
}

// ---------------------------------------------------------------------------
// Bug 2 — Menu bar `|` pipe separators must appear between items
// ---------------------------------------------------------------------------

/// `{* File | Edit | View | Help }` must emit a visible separator element between
/// each adjacent pair of menu items. We check for the `data-salt-widget="menu-separator"`
/// attribute which the fixed renderer emits.
#[test]
fn salt_menu_bar_pipe_separators_rendered_between_items() {
    let src = "@startsalt\n{\n{* File | Edit | View | Help }\n}\n@endsalt\n";
    let svg = render_svg(src);

    // The menu rect must be present.
    assert!(
        svg.contains("data-salt-widget=\"menu\""),
        "SVG must contain a salt menu widget rect; got: {svg}"
    );

    // Between 4 items there are exactly 3 separators.
    let separator_count = svg.matches("data-salt-widget=\"menu-separator\"").count();
    assert_eq!(
        separator_count, 3,
        "four-item menu bar must have exactly 3 separators; got {separator_count} in: {svg}"
    );

    // All four item labels must also be present.
    assert!(svg.contains("File"), "menu bar must include 'File'");
    assert!(svg.contains("Edit"), "menu bar must include 'Edit'");
    assert!(svg.contains("View"), "menu bar must include 'View'");
    assert!(svg.contains("Help"), "menu bar must include 'Help'");
}

/// Two-item menu bar must have exactly one separator.
#[test]
fn salt_menu_bar_two_items_one_separator() {
    let src = "@startsalt\n{\n{* File | Help }\n}\n@endsalt\n";
    let svg = render_svg(src);

    let separator_count = svg.matches("data-salt-widget=\"menu-separator\"").count();
    assert_eq!(
        separator_count, 1,
        "two-item menu bar must have exactly 1 separator; got {separator_count}"
    );
}

/// Single-item menu bar must have no separators.
#[test]
fn salt_menu_bar_single_item_no_separator() {
    let src = "@startsalt\n{\n{* File }\n}\n@endsalt\n";
    let svg = render_svg(src);

    let separator_count = svg.matches("data-salt-widget=\"menu-separator\"").count();
    assert_eq!(
        separator_count, 0,
        "single-item menu bar must have no separators; got {separator_count}"
    );
}

// ---------------------------------------------------------------------------
// Bug 3 — Combobox dropdown arrow must point DOWN (▼)
// ---------------------------------------------------------------------------

/// A combo widget `[label v]` renders a polygon that acts as the dropdown arrow.
/// The arrow must point DOWN: two vertex y-coordinates near the top and one near
/// the bottom (in SVG coordinates where y increases downward).
///
/// The renderer emits the polygon as:
///   `<polygon points="x1,y1 x2,y2 x3,y3" .../>` where the THREE points define
///   the arrow triangle. For a down-pointing arrow, two of the y-values are
///   smaller (top of the combo box region) and one is larger (bottom).
#[test]
fn salt_combo_arrow_points_down() {
    let src = "@startsalt\n{\n| [Workspace v] |\n}\n@endsalt\n";
    let svg = render_svg(src);

    // The combo widget rect must be present.
    assert!(
        svg.contains("data-salt-widget=\"combo\""),
        "SVG must contain a combo widget rect; got: {svg}"
    );

    // Extract the first polygon's points from the SVG.
    // The polygon is emitted as `<polygon points="x1,y1 x2,y2 x3,y3" ...`.
    let poly_start = svg
        .find("<polygon points=\"")
        .expect("SVG must contain a polygon (the combo arrow)");
    let after_quote = poly_start + "<polygon points=\"".len();
    let quote_end = svg[after_quote..]
        .find('"')
        .expect("polygon points attribute must be terminated")
        + after_quote;
    let points_str = &svg[after_quote..quote_end];

    // Parse the three (x,y) pairs.
    let coords: Vec<i64> = points_str
        .split_whitespace()
        .flat_map(|pair| pair.split(','))
        .filter_map(|v| v.parse::<i64>().ok())
        .collect();
    assert_eq!(
        coords.len(),
        6,
        "combo arrow polygon must have exactly 3 points (6 coordinates); got: {points_str}"
    );

    let y1 = coords[1];
    let y2 = coords[3];
    let y3 = coords[5];

    // For a DOWN-pointing arrow: two points share a smaller y (near top) and the
    // apex has a larger y (near bottom). Equivalently, max(y) > both y values at
    // the base — but the simplest invariant is that NOT all three y-values are equal
    // AND the maximum y differs from the minimum.
    let y_min = y1.min(y2).min(y3);
    let y_max = y1.max(y2).max(y3);
    assert!(
        y_max > y_min,
        "combo arrow must be a non-degenerate triangle; got points: {points_str}"
    );

    // The apex (single outlier) must be at the BOTTOM (highest y in SVG coords).
    // Count how many points have y == y_max.
    let apex_count = [y1, y2, y3].iter().filter(|&&y| y == y_max).count();
    assert_eq!(
        apex_count, 1,
        "down-pointing arrow must have exactly one apex at the bottom (max y); got points: {points_str}"
    );

    // And the base must be at the TOP (two points with y == y_min).
    let base_count = [y1, y2, y3].iter().filter(|&&y| y == y_min).count();
    assert_eq!(
        base_count, 2,
        "down-pointing arrow must have exactly two base points at the top (min y); got points: {points_str}"
    );
}

/// Open-combo `^label^^item1^^item2^` also uses a down-pointing arrow.
#[test]
fn salt_open_combo_arrow_points_down() {
    let src = "@startsalt\n{\n^Workspace^^System default^^Light^^Dark^\n}\n@endsalt\n";
    let svg = render_svg(src);

    assert!(
        svg.contains("data-salt-widget=\"open-combo\""),
        "SVG must contain an open-combo widget; got: {svg}"
    );

    let poly_start = svg
        .find("<polygon points=\"")
        .expect("SVG must contain a polygon (the open-combo arrow)");
    let after_quote = poly_start + "<polygon points=\"".len();
    let quote_end = svg[after_quote..]
        .find('"')
        .expect("polygon points must be terminated")
        + after_quote;
    let points_str = &svg[after_quote..quote_end];

    let coords: Vec<i64> = points_str
        .split_whitespace()
        .flat_map(|pair| pair.split(','))
        .filter_map(|v| v.parse::<i64>().ok())
        .collect();
    assert_eq!(
        coords.len(),
        6,
        "open-combo arrow must have 3 points; got: {points_str}"
    );

    let y1 = coords[1];
    let y2 = coords[3];
    let y3 = coords[5];
    let y_min = y1.min(y2).min(y3);
    let y_max = y1.max(y2).max(y3);

    assert!(
        y_max > y_min,
        "open-combo arrow must be non-degenerate; got: {points_str}"
    );

    let apex_count = [y1, y2, y3].iter().filter(|&&y| y == y_max).count();
    assert_eq!(
        apex_count, 1,
        "open-combo must have one apex at bottom (max y = DOWN); got: {points_str}"
    );
}

// ---------------------------------------------------------------------------
// Bug 4 — `..` and `==` horizontal-rule separators render as lines, not text
// ---------------------------------------------------------------------------

/// A `..` line in a salt diagram must be treated as a dotted horizontal separator
/// and render as an SVG `<line>` element, NOT as visible label text `..`.
#[test]
fn salt_dotted_separator_renders_as_line_not_text() {
    let src = "@startsalt\n{\n  Header\n  ..\n  Body\n  [OK]\n}\n@endsalt\n";
    let svg = render_svg(src);

    // The two-dot sequence must NOT appear as a raw text label.
    assert!(
        !svg.contains(">..</"),
        "`..` must not be rendered as visible label text `..`; got: {svg}"
    );

    // A <line> element must be present (the separator rule).
    assert!(
        svg.contains("<line"),
        "a separator line element must be present in the SVG; got: {svg}"
    );

    // Body and button text that follow the separator must still appear.
    assert!(
        svg.contains("Body"),
        "content after `..` separator must render"
    );
    assert!(
        svg.contains("OK"),
        "button after `..` separator must render"
    );
}

/// A `==` line in a salt diagram must be treated as a thick horizontal separator
/// and render as a `<line>` element, NOT as literal `==` text.
#[test]
fn salt_thick_separator_renders_as_line_not_text() {
    let src = "@startsalt\n{\n  Header\n  ==\n  Footer\n  [OK]\n}\n@endsalt\n";
    let svg = render_svg(src);

    // The `==` sequence must NOT appear as a raw text label.
    assert!(
        !svg.contains(">==<"),
        "`==` must not be rendered as visible label text; got: {svg}"
    );

    // A <line> element must be present.
    assert!(
        svg.contains("<line"),
        "a separator line element must be present in the SVG; got: {svg}"
    );

    assert!(
        svg.contains("Footer"),
        "content after `==` separator must render"
    );
    assert!(
        svg.contains("OK"),
        "button after `==` separator must render"
    );
}

/// Both `..` and `==` separators in the same diagram must each render as a line.
/// The doc fixture `docs/examples/salt/03_separator.puml` exercises this case.
#[test]
fn salt_separator_fixture_renders_both_rules() {
    let src =
        "@startsalt\n{\n  Header\n  ..\n  Body content\n  ==\n  Footer\n  [OK]\n}\n@endsalt\n";
    let svg = render_svg(src);

    // Neither `..` nor `==` should appear as label text.
    assert!(!svg.contains(">..</"), "`..` must not be a text label");
    assert!(!svg.contains(">==<"), "`==` must not be a text label");

    // Body, footer, and button must still render.
    assert!(
        svg.contains("Body content"),
        "body text must survive separators"
    );
    assert!(
        svg.contains("Footer"),
        "footer text must survive separators"
    );
    assert!(svg.contains("OK"), "button must survive separators");
}
