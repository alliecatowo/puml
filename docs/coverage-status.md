# Coverage Status

Last measured: 2026-05-15 (America/Los_Angeles)

Command:

```console
cargo llvm-cov --all-features --workspace --fail-under-lines 90
```

Result:
- Gate: failed
- Total line coverage: 86.29%
- Target: 90%
- Gap: 3.71 points

Top low-coverage modules from latest run:
- `main.rs`: 75.51% lines
- `normalize.rs`: 79.48% lines

High-confidence next lift areas:
- `main.rs`: hard-to-hit CLI and output error branches.
- `normalize.rs`: lifecycle edge/error paths and rare directive/style branches.
