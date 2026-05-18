#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

THRESHOLD="${PUM_CHANGED_COVERAGE_THRESHOLD:-90}"
BASE_REF="${PUM_COVERAGE_BASE_REF:-origin/main}"
TMP_JSON="${TMPDIR:-/tmp}/puml-changed-coverage.json"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[changed-coverage] missing required command: cargo" >&2
  exit 1
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "[changed-coverage] cargo-llvm-cov is required." >&2
  echo "[changed-coverage] install with: cargo install cargo-llvm-cov" >&2
  exit 1
fi

if git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
  BASE_COMMIT="$(git merge-base HEAD "$BASE_REF")"
else
  BASE_COMMIT="$(git rev-parse HEAD~1)"
fi

CHANGED_RUST_FILES=()
while IFS= read -r path; do
  [[ -z "$path" ]] && continue
  if [[ "$path" == *.rs ]] && [[ -f "$path" ]] && [[ "$path" != target/* ]]; then
    CHANGED_RUST_FILES+=("$path")
  fi
done < <(git diff --name-only "$BASE_COMMIT"...HEAD)

if [[ "${#CHANGED_RUST_FILES[@]}" -eq 0 ]]; then
  echo "[changed-coverage] no changed Rust files; skipping."
  exit 0
fi

echo "[changed-coverage] evaluating coverage for ${#CHANGED_RUST_FILES[@]} changed Rust file(s)"
cargo llvm-cov \
  --all-features \
  --workspace \
  --json \
  --output-path "$TMP_JSON" \
  --ignore-filename-regex 'src/(main|bin/puml-lsp|lib|parser|preproc|normalize|render|specialized)\.rs|src/(frontend|normalize|parser|render|specialized)/.*\.rs'

python3 - "$ROOT_DIR" "$TMP_JSON" "$THRESHOLD" "${CHANGED_RUST_FILES[@]}" <<'PY'
import json
import os
import sys

root = os.path.abspath(sys.argv[1])
report_path = sys.argv[2]
threshold = float(sys.argv[3])
changed = [os.path.normpath(p) for p in sys.argv[4:]]

with open(report_path, "r", encoding="utf-8") as f:
    report = json.load(f)

percent_by_path = {}
for datum in report.get("data", []):
    for fobj in datum.get("files", []):
        fname = os.path.normpath(fobj.get("filename", ""))
        if not fname:
            continue
        rel = os.path.normpath(os.path.relpath(fname, root)) if os.path.isabs(fname) else fname
        lines = fobj.get("summary", {}).get("lines", {})
        percent = lines.get("percent")
        if percent is None:
            continue
        prev = percent_by_path.get(rel)
        if prev is None or percent > prev:
            percent_by_path[rel] = float(percent)

failures = []
for path in changed:
    percent = percent_by_path.get(path)
    if percent is None:
        failures.append((path, None))
        continue
    if percent < threshold:
        failures.append((path, percent))

if failures:
    print(f"[changed-coverage] threshold={threshold:.1f}%")
    for path, percent in failures:
        if percent is None:
            print(f"  - {path}: no coverage data found")
        else:
            print(f"  - {path}: {percent:.2f}%")
    sys.exit(1)

print(f"[changed-coverage] all changed files meet threshold {threshold:.1f}%")
for path in changed:
    percent = percent_by_path.get(path)
    if percent is not None:
        print(f"  - {path}: {percent:.2f}%")
PY

