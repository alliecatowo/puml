#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

MCP_BIN="agent-pack/bin/puml-mcp"

fail() {
  echo "[mcp-smoke:fail] $1" >&2
  exit 1
}

expect_contains() {
  local haystack="$1"
  local needle="$2"
  local context="$3"
  if ! printf '%s' "$haystack" | rg -F "$needle" >/dev/null; then
    fail "$context (missing: $needle)"
  fi
}

[[ -x "$MCP_BIN" ]] || fail "missing executable: $MCP_BIN"

echo "[mcp-smoke] legacy compatibility: puml_check"
legacy_resp=$(printf '{"tool":"puml_check","params":{"text":"@startuml\\nA->B: hi\\n@enduml"}}\n' | "$MCP_BIN")
expect_contains "$legacy_resp" '"ok": true' "legacy puml_check did not succeed"

echo "[mcp-smoke] initialize contract"
init_resp=$(printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' | "$MCP_BIN")
expect_contains "$init_resp" '"protocolVersion": "2025-06-18"' "initialize protocol version mismatch"
expect_contains "$init_resp" '"name": "puml-mcp"' "initialize server name mismatch"

echo "[mcp-smoke] tools/list discoverability"
tools_resp=$(printf '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' | "$MCP_BIN")
for tool in puml_check puml_render_svg puml_render_file; do
  expect_contains "$tools_resp" "\"name\": \"$tool\"" "tools/list missing expected tool"
done

echo "[mcp-smoke] tools/call puml_check"
call_check=$(printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"puml_check","arguments":{"text":"@startuml\\nA->B: hi\\n@enduml"}}}\n' | "$MCP_BIN")
expect_contains "$call_check" '"result"' "tools/call puml_check missing result"
expect_contains "$call_check" '"isError": false' "tools/call puml_check returned isError=true"

echo "[mcp-smoke] tools/call puml_render_svg"
call_svg=$(printf '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"puml_render_svg","arguments":{"text":"@startuml\\nA->B: hi\\n@enduml"}}}\n' | "$MCP_BIN")
expect_contains "$call_svg" '"isError": false' "tools/call puml_render_svg returned isError=true"
expect_contains "$call_svg" '<svg' "tools/call puml_render_svg missing svg payload"

echo "[mcp-smoke] complete"
