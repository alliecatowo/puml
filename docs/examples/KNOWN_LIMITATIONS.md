# Known Limitations

This document describes diagram families and syntax features that are not yet supported
by the PicoUML parser/renderer. Examples are provided for reference but cannot be
rendered to SVG with the current `puml` CLI.

## Families with SVG Render Support

The following families produce SVG output when rendered:

| Family | Render Type | Notes |
|---|---|---|
| sequence | Full SVG | Complete layout and rendering |
| class | Stub SVG | Parsed and normalized; rendered as a stub listing nodes/relations |
| object | Stub SVG | Parsed and normalized; rendered as a stub |
| usecase | Stub SVG | Parsed and normalized; rendered as a stub |
| state | Full SVG | Complete state machine rendering including nested/concurrent |
| gantt | Timeline SVG | Rendered as a text-based timeline listing |
| chronology | Timeline SVG | Rendered as a text-based event timeline |

## Families: Parsed but No SVG Render (E_FAMILY_*_UNSUPPORTED)

These families are syntactically parsed by the PicoUML parser but the renderer returns
`E_FAMILY_*_UNSUPPORTED` — no SVG is produced. Example files are provided for syntax
reference only.

| Family | Error Code | Status |
|---|---|---|
| component | E_FAMILY_COMPONENT_UNSUPPORTED | Parser recognizes `component`, `interface`, `port` keywords |
| deployment | E_FAMILY_DEPLOYMENT_UNSUPPORTED | Parser recognizes `node`, `artifact`, `cloud`, `frame`, `storage` |
| activity (old-style) | E_RENDER_ACTIVITY_UNSUPPORTED | Parser recognizes old `(*) -->`, `\|lane\|`, `#color:action;` syntax |
| timing | E_FAMILY_TIMING_UNSUPPORTED | Parser recognizes `robust`, `concise`, `clock`, `binary` keywords |
| mindmap | E_FAMILY_MINDMAP_UNSUPPORTED | Parser recognizes `@startmindmap`/`@endmindmap` blocks |
| wbs | E_FAMILY_WBS_UNSUPPORTED | Parser recognizes `@startwbs`/`@endwbs` blocks |

## Families: Not Yet in Parser (No @start* Support)

These PlantUML-compatible diagram types have no `@start*`/`@end*` markers in the parser.
The files exist as syntax reference documentation only.

| Family | @start marker | Reason |
|---|---|---|
| activity (new-style) | `@startuml` with `start`/`:action;`/`stop` | `start`/`stop` keywords detected as Activity but new-style graph not implemented |
| salt | `@startsalt` | No salt block parser implemented |
| json | `@startjson` | No json block parser; content treated as unknown/timing |
| yaml | `@startyaml` | No yaml block parser |
| nwdiag | `@startnwdiag` | No nwdiag block parser |
| archimate | ArchiMate macros in `@startuml` | ArchiMate `archimate` keyword not recognized |
| regex | `@startregex` | No regex block parser |
| ebnf | `@startebnf` | No EBNF block parser |
| chart | `@startchart` | No chart block parser |
| math | `@startmath` | No math/LaTeX block parser |
| sdl | `@startsdl` | No SDL block parser |
| ditaa | `@startditaa` | No ditaa/ASCII-art block parser |

## Syntax Limitations within Supported Families

### Sequence Diagrams
- `box "Label" #Color` — the `box` grouping keyword is not supported; omit the box wrapper
- `note left:` / `note right:` inline shorthand — use `note right of <participant>:` instead
- `note left` multiline without participant — use `note right of <participant>` form
- `abstract class` keyword — only plain `class` keyword is recognized

### Class Diagrams
- `abstract class` / `interface` keywords — `interface` triggers Component family detection; use plain `class`
- `package "..." { }` grouping blocks — not supported; omit the package wrapper
- `..>` dependency arrows — not recognized; use `-->` instead
- `+` / `-` / `#` / `~` visibility sigils on **standalone lines** (outside a `{}` block) — these trigger MindMap detection

### Themes
- Only two themes are built-in: `plain` and `spacelab`
- `!theme <name> from <url>` — remote theme sources are not supported
- Other PlantUML theme names (aws-orange, blueprint, cerulean, hacker, etc.) return `E_THEME_UNKNOWN`

### Skinparams
- Supported skinparams (sequence diagrams only): `arrowColor`, `lifelineBorderColor`,
  `participantBackgroundColor`, `participantBorderColor`, `noteBackgroundColor`,
  `noteBorderColor`, `groupBackgroundColor`, `groupBorderColor`, `footbox`, `maxMessageSize`
- All other skinparam keys are silently ignored (UnsupportedKey) or emit a warning

### Preprocessor
- `%invoke_procedure(...)` and `%call_user_func(...)` — dynamic invocation not supported
- `!$foo = { "k": 1 }` — JSON variable assignment not supported
- `##` concatenation operator in function parameter names — not supported
- `%true()`, `%boolval()` and other builtin functions — not supported in assert/log context

### Creole / Text Formatting
- Inline formatting (`**bold**`, `//italic//`, etc.) is passed through in message labels
  but is not applied in SVG rendering (labels are rendered as plain text)
- `<color:...>`, `<size:...>`, `<font:...>` HTML-like tags are not rendered
- `note right` / `note left` require `of <participant>` — standalone position-only notes are invalid
