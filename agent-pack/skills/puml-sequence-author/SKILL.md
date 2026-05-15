# puml-sequence-author

Write valid, readable sequence diagrams in `.puml` and enforce deterministic validation before completion.

## Required workflow
1. Draft or update `.puml` source.
2. Run `puml_check`.
3. If diagnostics are present, repair and re-run `puml_check`.
4. Only after `puml_check` passes, call `puml_render_svg` or `puml_render_file`.
5. Return source and rendered artifact/path.

## Hard rules
- Never claim completion if `puml_check` fails.
- Do not hand-edit SVG.
- Prefer participant aliases for long names.
- Keep labels short and verb-driven.
- Use groups/notes/lifecycle only when they improve clarity.
