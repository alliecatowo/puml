#!/usr/bin/env bash
# Sync a GitHub issue/PR event into the PUML Projects v2 board.
#
# Native Projects v2 workflows can be listed through GraphQL, but this script
# is the repo-side fallback for automation GitHub does not currently expose as
# create/update mutations.

set -euo pipefail

PROJECT_OWNER="${PROJECT_OWNER:-alliecatowo}"
PROJECT_NUMBER="${PROJECT_NUMBER:-3}"
PROJECT_ID="${PROJECT_ID:-PVT_kwHOBdlpmc4BX1zk}"
REPO="${GITHUB_REPOSITORY:-alliecatowo/puml}"

STATUS_FIELD_ID="${STATUS_FIELD_ID:-PVTSSF_lAHOBdlpmc4BX1zkzhS_7BE}"
STATUS_TODO_ID="${STATUS_TODO_ID:-332edcb8}"
STATUS_IN_PROGRESS_ID="${STATUS_IN_PROGRESS_ID:-867a988e}"
STATUS_MERGING_ID="${STATUS_MERGING_ID:-d4aa3b22}"
STATUS_DONE_ID="${STATUS_DONE_ID:-c603c19d}"

PRIORITY_FIELD_ID="${PRIORITY_FIELD_ID:-PVTSSF_lAHOBdlpmc4BX1zkzhS_7FE}"
PRIORITY_P0_ID="${PRIORITY_P0_ID:-5f502b32}"
PRIORITY_P1_ID="${PRIORITY_P1_ID:-e87779b3}"
PRIORITY_P2_ID="${PRIORITY_P2_ID:-011502f5}"
PRIORITY_P3_ID="${PRIORITY_P3_ID:-e085bc1a}"

DRY_RUN=false
CONTENT_URL=""
STATUS_NAME="auto"
PRIORITY_NAME="auto"
LINKED_ISSUE_STATUS_NAME=""

usage() {
  cat <<'USAGE'
Usage:
  scripts/project-v2-event-sync.sh
  scripts/project-v2-event-sync.sh --content-url URL [--status todo|in-progress|merging|done|none] [--priority P0|P1|P2|P3|none]

Environment:
  GH_TOKEN or GITHUB_TOKEN must be able to read/write the user Project v2 board.
  GITHUB_EVENT_NAME and GITHUB_EVENT_PATH are used in event mode.

Behavior:
  - Ensures the issue/PR is present on Project #3.
  - Updates only Status and Priority fields.
  - On PR open/update, moves the PR and closing issues to Merging.
  - On PR merge, moves the PR and closing issues to Done.
  - On issue close, moves the issue to Done; opened/reopened issues go to Todo.
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --content-url)
      CONTENT_URL="$2"
      shift 2
      ;;
    --content-url=*)
      CONTENT_URL="${1#*=}"
      shift
      ;;
    --status)
      STATUS_NAME="$2"
      shift 2
      ;;
    --status=*)
      STATUS_NAME="${1#*=}"
      shift
      ;;
    --priority)
      PRIORITY_NAME="$2"
      shift 2
      ;;
    --priority=*)
      PRIORITY_NAME="${1#*=}"
      shift
      ;;
    --dry-run|-n)
      DRY_RUN=true
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[project-sync] unknown flag: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "[project-sync] missing required command: $1" >&2
    exit 1
  fi
}

require_cmd gh
require_cmd jq

status_option_id() {
  case "$1" in
    todo|Todo) printf '%s\n' "${STATUS_TODO_ID}" ;;
    in-progress|in_progress|"In Progress") printf '%s\n' "${STATUS_IN_PROGRESS_ID}" ;;
    merging|Merging) printf '%s\n' "${STATUS_MERGING_ID}" ;;
    done|Done) printf '%s\n' "${STATUS_DONE_ID}" ;;
    none|"") printf '\n' ;;
    *)
      echo "[project-sync] unsupported status: $1" >&2
      exit 2
      ;;
  esac
}

priority_option_id() {
  case "$1" in
    P0|p0) printf '%s\n' "${PRIORITY_P0_ID}" ;;
    P1|p1) printf '%s\n' "${PRIORITY_P1_ID}" ;;
    P2|p2) printf '%s\n' "${PRIORITY_P2_ID}" ;;
    P3|p3) printf '%s\n' "${PRIORITY_P3_ID}" ;;
    none|"") printf '\n' ;;
    *)
      echo "[project-sync] unsupported priority: $1" >&2
      exit 2
      ;;
  esac
}

priority_from_labels() {
  local labels_json="$1"
  printf '%s' "${labels_json}" | jq -r '
    [.[].name // .[] | ascii_upcase]
    | if any(. == "P0") then "P0"
      elif any(. == "P1") then "P1"
      elif any(. == "P2") then "P2"
      elif any(. == "P3") then "P3"
      else "none"
      end
  '
}

load_event_defaults() {
  local event_name="${GITHUB_EVENT_NAME:-}"
  local event_path="${GITHUB_EVENT_PATH:-}"
  [[ -n "${event_path}" && -f "${event_path}" ]] || return 0

  case "${event_name}" in
    issues)
      CONTENT_URL="$(jq -r '.issue.html_url' "${event_path}")"
      local action state labels
      action="$(jq -r '.action' "${event_path}")"
      state="$(jq -r '.issue.state' "${event_path}")"
      labels="$(jq -c '.issue.labels // []' "${event_path}")"
      PRIORITY_NAME="$(priority_from_labels "${labels}")"
      case "${action}:${state}" in
        closed:closed) STATUS_NAME="done" ;;
        opened:open|reopened:open) STATUS_NAME="todo" ;;
        *) STATUS_NAME="none" ;;
      esac
      ;;
    pull_request)
      CONTENT_URL="$(jq -r '.pull_request.html_url' "${event_path}")"
      local action merged draft labels
      action="$(jq -r '.action' "${event_path}")"
      merged="$(jq -r '.pull_request.merged // false' "${event_path}")"
      draft="$(jq -r '.pull_request.draft // false' "${event_path}")"
      labels="$(jq -c '.pull_request.labels // []' "${event_path}")"
      PRIORITY_NAME="$(priority_from_labels "${labels}")"
      if [[ "${action}" == "closed" && "${merged}" == "true" ]]; then
        STATUS_NAME="done"
        LINKED_ISSUE_STATUS_NAME="done"
      elif [[ "${action}" != "closed" && "${draft}" == "true" ]]; then
        STATUS_NAME="in-progress"
        LINKED_ISSUE_STATUS_NAME="in-progress"
      elif [[ "${action}" != "closed" ]]; then
        STATUS_NAME="merging"
        LINKED_ISSUE_STATUS_NAME="merging"
      else
        STATUS_NAME="none"
      fi
      ;;
  esac
}

ensure_project_item() {
  local url="$1"
  local items_json item_id
  if ! items_json="$(gh project item-list "${PROJECT_NUMBER}" --owner "${PROJECT_OWNER}" --limit 1000 --format json)"; then
    echo "[project-sync] failed to list project items for ${PROJECT_OWNER}/${PROJECT_NUMBER}" >&2
    if [[ "${DRY_RUN}" == "true" ]]; then
      printf '%s\n' "DRY_RUN_ITEM_ID"
      return 0
    fi
    return 1
  fi

  item_id="$(
    printf '%s' "${items_json}" \
      | jq -r --arg url "${url}" '.items[] | select(.content.url == $url) | .id' \
      | head -n 1
  )"

  if [[ -n "${item_id}" ]]; then
    printf '%s\n' "${item_id}"
    return 0
  fi

  echo "[project-sync] item not found; adding ${url}" >&2
  if [[ "${DRY_RUN}" == "true" ]]; then
    printf '%s\n' "DRY_RUN_ITEM_ID"
    return 0
  fi

  gh project item-add "${PROJECT_NUMBER}" \
    --owner "${PROJECT_OWNER}" \
    --url "${url}" \
    --format json \
    --jq '.id'
}

edit_single_select() {
  local item_id="$1"
  local field_id="$2"
  local option_id="$3"
  local field_name="$4"
  [[ -n "${option_id}" ]] || return 0

  echo "[project-sync] set ${field_name} on ${item_id} -> ${option_id}"
  if [[ "${DRY_RUN}" == "true" ]]; then
    return 0
  fi

  gh project item-edit \
    --id "${item_id}" \
    --project-id "${PROJECT_ID}" \
    --field-id "${field_id}" \
    --single-select-option-id "${option_id}" \
    >/dev/null
}

sync_url() {
  local url="$1"
  local status_name="$2"
  local priority_name="$3"
  [[ -n "${url}" && "${url}" != "null" ]] || return 0

  local item_id status_id priority_id
  item_id="$(ensure_project_item "${url}")"
  status_id="$(status_option_id "${status_name}")"
  priority_id="$(priority_option_id "${priority_name}")"

  edit_single_select "${item_id}" "${STATUS_FIELD_ID}" "${status_id}" "Status"
  edit_single_select "${item_id}" "${PRIORITY_FIELD_ID}" "${priority_id}" "Priority"
}

linked_issue_urls_for_pr_event() {
  local event_path="${GITHUB_EVENT_PATH:-}"
  [[ -n "${event_path}" && -f "${event_path}" ]] || return 0
  [[ "${GITHUB_EVENT_NAME:-}" == "pull_request" ]] || return 0

  local pr_number
  pr_number="$(jq -r '.pull_request.number' "${event_path}")"
  [[ -n "${pr_number}" && "${pr_number}" != "null" ]] || return 0

  gh pr view "${pr_number}" \
    --repo "${REPO}" \
    --json closingIssuesReferences \
    --jq '.closingIssuesReferences[].url' 2>/dev/null || true
}

if [[ "${STATUS_NAME}" == "auto" || "${PRIORITY_NAME}" == "auto" || -z "${CONTENT_URL}" ]]; then
  load_event_defaults
fi

if [[ -z "${CONTENT_URL}" || "${CONTENT_URL}" == "null" ]]; then
  echo "[project-sync] no issue or PR URL found; nothing to do"
  exit 0
fi

echo "[project-sync] project=${PROJECT_OWNER}/${PROJECT_NUMBER} url=${CONTENT_URL} status=${STATUS_NAME} priority=${PRIORITY_NAME} dry_run=${DRY_RUN}"
sync_url "${CONTENT_URL}" "${STATUS_NAME}" "${PRIORITY_NAME}"

if [[ -n "${LINKED_ISSUE_STATUS_NAME}" ]]; then
  while IFS= read -r issue_url; do
    [[ -n "${issue_url}" ]] || continue
    echo "[project-sync] linked issue ${issue_url} -> ${LINKED_ISSUE_STATUS_NAME}"
    sync_url "${issue_url}" "${LINKED_ISSUE_STATUS_NAME}" "none"
  done < <(linked_issue_urls_for_pr_event)
fi
