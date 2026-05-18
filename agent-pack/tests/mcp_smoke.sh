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
  if ! printf '%s' "$haystack" | rg -F -- "$needle" >/dev/null; then
    fail "$context (missing: $needle)"
  fi
}

[[ -x "$MCP_BIN" ]] || fail "missing executable: $MCP_BIN"

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

fake_puml="$tmpdir/puml"
fake_args="$tmpdir/puml.args"
cat >"$fake_puml" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >"$PUML_MCP_FAKE_ARGS"
exit 0
SH
chmod +x "$fake_puml"

echo "[mcp-smoke] configured puml binary is preferred"
configured_resp=$(printf '{"tool":"puml_check","params":{"text":"@startuml\\nA->B: hi\\n@enduml"}}\n' | PUML_MCP_PUML_BIN="$fake_puml" PUML_MCP_FAKE_ARGS="$fake_args" "$MCP_BIN")
expect_contains "$configured_resp" '"ok": true' "configured puml binary puml_check did not succeed"
expect_contains "$(cat "$fake_args")" '--check' "configured puml binary was not invoked with --check"

echo "[mcp-smoke] legacy compatibility: puml_check"
legacy_resp=$(printf '{"tool":"puml_check","params":{"text":"@startuml\\nA->B: hi\\n@enduml"}}\n' | "$MCP_BIN")
expect_contains "$legacy_resp" '"ok": true' "legacy puml_check did not succeed"

echo "[mcp-smoke] initialize contract"
init_resp=$(printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}\n' | "$MCP_BIN")
expect_contains "$init_resp" '"protocolVersion": "2025-06-18"' "initialize protocol version mismatch"
expect_contains "$init_resp" '"name": "puml-mcp"' "initialize server name mismatch"

echo "[mcp-smoke] tools/list discoverability"
tools_resp=$(printf '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' | "$MCP_BIN")
for tool in puml_check puml_diagnostics puml_render_svg puml_render_file; do
  expect_contains "$tools_resp" "\"name\": \"$tool\"" "tools/list missing expected tool"
done

echo "[mcp-smoke] tools/call puml_check"
call_check=$(printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"puml_check","arguments":{"text":"@startuml\\nA->B: hi\\n@enduml"}}}\n' | "$MCP_BIN")
expect_contains "$call_check" '"result"' "tools/call puml_check missing result"
expect_contains "$call_check" '"isError": false' "tools/call puml_check returned isError=true"

echo "[mcp-smoke] tools/call puml_diagnostics structured contract"
call_diag=$(printf '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"puml_diagnostics","arguments":{"text":"@startuml\\n!include https://example.com/lib.puml\\n@enduml"}}}\n' | "$MCP_BIN")
expect_contains "$call_diag" '"isError": true' "tools/call puml_diagnostics should return isError=true for invalid input"
expect_contains "$call_diag" '\"schema\": \"puml.diagnostics\"' "puml_diagnostics missing diagnostics schema"
expect_contains "$call_diag" '\"schema_version\": 1' "puml_diagnostics missing schema version"
expect_contains "$call_diag" 'E_INCLUDE_URL_DISABLED' "puml_diagnostics missing structured include diagnostic"

echo "[mcp-smoke] tools/call puml_render_svg"
call_svg=$(printf '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"puml_render_svg","arguments":{"text":"@startuml\\nA->B: hi\\n@enduml"}}}\n' | "$MCP_BIN")
expect_contains "$call_svg" '"isError": false' "tools/call puml_render_svg returned isError=true"
expect_contains "$call_svg" '<svg' "tools/call puml_render_svg missing svg payload"

echo "[mcp-smoke] URL includes disabled by default"
call_remote=$(printf '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"puml_check","arguments":{"text":"@startuml\\n!include https://example.com/lib.puml\\n@enduml"}}}\n' | "$MCP_BIN")
expect_contains "$call_remote" '"isError": true' "remote include should fail by default"
expect_contains "$call_remote" 'E_INCLUDE_URL_DISABLED' "remote include did not use disabled diagnostic"

echo "[mcp-smoke] complete"
