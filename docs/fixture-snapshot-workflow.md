# Fixture and Snapshot Workflow

This project relies on fixture-driven integration tests and Insta snapshots.

## Where Things Live

- Input fixtures: `tests/fixtures/**`
- Snapshot assertions: `tests/snapshots/**`
- Integration test entrypoints: `tests/integration.rs`, `tests/render_e2e.rs`, `tests/coverage_contract.rs`, `tests/coverage_edges.rs`

## Adding a New Fixture-Based Test

1. Add a fixture file under the nearest matching folder in `tests/fixtures/`.
2. Add or extend a test in the relevant `tests/*.rs` file.
3. Run:

```console
cargo test
```

4. If a new snapshot is created or intentionally changed, review it in `tests/snapshots/`.

## Updating Snapshots Safely

Use this only for intentional output changes:

```console
INSTA_UPDATE=always cargo test
```

Then:
- review snapshot diffs carefully
- ensure changed output matches the intended contract
- update docs if user-visible behavior changed

## Guardrails

- Prefer small, targeted fixtures over one giant fixture.
- Keep fixture names explicit (`valid_*`, `invalid_*`, behavior-oriented names).
- Avoid accepting snapshot churn without corresponding rationale in PR notes.

## SVG Bounds Audit

A regression audit verifies global SVG geometric sanity against each render's `viewBox`.

- Script: `scripts/svg_bounds_audit.py`
- Test hook: `tests/svg_bounds_audit.rs` (runs as part of `cargo test`)
- Checks include:
  - no negative `width`/`height` on `<rect>` elements
  - tracked `<text text-anchor=\"...\">` positions inside `viewBox` with margin
  - key `<rect>` bounds inside `viewBox` with margin

Run manually:

```console
python3 scripts/svg_bounds_audit.py
```
