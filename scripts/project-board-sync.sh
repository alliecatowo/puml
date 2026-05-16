#!/usr/bin/env bash
# project-board-sync.sh — Sync closed GitHub issues to "Done" on the project board.
#
# Usage:
#   ./scripts/project-board-sync.sh [--dry-run] [--project-title <title>]
#
# Prerequisites:
#   - gh CLI authenticated (gh auth login)
#   - gh CLI version >= 2.40 (for `gh project` sub-commands)
#   - The calling user must have write access to the project board
#
# What it does:
#   1. Lists all GitHub Projects (v2) owned by @me.
#   2. Identifies the PUML project (default title match: "PUML" or first project if only one).
#   3. For each project item in status "In Progress" or "?" (uncategorised):
#      a. Looks up whether the linked issue is closed in the GitHub API.
#      b. If closed → updates the item's status field to "Done".
#   4. Prints a summary table.
#
# Environment variables:
#   GITHUB_OWNER        GitHub login/org (defaults to @me)
#   PUML_PROJECT_TITLE  Board title to match (default: "PUML")
#
# Exit codes:
#   0  — success (or dry-run completed)
#   1  — gh CLI not found or not authenticated
#   2  — project not found

set -euo pipefail

DRY_RUN=false
PROJECT_TITLE="${PUML_PROJECT_TITLE:-PUML}"
OWNER="${GITHUB_OWNER:-@me}"

# ---------------------------------------------------------------------------
# Parse flags
# ---------------------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run|-n)
      DRY_RUN=true
      shift
      ;;
    --project-title)
      PROJECT_TITLE="$2"
      shift 2
      ;;
    --project-title=*)
      PROJECT_TITLE="${1#*=}"
      shift
      ;;
    --owner)
      OWNER="$2"
      shift 2
      ;;
    -h|--help)
      grep '^#' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "[board-sync] unknown flag: $1" >&2
      exit 1
      ;;
  esac
done

# ---------------------------------------------------------------------------
# Verify gh CLI
# ---------------------------------------------------------------------------
if ! command -v gh >/dev/null 2>&1; then
  echo "[board-sync] ERROR: gh CLI not found. Install from https://cli.github.com/" >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "[board-sync] ERROR: gh CLI not authenticated. Run: gh auth login" >&2
  exit 1
fi

echo "[board-sync] owner=${OWNER} project_title=${PROJECT_TITLE} dry_run=${DRY_RUN}"

# ---------------------------------------------------------------------------
# Step 1: Find the project
# ---------------------------------------------------------------------------
echo "[board-sync] listing projects for ${OWNER}..."

# gh project list outputs: NUMBER  TITLE  URL  ...
PROJECT_LIST="$(gh project list --owner "${OWNER}" --format json 2>/dev/null)" || {
  echo "[board-sync] ERROR: could not list projects (check auth and permissions)" >&2
  exit 1
}

PROJECT_NUMBER="$(
  printf '%s' "${PROJECT_LIST}" \
  | python3 -c "
import json, sys
data = json.load(sys.stdin)
projects = data.get('projects', [])
title = '${PROJECT_TITLE}'.lower()
# Prefer exact case-insensitive match on title
for p in projects:
    if p.get('title', '').lower() == title:
        print(p['number'])
        sys.exit(0)
# Fallback: partial match
for p in projects:
    if title in p.get('title', '').lower():
        print(p['number'])
        sys.exit(0)
# If only one project, use it
if len(projects) == 1:
    print(projects[0]['number'])
    sys.exit(0)
sys.exit(2)
" 2>/dev/null
)" || {
  echo "[board-sync] ERROR: could not find project matching '${PROJECT_TITLE}'" >&2
  echo "[board-sync] Available projects:" >&2
  printf '%s' "${PROJECT_LIST}" | python3 -c "
import json, sys
data = json.load(sys.stdin)
for p in data.get('projects', []):
    print(f\"  #{p['number']} {p['title']}\")
" >&2
  exit 2
}

echo "[board-sync] found project #${PROJECT_NUMBER}"

# ---------------------------------------------------------------------------
# Step 2: Fetch all project items with their status and linked issue numbers
# ---------------------------------------------------------------------------
echo "[board-sync] fetching project items..."

# Use gh api (GraphQL) to get items; fall back to gh project item-list
ITEMS_JSON="$(gh project item-list "${PROJECT_NUMBER}" --owner "${OWNER}" --format json 2>/dev/null)" || {
  echo "[board-sync] ERROR: could not list items for project #${PROJECT_NUMBER}" >&2
  exit 1
}

# ---------------------------------------------------------------------------
# Step 3: Determine "Done" status option ID via GraphQL
# ---------------------------------------------------------------------------
# We need the field ID for the status column and the option ID for "Done".
STATUS_FIELD_INFO="$(gh api graphql -f query='
  query($owner: String!, $number: Int!) {
    projectV2: user(login: $owner) {
      projectV2(number: $number) {
        fields(first: 20) {
          nodes {
            ... on ProjectV2SingleSelectField {
              id
              name
              options { id name }
            }
          }
        }
      }
    }
  }
' -f owner="${OWNER/#@me/$(gh api user --jq .login)}" -F number="${PROJECT_NUMBER}" 2>/dev/null)" || {
  echo "[board-sync] WARN: could not fetch status field info via GraphQL; will use option name 'Done' directly" >&2
  STATUS_FIELD_INFO="{}"
}

DONE_OPTION_ID="$(
  printf '%s' "${STATUS_FIELD_INFO}" | python3 -c "
import json, sys
try:
    data = json.load(sys.stdin)
    fields = (data.get('data', {})
                  .get('projectV2', {})
                  .get('projectV2', {})
                  .get('fields', {})
                  .get('nodes', []))
    for f in fields:
        if f.get('name', '').lower() == 'status':
            for opt in f.get('options', []):
                if opt.get('name', '').lower() == 'done':
                    print(json.dumps({'field_id': f['id'], 'option_id': opt['id']}))
                    sys.exit(0)
except Exception:
    pass
print('{}')
" 2>/dev/null
)"

FIELD_ID="$(printf '%s' "${DONE_OPTION_ID}" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('field_id',''))" 2>/dev/null || true)"
OPTION_ID="$(printf '%s' "${DONE_OPTION_ID}" | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('option_id',''))" 2>/dev/null || true)"

# ---------------------------------------------------------------------------
# Step 4: Iterate items and move closed issues to Done
# ---------------------------------------------------------------------------
UPDATED=0
SKIPPED=0
ALREADY_DONE=0
ERRORS=0

# Parse items: each item has an id, type, status, and content (issue number)
ITEM_RECORDS="$(
  printf '%s' "${ITEMS_JSON}" | python3 -c "
import json, sys
data = json.load(sys.stdin)
items = data.get('items', [])
for item in items:
    item_id = item.get('id', '')
    # status may live in fieldValues or a top-level 'status' key depending on gh version
    status = item.get('status', '') or ''
    content = item.get('content', {}) or {}
    issue_number = content.get('number', '')
    issue_type = item.get('type', '')
    print(json.dumps({'id': item_id, 'status': status, 'issue_number': issue_number, 'type': issue_type}))
" 2>/dev/null
)"

echo "[board-sync] processing items..."

while IFS= read -r LINE; do
  [[ -z "${LINE}" ]] && continue

  ITEM_ID="$(printf '%s' "${LINE}" | python3 -c "import json,sys; print(json.loads(sys.stdin.read())['id'])" 2>/dev/null || true)"
  ITEM_STATUS="$(printf '%s' "${LINE}" | python3 -c "import json,sys; print(json.loads(sys.stdin.read())['status'])" 2>/dev/null || true)"
  ISSUE_NUM="$(printf '%s' "${LINE}" | python3 -c "import json,sys; print(json.loads(sys.stdin.read())['issue_number'])" 2>/dev/null || true)"
  ITEM_TYPE="$(printf '%s' "${LINE}" | python3 -c "import json,sys; print(json.loads(sys.stdin.read())['type'])" 2>/dev/null || true)"

  [[ -z "${ITEM_ID}" ]] && continue

  # Only process "In Progress" or empty/unknown status items
  STATUS_LOWER="$(printf '%s' "${ITEM_STATUS}" | tr '[:upper:]' '[:lower:]')"
  if [[ "${STATUS_LOWER}" != "in progress" && "${STATUS_LOWER}" != "" && "${STATUS_LOWER}" != "?" ]]; then
    if [[ "${STATUS_LOWER}" == "done" ]]; then
      (( ALREADY_DONE++ )) || true
    else
      (( SKIPPED++ )) || true
    fi
    continue
  fi

  # Skip non-issue items (e.g. draft notes)
  if [[ "${ITEM_TYPE}" != "ISSUE" && -n "${ITEM_TYPE}" ]]; then
    (( SKIPPED++ )) || true
    continue
  fi

  # Check whether the issue is closed
  if [[ -z "${ISSUE_NUM}" ]]; then
    (( SKIPPED++ )) || true
    continue
  fi

  ISSUE_STATE="$(gh issue view "${ISSUE_NUM}" --json state --jq '.state' 2>/dev/null || true)"
  if [[ "${ISSUE_STATE}" != "CLOSED" ]]; then
    echo "  [skip] issue #${ISSUE_NUM} is ${ISSUE_STATE:-unknown} (status='${ITEM_STATUS}')"
    (( SKIPPED++ )) || true
    continue
  fi

  echo "  [update] issue #${ISSUE_NUM} is CLOSED → setting status to Done (item=${ITEM_ID})"

  if [[ "${DRY_RUN}" == "true" ]]; then
    echo "    [dry-run] would run: gh project item-edit --id ${ITEM_ID} --field-id ${FIELD_ID} --single-select-option-id ${OPTION_ID} --project-id ${PROJECT_NUMBER}"
    (( UPDATED++ )) || true
    continue
  fi

  if [[ -n "${FIELD_ID}" && -n "${OPTION_ID}" ]]; then
    if gh project item-edit \
        --id "${ITEM_ID}" \
        --field-id "${FIELD_ID}" \
        --single-select-option-id "${OPTION_ID}" \
        --project-id "${PROJECT_NUMBER}" 2>/dev/null; then
      (( UPDATED++ )) || true
    else
      echo "  [error] failed to update item ${ITEM_ID}" >&2
      (( ERRORS++ )) || true
    fi
  else
    echo "  [warn] status field/option IDs not resolved; cannot update item ${ITEM_ID}" >&2
    (( ERRORS++ )) || true
  fi

done <<< "${ITEM_RECORDS}"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
echo ""
echo "=== project-board-sync summary ==="
echo "  Project:      #${PROJECT_NUMBER} (${PROJECT_TITLE})"
echo "  Dry run:      ${DRY_RUN}"
echo "  Updated:      ${UPDATED}"
echo "  Already Done: ${ALREADY_DONE}"
echo "  Skipped:      ${SKIPPED}"
echo "  Errors:       ${ERRORS}"
echo "=================================="

[[ "${ERRORS}" -eq 0 ]]
