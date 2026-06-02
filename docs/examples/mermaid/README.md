# Mermaid examples

These `.mmd` fixtures exercise PUML's Mermaid front-end adapter
(`src/frontend/mermaid/`). They are rendered through the standard PUML pipeline
once the front-end translates them to PlantUML-shaped source, so every fixture
ships alongside a matching `.puml` document for human inspection.

## Currently supported families

| Family | Adapter | Status |
|---|---|---|
| `flowchart` / `graph` | `src/frontend/mermaid/flowchart.rs` | Basic shapes + arrows + subgraphs |
| `sequenceDiagram` | `src/frontend/mermaid/sequence.rs` | Messages, activations, notes |
| `classDiagram` | `src/frontend/mermaid/class.rs` | Classes, members, relations |
| `stateDiagram` / `stateDiagram-v2` | `src/frontend/mermaid/state.rs` | States, transitions |
| `erDiagram` | `src/frontend/mermaid/er.rs` | Entities, relationships |

## Running

```bash
# Explicit dialect
./target/release/puml --dialect mermaid 01_basic_flowchart.mmd -o /tmp/flow.svg

# Or rely on the `.mmd` extension auto-detection (added in this PR)
./target/release/puml 01_basic_flowchart.mmd -o /tmp/flow.svg

# Render to PNG for multimodal review
./target/release/puml --format png 01_basic_flowchart.mmd -o /tmp/flow.png
```

See `docs/internal/spec/mermaid/` for the ingested upstream syntax + architecture
documentation that drives the adapter implementations.
