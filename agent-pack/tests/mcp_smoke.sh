#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

# legacy request compatibility
legacy_resp=$(printf '{"tool":"puml_check","params":{"text":"@startuml\\nA->B: hi\\n@enduml"}}\n' | agent-pack/bin/puml-mcp)
echo "$legacy_resp" | rg '"ok": true' >/dev/null

# MCP initialize
init_resp=$(printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' | agent-pack/bin/puml-mcp)
echo "$init_resp" | rg '"protocolVersion": "2025-06-18"' >/dev/null

# MCP tools/list
tools_resp=$(printf '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' | agent-pack/bin/puml-mcp)
echo "$tools_resp" | rg '"name": "puml_check"' >/dev/null

# MCP tools/call
call_resp=$(printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"puml_check","arguments":{"text":"@startuml\\nA->B: hi\\n@enduml"}}}\n' | agent-pack/bin/puml-mcp)
echo "$call_resp" | rg '"result"' >/dev/null
echo "$call_resp" | rg '"isError": false' >/dev/null
