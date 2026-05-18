<!--
Thanks for sending a PR to PUML!

A few quick things to keep in mind:
- Small, focused PRs land faster than large ones. If this is a big change, consider splitting it.
- Visual changes need a regenerated PNG/SVG in the description so reviewers can see them at a glance.
- Tests + parity harness are required to pass on every PR. The PR gate runs them for you.
-->

## What this changes

<!-- One or two sentences. What does this PR do, and why? -->

## Type

<!-- Check one or more. Delete the others. -->

- [ ] 🐛 Bug fix (non-breaking change that fixes an issue)
- [ ] ✨ Feature (non-breaking change that adds functionality)
- [ ] 💥 Breaking change (fix or feature that changes existing behavior in a non-backwards-compatible way)
- [ ] 🎨 Visual / rendering change (touches the SVG/PNG output for any diagram family)
- [ ] 📦 Refactor (no behavior change)
- [ ] 🚧 Internal infra (CI, scripts, dev workflow)
- [ ] 📝 Docs

## Linked issues

<!-- One per line. Use `Closes #N` to auto-close on merge, or `Refs #N` to link without closing. -->

Closes #
Refs #

## Visual evidence (required for rendering changes)

<!--
If this PR touches anything under src/render/, src/normalize/, src/parser/, src/layout/, src/theme/,
or any *.puml fixture, embed a before/after PNG below. The reviewer needs to SEE the difference.

Render with:
  ./target/release/puml --format png path/to/fixture.puml -o /tmp/after.png

For multi-fixture changes, attach a small gallery.
-->

| Before | After |
|--------|-------|
| (n/a)  | (n/a) |

## Test plan

<!-- How did you verify this works? Tick whichever apply, add specifics. -->

- [ ] `cargo test --release` passes locally
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `python3 scripts/parity_harness.py --fail-on-doc-drift` passes
- [ ] Visual baselines re-blessed where intentional change: `cargo test --release --test visual_regression bless_baselines -- --ignored`
- [ ] Regenerated affected `docs/examples/*.svg` and `docs/diagrams/*.svg`
- [ ] Multimodal verification: rendered the affected fixture to PNG and looked at it myself

## Self-review checklist

- [ ] One logical change per commit
- [ ] No `unwrap()` / `panic!()` in code paths that handle user input
- [ ] Determinism preserved (no `HashMap` iteration without explicit sort)
- [ ] No new compiler warnings (`cargo build --release` clean)
- [ ] No new clippy lints
- [ ] User-facing change has a CHANGELOG-worthy commit message
- [ ] If schema/contract changed, related docs + tests updated in the same PR

## Notes for the reviewer

<!--
Anything the reviewer should know? Tradeoffs you considered? Pieces you want eyes on?
"This change is intentionally minimal because…" or "I'd appreciate a sanity check on…" go here.
-->

---

<sub>By submitting this PR you confirm that your contribution is licensed under the project's [MIT license](../LICENSE).</sub>
