<!--
Thanks for sending a PR to PUML!

A few quick things to keep in mind:
- Small, focused PRs land faster than large ones. If this is a big change, consider splitting it.
- Visual changes need a regenerated PNG/SVG in the description so reviewers can see them at a glance.
- Tests + docs render check are required to pass on every PR. The PR gate runs them for you.
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

## Visual evidence

<!--
For renderer / layout / theme / fixture changes, help the reviewer SEE the change.

Easiest (humans dragging-and-dropping a PNG): paste an image directly into the table
below — GitHub turns it into an attached upload.

If you're an agent or working from CLI without browser access, prefer one of:
  (a) commit the regenerated docs/diagrams/<name>.png alongside the SVG and link it here
      as `![after](../docs/diagrams/<name>.png)`
  (b) list the affected fixture paths and the render command — the reviewer can re-run

Skip this section for pure-refactor / docs / CI PRs.
-->

| Before | After |
|--------|-------|
|        |       |

**Affected fixtures / diagrams:**

<!-- e.g. `docs/diagrams/architecture-overview.puml`, `docs/examples/class/12_all_relations.puml` -->

**Render command:**

```sh
./target/release/puml --format png <fixture>.puml -o /tmp/after.png
```

## Test plan

<!-- How did you verify this works? Tick whichever apply, add specifics. -->

- [ ] `cargo test --release` passes locally
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] `python3 scripts/render_check.py --fail-on-doc-drift` passes
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
