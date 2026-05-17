# Visual regression framework

This directory holds the fixture manifest and committed PNG baselines for the
`tests/visual_regression.rs` test suite.

## What it checks

### Text-content sweep (`visual_regression_all_fixtures`)

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

### PNG baseline-diff sweep (`png_regression_all_fixtures`)

For every fixture in `manifest.json`:

1. Render SVG via `puml`, then rasterise to PNG at 96 DPI (scaled to ≤640 px
   wide) using the same `resvg` + `tiny-skia` chain as the CLI.
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
# Run only the fast unit tests (text extractor, pixel-diff helpers).
cargo test --test visual_regression

# Run the full text-content sweep (currently #[ignore] until #238 is fixed).
cargo test --test visual_regression -- --ignored visual_regression_all_fixtures

# Run the full PNG baseline sweep (currently #[ignore] until #238 is fixed
# and baselines are generated).
cargo test --test visual_regression -- --ignored png_regression_all_fixtures

# Run ALL ignored tests (text sweep + PNG sweep + bless).
cargo test --test visual_regression -- --ignored
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

2. **Generate the baseline PNG** (once `#238` is fixed and the renderer
   produces correct output):

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
| Baselines do not exist yet | Run bless after `#238` is fixed; do not bless broken renders |
| Dimensions changed unexpectedly | Check if viewport/canvas size changed in `src/render.rs`; could be a real regression |

## Baseline storage

Baseline PNGs live under `tests/visual_baselines/<family>/<fixture>.png` and
are committed to git. They are kept small (≤640 px wide, 96 DPI) to avoid
bloating git history. PNG compression is lossless, so repeated bless runs on
the same render produce identical files.

## CI integration

> **TODO(post-#238):** add `visual_regression_all_fixtures` and
> `png_regression_all_fixtures` to the PR Gate workflow once the
> missing-label renderer bug is fixed and real baselines exist.

Both sweeps are currently `#[ignore]`'d. The PR Gate should run:

```sh
cargo test --test visual_regression
# (unit tests only, no --ignored)
```

After `#238` lands, add a second step:

```sh
cargo test --test visual_regression -- --ignored visual_regression_all_fixtures
cargo test --test visual_regression -- --ignored png_regression_all_fixtures
```
