+++
title = "User guide"
description = "Everything you need to author and render UML with puml."
sort_by = "weight"
template = "section.html"
page_template = "page.html"
+++

`puml` is a diagram compiler. You write `.puml` source &mdash; in PicoUML, PlantUML, or Mermaid syntax &mdash; and `puml` emits deterministic SVG (and, optionally, PNG).

This guide walks you from a one-line "hello" diagram through every supported family, then into the CLI and styling primitives you'll reach for daily. Every snippet on these pages is also a fixture in the [gallery](@/gallery.md) and a test in the repo, so what you read here is what the compiler executes.

## Start here

- [Getting started](@/guide/getting-started.md) &mdash; install, render your first diagram, integrate with your editor.
- [CLI reference](@/guide/cli.md) &mdash; modes, flags, exit codes, diagnostic streams.
- [Syntax primer](@/guide/syntax.md) &mdash; the shared core all three dialects compile down to.

## Diagram families

- [Sequence diagrams](@/guide/sequence.md)
- [Class diagrams](@/guide/class.md)
- [Activity diagrams](@/guide/activity.md)
- [State diagrams](@/guide/state.md)
- [All diagram families](@/guide/families.md) &mdash; one-line summaries with links into the gallery for each.

## Styling and integration

- [Themes and styling](@/guide/themes.md)
- [Markdown fences](@/guide/markdown-fences.md)
