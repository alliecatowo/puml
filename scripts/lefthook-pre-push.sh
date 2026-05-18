#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PUSHING_MAIN=0

while IFS=' ' read -r local_ref local_sha remote_ref remote_sha; do
  [[ -z "${remote_ref:-}" ]] && continue
  if [[ "$remote_ref" == "refs/heads/main" ]]; then
    PUSHING_MAIN=1
  fi
done

echo "[hook:pre-push] running quick local gate"
./scripts/local-gate.sh quick

if [[ "$PUSHING_MAIN" -eq 1 ]]; then
  echo "[hook:pre-push] target includes main; running strict local gate"
  ./scripts/local-gate.sh strict
fi

