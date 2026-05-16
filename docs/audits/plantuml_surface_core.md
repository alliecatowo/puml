# PlantUML Core Surface Audit (UML Families + Preprocessor)

Date: 2026-05-15

Scope: authoritative feature inventory for PlantUML core UML families plus preprocessing/theme/stdlib surface, mapped against current `puml` runtime behavior.

Legend:
- `implemented`: available in current `puml` behavior
- `partial`: some behavior exists but does not match PlantUML breadth/semantics
- `missing`: not implemented in current `puml`

Current `puml` baseline used for parity status:
- Sequence parsing/rendering is implemented.
- Class/object/usecase families now have bootstrap parser + model + deterministic stub SVG routing.
- Non-bootstrap families (`component`, `deployment`, `state`, `activity`, `timing`) return deterministic unsupported-family diagnostics with explicit family codes.
- Preprocessor support currently focuses on `!include` (local path), `!define`, `!undef`, and deterministic include safety checks.

## Class Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| Declarative element forms | Class-like declarations (`class`, `interface`, `enum`, etc.) | partial | Bootstrap supports `class` declarations/aliases only. | https://plantuml.com/class-diagram |
| Relationship syntaxes | Inheritance, implementation, composition, aggregation, dependency links | partial | Bootstrap captures generic arrow relations; no semantic relation typing yet. | https://plantuml.com/class-diagram |
| Relation labels and cardinality | Labeled links, cardinalities, directional labels | partial | Simple `: label` relation text is preserved; cardinality semantics are absent. | https://plantuml.com/class-diagram |
| Attributes/methods | Fields, methods, grouped class bodies | missing | Non-sequence families are rejected. | https://plantuml.com/class-diagram |
| Visibility markers | `+ - # ~` visibility and class visibility controls | missing | Non-sequence families are rejected. | https://plantuml.com/class-diagram |
| Abstract/static/interface semantics | Abstract/static members, interfaces, abstract classes | missing | Non-sequence families are rejected. | https://plantuml.com/class-diagram |
| Notes/stereotypes | Notes on classes, links, fields, methods; stereotype rendering | missing | Non-sequence families are rejected. | https://plantuml.com/class-diagram |
| Hide/remove controls | Hide/remove members/classes/tags/wildcards | missing | Non-sequence families are rejected. | https://plantuml.com/class-diagram |
| Packages/namespaces | Package and namespace blocks, automatic package creation | missing | Non-sequence families are rejected. | https://plantuml.com/class-diagram |
| Advanced relation/style controls | Generics, association class, lollipop, inline styles, orientation | missing | Non-sequence families are rejected. | https://plantuml.com/class-diagram |

## Object Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| Object declarations | Object instances and aliases | partial | Bootstrap supports `object` declarations/aliases. | https://plantuml.com/object-diagram |
| Object relations | Links between object instances | partial | Bootstrap captures generic object relation arrows and labels. | https://plantuml.com/object-diagram |
| Object associations | Association-style object links | partial | Association-like arrows are captured as generic relations only. | https://plantuml.com/object-diagram |
| Object fields | Adding attributes/fields to objects | missing | Non-sequence families are rejected. | https://plantuml.com/object-diagram |
| Map/associative-array syntax | Map table support in object diagrams | missing | Non-sequence families are rejected. | https://plantuml.com/object-diagram |
| PERT via map | PERT-style map usage in object surface | missing | Non-sequence families are rejected. | https://plantuml.com/object-diagram |
| JSON display crossover | JSON display support on class/object pages | missing | Non-sequence families are rejected. | https://plantuml.com/object-diagram |

## Use Case Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| Actor/usecase declarations | Actor and use case declarations | partial | Bootstrap supports `usecase` declarations/aliases (actor forms still missing). | https://plantuml.com/use-case-diagram |
| Actor styles | Actor visual style variants | missing | Non-sequence families are rejected. | https://plantuml.com/use-case-diagram |
| Usecase descriptions | Use case text/description forms | missing | Non-sequence families are rejected. | https://plantuml.com/use-case-diagram |
| Packaging | Use case packages/boundaries | missing | Non-sequence families are rejected. | https://plantuml.com/use-case-diagram |
| Relation semantics | Include/extend/generalization link patterns | partial | Bootstrap preserves generic relation arrows/labels without semantic include/extend handling. | https://plantuml.com/use-case-diagram |
| Notes/stereotypes | Notes and stereotype styling | missing | Non-sequence families are rejected. | https://plantuml.com/use-case-diagram |
| Arrow direction controls | Directional control of relation arrows | missing | Non-sequence families are rejected. | https://plantuml.com/use-case-diagram |
| Split diagrams | Splitting usecase diagrams across pages | missing | Non-sequence families are rejected. | https://plantuml.com/use-case-diagram |
| Layout direction | Left-to-right direction support | missing | Non-sequence families are rejected. | https://plantuml.com/use-case-diagram |
| Skin/style and JSON display | Skinparam, inline style, JSON-display examples | missing | Non-sequence families are rejected. | https://plantuml.com/use-case-diagram |

## Component Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| Component declarations | Component elements and aliases | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Interfaces | Provided/required interface notation | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Notes | Notes on components/relations | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Grouping | Grouping/packaging components | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Arrow direction | Arrow direction controls | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Notation variants | UML2/UML1/rectangle notation toggles | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Colors/sprites/stereotypes | Color controls and sprite stereotypes | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Skinparam controls | Component-specific skin parameters | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Hide/remove controls | Hide/remove unlinked/tagged component behaviors | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |
| Ports and JSON display | `port`, `portIn`, `portOut`, JSON display examples | missing | Non-sequence families are rejected. | https://plantuml.com/component-diagram |

## Deployment Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| Element declarations | Deployment nodes/artifacts/components/interfaces/etc. | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Short-form declarations | Abbreviated declaration forms | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Linking/arrows | Deployment links and arrow variants | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Bracketed arrow style | Bracketed line style controls | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Inline arrow styling | Per-link color/style overrides | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Inline element styling | Per-element color/style overrides | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Nesting/packages | Nestable elements and package nesting | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Alias and shape details | Alias behavior and round-corner controls | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Skinparam and ports | Deployment skinparams plus port primitives | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |
| Orientation and JSON display | Orientation controls and JSON display examples | missing | Non-sequence families are rejected. | https://plantuml.com/deployment-diagram |

## State Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| State declarations/transitions | Basic states and transitions | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| State rendering modes | Alternate state rendering forms | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Composite states | Nested/composite states | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Long names/descriptions | Long state names and descriptions | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| History states | `[H]` and `[H*]` history nodes | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Fork/join and concurrency | Fork/join and concurrent state separators | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Conditional/choice | Choice states and conditional branches | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Entry/exit/pin/expansion points | entryPoint/exitPoint/inputPin/outputPin/expansion forms | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Arrow style controls | Direction, color/style, head/tail customization | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Notes and inline colors | Notes on state/link, inline colors | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Skin/style controls | skinparam and style controls for states | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |
| Aliases and JSON display | Alias support and JSON display examples | missing | Non-sequence families are rejected. | https://plantuml.com/state-diagram |

## Activity Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| New syntax actions | Simple/list actions and flow syntax | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Start/stop/end | Start/end markers and flow termination | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| If/then/else | Conditional branching blocks | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Switch/case | Switch/case/endswitch constructs | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Repeat/while loops | Repeat/while plus loop controls (`break`) | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Goto/label | Label and goto processing | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Parallel/split processing | Fork/fork again/end fork/end merge and split processing | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Notes/connectors/arrows | Notes, connector circles, line/arrow styling | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Partition/swimlanes | Grouping partitions and swimlanes | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Kill/detach | Activity stop controls (`kill`, `detach`) | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Global style/Creole | Global style directives and Creole text support | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-beta |
| Legacy syntax support | Legacy activity syntax remains documented | missing | Non-sequence families are rejected. | https://plantuml.com/activity-diagram-legacy |

## Timing Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| Participant/timeline declaration | Robust/concise/clock/binary style participants | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| Binary/clock tracks | Binary and clock signal support | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| Time/message statements | Adding messages and timeline transitions | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| Relative/anchored time | Relative time and anchor points (including decimal offsets) | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| Scale and axis controls | Scale setup, axis labels, hide time axis | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| State forms | Initial/intricated/hidden/negative-time states | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| Date/time formatting | Time/date usage and date-format controls | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| Constraints and highlighted periods | Constraints and highlighted period windows | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| Notes/text styling | Notes, added text, line color/style control | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |
| Analog signal customization | Compact mode, analog scaling/customization, state ordering | missing | Non-sequence families are rejected. | https://plantuml.com/timing-diagram |

## Sequence Family

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| Basic messages and participants | Core sequence grammar and message lines | implemented | Implemented end-to-end. | https://plantuml.com/sequence-diagram |
| Participant role keywords | `participant`, `actor`, `boundary`, `control`, `entity`, `database`, `collections`, `queue` | implemented | Role parsing implemented. | https://plantuml.com/sequence-diagram |
| Message-to-self | Self-message syntax | implemented | Supported by same message pipeline. | https://plantuml.com/sequence-diagram |
| Arrow style breadth | Full PlantUML arrow catalog including uncommon variants | partial | Subset implemented; many variants absent. | https://plantuml.com/sequence-diagram |
| Arrow color/style controls | Inline arrow color/style forms | partial | Limited via subset and selected skinparams. | https://plantuml.com/sequence-diagram |
| Autonumbering | Message numbering commands | partial | Deterministic subset implemented; full format surface is narrower. | https://plantuml.com/sequence-diagram |
| Title/header/footer | Page title/header/footer commands | implemented | Implemented in model/page output. | https://plantuml.com/sequence-diagram |
| Splitting diagrams (`newpage`) | Multipage sequence support | implemented | Core multipage eventing implemented, with CLI contract differences. | https://plantuml.com/sequence-diagram |
| `ignore newpage` | Ignore-page-split directive | implemented | Parsed and applied during normalize. | https://plantuml.com/sequence-diagram |
| Grouping messages | `alt/else/opt/loop/par/critical/break/group/end` | partial | Core group forms implemented; broader PlantUML semantics remain narrower. | https://plantuml.com/sequence-diagram |
| References (`ref`) | `ref over ...` syntax | partial | Baseline ref support exists but is not full-surface equivalent. | https://plantuml.com/sequence-diagram |
| Notes | `note`, `hnote`, `rnote`, `left/right/over/across` | partial | Parsed as note forms; shape/styling breadth is reduced. | https://plantuml.com/sequence-diagram |
| Divider/separator/delay/space | `==`, `...`, `||`, and spacing constructs | implemented | Implemented in parse/normalize/layout/render. | https://plantuml.com/sequence-diagram |
| Lifeline activate/deactivate/destroy | Lifecycle commands and shortcuts | implemented | Explicit + shortcut lifecycle flows are implemented. | https://plantuml.com/sequence-diagram |
| Return statements | `return` handling | implemented | Return inference and explicit labels supported. | https://plantuml.com/sequence-diagram |
| Participant creation | `create` and lifecycle creation flows | implemented | Implemented in normalize lifecycle rules. | https://plantuml.com/sequence-diagram |
| Incoming/outgoing endpoints | Short endpoint arrows and endpoint variants | partial | Endpoint forms supported with reduced fidelity semantics. | https://plantuml.com/sequence-diagram |
| Skinparam for sequence | Sequence skinparam surface | partial | Selected keys implemented; unsupported keys warn deterministically. | https://plantuml.com/sequence-diagram |
| `!theme` usage in sequence | Theme inclusion in sequence diagrams | partial | Local built-in catalog subset (`plain`, `spacelab`) is applied; unsupported/remote forms reject deterministically. | https://plantuml.com/sequence-diagram |
| Teoz/parallel-message layout options | Teoz-specific behaviors | missing | `!pragma` lines are ignored; no dedicated teoz semantics. | https://plantuml.com/sequence-diagram |

## Preprocessor / Includes / Import / Theme / Stdlib

| Feature | PlantUML surface | `puml` status | Notes | Source |
|---|---|---|---|---|
| Variable definition (`!$var`, `=`, `?=`) | Preprocessor variable assignment | partial | Deterministic assignment/reference subset implemented (`=` and `?=`) with scoped text expansion. | https://plantuml.com/preprocessing |
| Boolean expressions | Boolean evaluation in preprocessor | partial | Simple deterministic subset implemented (`defined()`, `==`, `!=`, numeric/bool literals); broader expression language is missing. | https://plantuml.com/preprocessing |
| Conditional blocks | `!if` / `!elseif` / `!else` / `!endif` | partial | Implemented for deterministic subset with explicit balance/order diagnostics. | https://plantuml.com/preprocessing |
| Preprocessor loops | `!while` / `!endwhile` | partial | Implemented for deterministic subset with bounded iteration guard; advanced loop semantics are missing. | https://plantuml.com/preprocessing |
| Procedures | `!procedure` / `!endprocedure` | partial | Definitions plus deterministic call/argument subset implemented; unsupported forms fail with stable diagnostics. | https://plantuml.com/preprocessing |
| Functions | `!function` / `!endfunction` | partial | Definitions plus `%name(...)` call/`!return` subset implemented with deterministic boundary diagnostics. | https://plantuml.com/preprocessing |
| Default args / keyword args / unquoted | Advanced proc/function argument handling | partial | Deterministic positional/default/keyword subset implemented; mismatches rejected with stable diagnostics. | https://plantuml.com/preprocessing |
| `!define` | Macro/define directives | partial | Token-substitution subset implemented. | https://plantuml.com/preprocessing |
| `!undef` | Undefine macro variables | partial | Supported in current substitution model. | https://plantuml.com/preprocessing |
| Local file include | `!include` file includes | partial | Local include works with include-root safety model. | https://plantuml.com/preprocessing |
| URL include | `!include` URL targets and `!includeurl` style use | missing | URL include is explicitly rejected. | https://plantuml.com/preprocessing |
| Include multiplicity controls | `!include_many`, `!include_once` | missing | Not implemented. | https://plantuml.com/preprocessing |
| Subpart include commands | `!startsub` / `!endsub` / `!includesub` | partial | Tagged extraction via `file!TAG` exists; full command surface absent. | https://plantuml.com/preprocessing |
| Builtin `%` functions | Builtin preprocessor function surface | missing | Function family not implemented. | https://plantuml.com/preprocessing |
| `!log` | Preprocessor logging | missing | Not implemented. | https://plantuml.com/preprocessing |
| `!dump_memory` | Memory dump directive | missing | Not implemented. | https://plantuml.com/preprocessing |
| `!assert` | Assertion directive | missing | Not implemented. | https://plantuml.com/preprocessing |
| `!import` library building | Import/build custom library | missing | Not implemented. | https://plantuml.com/preprocessing |
| Search path behavior | Preprocessor search path semantics | partial | `include_root` exists but broader search-path surface is absent. | https://plantuml.com/preprocessing |
| Argument concatenation `##` | Macro argument concatenation | partial | Explicit deterministic rejection implemented (`E_PREPROC_CONCAT_UNSUPPORTED`) in callable signatures/call args. | https://plantuml.com/preprocessing |
| Dynamic invocation | `%invoke_procedure`, `%call_user_func` | missing | Not implemented. | https://plantuml.com/preprocessing |
| JSON preprocessing | JSON variable/object preprocessing surface | missing | Not implemented. | https://plantuml.com/preprocessing-json |
| `!theme` include | Theme include command | partial | Deterministic local-catalog semantics implemented for sequence (`plain`, `spacelab`); remote/source forms are rejected. | https://plantuml.com/preprocessing |
| Theme catalog usage | Built-in theme listing/local/internet themes | partial | Local built-in catalog subset only; broader catalog/listing/fetch remains unimplemented. | https://plantuml.com/theme |
| Stdlib catalog usage | Standard library include/catalog (`awslib`, `C4`, etc.) | missing | Stdlib resolution and library loading not implemented. | https://plantuml.com/stdlib |
| Include identifiers in sources docs | Include identifier and include-definition patterns in source files | partial | Some tag extraction supported; full sources-surface parity is not. | https://plantuml.com/sources |
