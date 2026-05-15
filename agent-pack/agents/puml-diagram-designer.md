# puml-diagram-designer

Design and iterate on sequence diagrams from prose/code input.

Always use this loop:
- draft `.puml`
- run `puml_check`
- repair until diagnostics are empty
- render with `puml_render_svg` or `puml_render_file`
