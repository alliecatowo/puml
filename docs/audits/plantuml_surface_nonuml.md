# PlantUML Non-UML Surface Inventory

Date: 2026-05-15
Scope: official PlantUML non-UML families + special diagrams for parity planning.
Runtime baseline in this repo: `puml` is sequence-only today and emits `"puml currently renders sequence diagrams only"` for non-sequence inputs (see `README.md` feature matrix and check-mode diagnostics behavior).

## Gantt
Official source: https://plantuml.com/gantt-diagram

Grouped features:
- Task lifecycle grammar: `requires`, `starts`, `ends`, one-line declarations (`and`), aliases (`as`), same-name tasks.
- Scheduling semantics: constraints (`starts at ...`), milestones (relative/absolute), simplified task succession, task-between-milestones.
- Calendar/time controls: daily/weekly/monthly/quarterly/yearly scales, zoom, week numbering/date modes, close days, working days, language of calendar.
- Presentation and metadata: completion %, color customization, separators, resources (allocation + hide modes), notes, links, title/header/footer/caption/legend, style system.

## MindMap
Official source: https://plantuml.com/mindmap-diagram

Grouped features:
- Multiple syntaxes: OrgMode, Markdown headers/indented list, arithmetic notation.
- Structure controls: multiroot maps, multiline nodes, branch direction, whole-diagram orientation controls.
- Visual controls: inline colors, style classes, boxless nodes, depth/node style selectors, word wrap.
- Shared commands: title/header/footer/legend/caption and Creole content support.

## WBS
Official source: https://plantuml.com/wbs-diagram

Grouped features:
- Core syntax variants: OrgMode and arithmetic notation.
- Hierarchy/flow controls: direction markers, multiline items, skipping layer, arrows between elements.
- Visual controls: inline/style colors, boxless modes, style system, word wrap.
- Shared commands: Creole support and general diagram metadata commands.

## Salt (Wireframe)
Official source: https://plantuml.com/salt

Grouped features:
- UI widgets and layout DSL: basic widgets, textarea, droplist, tree/tree-table, tabs, menus.
- Container/grid primitives: grid tokens (`|`, `#`, `!`, `-`, `+`), group box, separators, brackets.
- UX/presentation controls: scroll bars, colors, pseudo-sprites, OpenIconic, zoom/scale/DPI.
- Integration: title/header/footer/caption/legend, style/skinparam support, embedding in activity-diagram contexts.

## Archimate
Official source: https://plantuml.com/archimate-diagram

Grouped features:
- Native keyword support: `archimate` elements with stereotypes/icons and Archimate color domains.
- Relation modeling: relation vocabulary + directional connectors.
- Macro/library integration: `!include <archimate/Archimate>` standard-library macro surface for element/relationship shorthand.
- Junction and composition helpers: junction patterns and macro-based enterprise architecture assembly.

## nwdiag / Network
Official source: https://plantuml.com/nwdiag

Grouped features:
- Network grammar: `@startnwdiag`, `network` blocks, nodes, addresses, multi-address declarations.
- Grouping and topology: in-network and global groups, peer networks, shared nodes across multiple networks.
- Extended attributes: network/group/node attributes (color, description, shape, width), internal-network variants.
- Styling/integration: sprites/OpenIconic, title/header/footer/caption/legend, global styles, shadow toggles.

## JSON
Official source: https://plantuml.com/json

Grouped features:
- Standalone JSON diagrams: `@startjson`/`@endjson`, object/list rendering, numbers/strings/unicode/escape sequences.
- Targeted highlighting: `#highlight` path addressing and multi-style highlight classes.
- Styling: global `jsonDiagram` style controls.
- Cross-diagram projection: render JSON data into class/object, deployment/usecase/component/deployment, and state diagram contexts.

## YAML
Official source: https://plantuml.com/yaml

Grouped features:
- Standalone YAML diagrams: `@startyaml`/`@endyaml`, complex examples and symbol/unicode key handling.
- Targeted highlighting: YAML-path highlight directives and custom highlight styles.
- Styling: global `yamlDiagram` style controls.
- Text rendering: Creole content support in YAML views.

## Regex
Official source: https://plantuml.com/regex

Grouped features:
- Standalone regex diagrams: `@startregex`/`@endregex`.
- Regex core coverage: literals, shorthand classes, ranges, dot, escapes, repetitions, alternation.
- Internationalized labels: descriptive-name mode and language selection (`!option useDescriptiveNames`, `!option language`).
- Unicode handling: categories/scripts/blocks plus octal/unicode escape support.

## EBNF
Official source: https://plantuml.com/ebnf

Grouped features:
- Standalone grammar diagrams: `@startebnf`/`@endebnf` with rule/edge primitives.
- Extended syntax support: special sequence (`?`), repetition (`*`), optional/group/alternation constructs.
- Rendering modes/history: expanded mode baseline and compact-mode historical behavior.
- Authoring ergonomics: notes-on-elements/comments, global style controls, large grammar examples.

## Math (AsciiMath / JLaTeXMath)
Official source: https://plantuml.com/ascii-math

Grouped features:
- Inline math embedding: `<math>...</math>` and `<latex>...</latex>` inside diagram text.
- Standalone formula diagrams: `@startmath`/`@endmath` and `@startlatex`/`@endlatex`.
- Dual-engine model: AsciiMath conversion + JLaTeXMath rendering path.
- Versioned runtime notes: documented AsciiMath Java-port improvements and JLaTeXMath dependency expectations.

## SDL (Specification and Description Language)
Official source: https://plantuml.com/activity-diagram-beta#SDL-Specification-and-Description-Language-with-SDL-sterotype

Grouped features:
- Activity-syntax SDL rendering mode via alternate action terminators (`|`, `<`, `>`, `/`, `\\`, `]`, `}`).
- SDL stereotype mapping table (input/output/procedure/etc.) in beta activity grammar.
- SDL/UML shape interoperability through stereotype-based shape selection.
- Works inside broader activity grammar (fork/split/condition/grouping/style controls).

## Ditaa
Official source: https://plantuml.com/ditaa

Grouped features:
- Ditaa block integration (`@startditaa`/`@endditaa`) as ASCII-art-to-diagram pipeline.
- PlantUML option pass-through: separator/shadow/scale behavior controls.
- Tag/generalization support for ditaa blocks.
- Documentation linkage to external ditaa behavior notes.

## Chronology
Official source: https://plantuml.com/chronology-diagram

Grouped features:
- Standalone chronology diagrams: `@startchronology`/`@endchronology` activation.
- Natural-language timeline statements (`[item] happens on <datetime>` style).
- Task/milestone declaration model with timestamped events.
- Explicit positioning as Gantt-adapted timeline syntax.

## Chart
Official source: https://plantuml.com/chart-diagram

Grouped features:
- Standalone chart diagrams: `@startchart`/`@endchart`.
- Chart types: bar, line, area, scatter; grouped/stacked/horizontal bar variants.
- Axis system: h/v axes, secondary Y-axis, spacing/ticks/labels, coordinate-pair notation.
- Presentation controls: legends, grid lines, annotations, labels, inline colors, global chart style blocks.
- Version gate: documented as available starting PlantUML `1.2026.0`.

## Parity Implication For `puml`
Given current sequence-only runtime scope, all families above are currently **missing** at parser+model+render levels. Primary implementation workstreams are:
- Family-specific parser frontends and AST nodes.
- Family-specific layout/render engines (or adapters where appropriate).
- Conformance fixtures + deterministic snapshot contract per family.
- Diagnostics and CLI/dump compatibility for each new family start/end block.
