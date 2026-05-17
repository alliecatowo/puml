# Visual regression framework

This directory holds the manifest and (eventually) PNG baselines for the
`tests/visual_regression.rs` test.

## What it checks

For every fixture in `manifest.json`, the test renders the diagram to SVG via
the `puml` binary and asserts:

1. **No empty `<text>` elements.** Catches the family of bugs where the
   renderer emits `<text></text>` (i.e. labels are missing). This was the
   single most pervasive defect found in the 2026-05-17 visual audit
   (see issue #238).
2. **All `expected_text` substrings appear somewhere in the SVG.** Catches
   regressions where specific labels (participant names, class names,
   message bodies, etc.) silently stop being emitted.
3. **At least `min_text_elements` non-empty `<text>` elements exist.**
   Catches regressions that reduce overall text density (e.g. a refactor
   that suppresses labels in one family).

## Adding a fixture

Append to `manifest.json`:

```json
{
  "path": "docs/examples/<family>/<file>.puml",
  "family": "<family>",
  "expected_text": ["String that should appear", "Another label"],
  "min_text_elements": 2
}
```

`expected_text` can be empty (`[]`) if the only goal is the
non-empty-`<text>` check.

## What's NOT here (yet)

- **PNG baseline diffs.** Planned for a follow-up wave. The intent is to
  rasterize each fixture via `resvg` and compare against a stored
  baseline PNG with a small perceptual tolerance. Until then, this test
  catches the structural text-emission regressions, which is where the
  worst bugs have been.
- **Layout/geometry assertions.** A future test could parse the SVG to
  assert that, e.g., mindmap has its root centered or WBS has no edge
  crossings.

## Running locally

```
cargo test --test visual_regression
```

Tests are skipped automatically if the `puml` binary cannot be built. To
debug a failing fixture, look at the captured SVG in `target/visual-diff/`.
