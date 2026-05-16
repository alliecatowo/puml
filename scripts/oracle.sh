#!/usr/bin/env bash
# oracle.sh — Differential SVG oracle using PlantUML reference JAR.
#
# Usage:
#   ./scripts/oracle.sh [--corpus-dir <dir>]
#
# Environment:
#   PUML_ORACLE_JAR   Path to plantuml.jar (falls back to ./oracle/plantuml.jar)
#   PUML_ORACLE_JAVA  Path to java binary (falls back to "java" on PATH)
#
# Output (stdout): newline-terminated JSON object:
#   {
#     "oracle_version": "1",
#     "jar": "<path>",
#     "skipped": false,
#     "total": N,
#     "identical": K,
#     "diff_count": D,
#     "diffs": [ { "file": "...", "our_bytes": N, "ref_bytes": M, "identical": false } ]
#   }
#
# Exit codes:
#   0  — completed (diffs may be present — caller decides if that's a failure)
#   0  — JAR absent → skipped JSON emitted, nothing ran
#   1  — unexpected internal error (bad invocation, etc.)
#
# CI contract: this script NEVER exits non-zero due to diff counts.
# The caller (parity_harness.py --oracle) decides severity.

set -euo pipefail

# ---------------------------------------------------------------------------
# Resolve JAR path
# ---------------------------------------------------------------------------
JAR="${PUML_ORACLE_JAR:-./oracle/plantuml.jar}"
JAVA_BIN="${PUML_ORACLE_JAVA:-java}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

CORPUS_DIR="${ROOT_DIR}/docs/examples"

# Parse flags
while [[ $# -gt 0 ]]; do
  case "$1" in
    --corpus-dir)
      CORPUS_DIR="$2"
      shift 2
      ;;
    --corpus-dir=*)
      CORPUS_DIR="${1#*=}"
      shift
      ;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "[oracle] unknown flag: $1" >&2
      exit 1
      ;;
  esac
done

# ---------------------------------------------------------------------------
# Skip if JAR absent
# ---------------------------------------------------------------------------
if [[ ! -f "${JAR}" ]]; then
  printf '{"oracle_version":"1","jar":"%s","skipped":true,"skip_reason":"JAR not found — set PUML_ORACLE_JAR or place plantuml.jar at ./oracle/plantuml.jar","total":0,"identical":0,"diff_count":0,"diffs":[]}\n' \
    "${JAR}"
  exit 0
fi

# ---------------------------------------------------------------------------
# Verify java is available
# ---------------------------------------------------------------------------
if ! command -v "${JAVA_BIN}" >/dev/null 2>&1; then
  printf '{"oracle_version":"1","jar":"%s","skipped":true,"skip_reason":"java binary not found on PATH","total":0,"identical":0,"diff_count":0,"diffs":[]}\n' \
    "${JAR}"
  exit 0
fi

# ---------------------------------------------------------------------------
# Resolve JAR to absolute path
# ---------------------------------------------------------------------------
JAR_ABS="$(cd "$(dirname "${JAR}")" && pwd)/$(basename "${JAR}")"

# ---------------------------------------------------------------------------
# Discover .puml files under corpus dir
# ---------------------------------------------------------------------------
mapfile -t PUML_FILES < <(find "${CORPUS_DIR}" -name '*.puml' | sort)

TOTAL=${#PUML_FILES[@]}
IDENTICAL=0
DIFF_COUNT=0
DIFFS_JSON=""

TMPDIR_WORK="$(mktemp -d)"
trap 'rm -rf "${TMPDIR_WORK}"' EXIT

for F in "${PUML_FILES[@]}"; do
  REL="${F#"${ROOT_DIR}/"}"

  # --- Reference SVG via PlantUML JAR ---
  REF_DIR="${TMPDIR_WORK}/ref"
  mkdir -p "${REF_DIR}"
  if ! "${JAVA_BIN}" -jar "${JAR_ABS}" -tsvg -o "${REF_DIR}" "${F}" 2>/dev/null; then
    # JAR failed on this file — record as diff, move on
    ENTRY="$(printf '{"file":"%s","our_bytes":null,"ref_bytes":null,"identical":false,"notes":"plantuml JAR exited non-zero"}' "${REL}")"
    DIFFS_JSON="${DIFFS_JSON:+${DIFFS_JSON},}${ENTRY}"
    (( DIFF_COUNT++ )) || true
    continue
  fi

  # PlantUML writes <basename>.svg next to the source file in -o dir; resolve name.
  BASENAME="$(basename "${F}" .puml)"
  REF_SVG="${REF_DIR}/${BASENAME}.svg"
  if [[ ! -f "${REF_SVG}" ]]; then
    ENTRY="$(printf '{"file":"%s","our_bytes":null,"ref_bytes":null,"identical":false,"notes":"reference SVG not produced by JAR"}' "${REL}")"
    DIFFS_JSON="${DIFFS_JSON:+${DIFFS_JSON},}${ENTRY}"
    (( DIFF_COUNT++ )) || true
    continue
  fi

  # --- Our SVG via cargo run ---
  OUR_SVG_FILE="${TMPDIR_WORK}/ours/${BASENAME}.svg"
  mkdir -p "${TMPDIR_WORK}/ours"
  if ! cargo run --quiet --manifest-path "${ROOT_DIR}/Cargo.toml" -- "${F}" \
        > "${OUR_SVG_FILE}" 2>/dev/null; then
    ENTRY="$(printf '{"file":"%s","our_bytes":null,"ref_bytes":%d,"identical":false,"notes":"our renderer exited non-zero"}' \
      "${REL}" "$(wc -c < "${REF_SVG}")")"
    DIFFS_JSON="${DIFFS_JSON:+${DIFFS_JSON},}${ENTRY}"
    (( DIFF_COUNT++ )) || true
    continue
  fi

  OUR_BYTES="$(wc -c < "${OUR_SVG_FILE}")"
  REF_BYTES="$(wc -c < "${REF_SVG}")"

  if cmp -s "${OUR_SVG_FILE}" "${REF_SVG}"; then
    (( IDENTICAL++ )) || true
  else
    ENTRY="$(printf '{"file":"%s","our_bytes":%d,"ref_bytes":%d,"identical":false,"notes":"byte-level diff"}' \
      "${REL}" "${OUR_BYTES}" "${REF_BYTES}")"
    DIFFS_JSON="${DIFFS_JSON:+${DIFFS_JSON},}${ENTRY}"
    (( DIFF_COUNT++ )) || true
  fi
done

# ---------------------------------------------------------------------------
# Emit JSON result
# ---------------------------------------------------------------------------
printf '{"oracle_version":"1","jar":"%s","skipped":false,"total":%d,"identical":%d,"diff_count":%d,"diffs":[%s]}\n' \
  "${JAR_ABS}" \
  "${TOTAL}" \
  "${IDENTICAL}" \
  "${DIFF_COUNT}" \
  "${DIFFS_JSON}"
