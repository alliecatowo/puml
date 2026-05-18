# Deep Architecture and Correctness Audit

Date: 2026-05-15
Scope: parse pipeline boundaries, determinism, parser/normalize/layout/render correctness, and regression-hiding test gaps.

## Severity Ranking

### High

1. Library pipeline determinism leak through `CompatMode::Extended` cwd fallback
- Area: layering and determinism boundaries
- Evidence:
  - `parse_with_pipeline_options` calls `interpret_parser_contract` in `src/lib.rs:98-129`.
  - In extended mode, include root defaults via `std::env::current_dir()` (`src/lib.rs:124-128`).
- Risk:
  - Library behavior depends on ambient process cwd even when callers provide no include root.
  - Same source/options can yield different parse outcomes across environments, violating deterministic API expectations and coupling core library semantics to CLI execution context.
- Test gap:
  - CLI has an extended-mode stdin test (`tests/integration.rs:137-152`), but no library-level assertion that core parsing is environment-agnostic.

2. Lifeline anchor ignores computed participant box height
- Area: layout/render geometric correctness
- Evidence:
  - Participant height is dynamically computed from wrapped display text (`src/layout.rs:37-50`).
  - Lifeline start uses fixed option height (`src/layout.rs:314`) rather than computed `participant_height`.
- Risk:
  - Long or wrapped participant labels can cause lifelines to begin inside participant headers, creating overlapping geometry and rendering inaccuracies.
- Test gap:
  - Existing rendering tests focus on snapshots/overflow behavior but do not assert participant box vs lifeline start invariants (`tests/render_e2e.rs`).

3. Parser rejects valid `!pragma` directive forms with arguments
- Area: parser correctness / compatibility subset boundaries
- Evidence:
  - Parser only skipped exact `!pragma` lines in `src/parser.rs:361`, so common forms like `!pragma teoz true` were treated as unknown syntax.
- Risk:
  - Valid control directives in supported sequence sources trigger false parse failures.
- Test gap:
  - No fixture or parser unit test covering `!pragma` with arguments.

### Medium

4. Scene dump path is structurally decoupled from actual layout pipeline
- Area: pipeline boundary correctness
- Evidence:
  - `--dump scene` uses synthetic spacing logic in `scene_to_json` (`src/main.rs:1381-1417`) instead of serializing actual `layout::layout_pages` output used for rendering (`src/main.rs:333-334`).
- Risk:
  - Scene dump can drift from render behavior, making it less trustworthy for debugging layout regressions.
- Test gap:
  - Scene snapshot tests validate contract shape and ordering, but not equivalence to renderer geometry.

5. Parser/normalizer virtual-endpoint metadata is duplicated across layers
- Area: parser/normalize layering hygiene
- Evidence:
  - Virtual endpoint derivation appears in parser (`src/parser.rs:718-737`) and again in normalizer (`src/normalize.rs:681-700`).
- Risk:
  - Dual implementations increase risk of subtle divergence for endpoint semantics over time.
- Test gap:
  - Existing fidelity tests are strong for current semantics, but architectural duplication still raises future regression risk.

## Prioritized Fix Plan

1. Move cwd fallback decision to CLI boundary; keep library parse contract explicit and environment-independent.
2. Anchor lifeline start to computed participant geometry and add invariant coverage.
3. Treat `!pragma ...` control directives as ignorable parser control lines in the deterministic subset.

## Audit Notes

- The codebase already demonstrates strong deterministic intent (sorted maps, stable diagnostics shaping, snapshot-heavy integration coverage).
- The highest-impact remaining risks are semantic parity and boundary placement, not broad instability.
