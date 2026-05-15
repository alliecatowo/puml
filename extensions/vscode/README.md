# puml-vscode

VS Code scaffolding for `puml` that stays aligned with the current CLI/LSP contracts.

## Current scope (Issue #22 slice)

- Registers `puml` language ID and extensions: `.puml`, `.plantuml`, `.iuml`.
- Ships starter TextMate grammar, language config, and snippets.
- Starts `puml-lsp` as a thin language-client shell.
- Adds **PUML: Open Preview** command wired to LSP `workspace/executeCommand` with `puml.renderSvg`.
- Adds **PUML: Restart Language Server** command.
- Shows SVG output and diagnostics returned by LSP (no private parser/renderer in the extension host or webview).

## Settings

- `puml.lsp.enabled` (`true`): auto-start language client on activation.
- `puml.lsp.path` (`""`): optional absolute path to `puml-lsp`. Empty resolution order: bundled path (`bin/puml-lsp` or `bin/puml-lsp.exe`) if present, otherwise `puml-lsp` from `PATH`.
- `puml.lsp.trace` (`off|messages|verbose`): language client trace verbosity.

## Smoke checks

From repository root:

```bash
./scripts/vscode-smoke.sh
```

This verifies:
- CLI capability manifest still advertises `puml.applyFormat` and `puml.renderSvg`.
- Extension TypeScript build succeeds.
- Preview implementation remains LSP-backed and does not reintroduce private parsing.

## Planned next steps

- Wire command coverage tests via `@vscode/test-electron`.
- Add markdown fence preview integration.
- Add export command routes (SVG/JSON first; additional formats as capabilities land).
