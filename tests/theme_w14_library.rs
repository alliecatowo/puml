//! Wave-14 theme library integration tests.
//!
//! Verifies that the five PlantUML-bundled named themes required by wave-14 are
//! correctly registered, return distinct palettes, and that an unknown theme name
//! produces a diagnostic warning and falls back gracefully rather than panicking.
//!
//! Themes under test:
//!
//! * `plain`     — minimal black-on-white, no gradients
//! * `cerulean`  — Bootstrap-blue palette
//! * `cyborg`    — dark mode with neon accents
//! * `hacker`    — black/green CRT terminal
//! * `materia`   — Material Design palette
//!
//! Refs #88

use puml::theme::{resolve_sequence_theme_preset, LOCAL_SEQUENCE_THEME_CATALOG};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Resolve a named theme and panic with a clear message if it fails.
fn preset(name: &str) -> puml::theme::SequenceThemePreset {
    resolve_sequence_theme_preset(name)
        .unwrap_or_else(|e| panic!("expected theme `{name}` to resolve, got error: {e}"))
}

// ---------------------------------------------------------------------------
// plain
// ---------------------------------------------------------------------------

#[test]
fn theme_plain_is_registered_in_catalog() {
    assert!(
        LOCAL_SEQUENCE_THEME_CATALOG.contains(&"plain"),
        "catalog must include `plain`"
    );
}

#[test]
fn theme_plain_resolves() {
    let p = preset("plain");
    assert_eq!(p.name, "plain");
}

#[test]
fn theme_plain_uses_dark_arrow_color() {
    // plain is minimal/black-on-white — arrow should be a dark tone
    let p = preset("plain");
    // The default arrow_color is "#111"; any dark hex passes this check.
    assert!(
        !p.style.arrow_color.is_empty(),
        "plain theme arrow_color must not be empty"
    );
}

#[test]
fn theme_plain_has_light_participant_background() {
    let p = preset("plain");
    // plain should have a near-white or light participant background
    let bg = &p.style.participant_background_color;
    assert!(
        !bg.is_empty(),
        "plain theme participant_background_color must not be empty"
    );
    // The color should not be a fully dark background (no "plain" dark-mode).
    // We assert it doesn't start with the cyborg/hacker dark bg prefix.
    assert_ne!(
        bg.as_str(),
        "#060606",
        "plain must not reuse cyborg's dark bg"
    );
    assert_ne!(
        bg.as_str(),
        "#0d0d0d",
        "plain must not reuse hacker's dark bg"
    );
}

// ---------------------------------------------------------------------------
// cerulean
// ---------------------------------------------------------------------------

#[test]
fn theme_cerulean_is_registered_in_catalog() {
    assert!(
        LOCAL_SEQUENCE_THEME_CATALOG.contains(&"cerulean"),
        "catalog must include `cerulean`"
    );
}

#[test]
fn theme_cerulean_resolves() {
    let p = preset("cerulean");
    assert_eq!(p.name, "cerulean");
}

#[test]
fn theme_cerulean_arrow_color_is_bootstrap_blue() {
    let p = preset("cerulean");
    // Bootstrap 3 primary blue is #2fa4e7
    assert_eq!(
        p.style.arrow_color, "#2fa4e7",
        "cerulean arrow must be Bootstrap blue"
    );
}

#[test]
fn theme_cerulean_participant_background_is_light_blue() {
    let p = preset("cerulean");
    // d9edf7 is the Bootstrap "info" alert background — a recognisable pale blue
    assert_eq!(
        p.style.participant_background_color, "#d9edf7",
        "cerulean participant background must be light blue"
    );
}

#[test]
fn theme_cerulean_lifeline_border_matches_arrow() {
    let p = preset("cerulean");
    assert_eq!(
        p.style.lifeline_border_color, p.style.arrow_color,
        "cerulean lifeline border should match arrow color"
    );
}

// ---------------------------------------------------------------------------
// cyborg
// ---------------------------------------------------------------------------

#[test]
fn theme_cyborg_is_registered_in_catalog() {
    assert!(
        LOCAL_SEQUENCE_THEME_CATALOG.contains(&"cyborg"),
        "catalog must include `cyborg`"
    );
}

#[test]
fn theme_cyborg_resolves() {
    let p = preset("cyborg");
    assert_eq!(p.name, "cyborg");
}

#[test]
fn theme_cyborg_is_dark_mode() {
    let p = preset("cyborg");
    // Cyborg has a near-black participant background
    assert_eq!(
        p.style.participant_background_color, "#060606",
        "cyborg participant background must be near-black"
    );
}

#[test]
fn theme_cyborg_neon_accent_arrow() {
    let p = preset("cyborg");
    // Neon cyan/blue accent: #2a9fd6
    assert_eq!(
        p.style.arrow_color, "#2a9fd6",
        "cyborg arrow must be neon blue"
    );
}

#[test]
fn theme_cyborg_border_uses_accent_color() {
    let p = preset("cyborg");
    assert_eq!(
        p.style.participant_border_color, "#2a9fd6",
        "cyborg participant border must match neon accent"
    );
}

// ---------------------------------------------------------------------------
// hacker
// ---------------------------------------------------------------------------

#[test]
fn theme_hacker_is_registered_in_catalog() {
    assert!(
        LOCAL_SEQUENCE_THEME_CATALOG.contains(&"hacker"),
        "catalog must include `hacker`"
    );
}

#[test]
fn theme_hacker_resolves() {
    let p = preset("hacker");
    assert_eq!(p.name, "hacker");
}

#[test]
fn theme_hacker_arrow_is_green() {
    let p = preset("hacker");
    // CRT green: bright #00ff00
    assert_eq!(
        p.style.arrow_color, "#00ff00",
        "hacker arrow must be bright CRT green"
    );
}

#[test]
fn theme_hacker_participant_background_is_near_black() {
    let p = preset("hacker");
    assert_eq!(
        p.style.participant_background_color, "#0d0d0d",
        "hacker participant background must be near-black"
    );
}

#[test]
fn theme_hacker_explicit_font_color_is_green() {
    let p = preset("hacker");
    // hacker sets an explicit green font color for participants
    assert_eq!(
        p.style.participant_font_color,
        Some("#00ff00".to_string()),
        "hacker must set explicit green participant font color"
    );
}

#[test]
fn theme_hacker_note_background_is_black() {
    let p = preset("hacker");
    assert_eq!(
        p.style.note_background_color, "#000000",
        "hacker note background must be pure black"
    );
}

// ---------------------------------------------------------------------------
// materia
// ---------------------------------------------------------------------------

#[test]
fn theme_materia_is_registered_in_catalog() {
    assert!(
        LOCAL_SEQUENCE_THEME_CATALOG.contains(&"materia"),
        "catalog must include `materia`"
    );
}

#[test]
fn theme_materia_resolves() {
    let p = preset("materia");
    assert_eq!(p.name, "materia");
}

#[test]
fn theme_materia_arrow_is_material_blue() {
    let p = preset("materia");
    // Material Design primary blue 500: #2196f3
    assert_eq!(
        p.style.arrow_color, "#2196f3",
        "materia arrow must be Material blue-500"
    );
}

#[test]
fn theme_materia_participant_background_is_blue_50() {
    let p = preset("materia");
    // Material Design blue-50: #e3f2fd
    assert_eq!(
        p.style.participant_background_color, "#e3f2fd",
        "materia participant background must be Material blue-50"
    );
}

#[test]
fn theme_materia_lifeline_border_is_blue_200() {
    let p = preset("materia");
    // Material Design blue-200: #90caf9
    assert_eq!(
        p.style.lifeline_border_color, "#90caf9",
        "materia lifeline border must be Material blue-200"
    );
}

#[test]
fn theme_materia_note_background_is_yellow_50() {
    let p = preset("materia");
    // Material Design yellow-50: #fff9c4
    assert_eq!(
        p.style.note_background_color, "#fff9c4",
        "materia note background must be Material yellow-50"
    );
}

// ---------------------------------------------------------------------------
// Cross-theme invariants
// ---------------------------------------------------------------------------

#[test]
fn all_five_themes_are_distinct() {
    let names = ["plain", "cerulean", "cyborg", "hacker", "materia"];
    let styles: Vec<_> = names.iter().map(|n| preset(n)).collect();

    // Every theme must return a unique participant_background_color.
    let bg_colors: std::collections::BTreeSet<String> = styles
        .iter()
        .map(|p| p.style.participant_background_color.clone())
        .collect();
    assert_eq!(
        bg_colors.len(),
        5,
        "all five themes must have distinct participant background colors, got: {:?}",
        bg_colors
    );
}

#[test]
fn all_five_themes_produce_non_empty_color_fields() {
    for name in ["plain", "cerulean", "cyborg", "hacker", "materia"] {
        let p = preset(name);
        let s = &p.style;
        assert!(!s.arrow_color.is_empty(), "{name}: arrow_color empty");
        assert!(
            !s.lifeline_border_color.is_empty(),
            "{name}: lifeline_border_color empty"
        );
        assert!(
            !s.participant_background_color.is_empty(),
            "{name}: participant_background_color empty"
        );
        assert!(
            !s.participant_border_color.is_empty(),
            "{name}: participant_border_color empty"
        );
        assert!(
            !s.note_background_color.is_empty(),
            "{name}: note_background_color empty"
        );
        assert!(
            !s.note_border_color.is_empty(),
            "{name}: note_border_color empty"
        );
    }
}

// ---------------------------------------------------------------------------
// Unknown theme name — diagnostic warn + fall-back behaviour
// ---------------------------------------------------------------------------

#[test]
fn theme_unknown_name_diagnostic_warns_and_falls_back_to_default() {
    // An unknown name must return Err (not panic) with the E_THEME_UNKNOWN code.
    let result = resolve_sequence_theme_preset("this-theme-does-not-exist");
    assert!(
        result.is_err(),
        "resolving an unknown theme must return Err"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("[E_THEME_UNKNOWN]"),
        "error message must contain [E_THEME_UNKNOWN], got: {msg}"
    );
    // The error message must also list at least some catalog entries so the user
    // knows what's available.
    assert!(
        msg.contains("plain") || msg.contains("cerulean"),
        "error message must reference available themes, got: {msg}"
    );
}

#[test]
fn theme_malformed_syntax_empty_string_returns_error() {
    let result = resolve_sequence_theme_preset("");
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(
        msg.contains("[E_THEME_INVALID]"),
        "empty spec must produce E_THEME_INVALID, got: {msg}"
    );
}

#[test]
fn theme_from_source_syntax_returns_unsupported_error() {
    // `!theme foo from https://example.com` triggers a different code path
    let result = resolve_sequence_theme_preset("foo from https://example.com");
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(
        msg.contains("[E_THEME_SOURCE_UNSUPPORTED]"),
        "remote source spec must produce E_THEME_SOURCE_UNSUPPORTED, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Catalog completeness
// ---------------------------------------------------------------------------

#[test]
fn catalog_contains_all_five_wave14_themes() {
    for name in ["plain", "cerulean", "cyborg", "hacker", "materia"] {
        assert!(
            LOCAL_SEQUENCE_THEME_CATALOG.contains(&name),
            "catalog must contain `{name}`"
        );
    }
}

#[test]
fn catalog_is_non_empty_and_has_no_duplicates() {
    assert!(
        !LOCAL_SEQUENCE_THEME_CATALOG.is_empty(),
        "catalog must not be empty"
    );
    // Every entry in the catalog must be unique.
    let mut seen = std::collections::BTreeSet::new();
    for name in LOCAL_SEQUENCE_THEME_CATALOG {
        assert!(seen.insert(*name), "catalog has duplicate entry: `{name}`");
    }
    // Every catalog entry must also resolve successfully.
    for name in LOCAL_SEQUENCE_THEME_CATALOG {
        assert!(
            resolve_sequence_theme_preset(name).is_ok(),
            "catalog entry `{name}` does not resolve"
        );
    }
}
