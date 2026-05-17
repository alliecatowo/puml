# Project Board Sync

## Purpose

`scripts/project-board-sync.sh` is the manual/backfill sweep for keeping the
GitHub Project (v2) board up to date by transitioning items linked to closed
issues from "In Progress" (or uncategorised) to "Done".

`.github/workflows/project-sync.yml` and `scripts/project-v2-event-sync.sh` are
the event-driven fallback automation for the PUML board. GitHub's GraphQL API
can list Projects v2 workflows, but it does not currently expose create/update
mutations for enabling native workflow rules. The repo workflow therefore edits
only the existing `Status` and `Priority` fields on issue/PR events.

## Prerequisites

- `gh` CLI >= 2.40 installed and authenticated (`gh auth login`)
- Write access to the GitHub Project board
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

## Usage

```bash
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
