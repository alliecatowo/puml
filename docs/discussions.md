# GitHub Discussions setup

GitHub Discussions is enabled for this repository. Use it for community
conversation that should not start as an actionable issue.

## API support checked

Checked on 2026-05-17 with `gh api graphql` against `alliecatowo/puml`.
Also rechecked with REST on 2026-05-17:

- `GET /repos/alliecatowo/puml` reports `has_discussions: true`.
- `GET /repos/alliecatowo/puml/discussions/301` returns the welcome
  discussion in `Announcements`.

Live state on 2026-05-17:

- `#301` is globally pinned and titled `Welcome to puml Discussions`.
- Existing category slugs are `announcements`, `general`, `ideas`, `polls`,
  `q-a`, and `show-and-tell`.
- `Parity reports` and `Swarm lab` still need to be created in the UI.
- Fallback labels created through `gh label create`: `discussion: parity
  report` and `discussion: development notes`.

Available through GitHub GraphQL:

- Read repository discussion categories with `Repository.discussionCategories`.
- Read globally pinned discussions with `Repository.pinnedDiscussions`.
- Create, update, and delete discussion posts with `createDiscussion`,
  `updateDiscussion`, and `deleteDiscussion`.
- Add, update, and delete discussion comments.
- Mark and unmark accepted answers in Q&A categories.

Not exposed in the live GraphQL mutation schema or as first-class `gh`
commands:

- Create, edit, delete, or reorder discussion categories.
- Change category emoji, description, format, or section.
- Pin, unpin, or style pinned discussions.

Those actions must be completed in the GitHub web UI.

Discussion category forms are repo-managed files. The forms in
`.github/DISCUSSION_TEMPLATE/` are named after the target category slugs and
will apply once those categories exist on the default branch.
The `parity-reports` and `swarm-lab` forms also auto-apply the fallback labels
above after the matching categories exist.

## Target category set

Configure these categories in
<https://github.com/alliecatowo/puml/discussions/categories>.

| Category | Format | Purpose |
|---|---|---|
| Announcements | Announcement | Maintainer updates, releases, roadmap notes, and project-wide notices. |
| Q&A | Question and Answer | Usage help, support questions, PlantUML compatibility questions, and "how do I..." threads. |
| Ideas | Open-ended discussion | Feature proposals, PicoUML language ideas, renderer ergonomics, and early design sketches. |
| Show and tell | Open-ended discussion | Diagrams, integrations, benchmarks, demos, and writeups from users or contributors. |
| Parity reports | Open-ended discussion | PlantUML compatibility gaps that need clarification before they become scoped issues. |
| Swarm lab / Development notes | Open-ended discussion | Notes and questions about the AI-assisted development process, agent workflows, development process, and coordination experiments. |
| General | Open-ended discussion | Anything relevant to `puml` that does not fit another category. |

The default Polls category is optional. Delete it if the goal is to keep the
sidebar limited to the categories above; keep it only if maintainers expect to
run explicit community polls.

## Exact UI steps

1. Open <https://github.com/alliecatowo/puml/discussions>.
2. In the left sidebar, next to "Categories", click the edit pencil.
3. For existing categories, use the menu beside each category and choose
   "Edit" to update the title, description, emoji, and format.
4. Click "New category" for `Parity reports` and `Swarm lab`.
5. Use "Announcement" only for `Announcements`, "Question and Answer" only for
   `Q&A`, and "Open-ended discussion" for the remaining categories.
6. Keep category slugs aligned with the discussion form filenames:
   `q-a`, `ideas`, `show-and-tell`, `parity-reports`, `swarm-lab`, and
   `general`.
7. Delete `Polls` if it should not be part of the final category set. If GitHub
   asks where to move existing posts, move them to `General`.
8. Save each category.
9. Open the welcome discussion:
   <https://github.com/alliecatowo/puml/discussions/301>.
10. If it is not pinned globally, use the right sidebar action "Pin discussion",
   choose a simple style, and confirm. GitHub allows up to four globally pinned
   discussions.

## Issue vs discussion routing

Open an issue when the work is actionable:

- A reproducible bug or renderer mismatch with a small `.puml` input.
- A scoped compatibility gap with expected output or PlantUML evidence.
- A docs, CLI, site, LSP, WASM, or tooling task that someone can implement.
- A failing test, regression, or release-blocking defect.

Start a discussion when the work needs conversation first:

- A usage question or "how should this render?" question.
- A feature idea that needs shaping before implementation.
- A PlantUML parity report that needs examples, expected behavior, or priority
  discussion before it becomes an issue.
- A diagram, integration, benchmark, workflow, or demo to share.
- Notes about the AI-assisted development process or contributor workflow.

When a discussion produces concrete work, open an issue and link back to the
discussion. For Q&A threads, mark the best answer so future readers can find it
quickly. For long-running design or parity threads, add a maintainer summary
comment before creating follow-up issues.
