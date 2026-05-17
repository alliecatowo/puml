#!/usr/bin/env bash
# oracle.sh — Differential SVG conformance suite against the Java PlantUML reference JAR.
#
# Usage:
#   ./scripts/oracle.sh [--corpus-dir <dir>] [--examples-dir <dir>] [--report-file <path>]
#
# Environment:
#   PUML_ORACLE_JAR   Absolute path to plantuml.jar.
#                     If unset or empty, exits 0 with a skip-sentinel JSON (CI-safe).
#   PUML_ORACLE_JAVA  Path to java binary (falls back to "java" on PATH).
#
# Output (stdout): newline-terminated JSON object (sentinel or full report).
# Report written to: docs/benchmarks/oracle_report.json unless --report-file is set.
#
# Metrics per fixture (when both sides render):
#   elem_count  — count of <rect>, <text>, <line>, <polygon>, <circle>, <path> tags
#   viewbox     — "W H" extracted from viewBox attribute
#   text_set    — sorted unique text-node strings (joined, comma-separated)
#   color_set   — sorted unique fill/stroke hex codes (#[0-9a-fA-F]{3,6})
#
# Categorization (per fixture):
#   match      — all metrics within 10% of each other (or identical text/color sets)
#   drift      — any metric deviates >10%
#   puml-only  — our renderer produces SVG; JAR fails or produces nothing
#   jar-only   — JAR produces SVG; our renderer fails
#   both-fail  — both sides fail to produce SVG
#
# Exit codes:
#   0  — PUML_ORACLE_JAR unset (skip) OR match% >= 80%
#   1  — 50% <= match% < 80%
#   2  — match% < 50%

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
JAVA_BIN="${PUML_ORACLE_JAVA:-java}"

CORPUS_DIR="${ROOT_DIR}/tests/fixtures"
EXAMPLES_DIR="${ROOT_DIR}/docs/examples"
REPORT_FILE="${ROOT_DIR}/docs/benchmarks/oracle_report.json"
BINARY="${ROOT_DIR}/target/release/puml"

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
    --examples-dir)
      EXAMPLES_DIR="$2"
      shift 2
      ;;
    --examples-dir=*)
      EXAMPLES_DIR="${1#*=}"
      shift
      ;;
    --report-file)
      REPORT_FILE="$2"
      shift 2
      ;;
    --report-file=*)
      REPORT_FILE="${1#*=}"
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
# Skip sentinel: PUML_ORACLE_JAR not set
# ---------------------------------------------------------------------------
if [[ -z "${PUML_ORACLE_JAR:-}" ]]; then
  SENTINEL='{"schema_version":"1.0","skipped":true,"reason":"PUML_ORACLE_JAR not set","comparison_only":true,"runtime_dependency":false,"build_dependency":false,"java_attempted":false}'
  printf '%s\n' "${SENTINEL}"
  mkdir -p "$(dirname "${REPORT_FILE}")"
  printf '%s\n' "${SENTINEL}" > "${REPORT_FILE}"
  exit 0
fi

JAR="${PUML_ORACLE_JAR}"

# ---------------------------------------------------------------------------
# Skip sentinel: JAR file not found
# ---------------------------------------------------------------------------
if [[ ! -f "${JAR}" ]]; then
  SENTINEL="$(printf '{"schema_version":"1.0","skipped":true,"reason":"JAR file not found: %s","comparison_only":true,"runtime_dependency":false,"build_dependency":false,"java_attempted":false}' "${JAR}")"
  printf '%s\n' "${SENTINEL}"
  mkdir -p "$(dirname "${REPORT_FILE}")"
  printf '%s\n' "${SENTINEL}" > "${REPORT_FILE}"
  exit 0
fi

# ---------------------------------------------------------------------------
# Skip sentinel: java not available
# ---------------------------------------------------------------------------
if ! command -v "${JAVA_BIN}" >/dev/null 2>&1; then
  SENTINEL="$(printf '{"schema_version":"1.0","skipped":true,"reason":"java binary not found: %s","comparison_only":true,"runtime_dependency":false,"build_dependency":false,"java_attempted":false}' "${JAVA_BIN}")"
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
# Build our binary (release) if not present
# ---------------------------------------------------------------------------
if [[ ! -x "${BINARY}" ]]; then
  echo "[oracle] building release binary…" >&2
  cargo build --release --quiet --manifest-path "${ROOT_DIR}/Cargo.toml" >&2
fi

# ---------------------------------------------------------------------------
# Capture JAR version string
# ---------------------------------------------------------------------------
JAR_VERSION="$("${JAVA_BIN}" -jar "${JAR_ABS}" -version 2>&1 | head -1 || true)"
JAR_VERSION="${JAR_VERSION//\"/\'}"   # escape double-quotes for JSON

# ---------------------------------------------------------------------------
# Metric helpers
# ---------------------------------------------------------------------------

# Count structural SVG elements: rect, text, line, polygon, circle, path
count_structural_elements() {
  local f="$1"
  grep -oiE '<(rect|text|line|polygon|circle|path)([[:space:]]|/>|>)' "${f}" 2>/dev/null \
    | wc -l | tr -d ' '
}

# Extract "W H" from viewBox (first occurrence); returns "" if absent
extract_viewbox_dims() {
  local f="$1"
  local vb
  vb="$(grep -o 'viewBox="[^"]*"' "${f}" 2>/dev/null | head -1 | sed 's/viewBox="//;s/"//' || true)"
  # viewBox="x y w h" → return "w h"
  if [[ -n "${vb}" ]]; then
    read -r _x _y w h <<< "${vb}" || true
    printf '%s %s' "${w:-0}" "${h:-0}"
  else
    printf '0 0'
  fi
}

# Extract sorted, unique text-node content (very rough: strip tags, squish whitespace)
extract_text_set() {
  local f="$1"
  grep -o '<text[^>]*>[^<]*</text>' "${f}" 2>/dev/null \
    | sed 's/<[^>]*>//g' \
    | tr '[:space:]' ' ' \
    | tr -s ' ' \
    | tr ',' '\n' \
    | sort -u \
    | tr '\n' ',' \
    | sed 's/,$//'
}

# Extract sorted unique fill/stroke hex colours
extract_color_set() {
  local f="$1"
  grep -oiE '(fill|stroke)="#[0-9a-fA-F]{3,8}"' "${f}" 2>/dev/null \
    | grep -oiE '#[0-9a-fA-F]{3,8}' \
    | tr '[:upper:]' '[:lower:]' \
    | sort -u \
    | tr '\n' ',' \
    | sed 's/,$//'
}

# Compute absolute percentage drift between two integers (avoids div-by-zero)
pct_drift() {
  local a="$1" b="$2"
  if [[ "${b}" -eq 0 && "${a}" -eq 0 ]]; then
    echo 0; return
  fi
  if [[ "${b}" -eq 0 ]]; then
    echo 100; return
  fi
  local diff=$(( a - b ))
  [[ "${diff}" -lt 0 ]] && diff=$(( -diff ))
  echo $(( (diff * 100) / b ))
}

# ---------------------------------------------------------------------------
# Discover .puml files across both corpus dirs
# ---------------------------------------------------------------------------
mapfile -t PUML_FILES < <(
  {
    [[ -d "${CORPUS_DIR}" ]]  && find "${CORPUS_DIR}"  -name '*.puml' -type f
    [[ -d "${EXAMPLES_DIR}" ]] && find "${EXAMPLES_DIR}" -name '*.puml' -type f
  } | sort -u
)

TOTAL=${#PUML_FILES[@]}

N_MATCH=0
N_DRIFT=0
N_PUML_ONLY=0
N_JAR_ONLY=0
N_BOTH_FAIL=0

FIXTURES_JSON=""

TMPDIR_WORK="$(mktemp -d)"
trap 'rm -rf "${TMPDIR_WORK}"' EXIT

mkdir -p "${TMPDIR_WORK}/ref" "${TMPDIR_WORK}/ours"

# ---------------------------------------------------------------------------
# Process each fixture
# ---------------------------------------------------------------------------
for F in "${PUML_FILES[@]}"; do
  REL="${F#"${ROOT_DIR}/"}"
  BASENAME="$(basename "${F}" .puml)"
  # Make a hash-safe name from the relative path
  SLUG="$(printf '%s' "${REL}" | tr '/' '_' | tr -dc 'a-zA-Z0-9_.-')"

  # ---- Reference SVG via PlantUML JAR (stdin pipe mode for reliability) ----
  REF_SVG="${TMPDIR_WORK}/ref/${SLUG}.svg"
  JAR_OK=true
  if ! "${JAVA_BIN}" -jar "${JAR_ABS}" -tsvg -pipe \
       < "${F}" > "${REF_SVG}" 2>/dev/null; then
    JAR_OK=false
  fi
  # Treat empty or missing output as failure
  if [[ "${JAR_OK}" == "true" ]] && [[ ! -s "${REF_SVG}" ]]; then
    JAR_OK=false
  fi

  # ---- Our SVG via the release binary ----
  OUR_SVG="${TMPDIR_WORK}/ours/${SLUG}.svg"
  OUR_OK=true
  if "${BINARY}" "${F}" --output "${OUR_SVG}" 2>/dev/null && [[ -s "${OUR_SVG}" ]]; then
    OUR_OK=true
  # Fallback: try stdin → stdout mode.
  elif "${BINARY}" - < "${F}" > "${OUR_SVG}" 2>/dev/null && [[ -s "${OUR_SVG}" ]]; then
    OUR_OK=true
  else
    OUR_OK=false
  fi
  [[ -s "${OUR_SVG}" ]] || OUR_OK=false

  # ---- Categorize ----
  if [[ "${OUR_OK}" == "false" && "${JAR_OK}" == "false" ]]; then
    (( N_BOTH_FAIL++ )) || true
    ENTRY="$(printf '{"path":"%s","category":"both-fail","metrics":{}}' "${REL}")"
    FIXTURES_JSON="${FIXTURES_JSON:+${FIXTURES_JSON},}${ENTRY}"
    continue
  fi

  if [[ "${OUR_OK}" == "true" && "${JAR_OK}" == "false" ]]; then
    (( N_PUML_ONLY++ )) || true
    OUR_ELEMS="$(count_structural_elements "${OUR_SVG}")"
    ENTRY="$(printf '{"path":"%s","category":"puml-only","metrics":{"our_elem_count":%s}}' \
      "${REL}" "${OUR_ELEMS}")"
    FIXTURES_JSON="${FIXTURES_JSON:+${FIXTURES_JSON},}${ENTRY}"
    continue
  fi

  if [[ "${OUR_OK}" == "false" && "${JAR_OK}" == "true" ]]; then
    (( N_JAR_ONLY++ )) || true
    REF_ELEMS="$(count_structural_elements "${REF_SVG}")"
    ENTRY="$(printf '{"path":"%s","category":"jar-only","metrics":{"ref_elem_count":%s}}' \
      "${REL}" "${REF_ELEMS}")"
    FIXTURES_JSON="${FIXTURES_JSON:+${FIXTURES_JSON},}${ENTRY}"
    continue
  fi

  # Both rendered — compute metrics
  OUR_ELEMS="$(count_structural_elements "${OUR_SVG}")"
  REF_ELEMS="$(count_structural_elements "${REF_SVG}")"
  OUR_VB="$(extract_viewbox_dims "${OUR_SVG}")"
  REF_VB="$(extract_viewbox_dims "${REF_SVG}")"
  OUR_TEXTS="$(extract_text_set "${OUR_SVG}")"
  REF_TEXTS="$(extract_text_set "${REF_SVG}")"
  OUR_COLORS="$(extract_color_set "${OUR_SVG}")"
  REF_COLORS="$(extract_color_set "${REF_SVG}")"

  # viewBox W/H drift
  OUR_VBW="$(echo "${OUR_VB}" | awk '{print $1+0}')"
  OUR_VBH="$(echo "${OUR_VB}" | awk '{print $2+0}')"
  REF_VBW="$(echo "${REF_VB}" | awk '{print $1+0}')"
  REF_VBH="$(echo "${REF_VB}" | awk '{print $2+0}')"

  ELEM_DRIFT="$(pct_drift "${OUR_ELEMS}" "${REF_ELEMS}")"
  VBW_DRIFT="$(pct_drift "${OUR_VBW}" "${REF_VBW}")"
  VBH_DRIFT="$(pct_drift "${OUR_VBH}" "${REF_VBH}")"

  # Text and colour set drift: if sets match → 0%, else count mismatches
  TEXT_MATCH=true
  [[ "${OUR_TEXTS}" == "${REF_TEXTS}" ]] || TEXT_MATCH=false
  COLOR_MATCH=true
  [[ "${OUR_COLORS}" == "${REF_COLORS}" ]] || COLOR_MATCH=false

  # Categorize
  IS_MATCH=true
  if [[ "${ELEM_DRIFT}" -gt 10 ]] \
    || [[ "${VBW_DRIFT}" -gt 10 ]] \
    || [[ "${VBH_DRIFT}" -gt 10 ]] \
    || [[ "${TEXT_MATCH}" == "false" ]] \
    || [[ "${COLOR_MATCH}" == "false" ]]; then
    IS_MATCH=false
  fi

  if [[ "${IS_MATCH}" == "true" ]]; then
    (( N_MATCH++ )) || true
    CATEGORY="match"
  else
    (( N_DRIFT++ )) || true
    CATEGORY="drift"
  fi

  # Escape strings for JSON (basic)
  safe_json() { printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'; }

  ENTRY="$(printf \
    '{"path":"%s","category":"%s","metrics":{"elem_count":{"ours":%s,"ref":%s,"drift_pct":%s},"viewbox":{"ours":"%s","ref":"%s","w_drift_pct":%s,"h_drift_pct":%s},"text_set":{"match":%s},"color_set":{"match":%s}}}' \
    "${REL}" \
    "${CATEGORY}" \
    "${OUR_ELEMS}" "${REF_ELEMS}" "${ELEM_DRIFT}" \
    "$(safe_json "${OUR_VB}")" "$(safe_json "${REF_VB}")" "${VBW_DRIFT}" "${VBH_DRIFT}" \
    "$( [[ "${TEXT_MATCH}"  == "true" ]] && echo true || echo false )" \
    "$( [[ "${COLOR_MATCH}" == "true" ]] && echo true || echo false )")"
  FIXTURES_JSON="${FIXTURES_JSON:+${FIXTURES_JSON},}${ENTRY}"
done

# ---------------------------------------------------------------------------
# Compute match percentage
# ---------------------------------------------------------------------------
MATCH_PCT=0
if [[ "${TOTAL}" -gt 0 ]]; then
  MATCH_PCT=$(( (N_MATCH * 100) / TOTAL ))
fi

# ---------------------------------------------------------------------------
# ISO-8601 UTC timestamp
# ---------------------------------------------------------------------------
TIMESTAMP="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# ---------------------------------------------------------------------------
# Build report JSON
# ---------------------------------------------------------------------------
REPORT="$(printf \
  '{"schema_version":"1.0","timestamp":"%s","jar_version":"%s","summary":{"total":%d,"match":%d,"drift":%d,"puml_only":%d,"jar_only":%d,"both_fail":%d},"fixtures":[%s]}' \
  "${TIMESTAMP}" \
  "${JAR_VERSION}" \
  "${TOTAL}" \
  "${N_MATCH}" \
  "${N_DRIFT}" \
  "${N_PUML_ONLY}" \
  "${N_JAR_ONLY}" \
  "${N_BOTH_FAIL}" \
  "${FIXTURES_JSON}")"

printf '%s\n' "${REPORT}"
mkdir -p "$(dirname "${REPORT_FILE}")"
printf '%s\n' "${REPORT}" > "${REPORT_FILE}"
echo "[oracle] report written to ${REPORT_FILE}" >&2
echo "[oracle] summary: total=${TOTAL} match=${N_MATCH} drift=${N_DRIFT} puml_only=${N_PUML_ONLY} jar_only=${N_JAR_ONLY} both_fail=${N_BOTH_FAIL} match_pct=${MATCH_PCT}%" >&2

# ---------------------------------------------------------------------------
# Exit codes: 0 = >=80% match; 1 = 50–79%; 2 = <50%
# ---------------------------------------------------------------------------
if [[ "${MATCH_PCT}" -ge 80 ]]; then
  exit 0
elif [[ "${MATCH_PCT}" -ge 50 ]]; then
  exit 1
else
  exit 2
fi
