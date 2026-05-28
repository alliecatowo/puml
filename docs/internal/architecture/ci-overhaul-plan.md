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

## Projected new wall-time budget (per-PR typical Rust change)

| Phase | Old (sequential) | New (parallel) | Delta |
|---|---|---|---|
| classify | 30s | 30s | — |
| lint (fmt+clippy) | 2:30 (two jobs) | 1:45 (one job) | -45s |
| test shards | 3:00 (single) | 1:45 (2×parallel) | -75s |
| coverage (llvm-cov) | 3:00 (serialized after tests) | 2:30 (parallel) | -30s critical-path |
| binary_size | 2:30 (after llvm-cov) | 1:15 (parallel, release-ci) | -75s critical-path |
| wasm check | 1:30 (parallel) | 1:00 (sccache warm) | -30s |
| required_check | 15s | 15s | — |
| **Critical path total** | **~8-9 min** | **~3:30-4 min** | **-4.5 min** |

Critical path: `classify → lint (parallel with shards+coverage+binary_size) → required_check`

The longest parallel group is `{lint, test-shard-1, test-shard-2, coverage, binary_size}`.
On a warm cache, the limiting job is `coverage` at ~2:30. On cold cache it's ~3:30.

---

## What was NOT done (future opportunities)

- **sccache S3 backend**: external S3 bucket would give cross-PR cache hits (GHA
  cache backend is PR-scoped). Estimated additional savings: 30-60s per cold-cache
  PR. Blocked on IAM setup.
- **Bench removal from PR gate**: benchmarks already skipped in PR gate via
  `--skip-bench` in the `quality` job invocation. No action needed.
- **Doc-only skip for heavy jobs**: the classifier already gates on `run_full_gate`.
  When only `*.md` / `docs/` / `site/**` change, all Rust compile jobs are skipped.
  This is already wired; no additional work needed.
- **Three-way test partition**: would save another ~30s but adds a third runner
  (~$0.006/min × 30min × $0.002/min = negligible cost). Can be done if 2-way
  partition still hits timeout on test suite growth.
- **`mold` linker**: the `mold` linker is significantly faster than GNU `ld` for
  incremental links. Add `RUSTFLAGS="-C link-arg=-fuse-ld=mold"` and
  `sudo apt-get install -y mold` in compile-heavy jobs. Estimated savings: 15-30s
  per link. Low-risk, high-reward.
