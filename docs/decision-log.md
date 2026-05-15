# Decision Log

This log records intentional contract deviations and updates adopted in the current implementation.

## 2026-05-15

### D-001: Sequence-only scope
- Decision: Treat sequence diagrams as the only supported diagram family.
- Rationale: Keeps parser, normalization, and rendering behavior deterministic for the delivered MVP.
- Impact: Class/state and other non-sequence syntax is rejected at validation time (exit code `1`).

### D-002: Explicit opt-in for multi-diagram parsing
- Decision: Require `--multi` to accept inputs containing multiple `@startuml`/`@enduml` blocks.
- Rationale: Prevents accidental behavioral changes for single-diagram workflows and output paths.
- Impact: Multi-block input without `--multi` fails validation and instructs the user to rerun with `--multi`.

### D-003: Directive tokens recognized, execution deferred (superseded)
- Decision: Initial policy (superseded by D-007) parsed `!include`, `!define`, and `!undef` tokens but deferred executable preprocessing behavior.
- Rationale: Preserves historical context for the contract transition.
- Impact: This entry is historical only; current runtime behavior is defined by D-007 and later decisions.
- Spec/implementation contradiction and resolution: superseded.

### D-004: `skinparam` contract narrowed
- Decision: Support deterministic sequence styling keys (`maxmessagesize`, `footbox`/`sequenceFootbox`, `ArrowColor`, `SequenceLifeLineBorderColor`, `ParticipantBackgroundColor`, `ParticipantBorderColor`, `NoteBackgroundColor`, `NoteBorderColor`, `GroupBackgroundColor`, `GroupBorderColor`) and keep other `skinparam` keys plus `!theme` non-fatal with deterministic warnings.
- Rationale: These keys have clear, stable rendering effects and improve practical parity without introducing non-deterministic theming behavior.
- Impact: Unsupported styling directives emit warning diagnostics to `stderr` in check/dump/render flows, while successful runs keep exit code `0`.
- Spec/implementation contradiction and resolution: PlantUML accepts many additional styling/theme controls; this implementation intentionally supports a bounded subset and warns on the rest to preserve deterministic output.

### D-005: Include-root boundary for stdin mode
- Decision: Gate include resolution behind explicit `--include-root DIR` when reading from stdin.
- Rationale: Stdin input has no stable file-relative base path; explicit root prevents ambiguous or unsafe path resolution.
- Impact: Include-capable workflows from stdin must provide `--include-root` in `strict` mode; directive behavior follows bounded preprocessing rules defined in D-007 and later entries.

### D-006: Canonical include confinement
- Decision: Canonicalize include root and each `!include` target, and reject targets outside the canonical root.
- Rationale: Blocks both lexical `../` traversal escapes and symlink-based escapes that bypass path normalization.
- Impact: Include escapes now fail with explicit include diagnostics (for example `E_INCLUDE_ESCAPE`), while in-root includes continue to resolve.

### D-007: Preprocessor behavior clarified to match runtime
- Decision: Treat bounded preprocessing as executable today: `!include` resolution (with read/cycle/root guards) plus simple `!define`/`!undef` token substitution before normalization.
- Rationale: Runtime audit on 2026-05-15 showed shipped behavior already performs these preprocessing steps, so docs must describe actual executable behavior.
- Impact: Contract docs should describe include + define/undef substitution as supported within explicit safety/feature boundaries.
- Spec/implementation contradiction and resolution: PlantUML preprocessing remains broader than this implementation; we intentionally keep a narrower bounded contract instead of implying full preprocessing parity.

### D-008: Strict include baseline for include-id and URL handling
- Decision: Add bounded include-id extraction for `!include file!TAG` using local `!startsub TAG`/`!endsub` blocks, and hard-reject URL includes with a dedicated deterministic diagnostic.
- Rationale: This is the first strict-mode foundation slice for preprocessor parity: expand local include capabilities while keeping network behavior explicitly unsupported and deterministic.
- Impact: Missing tags now fail with `E_INCLUDE_TAG_NOT_FOUND`; URL targets fail with `E_INCLUDE_URL_UNSUPPORTED`; missing files continue to fail with deterministic `E_INCLUDE_READ`.
- Spec/implementation contradiction and resolution: PlantUML supports broader include variants; current behavior intentionally limits include-id extraction to local tagged sub-blocks only.

### D-009: Compat contract interpretation at parse boundary
- Decision: Interpret `compat` and `determinism` at one explicit parse-pipeline contract boundary; keep strict and extended on a single parser path.
- Rationale: Avoid split-brain routing where mode behavior drifts across CLI entry points, while creating explicit extension hooks for future parity work.
- Impact:
  - `compat=strict` keeps deterministic include behavior: stdin `!include` requires explicit `--include-root`.
  - `compat=extended` enables a minimal, real hook: stdin `!include` falls back to current working directory when `--include-root` is omitted.
  - `determinism` interpretation is explicit even though both modes currently execute the same deterministic runtime behavior.
- Spec/implementation contradiction and resolution: PlantUML offers broader preprocessing convenience; this implementation exposes explicit compatibility profiles while preserving deterministic behavior and a single shared parser pipeline.

### D-010: First-class polymorphic language policy
- Decision: Treat PicoUML, PlantUML, and Mermaid as first-class product surfaces; position PicoUML as canonical language, PlantUML as first-class 1:1 compatibility target, and Mermaid as first-class supported frontend.
- Rationale: Prevents hierarchy framing where one surface is presented as "extended mode" or second-class, and keeps product/docs contracts aligned with a single polymorphic engine architecture.
- Impact: User-facing docs and CLI help should describe language surfaces as first-class and compatibility/determinism as policy controls rather than product tiering.

### D-011: Batch lint/check mode contract for docs pipelines
- Decision: Add explicit batch check inputs via `--lint-input` (repeatable paths) and `--lint-glob` (repeatable patterns), with deterministic target ordering and a mandatory lint summary report on `stdout`.
- Rationale: Docs and CI pipelines need stable non-interactive validation across many files without shell-dependent ordering.
- Impact:
  - Any validation failure in the batch returns exit code `1`.
  - Diagnostics still follow the existing stream contract (`stderr`, human/json via `--diagnostics`).
  - Lint summary report is selectable via `--lint-report human|json` and remains on `stdout`.
### D-012: `newpage` + `ignore newpage` stdin contract
- Decision: Keep stdin multi-output behavior explicit: multipage stdin (`newpage`) requires `--multi`; `ignore newpage` collapses splits into single-output behavior.
- Rationale: Preserves deterministic CLI contracts while making mode-specific behavior explicit in help/docs/tests.
- Impact: File inputs still auto-emit numbered files for multi-page outputs; stdin workflows must opt in to multi-output JSON payloads.

### D-013: Transactional multi-file output writes
- Decision: Stage and publish multi-file outputs transactionally for both standard and markdown output paths.
- Rationale: Prevent partially updated output sets when any numbered output write fails.
- Impact: Multi-output failures now return I/O exit code `2` without leaving partial numbered files behind.

### D-014: Full release gate must include release-build validation
- Decision: Treat `cargo build --release` as a required full-gate contract step (alongside fmt, clippy, tests, and coverage).
- Rationale: Release readiness is incomplete without validating optimized build output under the same audited gate path.
- Impact:
  - `./scripts/check-all.sh` full mode now enforces `cargo build --release` before benchmark gate checks.
  - Release checklist/docs must keep this command chain explicit and deterministic.

### D-015: Deterministic-safe `skinparam` color token policy
- Decision: Accept sequence skinparam color values only as deterministic-safe tokens: hex forms (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`) or alphabetic color names; canonicalize accepted values to lowercase before render.
- Rationale: Prevent malformed/unsafe attribute injection in SVG and avoid renderer drift from ambiguous free-form color expressions.
- Impact:
  - Invalid color values emit deterministic `W_SKINPARAM_UNSUPPORTED_VALUE` warnings and keep existing style defaults/previous valid assignments.
  - Render output remains deterministic and free of raw unsafe color payloads.
### D-016: Scoped coverage gate for core runtime modules
- Decision: Keep the full gate line-coverage threshold at `90%`, but scope it away from CLI entrypoint binaries via `cargo llvm-cov --ignore-filename-regex 'src/(main|bin/puml-lsp)\.rs'`.
- Rationale: `src/main.rs` and `src/bin/puml-lsp.rs` contain integration-heavy process/IO orchestration branches that materially understate core parser/normalize/layout/render coverage when aggregated into the same gate.
- Impact:
  - `./scripts/check-all.sh` full mode now enforces scoped coverage with the same `90%` threshold.
  - Release-contract docs/tests must pin both the baseline coverage command string and scoped regex to keep policy explicit and reviewable.

### D-017: Regression gate adds absolute delta floor
- Decision: Keep benchmark regression percentage gates (`10%` full / `20%` quick) but require a minimum absolute slowdown delta before failing (`>20ms` full / `>30ms` quick).
- Rationale: Small timing jitter on short scenarios can exceed percentage thresholds without representing meaningful regressions, producing flaky release gates.
- Impact:
  - `scripts/bench.sh --enforce-gates` now fails regression checks only when both percentage and absolute delta thresholds are exceeded.
  - Benchmark docs and release checklist thresholds include both the percentage and absolute-delta criteria.

### D-018: Mode-scoped benchmark baselines and explicit baseline movement
- Decision: Compare regression only against mode-matching baseline artifacts (`baseline_full.json`, `baseline_quick.json`) and require explicit `--update-baseline` to move them.
- Rationale: A shared mutable `latest.json` baseline caused cross-mode false regressions and noisy drift after transient runs.
- Impact:
  - `scripts/bench.sh` now keeps regression comparisons mode-scoped and skips mismatch comparisons.
  - Baseline drift is controlled via explicit refresh commands instead of implicit every-run movement.
  - Gate/trend logic is extracted to `scripts/bench_gate.py` and guarded by dedicated tests.

### D-019: Preprocessor control-flow baseline (`!if`/`!ifdef`/`!while`) with deterministic unsupported diagnostics
- Decision: Extend bounded preprocessing to execute conditional directives (`!if`/`!elseif`/`!else`/`!endif`, `!ifdef`, `!ifndef`) and simple bounded loops (`!while`/`!endwhile`), while explicitly rejecting unsupported preprocessor directives (notably `!procedure`/`!function`) with deterministic error codes.
- Rationale: This closes the core control-flow parity gap from issue #112 without implying full PlantUML preprocessor breadth.
- Impact:
  - Conditionals and `!while` now run in preprocessing before parse/normalize with deterministic behavior.
  - Misordered/unbalanced control directives return explicit `E_PREPROC_COND_*` / `E_PREPROC_WHILE_*` diagnostics.
  - Unsupported directives return explicit `E_PREPROC_UNSUPPORTED` instead of falling through to generic parse-unknown errors.
- Spec/implementation contradiction and resolution: PlantUML preprocessor still has broader expression/function/procedure capabilities; this implementation deliberately ships a smaller deterministic subset and fails unsupported directives explicitly.

### D-020: Minimal-compatible preprocessor macro directive behavior
- Decision: Accept `!function`/`!procedure` block directives as non-executing preprocessor blocks, accept `!log` and `!dump_memory` as no-op directives, and enforce deterministic `!assert` pass/fail evaluation for simple literal expressions.
- Rationale: Expands macro/preprocessor compatibility beyond basic token substitution while keeping parser behavior deterministic and avoiding partial dynamic macro execution semantics.
- Impact:
  - `!function`/`!procedure` blocks are consumed by preprocessing and no longer leak into parser unknown-syntax errors.
  - `!assert` failures return deterministic error `E_PREPROC_ASSERT`.
  - Missing block terminators return deterministic errors (`E_FUNCTION_UNCLOSED`, `E_PROCEDURE_UNCLOSED`).
  - Dynamic invocation and JSON preprocessing behavior remain out of scope for this slice.

### D-021: Mermaid sequence subset expansion with construct-specific unsupported diagnostics
- Decision: Expand Mermaid frontend support beyond participants + arrows to include `Note over|left of|right of`, lifecycle directives (`activate`/`deactivate`/`destroy`), `autonumber`, `title`, and inline `%%` comments.
- Rationale: Closes high-frequency migration gaps while preserving one shared deterministic pipeline and explicit unsupported boundaries.
- Impact:
  - Mermaid `sequenceDiagram` inputs using the above constructs now pass through adaptation into the PlantUML shared parser path.
  - Unsupported Mermaid sequence block/control constructs now emit deterministic construct-class codes (`E_MERMAID_BLOCK_UNSUPPORTED`, `E_MERMAID_CREATE_UNSUPPORTED`, `E_MERMAID_LINK_UNSUPPORTED`) instead of only generic unsupported-construct diagnostics.
  - Generic unsupported Mermaid sequence constructs still emit `E_MERMAID_CONSTRUCT_UNSUPPORTED`.
