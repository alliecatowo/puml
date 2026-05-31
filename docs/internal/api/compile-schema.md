# PUML Compile API and Worker Protocol — Schema Reference

**Version:** 1  
**Source:** `src/api/compile.rs`, `src/api/worker.rs`  
**Stability:** Additive-only. Existing fields are never renamed or removed in a semver-compatible release. Unknown fields must be ignored by consumers.

---

## Overview

`puml::compile(source)` is the single stable entry point for cross-frontend diagram compilation. It runs the full parse → normalize → render pipeline and returns a [`CompileResult`](#compileresult) DTO that is JSON-serialisable and byte-stable.

The worker protocol (`src/api/worker.rs`) wraps `compile` and the other language-service operations behind a request/response envelope suitable for browser Web Workers and VS Code extension host workers.

---

## `CompileResult`

Returned by `puml::compile` and `puml::compile_with_options`.

```jsonc
{
  "ok": true,             // bool — false when at least one error diagnostic was emitted
  "family": "sequence",   // string — detected diagram family (see families below)
  "svg_pages": ["<svg…"], // array<string> — rendered SVG pages; empty when ok=false
  "diagnostics": […],     // array<DiagnosticDto>
  "model_summary": {…},   // ModelSummary
  "semantic_tokens": […], // array<SemanticTokenDto>
  "symbols": […],         // array<DocumentSymbolDto>
  "language_service": {…} // LanguageServiceSurface
}
```

### `family` values

| Value | Diagram type |
|---|---|
| `"sequence"` | Sequence diagram |
| `"class"` | Class diagram |
| `"state"` | State diagram |
| `"activity"` | Activity diagram |
| `"component"` | Component diagram |
| `"deployment"` | Deployment diagram |
| `"usecase"` | Use-case diagram |
| `"object"` | Object diagram |
| `"timing"` | Timing diagram |
| `"gantt"` | Gantt chart |
| `"mindmap"` | Mind map |
| `"wbs"` | Work breakdown structure |
| `"nwdiag"` | Network diagram |
| `"chen"` | Chen ER diagram |
| `"board"` | Board / kanban |
| `"files"` | File tree |
| `"wire"` | Wireframe |
| `"json"` | JSON visualisation |
| `"yaml"` | YAML visualisation |
| `"archimate"` | ArchiMate |
| `"unknown"` | Unrecognised input |

---

## `DiagnosticDto`

```jsonc
{
  "code": "E_PARSE",    // string|null — bracket-prefix error code if present
  "severity": "error",  // "error" | "warning"
  "category": "parse-error", // see categories below
  "message": "…",       // string — human-readable message (no [CODE] prefix)
  "span": {             // object|null — byte-offset span in source
    "start": 10,        // inclusive start byte
    "end": 25           // exclusive end byte
  },
  "line": 2,            // number|null — one-based line number
  "column": 1           // number|null — one-based column number
}
```

### `category` values

| Value | Meaning |
|---|---|
| `"parse-error"` | Syntax parse failure |
| `"unsupported-syntax"` | Recognised but unimplemented construct |
| `"malformed-syntax"` | Well-formed but semantically invalid |
| `"deferred-raw"` | Syntax deferred as raw passthrough |
| `"benign-passthrough"` | Passthrough with no functional impact |
| `"feature-loss"` | Cross-frontend feature that has no equivalent |
| `"warning"` | Generic warning |
| `"other"` | Uncategorised |

---

## `ModelSummary`

High-level counts of structural elements in the diagram.

```jsonc
{
  "kind": "sequence",   // string — family name (same as CompileResult.family)
  "warning_count": 0,   // number — warnings during normalisation
  "node_count": 2,      // number — participants / nodes / columns depending on family
  "edge_count": 1,      // number — messages / edges / relations depending on family
  "title": "My Diagram" // string|null — title from `title …` directive
}
```

---

## `SemanticTokenDto`

```jsonc
{
  "start": 10,         // number — byte offset of token start (inclusive)
  "end": 15,           // number — byte offset of token end (exclusive)
  "kind": "keyword"    // "keyword" | "operator"
}
```

---

## `DocumentSymbolDto`

```jsonc
{
  "name": "Alice",         // string — symbol name
  "kind": "participant",   // "participant" | "message" | "unknown"
  "span": {
    "start": 10,
    "end": 27
  }
}
```

---

## `LanguageServiceSurface`

Static capability flags. All are `true` in the current implementation.

```jsonc
{
  "hover": true,
  "completion": true,
  "diagnostics": true,
  "semantic_tokens": true,
  "document_symbols": true,
  "formatting": true,
  "definition": true,
  "references": true,
  "rename": true
}
```

---

## Worker Protocol

### Request envelope

```jsonc
{
  "id": "req-001",       // string — caller-assigned correlation ID
  "version": 1,          // number — protocol version; must be 1
  "payload": {
    "op": "compile",     // see operations below
    // operation-specific fields
  }
}
```

### Response envelope

```jsonc
{
  "id": "req-001",       // string — echoed from request
  "version": 1,          // number — always 1
  "ok": true,            // bool — false for infrastructure errors only
  "error": "…",          // string — present only when ok=false
  "payload": {
    "op": "compile",     // echoes request op
    // operation-specific result
  }
}
```

### Operations

#### `compile`

Request:
```jsonc
{ "op": "compile", "source": "…", "frontend": null }
```
- `frontend`: `null` (auto-detect), `"plantuml"`, `"mermaid"`, or `"picouml"`.

Response payload: [`CompileResult`](#compileresult) object with `"op": "compile"` added.

---

#### `render`

Request:
```jsonc
{ "op": "render", "source": "…", "format": "svg" }
```
- `format`: always `"svg"` in the worker protocol; other formats are CLI-only.

Response payload:
```jsonc
{ "op": "render", "svg_pages": ["<svg…"] }
```

---

#### `hover`

Request:
```jsonc
{ "op": "hover", "source": "…", "line": 2, "column": 5 }
```
- `line` and `column` are one-based.

Response payload:
```jsonc
{ "op": "hover", "markdown": "…" }
```
- `markdown` is `null` when no hover information is available.

---

#### `completion`

Request:
```jsonc
{ "op": "completion", "source": "…", "line": 2, "column": 5 }
```

Response payload:
```jsonc
{
  "op": "completion",
  "is_incomplete": false,
  "items": [
    {
      "label": "->",
      "kind": "operator",
      "detail": "…",
      "documentation": "…"
    }
  ]
}
```

---

#### `diagnostics`

Request:
```jsonc
{ "op": "diagnostics", "source": "…" }
```

Response payload:
```jsonc
{
  "op": "diagnostics",
  "diagnostics": [
    {
      "code": "E_PARSE",
      "severity": "error",
      "category": "parse-error",
      "message": "…",
      "start_line": 2,
      "start_column": 1,
      "end_line": 2,
      "end_column": 10
    }
  ]
}
```
All `start_*` / `end_*` fields are `null` when no position is available.

---

#### `semanticTokens`

Request:
```jsonc
{ "op": "semanticTokens", "source": "…" }
```

Response payload:
```jsonc
{
  "op": "semanticTokens",
  "tokens": [
    { "start": 0, "end": 9, "kind": "keyword" }
  ]
}
```

---

## Rust API quick reference

```rust
// Compile with defaults
let result: CompileResult = puml::compile(source)?;

// Compile with custom options
let result = puml::compile_with_options(source, &ParsePipelineOptions {
    frontend: FrontendSelection::Mermaid,
    ..Default::default()
})?;

// Worker dispatch (single-function entry point for all operations)
let response = puml::dispatch(WorkerRequest::compile("req-1", source));

// Worker dispatch with options
let response = puml::dispatch_with_options(request, &options)?;

// Access the worker module types directly
use puml::worker::{WorkerRequest, WorkerRequestPayload, CompileRequest};
```
