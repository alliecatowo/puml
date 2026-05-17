+++
title = "Contributing"
description = "Build, test, lint, and the contracts the gates enforce."
weight = 40
+++

## One-time setup

```bash
git clone https://github.com/alliecatowo/puml
cd puml
./scripts/setup.sh
```

## Fast local loop

```bash
./scripts/dev.sh        # fmt + clippy + test
```

## Quality gate

```bash
./scripts/check-all.sh           # full: fmt + clippy + test + coverage + release build + bench gates
./scripts/check-all.sh --quick   # skip coverage and release build; keep quick bench gates
```

CI runs the quick gate on PRs and the full gate on main. The contracts:

- `.github/workflows/pr-gate.yml` &rarr; fmt + clippy + test + coverage gate + quick check-all.
- `.github/workflows/main-gate.yml` &rarr; full check-all + bench evidence artifacts.

## Tests

The test corpus lives in `tests/` with snapshots under `tests/snapshots/`. The big ones to know:

- `integration.rs` &mdash; broad smoke and parity tests.
- `render_e2e.rs` &mdash; end-to-end render asserts.
- `docs_harness_contract_audit.rs` &mdash; docs-as-tests harness.
- `parity_*_audit.rs` &mdash; cross-dialect parity sweeps.
- `coverage_*.rs` &mdash; coverage gate enforcement.

```bash
cargo test
cargo test -- --nocapture render_e2e
```

## Docs-as-tests

Every example in `docs/examples/` is both:

1. Linked from the [gallery](@/gallery.md) on this site.
2. Asserted by the parity harness (`scripts/parity_harness.py`).

When you commit a new `.puml`, commit the matching `.svg` artifact alongside it. The site's `scripts/build-site.mjs` walks the corpus on build, so new examples surface automatically on the next deploy.

## Branch protection

The branch-protection contract is documented in [`docs/branch-protection.md`](https://github.com/alliecatowo/puml/blob/main/docs/branch-protection.md) and validated by:

```bash
./scripts/branch-protection.sh verify
```

## Benchmarks

```bash
./scripts/bench.sh                  # full benchmark refresh
./scripts/bench.sh --quick          # quick profile
./scripts/bench.sh --enforce-gates  # perf + binary-size gates
```

## Style

- Rust 2021 edition.
- `cargo fmt` and `cargo clippy -- -D warnings` are non-negotiable.
- No new production deps without a written justification.
- No `unwrap` / `expect` in library code outside of test fixtures.
- No `unsafe` outside of carefully scoped, audited FFI boundaries.

## Where to ask questions

- Open an issue at <https://github.com/alliecatowo/puml/issues>.
- Tag with the relevant family or area (`area/sequence`, `area/cli`, `area/wasm`).
