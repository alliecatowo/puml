# Issue 94 Handoff

## Summary

- Parser/AST/model baseline support is implemented for Gantt and Chronology.
- Supported Gantt subset: `@startgantt`/`@endgantt`, `Project starts`, bracketed tasks/milestones, aliases, `requires`, `starts`, `ends`, start/end constraints, milestone `happens`, and bracketed dependency arrows.
- Supported Chronology subset: `@startchronology`/`@endchronology`, metadata, and bracketed `[event] happens on <timestamp>` statements.
- Unsupported sub-syntax returns deterministic parser diagnostics; rendering returns deterministic family-specific render diagnostics.

## Validation

- `INSTA_UPDATE=always cargo test --test integration non_uml -- --nocapture`
- `cargo test --test integration non_uml -- --nocapture`
- `cargo test --test coverage_edges -- --nocapture`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

## Notes

- Rendering for both families is intentionally out of scope for this issue.
- PR #163 latest head observed locally was `7ea6566`; local validation was green while the remote GitHub Actions check was still running.
