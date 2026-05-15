# puml-agent-pack

Codex + Claude plugin bundle for deterministic `puml` sequence diagram authoring.

## Included
- `.codex-plugin/plugin.json`
- `.claude-plugin/plugin.json`
- marketplace metadata for both hosts
- `.mcp.json` tool contract
- `bin/puml-mcp` executable MCP-style tool bridge
- author/reviewer skills and agent profiles

## v0.0.1 limitations
- no LSP integration yet (no `puml-lsp` in repo)

## Local smoke test
```bash
printf '{"tool":"puml_check","params":{"text":"@startuml\\nA->B: hi\\n@enduml"}}\n' | agent-pack/bin/puml-mcp
```

## MCP JSON-RPC example
```bash
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' | agent-pack/bin/puml-mcp
```
