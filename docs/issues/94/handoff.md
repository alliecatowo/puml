# Issue 94 Handoff

## Summary

- Parser/AST/model baseline support is implemented for Gantt and Choronology.
- Supported Gantt subset: `@startgantt`/`@endgantt`, `Project starts`, bracketed tasks/milestones, aliases, `requires`, `starts`, `ends`, start/end constraints, milestone `happens`, and bracketed dependency arrows.
- Supported Chronology subset: `@startchronology`/`@endchronology`, metadata, and bracketed `[event] happens on <timestamp>` statements.
- Unsupported sub-syntax returns deterministic parse diagnostics; rendering returns deterministic family-specific render diagnostics.

## Validation

- `INSTA_UPDATE=always cargo test --test integration non_uml -- --nocapture`

## Notes

- Rendering for both families is intentionally out of scope for this issue.
