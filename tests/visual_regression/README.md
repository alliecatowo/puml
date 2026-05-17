# Visual regression framework

This directory holds the fixture manifest and committed PNG baselines for the
`tests/visual_regression.rs` test suite.

## What it checks

### Text-content sweeps

The full PR-gate sweep (`visual_regression_all_fixtures`) renders every manifest
fixture to SVG via the `puml` binary using stdin/stdout, so tracked
`docs/examples/*.svg` artifacts are not rewritten, and asserts:

1. **No empty `<text>` elements.** Catches the family of bugs where the
   renderer emits `<text></text>` (i.e. labels are missing). This was the
   single most pervasive defect found in the 2026-05-17 visual audit.
2. **All `expected_text` substrings appear somewhere in the SVG.** Catches
   regressions where specific labels (participant names, class names,
   message bodies, etc.) silently stop being emitted.
3. **At least `min_text_elements` non-empty `<text>` elements exist.**
   Catches regressions that reduce overall text density (e.g. a refactor
   that suppresses labels in one family).

### PNG baseline-diff sweeps

The default `png_regression_committed_baselines` test compares only reviewed
PNG baselines that are already committed under `tests/visual_baselines/`.
This lets baseline coverage grow fixture by fixture while keeping PR-gate cost
small.

The ignored `png_regression_all_fixtures` sweep covers every fixture in
`manifest.json` and is the eventual full baseline gate.

For every fixture in `manifest.json`:

1. Render SVG via `puml`, then rasterise to PNG at 96 DPI (scaled to ≤640 px
   wide) using the same `resvg` + `tiny-skia` chain as the CLI, with system
   fonts loaded and `Liberation Mono` selected for monospace text.
2. Load the stored baseline PNG from
   `tests/visual_baselines/<family>/<fixture>.png`.
3. Run a per-pixel RGBA diff with a threshold of 3 per-channel delta
   (to tolerate sub-pixel anti-aliasing differences while catching real layout
   regressions).
4. On mismatch, write artefacts to `target/visual-diff/`:
   - `<fixture>.png.new` — the current render.
   - `<fixture>.diff.png` — diff overlay (changed pixels in red,
     unchanged pixels dimmed).

Catches regressions that text-content checks miss: shapes moved, arrows
broken, swimlanes collapsed, etc.

## Running locally

```sh
# Run the default visual tests, including the full text sweep and committed baselines.
cargo test --test visual_regression

# Run only the full text-content sweep used by PR Gate.
cargo test --test visual_regression visual_regression_all_fixtures

# Run the committed-baselines PNG sweep used by the default test suite.
cargo test --test visual_regression png_regression_committed_baselines

# Run the full PNG baseline sweep (currently #[ignore] until every manifest
# fixture has a reviewed baseline).
cargo test --test visual_regression -- --ignored png_regression_all_fixtures

# Generate or refresh baselines after reviewing current output.
cargo test --test visual_regression bless_baselines -- --ignored
```

Debug artefacts (SVG and PNG renders) are written to `target/visual-diff/`
and are `.gitignore`'d — they are for local inspection only.

## Adding a fixture with a baseline

1. **Add the fixture to `manifest.json`:**

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

2. **Generate the baseline PNG** once the renderer output has been reviewed:

   ```sh
   cargo test --test visual_regression bless_baselines -- --ignored
   ```

   This writes
   `tests/visual_baselines/<family>/<fixture>.png` for every fixture
   that does not yet have a baseline.

3. **Commit the baseline:**

   ```sh
   git add tests/visual_baselines/
   git commit -m "test: add PNG baseline for <fixture>"
   ```

## Bless workflow — intentional render changes

When a developer intentionally changes renderer output (skinparam tweak,
layout fix, new feature that shifts elements), the PNG regression sweep will
fail. The workflow to promote the new output to a baseline is:

### 1. Verify the change is intentional

Inspect the diff artefacts in `target/visual-diff/`:

```sh
# After the test fails, look at the diff image:
open target/visual-diff/<family>/<fixture>.diff.png
# Red pixels = changed. Check <fixture>.png.new for the new render.
open target/visual-diff/<family>/<fixture>.png.new
# Compare to the stored baseline:
open tests/visual_baselines/<family>/<fixture>.png
```

### 2. Run the bless command

```sh
cargo test --test visual_regression bless_baselines -- --ignored
```

This re-renders every fixture and overwrites
`tests/visual_baselines/<family>/<fixture>.png` with the current output.
The test always prints a report of what was created/updated.

### 3. Commit the new baselines

```sh
git add tests/visual_baselines/
git commit -m "test: bless PNG baselines after <describe the render change>"
```

The committed PNGs appear in the PR diff so reviewers can inspect the
visual change directly in the GitHub UI.

## Regenerate vs. investigate

| Situation | Action |
|---|---|
| You changed the renderer on purpose and the test flags it | Inspect diff, run bless, commit baselines |
| CI fails on a change you did NOT make | **Investigate** — do NOT bless. Find the regression in the source |
| Baselines do not exist yet | Run bless after reviewing current output; do not bless broken renders |
| Dimensions changed unexpectedly | Check if viewport/canvas size changed in `src/render.rs`; could be a real regression |

## Baseline storage

Baseline PNGs live under `tests/visual_baselines/<family>/<fixture>.png` and
are committed to git. They are kept small (≤640 px wide, 96 DPI) to avoid
bloating git history. PNG compression is lossless, so repeated bless runs on
the same render and font stack produce identical files.

## CI integration

The PR Gate runs the default Rust test suite and a named full text-content
sweep:

```sh
cargo test
cargo test --test visual_regression visual_regression_all_fixtures
```

The default test suite also enforces every committed reviewed PNG baseline via
`png_regression_committed_baselines`. Once all manifest fixtures have real PNG
baselines committed, add the full PNG sweep to the PR Gate as:

```sh
cargo test --test visual_regression -- --ignored png_regression_all_fixtures
```
