#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="verify"
BRANCH="main"
REPO=""
STRICT_REQUIRED_STATUS_CHECKS="true"
CHECK_CONTEXTS=("fmt-clippy-test-coverage-quick")

usage() {
  cat <<'USAGE'
Usage:
  ./scripts/branch-protection.sh verify [--repo owner/name] [--branch main]
  ./scripts/branch-protection.sh apply  [--repo owner/name] [--branch main]

Modes:
  verify  Read branch protection/rulesets via GitHub API and fail if required policy is missing.
  apply   Attempt to enforce policy via branch protection API, then verify.

Required policy (issue #90):
  - Require status check context: fmt-clippy-test-coverage-quick
  - Require pull request review before merge
  - Disallow force pushes on main
  - Disallow branch deletion on main

Notes:
  - Requires `gh` authentication against the target repository.
  - If apply is blocked by permissions, verify mode still provides an auditable failure.
USAGE
}

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "[branch-protection] missing required command: $cmd" >&2
    exit 1
  fi
}

infer_repo_from_origin() {
  local remote_url path
  remote_url="$(git remote get-url origin)"
  case "$remote_url" in
    git@github.com:*)
      path="${remote_url#git@github.com:}"
      ;;
    https://github.com/*)
      path="${remote_url#https://github.com/}"
      ;;
    *)
      echo "[branch-protection] unable to infer GitHub repo from origin URL: $remote_url" >&2
      echo "[branch-protection] pass --repo owner/name explicitly." >&2
      exit 1
      ;;
  esac
  REPO="${path%.git}"
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    verify|apply)
      MODE="$1"
      shift
      ;;
    --repo)
      REPO="$2"
      shift 2
      ;;
    --branch)
      BRANCH="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[branch-protection] unknown option: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_cmd gh
require_cmd python3

if [[ -z "$REPO" ]]; then
  infer_repo_from_origin
fi

echo "[branch-protection] mode=$MODE repo=$REPO branch=$BRANCH"

verify_policy() {
  local tmp_protection tmp_rulesets
  tmp_protection="$(mktemp)"
  tmp_rulesets="$(mktemp)"

  if ! gh api \
    -H "Accept: application/vnd.github+json" \
    "/repos/$REPO/branches/$BRANCH/protection" >"$tmp_protection" 2>/dev/null; then
    echo "[branch-protection] branch protection API returned no policy for $REPO:$BRANCH." >&2
    echo '{}' >"$tmp_protection"
  fi

  if ! gh api \
    -H "Accept: application/vnd.github+json" \
    "/repos/$REPO/rulesets?targets=branch" >"$tmp_rulesets" 2>/dev/null; then
    echo "[branch-protection] unable to read rulesets for $REPO (continuing with branch protection only)." >&2
    echo '[]' >"$tmp_rulesets"
  fi

  set +e
  python3 - "$tmp_protection" "$tmp_rulesets" "$BRANCH" "${CHECK_CONTEXTS[0]}" <<'PY'
import json
import sys

protection_path, rulesets_path, branch, required_context = sys.argv[1:5]

with open(protection_path, "r", encoding="utf-8") as fh:
    protection = json.load(fh)
with open(rulesets_path, "r", encoding="utf-8") as fh:
    rulesets = json.load(fh)


def protection_satisfies(p):
    if not isinstance(p, dict) or not p:
        return False, []

    failures = []
    required = ((p.get("required_status_checks") or {}).get("contexts") or [])
    if required_context not in required:
        failures.append(f"missing required status check context: {required_context}")

    pr_reviews = p.get("required_pull_request_reviews") or {}
    if int(pr_reviews.get("required_approving_review_count") or 0) < 1:
        failures.append("required pull request review is not enforced")

    allow_force_pushes = ((p.get("allow_force_pushes") or {}).get("enabled"))
    if allow_force_pushes is not False:
        failures.append("force pushes are not disabled")

    allow_deletions = ((p.get("allow_deletions") or {}).get("enabled"))
    if allow_deletions is not False:
        failures.append("branch deletion is not disabled")

    return len(failures) == 0, failures


def branch_match(rule):
    cond = (rule.get("conditions") or {}).get("ref_name") or {}
    include = cond.get("include") or []
    needle = f"refs/heads/{branch}"
    return needle in include


def ruleset_satisfies(rule):
    if rule.get("enforcement") != "active":
        return False
    if not branch_match(rule):
        return False

    has_required_context = False
    has_pr_rule = False
    blocks_force_push = False
    blocks_deletion = False

    for r in rule.get("rules") or []:
        r_type = r.get("type")
        params = r.get("parameters") or {}

        if r_type == "required_status_checks":
            for chk in params.get("required_status_checks") or []:
                if chk.get("context") == required_context:
                    has_required_context = True

        if r_type == "pull_request":
            if params.get("required_approving_review_count", 0) >= 1:
                has_pr_rule = True

        if r_type == "non_fast_forward":
            blocks_force_push = True

        if r_type == "deletion":
            blocks_deletion = True

    return has_required_context and has_pr_rule and blocks_force_push and blocks_deletion

ok_bp, bp_failures = protection_satisfies(protection)
ok_rs = any(ruleset_satisfies(rule) for rule in (rulesets or []))

if ok_bp or ok_rs:
    source = "branch protection" if ok_bp else "ruleset"
    print(f"[branch-protection] verification passed via {source} policy.")
    sys.exit(0)

print("[branch-protection] verification failed.", file=sys.stderr)
for failure in bp_failures:
    print(f"[branch-protection] - {failure}", file=sys.stderr)
print("[branch-protection] No active matching ruleset found that enforces all required conditions.", file=sys.stderr)
sys.exit(1)
PY
  local verify_rc=$?
  set -e

  rm -f "$tmp_protection" "$tmp_rulesets"
  return "$verify_rc"
}

if [[ "$MODE" == "apply" ]]; then
  payload="$(mktemp)"
  cat >"$payload" <<JSON
{
  "required_status_checks": {
    "strict": ${STRICT_REQUIRED_STATUS_CHECKS},
    "contexts": ["${CHECK_CONTEXTS[0]}"]
  },
  "enforce_admins": true,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": false,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 1,
    "require_last_push_approval": false
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "block_creations": false,
  "required_conversation_resolution": true,
  "lock_branch": false,
  "allow_fork_syncing": true
}
JSON

  set +e
  gh api \
    -X PUT \
    -H "Accept: application/vnd.github+json" \
    "/repos/$REPO/branches/$BRANCH/protection" \
    --input "$payload" >/dev/null 2>&1
  apply_rc=$?
  set -e
  rm -f "$payload"

  if [[ "$apply_rc" -ne 0 ]]; then
    echo "[branch-protection] apply failed (likely missing admin permission)." >&2
    echo "[branch-protection] falling back to auditable verify mode (non-zero on missing protections)." >&2
    verify_policy
    exit $?
  fi

  echo "[branch-protection] apply succeeded; running verification."
  verify_policy
  exit $?
fi

verify_policy
