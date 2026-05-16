#!/usr/bin/env bash
# oracle.sh — differential oracle between puml and plantuml.jar
#
# Usage:
#   ./scripts/oracle.sh [FIXTURE_PATH]
#
# Behavior:
#   - If no JAR is present (PUML_ORACLE_JAR unset / file missing), exits 0 with skipped:true.
#   - If JAR is present, renders FIXTURE_PATH with both puml and plantuml.jar,
#     byte-compares SVG outputs, and exits 0 on match or non-zero on mismatch.
#
# Environment:
#   PUML_ORACLE_JAR   Path to plantuml.jar (default: plantuml.jar in repo root)
#   PUML_BIN          Path to puml binary (default: cargo-resolved target/debug/puml)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

JAR="${PUML_ORACLE_JAR:-$REPO_ROOT/plantuml.jar}"
PUML_BIN="${PUML_BIN:-$REPO_ROOT/target/debug/puml}"
FIXTURE="${1:-$REPO_ROOT/tests/fixtures/single_valid.puml}"

# If JAR is absent, skip gracefully.
if [[ ! -f "$JAR" ]]; then
    echo '{"skipped":true,"reason":"plantuml.jar not present; set PUML_ORACLE_JAR to enable oracle comparison"}'
    exit 0
fi

# Verify puml binary exists.
if [[ ! -x "$PUML_BIN" ]]; then
    echo '{"skipped":false,"error":"puml binary not found or not executable","binary":"'"$PUML_BIN"'"}'
    exit 1
fi

# Verify fixture exists.
if [[ ! -f "$FIXTURE" ]]; then
    echo '{"skipped":false,"error":"fixture not found","fixture":"'"$FIXTURE"'"}'
    exit 1
fi

TMPDIR_WORK="$(mktemp -d)"
trap 'rm -rf "$TMPDIR_WORK"' EXIT

PUML_SVG="$TMPDIR_WORK/puml.svg"
JAR_SVG="$TMPDIR_WORK/jar.svg"

# Render with puml.
"$PUML_BIN" --format svg --output "$PUML_SVG" "$FIXTURE" 2>/dev/null || {
    echo '{"skipped":false,"error":"puml render failed","fixture":"'"$FIXTURE"'"}'
    exit 1
}

# Render with plantuml.jar (requires Java).
java -jar "$JAR" -tsvg -o "$TMPDIR_WORK" "$FIXTURE" 2>/dev/null || {
    echo '{"skipped":false,"error":"plantuml.jar render failed","fixture":"'"$FIXTURE"'"}'
    exit 1
}

# plantuml.jar emits fixture_basename.svg in the output directory.
FIXTURE_BASE="$(basename "$FIXTURE" .puml)"
JAR_SVG_CANDIDATE="$TMPDIR_WORK/${FIXTURE_BASE}.svg"
if [[ ! -f "$JAR_SVG_CANDIDATE" ]]; then
    echo '{"skipped":false,"error":"plantuml.jar SVG output not found","expected":"'"$JAR_SVG_CANDIDATE"'"}'
    exit 1
fi
mv "$JAR_SVG_CANDIDATE" "$JAR_SVG"

# Compare outputs.
if diff -q "$PUML_SVG" "$JAR_SVG" >/dev/null 2>&1; then
    echo '{"skipped":false,"match":true,"fixture":"'"$FIXTURE"'"}'
    exit 0
else
    echo '{"skipped":false,"match":false,"fixture":"'"$FIXTURE"'","diff":"SVG outputs differ"}'
    exit 1
fi
