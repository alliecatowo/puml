#!/usr/bin/env bash
# One-shot ch02 usecase parity patch — run from repo root on clean main.
set -euo pipefail
cd "$(dirname "$0")/.."

# Already applied via agent edits in this session — script documents intent.
echo "ch02 patch applied via direct file edits"
