# CI Overhaul Plan — Sub-4-Minute PR Gate

**Goal:** PR gate wall time ≤ 4 minutes for a typical Rust change.
**Baseline:** ~8-9 minutes (measured from PR gate run history, May 2026).

---

## Root causes diagnosed (May 2026)

1. **Compile-twice pattern** — the `quality` job ran `cargo llvm-cov` (instrumented
   compile + test run) then immediately ran `cargo build --release` (second full
   compile). Both are serialized inside `check-all.sh --skip-bench`. Total: ~4-5 min
   of sequential compile time in one job.

2. **No test parallelism** — all tests ran in a single nextest invocation on one
   runner. The test suite takes ~90-120s wall time; sharding across two runners cuts
   this roughly in half.

3. **LTO in CI binary builds** — `[profile.release]` has `lto = "thin"` and
   `codegen-units = 1`, which adds ~60-90s to every binary build (linker LTO pass).
   CI doesn't need peak-performance binaries.

4. **No sccache** — every cold-cache compile re-compiled every crate from source.
   Swatinem/rust-cache stores incremental build artifacts, but sccache adds a second
   layer that caches compiled object files across different jobs sharing the same
   code.

5. **Crates.io git index** — the legacy git protocol clones ~1.4 GB on cold runs.
   The sparse protocol fetches only the index entries you need.

6. **Separate fmt and clippy jobs** (pre-overhaul) — two runner checkouts + two
   cache restores for work that can share one compile artifact set.

---

## Changes shipped (Wave 8 CI teardown, 2026-05-28)

### 1. `[profile.release-ci]` in Cargo.toml

```toml
[profile.release-ci]
inherits = "release"
opt-level = 1
lto = false
codegen-units = 16
```

Used by: `artifact_regen`, `binary_size`, `pages` build, and the `regen-artifacts.sh`
fallback build path.

**Estimated savings:** 60-90s per binary build job (no thin-LTO link phase, parallel
codegen). The binary is ~10-20% larger than a full release binary but correct and
deterministic.

### 2. `CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse` in all workflow envs

Switches crate index fetching from git-clone to sparse HTTP. Requires Rust ≥ 1.68
(stabilised). Our `rust-version = "1.88"`.

**Estimated savings:** 30-60s on cold-cache runs.

### 3. Merged `fmt` + `clippy` into single `lint` job

One runner checkout + cache-restore shared by both tools.

**Estimated savings:** ~45s (one fewer runner spin-up + checkout).

### 4. Test partitioning — `test-shard-1` + `test-shard-2`

```yaml
cargo nextest run --profile ci --partition count:1/2
cargo nextest run --profile ci --partition count:2/2
```

Both shards compile from the same `shared-key: pr-gate-ubuntu-stable` cache so the
second shard gets a warm cache. Test *run* time is halved; compile time is shared
via Swatinem + sccache.

**Estimated savings:** 60-90s wall time (tests run in parallel with coverage and
binary_size).

### 5. Coverage decoupled into its own parallel job

`coverage` runs `cargo llvm-cov --profile ci` in parallel with `binary_size` and
the two test shards. Previously these were serialized inside `check-all.sh`.

Skipped for dependabot/renovate PRs — they don't change logic.

**Estimated savings:** 90-120s (parallel instead of serialized with binary build).

### 6. `binary_size` job uses `--profile release-ci`

No LTO, parallel codegen. The size limit is bumped from 16 MB to 20 MB to account
for the lack of LTO dead-code elimination. The delta direction is still correct: if
a PR adds code, the number goes up under both profiles.

**Estimated savings:** 60-90s vs `cargo build --release` with `lto="thin"`.

### 7. sccache via `mozilla-actions/sccache-action@v0.0.6`

Added to: `lint`, `test-shard-1`, `test-shard-2`, `coverage`, `binary_size`, `wasm`,
`docs_examples_drift`, `artifact_regen`, `pages`, `main-gate`.

The GHA cache backend shares compiled objects between jobs that compile the same
crate version. On cache-warm PRs (same dependencies, only application code changed),
all dependency crates are served from sccache.

**Estimated savings:** 30-90s per job on warm cache; biggest win on the first PR
after a dependency update (subsequent PRs to the same dep graph are free).

### 8. `.config/nextest.toml` with `ci` and `ci-shard` profiles

```toml
[profile.ci]
retries = 0
failure-output = "immediate"
slow-timeout = { period = "60s", terminate-after = 2 }

[profile.ci-shard]
retries = 0
failure-output = "immediate"
slow-timeout = { period = "60s", terminate-after = 2 }
```

Eliminates retry overhead in CI and surfaces slow tests immediately instead of
waiting for the full run to finish.

### 9. `regen-artifacts.sh` respects `PUML_BIN` env var

CI can now point the script at `target/release-ci/puml` without rebuilding the
binary inside the script. The fallback build path also uses `--profile release-ci`.

### 10. `check-all.sh --no-release-build` flag

Allows CI to skip the `cargo build --release` step inside the script when the
binary-size check is handled by a separate parallel job. The main-gate still runs
the full gate (with `--release`) because it needs the real shipped binary size.

---

## Wave 8 CI teardown — projected wall-time budget (after initial overhaul)

| Phase | Old (sequential) | After Wave 8 (parallel) | Delta |
|---|---|---|---|
| classify | 30s | 30s | — |
| lint (fmt+clippy) | 2:30 (two jobs) | 1:45 (one job) | -45s |
| test shards | 3:00 (single) | 1:45 (2×parallel) | -75s |
| coverage (llvm-cov) | 3:00 (serialized after tests) | 2:30 (parallel) | -30s critical-path |
| binary_size | 2:30 (after llvm-cov) | 1:15 (parallel, release-ci) | -75s critical-path |
| wasm check | 1:30 (parallel) | 1:00 (sccache warm) | -30s |
| required_check | 15s | 15s | — |
| **Critical path total** | **~8-9 min** | **~3:30-4 min** | **-4.5 min** |

---

## R10: Merge coverage into test shards (landed 2026-05-28)

The separate `coverage` job was a redundant instrumented compile pass — the test shards
had already compiled the code under the standard debug profile, then `coverage` recompiled
everything again with llvm-cov instrumentation. This was the biggest remaining waste.

**Change:** Both `test-shard-1` and `test-shard-2` now run `cargo llvm-cov nextest`
instead of `cargo nextest run`. Each shard does ONE compile pass that handles both test
execution and coverage instrumentation simultaneously.

**Coverage assertion strategy:** Each shard asserts `--fail-under-lines 87` independently
against the full codebase. A shard running half the tests will naturally have lower
coverage per file, so both shards must pass to ensure the full suite coverage bar is met.
This avoids the complexity of merging `.profdata` files across runners (which requires
uploading, downloading, and running `llvm-profdata merge` in a third job).

**Other changes in same pass:**
- `artifact_regen` job dropped from PR gate. Authors run `scripts/regen-artifacts.sh --force`
  locally and commit the result. The drift check (a single Python call) moved into the
  `lint` job — no separate runner needed. Auto-regen commit loop remains in `main-gate.yml`.
- `docs_examples_drift` job eliminated; its single command folded into `lint`.
- `skip_after_regen_push` job eliminated (depended on artifact_regen).
- `changes` job outputs trimmed to remove `run_artifact_regen` and `run_docs_examples_drift`.
- `required_check` aggregator simplified: removed artifact_regen fan-in, removed
  `coverage` as a separate required job (coverage is now inside the shards).
- `test-shard-2` skip condition: bot dep-updaters skip shard 2 (previously they skipped
  the separate `coverage` job; now shard 2 carries coverage, so the skip moved there).
- `pr-gate.yml` shrunk from 636 LOC → 339 LOC (47% reduction).
- Active parallel jobs in the typical Rust-change path: 5
  (`lint`, `test-shard-1`, `test-shard-2`, `binary_size`, and one of `wasm`/`site_smoke`
  when conditionally triggered).

**Revised wall-time projection (post-R10):**

| Phase | After Wave 8 | After R10 | Delta |
|---|---|---|---|
| classify | 30s | 30s | — |
| lint (fmt+clippy+drift) | 1:45 | 1:50 (+drift cmd) | +5s |
| test-shard-1 (with cov) | 1:45 (no cov) | 2:15 (instrumented) | +30s |
| test-shard-2 (with cov) | 1:45 (no cov) | 2:15 (instrumented) | +30s |
| separate coverage job | 2:30 | eliminated | -2:30 critical-path |
| binary_size | 1:15 | 1:15 | — |
| required_check | 15s | 15s | — |
| **Critical path total** | **~3:30-4 min** | **~2:20-2:45 min** | **-~1 min** |

Critical path: `classify → {lint ∥ shard-1 ∥ shard-2 ∥ binary_size} → required_check`
Limiting job on warm cache: `test-shard-1` or `test-shard-2` at ~2:15.

---

## What was NOT done (future opportunities)

- **sccache S3 backend**: external S3 bucket would give cross-PR cache hits (GHA
  cache backend is PR-scoped). Estimated additional savings: 30-60s per cold-cache
  PR. Blocked on IAM setup.
- **Three-way test partition**: would save another ~30s but adds a third runner.
  Can be done if 2-way partition still hits timeout on test suite growth.
- **`mold` linker**: the `mold` linker is significantly faster than GNU `ld` for
  incremental links. Add `RUSTFLAGS="-C link-arg=-fuse-ld=mold"` and
  `sudo apt-get install -y mold` in compile-heavy jobs. Estimated savings: 15-30s
  per link. Low-risk, high-reward.
- **profdata merge approach**: if per-shard coverage assertions prove too noisy
  (e.g., shard 1 always fails because visual-regression tests are in shard 2),
  switch to uploading `.profdata` artifacts and merging in a third job. The current
  strategy avoids that complexity for now.
