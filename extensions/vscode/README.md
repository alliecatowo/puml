# puml-vscode

VS Code extension for `puml` — live preview, export, inline diagnostics, and LSP-backed
language features for `.puml` / `.plantuml` / `.iuml` files.

## Commands

| Command | Title | Description |
|---|---|---|
| `puml.preview.open` | **PUML: Open Live Preview** | Opens a side-by-side webview panel that renders the active `.puml` file to SVG. Updates automatically on keystrokes (500 ms debounce) and immediately on save. Stale responses are silently discarded. |
| `puml.export.svg` | **PUML: Export as SVG** | Prompts for a save path and exports the diagram to an SVG file via the `puml` CLI. |
| `puml.export.png` | **PUML: Export as PNG** | Prompts for a save path and exports the diagram to a PNG file via the `puml` CLI. |
| `puml.check` | **PUML: Check / Run Diagnostics** | Renders the file, surfaces parser errors and warnings as VS Code diagnostics in the Problems panel, and updates the status bar. |
| `puml.lsp.restart` | **PUML: Restart Language Server** | Stops and restarts `puml-lsp`. |

Commands are also available from the **editor title bar icon** and **right-click context menu** when a `.puml` file is open.

## Live Preview

The preview panel:
- Opens beside the editor (Column 2) and **persists** across editor switches.
- **Debounces** text changes by 500 ms (configurable via `puml.preview.debounceMs`).
- **Immediately refreshes** on document save, cancelling any pending debounce.
- Uses a **monotonic sequence counter** to drop stale render responses — slow renders
  from previous keystrokes never overwrite a more recent result.
- Shows a loading indicator while rendering.
- Displays the diagram family badge (when the renderer reports it) and a diagnostic summary strip at the bottom.
- Follows VS Code theme colours.

## Rendering strategy

Rendering uses **LSP-first, CLI-fallback**:

1. If `puml-lsp` is running, the `puml.renderSvg` workspace command is used (low latency, LSP-managed process).
2. If the LSP is not running or returns an error, the `puml` CLI binary is invoked as a subprocess (writes current document text to a temp file, reads SVG from stdout).

Export commands (`puml.export.svg`, `puml.export.png`) always use the CLI path so they
produce real output files rather than in-memory SVG strings.

## Settings

| Key | Default | Description |
|---|---|---|
| `puml.lsp.enabled` | `true` | Auto-start `puml-lsp` on activation. |
| `puml.lsp.path` | `""` | Absolute path to `puml-lsp`. Empty: tries `bin/puml-lsp` bundled in the extension, then `puml-lsp` from PATH. |
| `puml.lsp.trace` | `"off"` | Language client trace level: `off`, `messages`, `verbose`. |
| `puml.cli.path` | `""` | Absolute path to the `puml` CLI binary. Empty: uses `puml` (or `puml.exe`) from PATH. |
| `puml.preview.debounceMs` | `500` | Debounce delay (ms) for live preview updates on keystroke. |

## Status bar

When a `.puml` file is active, a status bar item appears on the right showing:

- `$(graph) [family]` — the diagram family detected by the renderer.
- `$(check)` — no diagnostics.
- `$(error) N` — N errors / warnings from the last render or check.

Clicking the status bar item opens the live preview panel.

## Smoke checks

```bash
cd extensions/vscode
npm run smoke
```

## Tests

```bash
cd extensions/vscode
npm test
```

Runs 35 fast Node.js unit tests covering: build artifact presence, source contracts,
live-preview wiring, renderer module, export commands, check command, status bar,
and `package.json` declarations. No VS Code process required.

Full `@vscode/test-electron` integration tests (activation, preview panel, restart)
are tracked in issue #401.

## Planned next steps

- Wire `@vscode/test-electron` integration tests for activation, preview, and restart (issue #401).
- Add markdown fence preview integration.
- PDF export once #444 lands.
