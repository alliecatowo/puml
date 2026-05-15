# Docs Examples Corpus

This markdown is the canonical top-layer docs parity source for `scripts/parity_harness.py`.

## Linked Source Files

- [basic_hello.puml](basic_hello.puml) -> [basic_hello.svg](basic_hello.svg)
- [groups_notes.puml](groups_notes.puml) -> [groups_notes.svg](groups_notes.svg)
- [lifecycle_autonumber.puml](lifecycle_autonumber.puml) -> [lifecycle_autonumber.svg](lifecycle_autonumber.svg)

## Inline Snippet

The harness also discovers fenced snippets and expects an artifact named `<markdown-stem>_snippet_<n>.svg`.

```puml
@startuml
participant Web
participant API

Web -> API: GET /health
API --> Web: 200 OK
@enduml
```
