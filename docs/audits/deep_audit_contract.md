# Deep Audit: Contract Parity + CLI/DX + Docs Truthfulness

Date: 2026-05-15  
Lane: 2 (Contract parity and docs truthfulness)  
Branch: `codex/audit-contract-deep`

## Scope audited

- `README.md`
- `docs/decision-log.md`
- `docs/parity-roadmap.md`
- `docs/release-contract-audit.md`
- specs under `docs/specs/`
- runtime surfaces in CLI (`src/main.rs`, `src/cli.rs`), LSP (`src/bin/puml-lsp.rs`), VS Code scaffold (`extensions/vscode/*`), and checks (`scripts/*`, `tests/*contract*`)

## Executable evidence log

Commands run:

1. `cargo test --test release_contract_audit --test ecosystem_rollout_contract_audit --test studio_spa_contract_audit`
2. CLI contract probes:
   - `cargo run -- --help`
   - `cargo run -- --check tests/fixtures/basic/hello.puml`
   - `cargo run -- --check --diagnostics json tests/fixtures/invalid_single.puml`
   - `cargo run -- does-not-exist.puml`
   - `cat tests/fixtures/structure/multi_three.puml | cargo run -- -`
   - `cat tests/fixtures/structure/multi_three.puml | cargo run -- --multi -`
   - `cargo run -- --check --lint-input tests/fixtures/basic/hello.puml --lint-report json --diagnostics json`
   - `cargo run -- --dialect picouml --check tests/fixtures/basic/hello.puml`
   - `cat tests/fixtures/styling/valid_skinparam_unsupported.puml | cargo run -- --diagnostics json -`
3. Markdown/multi naming probes:
   - `cat tests/fixtures/markdown/multipage_mixed.md | cargo run -- --from-markdown --multi -`
   - `cargo run -- --multi /tmp/puml-audit-md/demo.md`
4. VS Code scaffold smoke:
   - `./scripts/vscode-smoke.sh`
5. Docs/path truthfulness probe:
   - fixture reference existence sweep from `docs/parity-roadmap.md`

Selected raw outcomes:

- Exit codes observed align with documented matrix for tested paths:
  - validation/usage failures returned `1`
  - I/O read failure returned `2`
  - successful checks/renders returned `0`
- `--multi` guard observed for stdin multi diagrams:
  - without `--multi`: `multiple diagrams detected; rerun with --multi` + exit `1`
  - with `--multi`: JSON array payload + exit `0`
- Diagnostics stream behavior observed:
  - warnings/errors emitted on `stderr`
  - lint report emitted on `stdout`

## Severity-ranked findings

## High

1. VS Code default LSP startup path is non-functional in this repo checkout.

- Contract/docs claim: extension setting docs state empty `puml.lsp.path` uses bundled binary path (`bin/puml-lsp` / `.exe`).
- Runtime reality:
  - `extensions/vscode/src/client/lspClient.ts` defaults to `extensions/vscode/bin/...`.
  - `extensions/vscode/bin` does not exist in repository (`ls: cannot access 'extensions/vscode/bin': No such file or directory`).
  - current smoke only verifies build + preview marker, not executable LSP binary presence.
- Impact:
  - out-of-box extension startup fails unless user manually configures `puml.lsp.path`.
  - docs and DX promise do not match default behavior.

Proposed fix slice (`AUD-CONTRACT-H1`):
- Add runtime fallback: if bundled binary missing, fall back to `puml-lsp` on `PATH`.
- Update extension README + setting descriptions to document fallback order.
- Extend VS Code smoke check to enforce presence of fallback logic marker (and optionally binary existence when expected in packaged contexts).

## Medium

2. Decision log has contradictory/stale preprocessor contract language for `!define`/`!undef`.

- Doc mismatch:
  - `docs/decision-log.md` D-007 states `!define`/`!undef` remain out of scope for normalized sequence execution.
- Runtime reality:
  - parser preprocesses define/undef substitutions before normalization (`src/parser.rs`).
  - executable proof:
    - diagram with `!define USER Alice` normalized successfully; `--dump model` shows participant/message IDs as `Alice`.
    - diagram with `!define` then `!undef` resolves to original token (`A`) and succeeds.
- Impact:
  - contract text can mislead users and maintainers about supported directive behavior.

Proposed fix slice (`AUD-CONTRACT-M1`):
- Update decision-log entries to reflect actual shipped behavior: bounded substitution support for `!define`/`!undef` in preprocessing, with current limits.
- Add/extend doc-contract test to pin this claim.

3. `docs/parity-roadmap.md` includes a stale fixture path in “implemented fixture coverage”.

- Evidence command found one missing path:
  - `MISSING tests/fixtures/basic/valid_virtual_endpoints_directional.puml`
- Impact:
  - roadmap “implemented coverage” claim is not fully truthful.

Proposed fix slice (`AUD-CONTRACT-M2`):
- Replace stale path with current canonical fixture path(s) actually present.
- Add contract test that `docs/parity-roadmap.md` fixture references must resolve on disk.

## Low

4. Markdown renderer spec (`docs/specs/puml_markdown_fence_renderer_spec(1).md`) reads as shipped package contract without a “current runtime snapshot” section, unlike LSP/VSCode/Studio specs.

- Evidence:
  - repo has no `@puml/markdown` package or `puml-md` CLI implementation references outside this spec.
- Impact:
  - reader may interpret target architecture as current runtime availability.

Proposed fix slice (`AUD-CONTRACT-L1`):
- Add explicit “Runtime contract snapshot (Current)” and “Target-state sections” boundary, mirroring other specs.
- Optionally add contract audit test fixture to pin that boundary text.

## Areas verified as currently aligned

- Release gate contract docs and executable tests are in sync:
  - `tests/release_contract_audit.rs` passes.
  - `docs/release-contract-audit.md` reflects check-all gate chain.
- Ecosystem runtime snapshot token guards are present and passing:
  - `tests/ecosystem_rollout_contract_audit.rs` and `tests/studio_spa_contract_audit.rs` pass.
- CLI exit codes and stream contract for tested paths match `README.md`.

## Follow-up implementation plan

1. Follow-up PR A (`AUD-CONTRACT-H1`): VS Code LSP startup fallback + README/settings updates + smoke guard.
2. Follow-up PR B (`AUD-CONTRACT-M1`,`AUD-CONTRACT-M2`): decision-log correction + parity-roadmap stale path fix + new contract tests.
3. Follow-up PR C (`AUD-CONTRACT-L1`): markdown spec current-vs-target boundary + optional spec contract test.

## Rebase/iteration notes

- Rebase each follow-up branch onto `origin/main` before merge.
- Keep fixes narrowly scoped and test-backed where executable.
