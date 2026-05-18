# Issue 95: [P2][ci] Wire render_corpus.py into PR gate as informational artifact

## Title

`[P2][ci] Wire render_corpus.py into PR gate as informational artifact`

## Summary

`scripts/render_corpus.py` exists and produces `target/audit_corpus/manifest.json`
with render status for every `.puml`/`.txt` in the corpus. It is not yet wired into CI.
This issue covers integrating it into the PR gate as a non-blocking informational step.

## Motivation

- Audit agents need up-to-date PNGs. If CI renders them on every PR, agents can retrieve
  the manifest artifact and compare against the baseline instead of running a local render.
- Failed renders in the manifest expose sources that can't render — visual bugs we want to
  track continuously, not just on local runs.
- The manifest gives us a time-series of render success rate across PRs.

## Acceptance Criteria

1. A new workflow job (or a job added to `.github/workflows/pr-gate.yml`) runs
   `python3 scripts/render_corpus.py` on PR when any of these path globs change:
   - `docs/examples/**`
   - `tests/fixtures/**`
   - `src/**` (renderer changes affect all renders)
   - `scripts/render_corpus.py`

2. The job uploads `target/audit_corpus/manifest.json` as a GitHub Actions artifact
   named `render-corpus-manifest`.

3. The job DOES NOT fail the build on render warnings or even render failures
   (exit code from the script is 0 for render-level failures; only script-level
   errors are non-zero). The gate stays green.

4. Initial wiring uses `workflow_dispatch` only (or `if: false` on the PR trigger)
   so it cannot unexpectedly block PRs. Graduate to `pull_request` path filter
   after one full CI dry run confirms it completes within 5 minutes.

## Implementation Notes

- Start with `workflow_dispatch` trigger so it can be manually tested without PR risk.
- The render step needs `cargo build --release` or a cached binary. Prefer caching
  the binary artifact from a prior build job to avoid adding ~3 minutes of compile time.
- Consider uploading the full `target/audit_corpus/png/` tree only on explicit
  `workflow_dispatch` runs (too large for every PR artifact).
- Worker count should be set to `$(nproc)` in CI.

## Related

- `scripts/render_corpus.py` — the script to wire
- `scripts/visual_audit_batch.py` — downstream consumer for audit agents
- `docs/internal/visual-audit-pipeline.md` — architecture doc
- Parent: self-driving visual development loop
