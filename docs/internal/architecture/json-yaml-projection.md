# JSON/YAML Projection Rendering

Issue: #733

## Decision

Standalone `@startjson` and `@startyaml` diagrams render structured data with a flat
key/value table by default. Nesting is shown by indentation and lightweight tree
connectors in the key column, while scalar/container values stay aligned in a stable
value column.

This replaces the previous visual treatment where every row was a full-width rounded
box offset by depth. The old shape was readable, but it missed PlantUML's table-like
JSON/YAML data view and made arrays look like stacked nested nodes rather than rows.

## Scope

This is intentionally a renderer slice, not a data renderer rewrite.

- The parser and normalized JSON/YAML tree shape stay unchanged.
- Existing `data-json-*` and `data-yaml-*` SVG metadata remains stable.
- Highlight paths and style patches continue to resolve through the existing row
  pipeline.
- Invalid JSON/YAML fallback rows still render as single-column rows.

## Remaining Gaps

- Inline JSON/YAML projection inside other UML families still uses the existing family
  projection path and is not made PlantUML-complete here.
- Global JSON/YAML styling remains limited to the existing highlight/style patches.
- Creole support is limited to the current inline text renderer.
- Very wide keys can still collide with the value column; adaptive column sizing is a
  future fidelity pass.
