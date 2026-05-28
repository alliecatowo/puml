//! Wave-11 batch G — Salt UI mockup widget parity tests.
//!
//! Each test verifies that the parser + renderer pipeline correctly handles one
//! of the new widget families added in this batch: menu bar, tab strip,
//! scrollable container, tree outline (with `**`/`***` syntax), combo box with
//! chevron, and radio button groups.
//!
//! These tests ONLY assert on SVG data attributes and structural markers so that
//! layout pixel values need not be hard-coded.

/// Helper: render a salt source string and return the SVG output.
fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("salt source should render without error")
}

// ---------------------------------------------------------------------------
// 1. Menu bar
// ---------------------------------------------------------------------------

/// `{* File | Edit | View | Help }` renders as a horizontal menu bar row with
/// each entry rendered as a text label inside the menu rect, separated by space.
#[test]
fn salt_menu_bar_renders_horizontal_entries_with_separators() {
    let src = "@startsalt\n{\n{* File | Edit | View | Help }\n}\n@endsalt\n";
    let svg = render_svg(src);

    // The menu widget rect must be present.
    assert!(
        svg.contains("data-salt-widget=\"menu\""),
        "SVG must contain a salt menu widget rect; got: {svg}"
    );
    // All four entries must appear as text in the SVG.
    assert!(svg.contains("File"), "menu bar must include 'File' entry");
    assert!(svg.contains("Edit"), "menu bar must include 'Edit' entry");
    assert!(svg.contains("View"), "menu bar must include 'View' entry");
    assert!(svg.contains("Help"), "menu bar must include 'Help' entry");
}

// ---------------------------------------------------------------------------
// 2. Tab strip
// ---------------------------------------------------------------------------

/// `{/ Tab1 | Tab2 | Tab3 }` renders a tab strip where the first tab (index 0)
/// is the active (selected/raised) tab unless marked with `**...**`.
#[test]
fn salt_tab_strip_renders_selected_tab_raised() {
    let src = "@startsalt\n{\n{/ Tab1 | Tab2 | Tab3 }\n}\n@endsalt\n";
    let svg = render_svg(src);

    // At least one tab rect must be present.
    assert!(
        svg.contains("data-salt-widget=\"tab\""),
        "SVG must contain a salt tab widget; got: {svg}"
    );
    // The active tab must be flagged.
    assert!(
        svg.contains("data-salt-tab-active=\"true\""),
        "SVG must mark the active tab; got: {svg}"
    );
    // The inactive tabs must be flagged too.
    assert!(
        svg.contains("data-salt-tab-active=\"false\""),
        "SVG must mark inactive tabs; got: {svg}"
    );
    // Tab labels must appear.
    assert!(svg.contains("Tab1"), "tab strip must include 'Tab1'");
    assert!(svg.contains("Tab2"), "tab strip must include 'Tab2'");
    assert!(svg.contains("Tab3"), "tab strip must include 'Tab3'");
}

/// `{/ Tab1 | **Tab2** | Tab3 }` marks Tab2 as the active tab via `**...**`.
#[test]
fn salt_tab_strip_active_tab_selected_via_bold_markup() {
    let src = "@startsalt\n{\n{/ Tab1 | **Tab2** | Tab3 }\n}\n@endsalt\n";
    let svg = render_svg(src);

    assert!(
        svg.contains("data-salt-widget=\"tab\""),
        "SVG must contain a salt tab widget; got: {svg}"
    );
    // The strip underline must be present.
    assert!(
        svg.contains("data-salt-widget=\"tab-strip\""),
        "SVG must contain the tab-strip underline; got: {svg}"
    );
    // Tab labels appear without the markup.
    assert!(svg.contains("Tab1"), "tab strip must include 'Tab1'");
    assert!(svg.contains("Tab2"), "tab strip must include 'Tab2'");
    assert!(svg.contains("Tab3"), "tab strip must include 'Tab3'");
}

// ---------------------------------------------------------------------------
// 3. Scrollable container
// ---------------------------------------------------------------------------

/// `{S ... }` renders a scrollable container with a vertical scrollbar rect.
#[test]
fn salt_scrollable_container_renders_scrollbar() {
    let src = "@startsalt\n{\n{S\nScrollableArea\nItem 1\nItem 2\nItem 3\n}\n}\n@endsalt\n";
    let svg = render_svg(src);

    // The textarea (container) rect must be present.
    assert!(
        svg.contains("data-salt-widget=\"textarea\""),
        "SVG must contain a textarea container for the scrollable area; got: {svg}"
    );
    // The vertical scrollbar must be present.
    assert!(
        svg.contains("data-salt-scroll-vertical=\"true\""),
        "SVG must mark scroll-vertical on the container; got: {svg}"
    );
    // A scrollbar thumb/track rect must be present.
    assert!(
        svg.contains("data-salt-widget=\"scrollbar\""),
        "SVG must contain a scrollbar rect; got: {svg}"
    );
}

// ---------------------------------------------------------------------------
// 4. Tree outline (with `**`/`***` syntax via `{.` container)
// ---------------------------------------------------------------------------

/// `{. ** Folder A \n *** File 1 }` renders tree items indented by depth.
/// The `{.` container activates `**`/`***` tree parsing.
#[test]
fn salt_tree_outline_renders_with_indent_per_depth() {
    let src = concat!(
        "@startsalt\n",
        "{\n",
        "{.\n",
        "** Folder A\n",
        "*** File 1\n",
        "*** File 2\n",
        "** Folder B\n",
        "}\n",
        "}\n",
        "@endsalt\n"
    );
    let svg = render_svg(src);

    // Tree item widget markers must be present.
    assert!(
        svg.contains("data-salt-widget=\"tree\""),
        "SVG must contain tree widget markers; got: {svg}"
    );
    // At least one item must have depth 0 (Folder A, Folder B).
    assert!(
        svg.contains("data-salt-tree-depth=\"0\""),
        "SVG must contain a depth-0 tree item; got: {svg}"
    );
    // At least one item must have depth 1 (File 1, File 2).
    assert!(
        svg.contains("data-salt-tree-depth=\"1\""),
        "SVG must contain a depth-1 tree item; got: {svg}"
    );
    // Labels must appear.
    assert!(svg.contains("Folder A"), "tree must include 'Folder A'");
    assert!(svg.contains("File 1"), "tree must include 'File 1'");
    assert!(svg.contains("Folder B"), "tree must include 'Folder B'");
}

/// The classic `{T + ... ++ ... }` tree syntax must still work (regression guard).
#[test]
fn salt_tree_plus_syntax_still_renders_after_star_syntax_added() {
    let src = concat!(
        "@startsalt\n",
        "{\n",
        "{T\n",
        "+ Workspace\n",
        "++ Profile\n",
        "++ Security\n",
        "}\n",
        "}\n",
        "@endsalt\n"
    );
    let svg = render_svg(src);

    assert!(
        svg.contains("data-salt-widget=\"tree\""),
        "SVG must contain tree widget markers for plus-syntax; got: {svg}"
    );
    assert!(svg.contains("Workspace"), "tree must include 'Workspace'");
    assert!(svg.contains("Profile"), "tree must include 'Profile'");
}

// ---------------------------------------------------------------------------
// 5. Combo box with bracket syntax `[ label v ]`
// ---------------------------------------------------------------------------

/// `[ Combo Box v ]` renders as a combo box widget (with chevron glyph),
/// not a plain button.
#[test]
fn salt_combo_box_renders_chevron_glyph() {
    let src = "@startsalt\n{\n[ Combo Box v ]\n}\n@endsalt\n";
    let svg = render_svg(src);

    // The combo rect must be present (not a button).
    assert!(
        svg.contains("data-salt-widget=\"combo\""),
        "SVG must render [ label v ] as a combo box, not a button; got: {svg}"
    );
    // The chevron polygon must be present.
    assert!(
        svg.contains("<polygon"),
        "combo box must include a chevron polygon glyph; got: {svg}"
    );
    // The label text must appear.
    assert!(
        svg.contains("Combo Box"),
        "combo box must display the label text; got: {svg}"
    );
}

/// A plain `^label^` caret-combo also renders with the chevron glyph.
#[test]
fn salt_caret_combo_renders_chevron_glyph() {
    let src = "@startsalt\n{\n^Role^\n}\n@endsalt\n";
    let svg = render_svg(src);

    assert!(
        svg.contains("data-salt-widget=\"combo\""),
        "SVG must render ^label^ as a combo box; got: {svg}"
    );
    assert!(
        svg.contains("<polygon"),
        "caret combo box must include a chevron polygon glyph; got: {svg}"
    );
}

// ---------------------------------------------------------------------------
// 6. Radio button group
// ---------------------------------------------------------------------------

/// `( ) Radio A | (X) Radio B` renders radio circles: the unchecked one is
/// hollow, the checked one has a filled inner circle.
#[test]
fn salt_radio_button_group_renders_checked_filled() {
    let src = "@startsalt\n{\n| ( ) Radio A | (X) Radio B |\n}\n@endsalt\n";
    let svg = render_svg(src);

    // Both outer radio circles must be present.
    assert!(
        svg.contains("data-salt-widget=\"radio\""),
        "SVG must contain radio circle widgets; got: {svg}"
    );
    // The checked radio must have an inner filled circle (r="2").
    assert!(
        svg.contains("r=\"2\""),
        "checked radio must have a filled inner circle (r=2); got: {svg}"
    );
    // Labels must appear.
    assert!(
        svg.contains("Radio A"),
        "radio group must include 'Radio A'"
    );
    assert!(
        svg.contains("Radio B"),
        "radio group must include 'Radio B'"
    );
}

/// `(*)` is a PlantUML synonym for a checked radio button and must render
/// identically to `(X)` — with a filled inner circle.
#[test]
fn salt_radio_star_synonym_renders_as_checked() {
    let src = "@startsalt\n{\n| (*) Preferred | ( ) Other |\n}\n@endsalt\n";
    let svg = render_svg(src);

    assert!(
        svg.contains("data-salt-widget=\"radio\""),
        "SVG must contain radio circle widgets; got: {svg}"
    );
    // `(*)` must produce a checked radio with an inner filled dot.
    assert!(
        svg.contains("r=\"2\""),
        "(*) radio synonym must render as checked (filled inner circle); got: {svg}"
    );
    assert!(svg.contains("Preferred"), "radio must include 'Preferred'");
}
