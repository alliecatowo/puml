# Visual Audit Pipeline

## Why PNG, Not SVG

AI vision models (Claude, GPT-4o, Gemini) trigger their visual understanding pathway only on **raster image inputs** (PNG, JPG, WebP). SVG files are XML text — the model reads the markup, not the rendered diagram. That means:

- **SVG audit**: the agent sees `<rect x="10" y="20" width="50"/>` — tag soup.
- **PNG audit**: the agent sees boxes, arrows, labels, layout — the actual diagram.

For PUML's self-driving visual development loop, every audit agent must ingest PNG. This pipeline makes that fast and repeatable.

## Corpus vs. Blessed Baselines

| | **`tests/visual_baselines/`** | **`target/audit_corpus/png/`** |
|---|---|---|
| Managed by | `bless_baselines` / regression suite | `render_corpus.py` |
| Scope | Golden subset — blessed diagrams only | All examples + fixtures |
| Committed | Yes (PNGs in repo) | No (`target/` is gitignored) |
| Purpose | Catch regressions in CI | Feed AI visual audit agents |
| Updated via | Explicit human blessing | Re-running `render_corpus.py` |

The corpus is broader: it covers every `.puml` and `.txt` in the example trees, including diagrams not yet blessed. Failures in the corpus are themselves visual bugs — the manifest captures them with `"status": "failed"`.

## Source Directories Walked

```
docs/examples/              # All documented examples, organized by family
tests/visual_baselines/     # Baseline PNGs (no .puml sources here; skipped)
tests/fixtures/families/    # Fixture library used by regression tests
tests/fixtures/basic/       # Basic smoke fixtures
stdlib/examples/            # (Included when present; currently absent)
```

## How to Regenerate the Corpus

```bash
# Build the binary first (if not already built)
cargo build --release

# Render everything (skips already-up-to-date PNGs by default)
python3 scripts/render_corpus.py

# Force re-render all
python3 scripts/render_corpus.py --force

# Render only sequence diagrams
python3 scripts/render_corpus.py --filter sequence

# Use 8 workers, 144 DPI for sharper renders
python3 scripts/render_corpus.py --workers 8 --dpi 144
```

The script exits 0 even when some renders fail. Failed renders appear in the manifest with `"status": "failed"` and the captured stderr. They are the bugs to fix, not script errors.

Output location:
```
target/audit_corpus/
  png/               # Mirrors source tree with .puml.png / .txt.png filenames
  manifest.json      # Full listing: status, size, render time, warnings
```

## How to Slice for an Audit Agent

Use `visual_audit_batch.py` to print a list of PNGs for an agent to ingest:

```bash
# All successfully rendered PNGs (default)
python3 scripts/visual_audit_batch.py

# Only sequence diagrams
python3 scripts/visual_audit_batch.py --family sequence

# Only failed renders (the bugs)
python3 scripts/visual_audit_batch.py --status failed

# Cap at 30 entries so the agent's context window isn't overloaded
python3 scripts/visual_audit_batch.py --filter activity --max 30
```

Output format (pipe-separated, one entry per line):
```
PATH | family | source_size_bytes | render_status
/home/.../target/audit_corpus/png/docs/examples/sequence/01_basic.puml.png | sequence | 234 | ok
```

Feed these paths to an audit agent as image attachments. The agent can then report on layout quality, alignment, missing elements, or regression relative to a reference.

## CLI Invocation Used

```
target/release/puml --format png --dpi 96 <source> -o <output>
```

- `--format png` selects raster PNG output (verified: magic bytes `89 50 4E 47`).
- `--dpi` controls rasterization resolution (default 96; use 144 for sharper detail).
- Output path is passed with `-o` / `--output`.
- Exit code 0 = success; non-zero = render failure (captured in manifest `stderr`).

## Baseline Blessing vs. Corpus Rendering

```
render_corpus.py          # Broad: all sources → target/audit_corpus/png/  (not committed)
bless_baselines           # Narrow: hand-picked → tests/visual_baselines/  (committed)
```

Baseline blessing is the formal approval step for regression testing. Corpus rendering is a continuous, automated, non-gating snapshot for visual audits. Eventually these two workflows will share the same render path so DPI/format settings stay in sync.

## CI Integration (Planned)

See issue [#95] for the plan to wire `render_corpus.py` into the PR gate as an informational artifact. The initial integration will be `workflow_dispatch`-only (non-blocking) and will upload `manifest.json` as a GitHub Actions artifact so audit agents can retrieve it without checking out the repo.
