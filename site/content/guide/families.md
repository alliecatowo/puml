+++
title = "All diagram families"
description = "Every diagram family puml recognizes, with links into the gallery."
weight = 80
+++

`puml` is a multi-family engine. Some families have deep coverage, others have baseline rendering with partial-depth semantics. Current compatibility planning is tracked through executable examples, tests, and focused GitHub issues linked from [`docs/parity-roadmap.md`](https://github.com/alliecatowo/puml/blob/main/docs/parity-roadmap.md).

## Core UML families

| Family       | Status (high level)                               | Gallery folder |
|--------------|---------------------------------------------------|----------------|
| Sequence     | Core implemented, advanced rows partial           | `sequence/` |
| Class        | Core implemented, advanced rows partial           | `class/` |
| Object       | Implemented baseline                              | `object/` |
| Use case     | Implemented baseline                              | `usecase/` |
| Component    | Implemented baseline                              | `component/` |
| Deployment   | Implemented baseline                              | `deployment/` |
| State        | Implemented baseline                              | `state/` |
| Activity     | Modern syntax implemented; legacy partial         | `activity_new/`, `activity_old/` |
| Timing       | Implemented baseline                              | `timing/` |

## Non-UML families

| Family       | Status (high level)                               | Gallery folder |
|--------------|---------------------------------------------------|----------------|
| Gantt        | Baseline implemented                              | `gantt/` |
| Chronology   | Baseline implemented                              | `chronology/` |
| Salt (UI)    | Baseline implemented                              | `salt/` |
| Mindmap      | Baseline implemented                              | `mindmap/` |
| WBS          | Baseline implemented                              | `wbs/` |
| JSON         | Renders structured JSON                           | `json/` |
| YAML         | Renders structured YAML                           | `yaml/` |
| nwdiag       | Network diagrams                                  | `nwdiag/` |
| ArchiMate    | Baseline ArchiMate elements                       | `archimate/` |
| Regex        | Renders regex railroad                            | `regex/` |
| EBNF         | Renders grammar railroad                          | `ebnf/` |
| Chart        | Bar/line/pie variants                             | `chart/` |
| Math         | TeX-like math rendering                           | `math/` |
| SDL          | SDL family baseline                               | `sdl/` |
| Ditaa        | ASCII art &rarr; SVG                              | `ditaa/` |

## C4

C4 diagrams have a dedicated stdlib at [`stdlib/C4/`](https://github.com/alliecatowo/puml/tree/main/stdlib/C4) and example renders under `c4/`.

## Stdlib

`puml` ships a curated stdlib of icon sets and includes:

- `awslib14/` &mdash; AWS service icons (14th revision)
- `azure/` &mdash; Azure icons
- `gcp/` &mdash; Google Cloud icons
- `material/` &mdash; Material symbols
- `office/` &mdash; Microsoft Office icons
- `tupadr3/` &mdash; popular community icon collection

Include them with `!include <provider>/path/to/icon`.

## See it all

The [gallery](@/gallery.md) is the source of truth: every file you see there is a committed example pair `(family/name.puml, family/name.svg)`. Add another pair under `docs/examples/<family>/` and it surfaces on the next deploy.
