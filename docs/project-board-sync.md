# Project Board Sync

## Purpose

`scripts/project-board-sync.sh` automates keeping the GitHub Project (v2) board
up to date by transitioning items linked to closed issues from "In Progress" (or
uncategorised) to "Done".

## Prerequisites

- `gh` CLI >= 2.40 installed and authenticated (`gh auth login`)
- Write access to the GitHub Project board
- The project must use a **Single Select** status field with an option named **"Done"**

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

Run this as a scheduled workflow or in response to issue-closed events:

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
      projects: write
    steps:
      - uses: actions/checkout@v4
      - run: ./scripts/project-board-sync.sh
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          PUML_PROJECT_TITLE: PUML
```

Note: `projects: write` permission is required for the token used.
Classic `GITHUB_TOKEN` may lack project write access depending on org settings;
in that case use a PAT with `project` scope stored as a repository secret.

## Limitations

- Only processes `ISSUE`-type items (draft notes are skipped).
- Requires the Status field to be a Single Select field named exactly `"Status"`.
- GraphQL fallback may be needed if `gh project item-list` output format changes
  across gh CLI versions.
- Does not handle the case where a project has no Status field.
