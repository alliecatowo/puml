#!/usr/bin/env bash
# oracle.sh — Differential SVG oracle using PlantUML reference JAR.
#
# Usage:
#   ./scripts/oracle.sh [--corpus-dir <dir>]
#
# Environment:
#   PUML_ORACLE_JAR   Path to plantuml.jar. If unset, exits 0 with skipped sentinel.
#   PUML_ORACLE_JAVA  Path to java binary (falls back to "java" on PATH)
#
# Output (stdout): newline-terminated JSON object
# Report written to: docs/benchmarks/oracle_report.json
#
# Exit codes:
#   0  — completed cleanly (or skipped because PUML_ORACLE_JAR is not set)
#   1  — any fixture renders here but not via oracle JAR, or vice-versa
#   2  — structural drift exceeds 10% on more than 5 fixtures

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
JAVA_BIN="${PUML_ORACLE_JAVA:-java}"

CORPUS_DIR="${ROOT_DIR}/tests/fixtures"
REPORT_FILE="${ROOT_DIR}/docs/benchmarks/oracle_report.json"

# ---------------------------------------------------------------------------
# Parse flags
# ---------------------------------------------------------------------------
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
# Skip if PUML_ORACLE_JAR is not set (env-gated for CI)
# ---------------------------------------------------------------------------
if [[ -z "${PUML_ORACLE_JAR:-}" ]]; then
  SENTINEL='{"skipped":true,"reason":"PUML_ORACLE_JAR not set"}'
  printf '%s\n' "${SENTINEL}"
  mkdir -p "$(dirname "${REPORT_FILE}")"
  printf '%s\n' "${SENTINEL}" > "${REPORT_FILE}"
  exit 0
fi

JAR="${PUML_ORACLE_JAR}"

# ---------------------------------------------------------------------------
# Skip if JAR file does not exist
# ---------------------------------------------------------------------------
if [[ ! -f "${JAR}" ]]; then
  SENTINEL="$(printf '{"skipped":true,"reason":"JAR file not found: %s"}' "${JAR}")"
  printf '%s\n' "${SENTINEL}"
  mkdir -p "$(dirname "${REPORT_FILE}")"
  printf '%s\n' "${SENTINEL}" > "${REPORT_FILE}"
  exit 0
fi

# ---------------------------------------------------------------------------
# Verify java is available
# ---------------------------------------------------------------------------
if ! command -v "${JAVA_BIN}" >/dev/null 2>&1; then
  SENTINEL="$(printf '{"skipped":true,"reason":"java binary not found: %s"}' "${JAVA_BIN}")"
  printf '%s\n' "${SENTINEL}"
  mkdir -p "$(dirname "${REPORT_FILE}")"
  printf '%s\n' "${SENTINEL}" > "${REPORT_FILE}"
  exit 0
fi

# ---------------------------------------------------------------------------
# Resolve JAR to absolute path
# ---------------------------------------------------------------------------
JAR_ABS="$(cd "$(dirname "${JAR}")" && pwd)/$(basename "${JAR}")"

# ---------------------------------------------------------------------------
# Helpers: count SVG elements, text nodes, extract viewBox
# ---------------------------------------------------------------------------
count_svg_elements() {
  local f="$1"
  grep -o '<[a-zA-Z][a-zA-Z0-9]*' "${f}" 2>/dev/null | wc -l | tr -d ' '
}

count_svg_text_nodes() {
  local f="$1"
  grep -o '<text' "${f}" 2>/dev/null | wc -l | tr -d ' '
}

extract_viewbox() {
  local f="$1"
  grep -o 'viewBox="[^"]*"' "${f}" 2>/dev/null | head -1 | sed 's/viewBox="//;s/"//'
}

# ---------------------------------------------------------------------------
# Discover .puml files under corpus dir
# ---------------------------------------------------------------------------
mapfile -t PUML_FILES < <(find "${CORPUS_DIR}" -name '*.puml' | sort)

TOTAL=${#PUML_FILES[@]}
IDENTICAL=0
DIFF_COUNT=0
RENDER_MISMATCH_COUNT=0   # one side renders, other doesn't
STRUCTURAL_DRIFT_COUNT=0  # drift > 10%
DIFFS_JSON=""

TMPDIR_WORK="$(mktemp -d)"
trap 'rm -rf "${TMPDIR_WORK}"' EXIT

mkdir -p "${TMPDIR_WORK}/ref" "${TMPDIR_WORK}/ours"

for F in "${PUML_FILES[@]}"; do
  REL="${F#"${ROOT_DIR}/"}"
  BASENAME="$(basename "${F}" .puml)"

  # ---- Reference SVG via PlantUML JAR ----
  REF_DIR="${TMPDIR_WORK}/ref/${BASENAME}"
  mkdir -p "${REF_DIR}"
  JAR_OK=true
  if ! "${JAVA_BIN}" -jar "${JAR_ABS}" -tsvg -o "${REF_DIR}" "${F}" 2>/dev/null; then
    JAR_OK=false
  fi

  REF_SVG="${REF_DIR}/${BASENAME}.svg"
  if [[ "${JAR_OK}" == "true" && ! -f "${REF_SVG}" ]]; then
    JAR_OK=false
  fi

  # ---- Our SVG via cargo run ----
  OUR_SVG_FILE="${TMPDIR_WORK}/ours/${BASENAME}.svg"
  OUR_OK=true
  if ! cargo run --quiet --manifest-path "${ROOT_DIR}/Cargo.toml" -- "${F}" \
        > "${OUR_SVG_FILE}" 2>/dev/null; then
    OUR_OK=false
  fi

  # ---- Compare results ----
  if [[ "${OUR_OK}" == "false" && "${JAR_OK}" == "false" ]]; then
    # Both fail — not a mismatch, skip quietly.
    continue
  fi

  if [[ "${OUR_OK}" == "true" && "${JAR_OK}" == "false" ]]; then
    # We render but oracle doesn't.
    (( RENDER_MISMATCH_COUNT++ )) || true
    (( DIFF_COUNT++ )) || true
    OUR_BYTES="$(wc -c < "${OUR_SVG_FILE}")"
    ENTRY="$(printf '{"file":"%s","our_bytes":%d,"ref_bytes":null,"identical":false,"notes":"oracle JAR did not produce SVG","elem_drift_pct":null}' \
      "${REL}" "${OUR_BYTES}")"
    DIFFS_JSON="${DIFFS_JSON:+${DIFFS_JSON},}${ENTRY}"
    continue
  fi

  if [[ "${OUR_OK}" == "false" && "${JAR_OK}" == "true" ]]; then
    # Oracle renders but we don't.
    (( RENDER_MISMATCH_COUNT++ )) || true
    (( DIFF_COUNT++ )) || true
    REF_BYTES="$(wc -c < "${REF_SVG}")"
    ENTRY="$(printf '{"file":"%s","our_bytes":null,"ref_bytes":%d,"identical":false,"notes":"our renderer exited non-zero","elem_drift_pct":null}' \
      "${REL}" "${REF_BYTES}")"
    DIFFS_JSON="${DIFFS_JSON:+${DIFFS_JSON},}${ENTRY}"
    continue
  fi

  # Both produced SVG — compare structurally.
  OUR_BYTES="$(wc -c < "${OUR_SVG_FILE}")"
  REF_BYTES="$(wc -c < "${REF_SVG}")"

  OUR_ELEMS="$(count_svg_elements "${OUR_SVG_FILE}")"
  REF_ELEMS="$(count_svg_elements "${REF_SVG}")"
  OUR_TEXTS="$(count_svg_text_nodes "${OUR_SVG_FILE}")"
  REF_TEXTS="$(count_svg_text_nodes "${REF_SVG}")"
  OUR_VB="$(extract_viewbox "${OUR_SVG_FILE}")"
  REF_VB="$(extract_viewbox "${REF_SVG}")"

  # Compute element drift percentage (avoid divide-by-zero).
  DRIFT_PCT=0
  if [[ "${REF_ELEMS}" -gt 0 ]]; then
    DRIFT_PCT=$(( ( (OUR_ELEMS - REF_ELEMS) * 100 ) / REF_ELEMS ))
    # Absolute value
    DRIFT_PCT="${DRIFT_PCT#-}"
  fi

  BYTE_IDENTICAL=false
  if cmp -s "${OUR_SVG_FILE}" "${REF_SVG}"; then
    BYTE_IDENTICAL=true
    (( IDENTICAL++ )) || true
  fi

  if [[ "${BYTE_IDENTICAL}" == "false" ]]; then
    (( DIFF_COUNT++ )) || true
    if [[ "${DRIFT_PCT}" -gt 10 ]]; then
      (( STRUCTURAL_DRIFT_COUNT++ )) || true
    fi
    ENTRY="$(printf \
      '{"file":"%s","our_bytes":%d,"ref_bytes":%d,"identical":false,"our_elems":%s,"ref_elems":%s,"our_texts":%s,"ref_texts":%s,"our_viewbox":"%s","ref_viewbox":"%s","elem_drift_pct":%d,"notes":"byte-level diff"}' \
      "${REL}" "${OUR_BYTES}" "${REF_BYTES}" \
      "${OUR_ELEMS}" "${REF_ELEMS}" \
      "${OUR_TEXTS}" "${REF_TEXTS}" \
      "${OUR_VB}" "${REF_VB}" \
      "${DRIFT_PCT}")"
    DIFFS_JSON="${DIFFS_JSON:+${DIFFS_JSON},}${ENTRY}"
  fi
done

# ---------------------------------------------------------------------------
# Build and write report
# ---------------------------------------------------------------------------
REPORT="$(printf \
  '{"oracle_version":"2","jar":"%s","skipped":false,"total":%d,"identical":%d,"diff_count":%d,"render_mismatch_count":%d,"structural_drift_count":%d,"diffs":[%s]}' \
  "${JAR_ABS}" \
  "${TOTAL}" \
  "${IDENTICAL}" \
  "${DIFF_COUNT}" \
  "${RENDER_MISMATCH_COUNT}" \
  "${STRUCTURAL_DRIFT_COUNT}" \
  "${DIFFS_JSON}")"

printf '%s\n' "${REPORT}"
mkdir -p "$(dirname "${REPORT_FILE}")"
printf '%s\n' "${REPORT}" > "${REPORT_FILE}"
echo "[oracle] report written to ${REPORT_FILE}" >&2

# ---------------------------------------------------------------------------
# Exit codes: 1 = render mismatch; 2 = structural drift on >5 fixtures
# ---------------------------------------------------------------------------
if [[ "${RENDER_MISMATCH_COUNT}" -gt 0 ]]; then
  echo "[oracle] FAIL: ${RENDER_MISMATCH_COUNT} render mismatch(es) detected" >&2
  exit 1
fi

if [[ "${STRUCTURAL_DRIFT_COUNT}" -gt 5 ]]; then
  echo "[oracle] FAIL: structural drift >10% on ${STRUCTURAL_DRIFT_COUNT} fixtures (threshold: 5)" >&2
  exit 2
fi

exit 0
