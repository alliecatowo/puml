# Troubleshooting

## `cargo llvm-cov` command not found

Symptoms:
- `./scripts/check-all.sh` exits early with a coverage-tooling message.

Fix:

```console
./scripts/setup.sh
```

Manual fallback:

```console
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

## Coverage gate fails below 90%

Symptoms:
- `cargo llvm-cov --all-features --workspace --fail-under-lines 90` fails.

Fix approach:
- Inspect low-coverage modules in output.
- Add focused tests under `tests/` and/or fixtures under `tests/fixtures/`.
- Re-run `./scripts/check-all.sh`.

Reference: `docs/coverage-status.md`.

## Benchmark runs are noisy or unavailable

Symptoms:
- benchmark output varies significantly between runs
- `hyperfine` not found

Notes:
- `scripts/bench.sh` automatically falls back to `/usr/bin/time`.
- For more stable timing, install `hyperfine` and reduce machine load.

## `--include-root` errors in stdin mode

Symptoms:
- include-related failures when running via stdin.

Cause:
- stdin has no file-relative directory context.

Fix:

```console
cat diagram.puml | cargo run -- --check --include-root ./tests/fixtures/include -
```

## Diagnostics are hard to map back to source

Symptoms:
- validation or warning messages are unclear without location context.

Notes:
- source-related diagnostics include `line`, `column`, and a caret-marked source snippet in `--check`, `--dump`, and render modes.
- messages without source spans stay single-line by design.
- use `--diagnostics json` for machine-readable diagnostics payloads in CI/tooling.
- JSON contract is versioned as `schema: "puml.diagnostics"` + `schema_version: 1`.
- diagnostics are always emitted to `stderr`; mode outputs (SVG / dump JSON) stay on `stdout`.

Example:

```console
$ cargo run -- --check --diagnostics json tests/fixtures/arrows/invalid_malformed_arrows.puml
{
  "schema": "puml.diagnostics",
  "schema_version": 1,
  "diagnostics": [
    {
      "code": "E_ARROW_INVALID",
      "severity": "error",
      "message": "[E_ARROW_INVALID] malformed sequence arrow syntax: `A -x B: malformed`",
      "span": {"start": 10, "end": 27},
      "line": 2,
      "column": 1,
      "snippet": "A -x B: malformed",
      "caret": "^^^^^^^^^^^^^^^^^"
    }
  ]
}
```

## Batch lint mode reports no files

Symptoms:
- `--check --lint-glob ...` exits with "resolved no input files".

Cause:
- glob patterns are expanded by `puml`, so unmatched patterns yield an empty lint target set.

Fix:

```console
# quote globs so puml receives the pattern directly
cargo run -- --check --lint-glob 'docs/**/*.md'

# or pass explicit repeated files
cargo run -- --check --lint-input docs/guide.md --lint-input docs/reference.md
```

## `--from-markdown` seems to ignore content

Symptoms:
- diagram-like lines in markdown are ignored.

Cause:
- `--from-markdown` only reads fenced code blocks tagged as supported diagram fences:
  `puml`, `pumlx`, `picouml`, `plantuml`, `uml`, `puml-sequence`, `uml-sequence`, `mermaid`.
- fences can use backticks or tildes and may be indented by up to three leading spaces.
- if a supported fence is opened and never closed, extraction continues through end-of-file.
- all non-fence markdown content is intentionally ignored.

Fix:

```console
cargo run -- --from-markdown --check your-doc.md
```

If no supported fences are found, `puml` reports:

```text
no supported markdown diagram fences found; expected one of: puml, pumlx, picouml, plantuml, uml, puml-sequence, uml-sequence, mermaid
```

## Snapshot tests fail

Symptoms:
- failing `.snap` assertions in `tests/snapshots/`.

Fix workflow:
- Review whether output change is intentional.
- If intentional, update snapshots with `INSTA_UPDATE=always cargo test`.
- If unintentional, fix code and keep existing snapshots.

Reference: `docs/fixture-snapshot-workflow.md`.
