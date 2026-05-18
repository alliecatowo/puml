# puml-agent-pack

Codex + Claude plugin bundle for deterministic `puml` sequence diagram authoring.

## Included
- `.codex-plugin/plugin.json`
- `.claude-plugin/plugin.json`
- marketplace metadata for both hosts
- `.mcp.json` tool contract
- `.lsp.json` LSP contract
- `bin/puml-mcp` executable MCP-style tool bridge
- author/reviewer skills and agent profiles

## v0.0.1 limitations
- `.lsp.json` declares the expected LSP command and capabilities, but release archives
  may still depend on the host to provide or map `bin/puml-lsp`.

## Runtime resolution
`agent-pack/bin/puml-mcp` resolves the compiler in this order:

1. `PUML_MCP_PUML_BIN` or `PUML_BIN`
2. bundled `agent-pack/bin/puml`
3. `puml` on `PATH`
4. `cargo run --quiet --` only inside a source checkout for local development

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

## Codex/Claude Harness Runbook
```bash
./scripts/harness-check.sh --dry
./scripts/harness-check.sh --quick
./scripts/harness-check.sh
```

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
