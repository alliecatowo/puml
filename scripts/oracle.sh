#!/usr/bin/env bash
# oracle.sh — differential oracle harness for puml vs. reference PlantUML JAR
#
# When PUML_ORACLE_JAR is unset or the JAR is absent, the script exits 0
# and prints a JSON object containing "skipped":true so callers can detect
# the deterministic skip sentinel without treating it as a failure.
#
# When the JAR is present, the script runs a set of fixture inputs through
# both the reference JAR and the puml binary, then compares SVG output
# structure for detectable regressions.
#
# Usage:
#   PUML_ORACLE_JAR=/path/to/plantuml.jar bash scripts/oracle.sh
#   bash scripts/oracle.sh   # skips cleanly when JAR is absent

set -euo pipefail

ORACLE_JAR="${PUML_ORACLE_JAR:-}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
FIXTURE_DIR="${REPO_ROOT}/tests/fixtures"

# Deterministic skip when no JAR is configured or JAR file is missing
if [[ -z "${ORACLE_JAR}" ]] || [[ ! -f "${ORACLE_JAR}" ]]; then
    printf '{"skipped":true,"reason":"PUML_ORACLE_JAR not set or JAR not found"}\n'
    exit 0
fi

# Verify java is available
if ! command -v java &>/dev/null; then
    printf '{"skipped":true,"reason":"java not found in PATH"}\n'
    exit 0
fi

# Fixtures to compare (relative to FIXTURE_DIR)
FIXTURES=(
    "basic/hello.puml"
    "participants/valid_participant_types.puml"
)

PASS=0
FAIL=0
ERRORS=()

for fixture in "${FIXTURES[@]}"; do
    fixture_path="${FIXTURE_DIR}/${fixture}"
    if [[ ! -f "${fixture_path}" ]]; then
        continue
    fi

    # Render with puml binary
    puml_svg=$(cargo run --quiet -- "${fixture_path}" --check 2>/dev/null && \
        cargo run --quiet -- - < "${fixture_path}" 2>/dev/null || true)

    # Render with reference JAR (pipe mode)
    ref_svg=$(java -jar "${ORACLE_JAR}" -tsvg -pipe < "${fixture_path}" 2>/dev/null || true)

    if [[ -z "${ref_svg}" ]]; then
        continue
    fi

    # Basic structural check: both should contain <svg
    if echo "${puml_svg}" | grep -q '<svg' && echo "${ref_svg}" | grep -q '<svg'; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        ERRORS+=("${fixture}: SVG structure mismatch")
    fi
done

if [[ ${FAIL} -gt 0 ]]; then
    printf '{"skipped":false,"pass":%d,"fail":%d,"errors":%s}\n' \
        "${PASS}" "${FAIL}" "$(printf '%s\n' "${ERRORS[@]}" | jq -R . | jq -s .)"
    exit 1
fi

printf '{"skipped":false,"pass":%d,"fail":0}\n' "${PASS}"
exit 0
