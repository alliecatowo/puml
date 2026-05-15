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

### D-003: Directive tokens recognized, execution deferred
- Decision: Parse `!include`, `!define`, and `!undef` tokens but reject them during normalization.
- Rationale: Preserves forward-compatible syntax recognition while avoiding partial preprocessing semantics.
- Impact: Inputs using these directives fail with a validation warning error rather than being silently ignored.
- Spec/implementation contradiction and resolution: PlantUML directives typically imply preprocessing behavior; this implementation adopts fail-fast rejection until full preprocessing can be implemented safely.

### D-004: `skinparam` contract narrowed
- Decision: Support deterministic sequence styling keys (`maxmessagesize`, `footbox`/`sequenceFootbox`, `ArrowColor`, `SequenceLifeLineBorderColor`, `ParticipantBackgroundColor`, `ParticipantBorderColor`, `NoteBackgroundColor`, `NoteBorderColor`, `GroupBackgroundColor`, `GroupBorderColor`) and keep other `skinparam` keys plus `!theme` non-fatal with deterministic warnings.
- Rationale: These keys have clear, stable rendering effects and improve practical parity without introducing non-deterministic theming behavior.
- Impact: Unsupported styling directives emit warning diagnostics to `stderr` in check/dump/render flows, while successful runs keep exit code `0`.
- Spec/implementation contradiction and resolution: PlantUML accepts many additional styling/theme controls; this implementation intentionally supports a bounded subset and warns on the rest to preserve deterministic output.

### D-005: Include-root boundary for stdin mode
- Decision: Gate include resolution behind explicit `--include-root DIR` when reading from stdin.
- Rationale: Stdin input has no stable file-relative base path; explicit root prevents ambiguous or unsafe path resolution.
- Impact: Include-capable workflows from stdin must provide `--include-root`, and directive handling still follows D-003 fail-fast behavior.

### D-006: Canonical include confinement
- Decision: Canonicalize include root and each `!include` target, and reject targets outside the canonical root.
- Rationale: Blocks both lexical `../` traversal escapes and symlink-based escapes that bypass path normalization.
- Impact: Include escapes now fail with explicit include diagnostics (for example `E_INCLUDE_ESCAPE`), while in-root includes continue to resolve.

### D-007: Preprocessor behavior clarified to match runtime
- Decision: Treat `!include` as executable today (with read/cycle/root guards), while `!define`/`!undef` remain out of scope for sequence rendering semantics.
- Rationale: Practical audit on 2026-05-15 showed current runtime performs include resolution and surfaces include diagnostics, which contradicts the earlier "recognized but rejected" framing in D-003.
- Impact: Contract docs should describe include as active behavior with explicit safety boundaries, and keep `!define`/`!undef` documented as unsupported for normalized sequence execution.
- Spec/implementation contradiction and resolution: PlantUML preprocessing remains broader than this implementation; we intentionally keep a narrower contract instead of implying full preprocessing parity.

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
