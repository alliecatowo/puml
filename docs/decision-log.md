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
- Decision: Support only `skinparam maxmessagesize` semantically; keep other `skinparam` keys and `!theme` non-fatal with deterministic warnings.
- Rationale: Only `maxmessagesize` is required for current layout behavior and has deterministic downstream effects.
- Impact: Unsupported styling directives emit warning diagnostics to `stderr` in check/dump/render flows, while successful runs keep exit code `0`.
- Spec/implementation contradiction and resolution: PlantUML accepts many `skinparam` keys; this implementation limits support to one key to avoid implying styling parity that does not exist.

### D-005: Include-root boundary for stdin mode
- Decision: Gate include resolution behind explicit `--include-root DIR` when reading from stdin.
- Rationale: Stdin input has no stable file-relative base path; explicit root prevents ambiguous or unsafe path resolution.
- Impact: Include-capable workflows from stdin must provide `--include-root`, and directive handling still follows D-003 fail-fast behavior.

### D-006: Canonical include confinement
- Decision: Canonicalize include root and each `!include` target, and reject targets outside the canonical root.
- Rationale: Blocks both lexical `../` traversal escapes and symlink-based escapes that bypass path normalization.
- Impact: Include escapes now fail with explicit include diagnostics (for example `E_INCLUDE_ESCAPE`), while in-root includes continue to resolve.
