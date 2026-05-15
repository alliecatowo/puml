#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
resp=$(printf '{"tool":"puml_check","params":{"text":"@startuml\\nA->B: hi\\n@enduml"}}\n' | agent-pack/bin/puml-mcp)
echo "$resp" | rg '"ok": true' >/dev/null
