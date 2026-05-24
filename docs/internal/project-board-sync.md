# Project Board Sync

## Purpose

`scripts/project-board-sync.sh` is the manual/backfill sweep for keeping the
GitHub Project (v2) board up to date by transitioning items linked to closed
issues from "In Progress" (or uncategorised) to "Done".

This is a maintainer-only workflow unless the agent environment already has a
token with GitHub Projects v2 read/write scope. Normal implementation agents
should use `gh issue list`, `gh issue view`, and focused issue labels. They
should not block, retry, or invent workarounds when `gh project` reports missing
`project` / `read:project` scope.

`.github/workflows/project-sync.yml` and `scripts/project-v2-event-sync.sh` are
the event-driven fallback automation for the PUML board. GitHub's GraphQL API
can list Projects v2 workflows, but it does not currently expose create/update
mutations for enabling native workflow rules. The repo workflow therefore edits
only the existing `Status` and `Priority` fields on issue/PR events.

## Prerequisites

- `gh` CLI >= 2.40 installed and authenticated (`gh auth login`)
- Write access to the GitHub Project board. Repository issue access is not enough.
- The project must use a **Single Select** status field with an option named **"Done"**

## Live board state checked

Checked on 2026-05-17 with `gh api graphql` and `gh project item-list`:

- User project `alliecatowo/3` is titled `PUML`, public, open, and has ID
  `PVT_kwHOBdlpmc4BX1zk`.
- Project `shortDescription` and `readme` were set through `updateProjectV2`
  to describe the board and token-backed repo automation.
- Native workflows enabled: `Auto-add sub-issues to project` and
  `Auto-add to project`.
- Native workflows disabled: `Item added to project`, `Item closed`,
  `Pull request linked to issue`, `Pull request merged`, and `Auto-close issue`.
- The live GraphQL mutation schema exposes `deleteProjectV2Workflow`, but no
  create/update/enable mutation for Project v2 workflows.
- Status and Priority field IDs/options in `.github/workflows/project-sync.yml`
  match the live board.
- The `Priority` field accepts `P0`, `P1`, `P2`, and `P3`; `P3` was added on
  2026-05-17 to match the existing repository label and reduce manual board
  cleanup for lower-priority issues.

GitHub's current Projects docs support the repo-side approach used here:
GraphQL can update project items and fields, built-in workflows are enabled in
the project UI, and Actions/user tokens are the practical bridge for user-owned
Project v2 boards.

## Usage

```bash
# Report board/PR hygiene without mutating Project #3
./scripts/project-board-hygiene.py

# Return non-zero when stale board items or PR body issues are found
./scripts/project-board-hygiene.py --fail-on-findings

# Preview what would change (no mutations)
./scripts/project-board-sync.sh --dry-run

# Apply changes to the default "PUML" project
./scripts/project-board-sync.sh

# Target a different project title
./scripts/project-board-sync.sh --project-title "My Board"

# Target a specific org/owner
./scripts/project-board-sync.sh --owner myorg
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `GITHUB_OWNER` | `@me` | GitHub login or org owning the project |
| `PUML_PROJECT_TITLE` | `PUML` | Project board title to match |

## Hygiene Report

`scripts/project-board-hygiene.py` is the lightweight read-only sweep for issue
and PR board hygiene. It reports:

- closed issues and merged PRs on Project #3 whose `Status` is not `Done`
- open issues and open PRs whose Project #3 `Status` is blank
- open PRs whose body contains neither `Closes #...` nor the explicit phrase
  `Does not close an issue`

The default command is report-only:

```bash
./scripts/project-board-hygiene.py
```

Useful variants:

```bash
# Machine-readable output for CI or local scripting
./scripts/project-board-hygiene.py --json

# Gate on findings without editing the board
./scripts/project-board-hygiene.py --fail-on-findings

# Preview setting stale closed/merged items to Done
./scripts/project-board-hygiene.py --apply-done

# Actually edit stale closed/merged items to Done
./scripts/project-board-hygiene.py --apply-done --no-dry-run
```

`--apply-done` only updates stale closed issue / merged PR items. It does not
guess statuses for blank active items, and it does not edit PR descriptions.
Mutation remains dry-run by default. To apply changes, authenticate `gh` with a
token that can read repository issues/PRs and write the user-owned Projects v2
board. In practice for this repo that means `repo` plus `project` on a classic
PAT, or equivalent fine-grained access where GitHub supports Project v2 writes.

The script defaults to `alliecatowo/puml` and Project `alliecatowo/3`. Override
with `--repo`, `--project-owner`, or `--project-number` when testing elsewhere.
Because `gh project item-list` does not consistently include linked issue/PR
state, the script enriches non-Done items with `gh issue view` / `gh pr view`.
Use `--skip-state-enrichment` only for fixture/debug runs where state is already
present in the input JSON.

## What It Does

1. Lists all GitHub Projects v2 for `$GITHUB_OWNER` via `gh project list`.
2. Resolves the project number for the title matching `$PUML_PROJECT_TITLE`
   (case-insensitive, partial match fallback).
3. Fetches all project items via `gh project item-list`.
4. Resolves the `Status` field ID and the `Done` option ID via GraphQL so the
   update call is typed correctly.
5. For each item in status `"In Progress"` or `""` / `"?"`:
   - If the linked issue is CLOSED (`gh issue view #N --json state`), calls
     `gh project item-edit` to set its status to Done.
   - Otherwise, skips it.
6. Prints a summary: updated / already-done / skipped / errors.

## CI Integration

The event-driven workflow is installed as `.github/workflows/project-sync.yml`.
It runs on issue and pull request events and can also be run manually with a
specific issue or PR URL.

Behavior:

- issue opened/reopened -> `Status: Todo`
- issue closed -> `Status: Done`
- draft PR opened/updated -> `Status: In Progress`
- ready PR opened/reopened/ready/synchronized -> `Status: Merging`
- PR merged -> `Status: Done`
- closing issues referenced by an open PR -> `Status: Merging`
- closing issues referenced by a merged PR -> `Status: Done`
- labels named `P0`, `P1`, `P2`, or `P3` -> matching `Priority`

Recommended native workflows to enable in the project UI when a maintainer is
available:

| Workflow | Recommended state | Reason |
|---|---:|---|
| Auto-add to project | On | Already on; keeps newly matching repo items from being missed. |
| Auto-add sub-issues to project | On | Already on; keeps parent/sub-issue work visible. |
| Item closed | On | Low-risk duplicate safety for moving closed issues/PRs to Done. |
| Pull request merged | On | Low-risk duplicate safety for moving merged PRs to Done. |
| Pull request linked to issue | On | Useful if the project should surface PR-linked implementation work immediately. |
| Item added to project | Optional | Set to `Todo` only if manually added draft/triage items should always start there. |
| Auto-close issue | Off | Higher risk because changing board state would close issues. |

The workflow uses `secrets.PUML_PROJECT_TOKEN`. For user-owned Projects v2
boards, a PAT with `project` scope is required in practice because repository
`GITHUB_TOKEN` cannot add or edit items on the user project. If the secret is
missing or unavailable for an event, the workflow exits successfully with a
warning instead of failing unrelated PRs.

Live check on 2026-05-17: the repository Actions secrets API reported
`total_count: 0`, so `PUML_PROJECT_TOKEN` was not configured at that time.
Recent `Project Sync` workflow runs completed successfully by executing the
skip step and skipping the project edit steps.

The older sweep can still be run as a scheduled workflow or manually:

```yaml
on:
  issues:
    types: [closed]
  schedule:
    - cron: '0 9 * * 1'   # every Monday 09:00 UTC

jobs:
  sync:
    runs-on: ubuntu-latest
    permissions:
      issues: read
      repository-projects: write
    steps:
      - uses: actions/checkout@v4
      - run: ./scripts/project-board-sync.sh
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          PUML_PROJECT_TITLE: PUML
```

Note: project write permission is required for the token used. Classic
`GITHUB_TOKEN` may lack user Project v2 write access depending on owner/repo
settings; in that case use `PUML_PROJECT_TOKEN`.

## Limitations

- Only processes `ISSUE`-type items (draft notes are skipped).
- Requires the Status field to be a Single Select field named exactly `"Status"`.
- GraphQL fallback may be needed if `gh project item-list` output format changes
  across gh CLI versions.
- Does not handle the case where a project has no Status field.
- Native Projects v2 workflow rules such as "Item closed", "Pull request
  merged", and "Pull request linked to issue" remain UI-managed; the GraphQL
  schema currently exposes them as readable/deletable, not configurable.
- GitHub's current Actions guidance says repository `GITHUB_TOKEN` is scoped to
  the repository and cannot access Projects; user projects should use a personal
  access token saved as a secret.
- The hygiene report can be slower than `gh project item-list` alone because it
  enriches non-Done items with linked issue/PR state before classifying them.
