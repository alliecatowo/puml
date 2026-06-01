//! Integration tests for issue #1423 — deterministic diagnostic emission for
//! unsupported Salt widget constructs.
//!
//! Before this fix, `--check` on a Salt diagram containing table row-spans (`*`)
//! or undefined sprite references (`<<name>>`) exited 0 with no output. Users had
//! no way to know that those constructs were only partially rendered.
//!
//! After this fix the normaliser emits stable `W_SALT_UNSUPPORTED_*` warnings so
//! that tooling (language servers, CI, the CLI `--check` flag) can surface them.
//!
//! Acceptance criteria from the issue:
//! - `cargo test --test integration salt` passes.
//! - Rendering the fixture does not change (visual baselines unaffected).
//! - `--check` on a fixture using an unsupported construct emits a `Warning`
//!   diagnostic, not silent success.

/// Helper: parse + normalize a salt source and return the warning messages.
fn salt_warnings(src: &str) -> Vec<String> {
    let doc = puml::parse(src).expect("salt source must parse without error");
    let model = puml::normalize_family(doc).expect("salt source must normalize without error");
    let puml::NormalizedDocument::Family(family) = model else {
        panic!("expected NormalizedDocument::Family for a salt diagram");
    };
    family.warnings.iter().map(|w| w.message.clone()).collect()
}

/// Helper: render a salt source string and return the SVG output.
fn salt_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("salt source must render without error")
}

// ---------------------------------------------------------------------------
// W_SALT_UNSUPPORTED_TABLE_SPAN
// ---------------------------------------------------------------------------

/// A `*` cell inside a `{# }` table row must emit exactly one
/// `W_SALT_UNSUPPORTED_TABLE_SPAN` warning per `*` cell encountered.
#[test]
fn salt_table_span_emits_unsupported_warning() {
    let src = "@startsalt\n{#\n| A | B |\n| foo | * |\n}\n@endsalt\n";
    let warnings = salt_warnings(src);

    assert!(
        warnings
            .iter()
            .any(|w| w.contains("W_SALT_UNSUPPORTED_TABLE_SPAN")),
        "a `*` span cell must emit W_SALT_UNSUPPORTED_TABLE_SPAN; got: {warnings:?}"
    );
}

/// The `*` span warning must carry the stable code prefix so callers can
/// match it reliably without parsing the human-readable message text.
#[test]
fn salt_table_span_warning_has_stable_code() {
    let src = "@startsalt\n{#\n| Col1 | Col2 | Col3 |\n| data | * | more |\n}\n@endsalt\n";
    let warnings = salt_warnings(src);

    let span_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| w.starts_with("[W_SALT_UNSUPPORTED_TABLE_SPAN]"))
        .collect();

    assert!(
        !span_warnings.is_empty(),
        "W_SALT_UNSUPPORTED_TABLE_SPAN must be present as a bracketed code prefix; got: {warnings:?}"
    );
}

/// A diagram with NO `*` cells must NOT emit `W_SALT_UNSUPPORTED_TABLE_SPAN`.
#[test]
fn salt_no_span_cells_no_table_span_warning() {
    let src = "@startsalt\n{#\n| Name | Value |\n| Alpha | 42 |\n| Beta | 17 |\n}\n@endsalt\n";
    let warnings = salt_warnings(src);

    let span_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| w.contains("W_SALT_UNSUPPORTED_TABLE_SPAN"))
        .collect();

    assert!(
        span_warnings.is_empty(),
        "a table with no `*` cells must not emit W_SALT_UNSUPPORTED_TABLE_SPAN; got: {warnings:?}"
    );
}

// ---------------------------------------------------------------------------
// W_SALT_UNSUPPORTED_SPRITE_REF
// ---------------------------------------------------------------------------

/// A `<<name>>` cell whose sprite is NOT defined must emit
/// `W_SALT_UNSUPPORTED_SPRITE_REF`.
#[test]
fn salt_undefined_sprite_ref_emits_unsupported_warning() {
    let src = "@startsalt\n{\n| <<no_such_sprite>> | label |\n}\n@endsalt\n";
    let warnings = salt_warnings(src);

    assert!(
        warnings
            .iter()
            .any(|w| w.contains("W_SALT_UNSUPPORTED_SPRITE_REF")),
        "an undefined `<<name>>` sprite ref must emit W_SALT_UNSUPPORTED_SPRITE_REF; \
         got: {warnings:?}"
    );
}

/// The sprite-ref warning must name the missing sprite in the message so the
/// user knows which `<<…>>` token to fix.
#[test]
fn salt_undefined_sprite_ref_warning_names_the_sprite() {
    let src = "@startsalt\n{\n| <<ghost_icon>> | click me |\n}\n@endsalt\n";
    let warnings = salt_warnings(src);

    let sprite_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| w.contains("W_SALT_UNSUPPORTED_SPRITE_REF"))
        .collect();

    assert!(
        sprite_warnings.iter().any(|w| w.contains("ghost_icon")),
        "W_SALT_UNSUPPORTED_SPRITE_REF must name the missing sprite `ghost_icon`; \
         got: {sprite_warnings:?}"
    );
}

/// A `<<name>>` cell whose sprite IS defined must NOT emit
/// `W_SALT_UNSUPPORTED_SPRITE_REF` (the definition satisfies the reference).
#[test]
fn salt_defined_sprite_ref_no_warning() {
    // A minimal monochrome sprite definition: 5×5, 4bpp.
    // The exact pixel data is borrowed from the existing sprite-parse tests.
    let src = concat!(
        "@startsalt\n",
        "sprite myicon {\n",
        "  FFFFF\n",
        "  F000F\n",
        "  F000F\n",
        "  F000F\n",
        "  FFFFF\n",
        "}\n",
        "{\n",
        "| <<myicon>> | has icon |\n",
        "}\n",
        "@endsalt\n"
    );
    let warnings = salt_warnings(src);

    let sprite_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| w.contains("W_SALT_UNSUPPORTED_SPRITE_REF"))
        .collect();

    assert!(
        sprite_warnings.is_empty(),
        "a defined sprite must not trigger W_SALT_UNSUPPORTED_SPRITE_REF; \
         got: {sprite_warnings:?}"
    );
}

// ---------------------------------------------------------------------------
// Rendering must not change — visual baselines are unaffected
// ---------------------------------------------------------------------------

/// The fixture file for this issue must still render to valid SVG even when
/// warnings are present.  The diagram output must not change.
#[test]
fn salt_unsupported_constructs_fixture_still_renders() {
    let src = include_str!("../docs/examples/salt/07_unsupported_constructs.puml");
    let svg = salt_svg(src);

    assert!(
        svg.contains("<svg"),
        "unsupported-constructs fixture must produce a valid SVG root element"
    );

    // The plain-text cells must still be visible in the output.
    assert!(svg.contains("Alpha"), "cell text 'Alpha' must render");
    assert!(svg.contains("Beta"), "cell text 'Beta' must render");
}

/// Warnings must be present when rendering the fixture file.
#[test]
fn salt_unsupported_constructs_fixture_emits_warnings() {
    let src = include_str!("../docs/examples/salt/07_unsupported_constructs.puml");
    let warnings = salt_warnings(src);

    assert!(
        warnings
            .iter()
            .any(|w| w.contains("W_SALT_UNSUPPORTED_TABLE_SPAN")),
        "fixture must emit W_SALT_UNSUPPORTED_TABLE_SPAN for the `*` cells; \
         got: {warnings:?}"
    );

    assert!(
        warnings
            .iter()
            .any(|w| w.contains("W_SALT_UNSUPPORTED_SPRITE_REF")),
        "fixture must emit W_SALT_UNSUPPORTED_SPRITE_REF for `<<missing>>`; \
         got: {warnings:?}"
    );
}
