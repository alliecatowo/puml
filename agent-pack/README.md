# puml-agent-pack

Codex + Claude Code plugin bundle for deterministic `puml` diagram authoring across all diagram families.

## Included
- `.codex-plugin/plugin.json` — Codex plugin manifest
- `.claude-plugin/plugin.json` — Claude Code plugin manifest (v0.1.0)
- marketplace metadata for both hosts
- `.mcp.json` tool contract (5 tools including `puml_render_png`)
- `.lsp.json` LSP contract (full capability list)
- `bin/puml-mcp` — MCP server (Python, JSON-RPC 2.0 + legacy)
- `bin/puml-lsp` — LSP server wrapper (shell script, resolves compiled binary)
- Skills: `puml-sequence-author`, `puml-sequence-reviewer`, `puml-class-author`, `puml-writing-guide`
- Agents: `puml-diagram-designer`, `puml-diagram-reviewer`

## Runtime resolution
`agent-pack/bin/puml-mcp` resolves the compiler in this order:

1. `PUML_MCP_PUML_BIN` or `PUML_BIN`
2. bundled `agent-pack/bin/puml`
3. `puml` on `PATH`
4. `cargo run --quiet --` only inside a source checkout for local development

`agent-pack/bin/puml-lsp` resolves the LSP server in this order:

1. `PUML_LSP_BIN` env var
2. `target/release/puml-lsp` (pre-built release binary)
3. `target/debug/puml-lsp` (pre-built debug binary)
4. `cargo run --bin puml-lsp --` only inside a source checkout

## Local smoke test
```bash
printf '{"tool":"puml_check","params":{"text":"@startuml\\nA->B: hi\\n@enduml"}}\n' | agent-pack/bin/puml-mcp
```

URL includes are disabled by default in MCP tool calls. Pass
`"allow_url_includes": true` only when remote fetching is intentional.

## MCP JSON-RPC example
```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' | agent-pack/bin/puml-mcp
```

Structured diagnostics are available through `tools/call` with
`name: "puml_diagnostics"`. The text content is JSON shaped as
`schema: "puml.diagnostics"` with `schema_version: 1`.

### PNG rendering
`puml_render_png` returns a `png_base64` field with the base64-encoded PNG.
Pass `output_path` to also save the file to a workspace path.

## Codex/Claude Harness Runbook
```bash
python3 ./scripts/validate_agent_pack.py
./scripts/harness-check.sh --dry
./scripts/harness-check.sh --quick
./scripts/harness-check.sh
```

`validate_agent_pack.py` checks plugin manifests, marketplace metadata, MCP
runtime/spec parity, and the `.lsp.json` language-server manifest before a
package is shipped.

## Agent pre-PR checklist

Before opening a PR, **always** run:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --quiet
```

Checklist:

- [ ] `cargo fmt` — no formatting violations (CI rejects unformatted code; see issues #253)
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings
- [ ] `cargo test --quiet` — all tests pass
- [ ] PR body references the relevant issue (`Closes #NNN`)

Agents working in a local checkout can optionally install lefthook hooks that
enforce these checks automatically on `git commit` and `git push`:

```bash
./scripts/install-hooks.sh
```

See [CONTRIBUTING.md](../CONTRIBUTING.md) for full details.
