# Coverage Status

Last measured: 2026-05-15 (America/Los_Angeles)

Command:

```console
cargo llvm-cov --all-features --workspace --fail-under-lines 90
```

Result:
- Gate: failed
- Total line coverage: 80.66%
- Target: 90%
- Gap: 9.34 points

Top low-coverage modules from latest run:
- `main.rs`: 72.42% lines
- `normalize.rs`: 77.20% lines
- `diagnostic.rs`: 25.00% lines
- `source.rs`: 16.67% lines
- `theme.rs`: 0.00% lines

Interpretation:
- The primary leverage for coverage gains is focused testing around CLI error/IO branches (`main.rs`) and semantic edge paths (`normalize.rs`).
- Utility modules with little direct behavior (`theme.rs`) can be covered with lightweight smoke tests, or intentionally excluded if policy allows.
