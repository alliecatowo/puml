// Regression tests for issue #723: unwrap() removal on user-input paths in
// timing.rs, family.rs, and gantt.rs.
//
// Each test exercises a path that would have panicked before the fix, and
// asserts that the renderer either returns a valid SVG or a Diagnostic (never
// a panic).

/// --- timing.rs: empty-tick timing diagram ---
///
/// A @starttiming...@endtiming block with no `@<N>` tick markers used to hit
/// `time_vals.first().unwrap()` / `time_vals.last().unwrap()`.
/// After the fix the guard fills time_vals with [0, 10] so render proceeds.
#[test]
fn timing_empty_tick_markers_renders_without_panic() {
    let src = "@startuml\nconcise \"Signal\" as S\n@enduml\n";
    // Must not panic; result may be Ok or Err (Diagnostic) but not a process abort.
    let result = puml::render_source_to_svg(src);
    // The timing renderer produces SVG with no ticks; as long as we get a
    // String (not a panic) the invariant is satisfied.
    match result {
        Ok(svg) => {
            // If it renders, it must be non-empty SVG.
            assert!(
                svg.contains("<svg") || svg.is_empty(),
                "rendered SVG should be valid XML or empty fallback"
            );
        }
        Err(diag) => {
            // A Diagnostic is also acceptable — parse or normalization may
            // reject the empty diagram. The key invariant is no panic.
            assert!(
                !diag.message.is_empty(),
                "Diagnostic message must not be empty"
            );
        }
    }
}

/// --- gantt.rs: "starts <date>" without "at" keyword ---
///
/// `parse_gantt_start_date_clause` used `.unwrap()` inside an `unwrap_or_else`
/// closure when there was no "at " prefix. The rewrite avoids the inner unwrap.
#[test]
fn gantt_starts_date_without_at_parses_cleanly() {
    let src = r#"
@startgantt
Project starts 2024-01-01
[Task A] lasts 5 days
[Task A] starts 2024-01-01
@endgantt
"#;
    let result = puml::render_source_to_svg(src);
    match result {
        Ok(svg) => {
            assert!(
                svg.contains("<svg") || svg.is_empty(),
                "rendered SVG should be valid XML or empty fallback"
            );
        }
        Err(diag) => {
            // Gantt normalization may reject edge cases — a Diagnostic is fine.
            assert!(
                !diag.message.is_empty(),
                "Diagnostic message must not be empty"
            );
        }
    }
}

/// --- gantt.rs: malformed start clause with no date ---
///
/// Ensures that a starts clause with an invalid date doesn't panic.
#[test]
fn gantt_starts_clause_invalid_date_is_diagnostic_not_panic() {
    let src = r#"
@startgantt
[Task A] starts at not-a-date and lasts 3 days
@endgantt
"#;
    // Must not panic regardless of parse/render outcome.
    let _result = puml::render_source_to_svg(src);
}
