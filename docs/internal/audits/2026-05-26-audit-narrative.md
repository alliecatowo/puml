# Forensic Audit — Narrative Assessment (2026-05-26)

> Companion to `2026-05-26-forensic-codebase-audit.md`. That file is the structured,
> file:line, table-driven record. **This file is the orchestrator's narrative read** —
> the "what's actually going on" judgment, preserved largely verbatim because the prose
> carries signal the tables flatten. Two parts: (1) the master synthesis delivered right
> after the multi-agent sweep, and (2) the deeper, blunter follow-up. Point-in-time;
> the code wins on any conflict.

---

## Part 1 — Master synthesis ("What I think, straight")

For "100% vibe coded," this is **much better than the label fears** — and that's the honest
headline, not flattery. ~116K LOC, ~1,794 real tests, a genuine Sugiyama layout engine, a clean
SVG→raster path, and the determinism/no-panic invariants are *actually upheld* on the parse path
(zero unwrap/expect/panic in `src/parser/`). TODO density is ~zero. The dead features CLAUDE.md
says were deleted really are deleted.

**Your specific fear — sprawl, dead/divergent codepaths — is real but concentrated and nameable,
not diffuse rot.** It has five addresses, not five hundred.

### The thing you didn't ask about but is most important

**CLAUDE.md — the agents' own source of truth — is lying to your agents in at least 8 concrete
ways.** This is the highest leverage-per-effort fix in the whole repo:

- "`family.rs` is the most contended god file" → it's **31 lines**, a re-export shim. There is no
  god-file anywhere.
- "PEG-based grammar" → it's a hand-rolled line matcher; `winnow` is in Cargo.toml with **zero
  usages** (dead dep).
- "stage 1 complete" layout → code says **Stage 3**. "orthogonal routing wired into sequence" →
  **sequence uses straight lines**, it's class/component/deployment.
- #399 listed as an open epic → **it's closed**.
- "85% coverage gate" → no file uses 85 (PR gate=83, check-all=87), **and the gate excludes the
  entire renderer**.
- "language service accessible via MCP" → MCP exposes diagnostics only.

Every future agent reads this file and mis-targets. Fix it first; it's an afternoon.

### Master priority list

| # | Finding | Sev | Effort |
|---|---|---|---|
| **P1** | Parser is a **34-`include!` monolith** — ~13K LOC, one flat namespace, no privacy. Root of the recurring family-misdetection bug class. | HIGH | High |
| **P2** | **Typed-scene migration stalled** at ~6/30 families. The real validator **re-parses the SVG string with regex**. Scene is advisory, not authoritative. | HIGH | High |
| **P3** | **One unknown line aborts the entire diagram** (`E_*_UNSUPPORTED_SYNTAX`). PlantUML degrades gracefully; this doesn't. Biggest real-world UX risk. | HIGH | Med |
| **P4** | Browser **reimplements `!include` + tokenizer in JS**; can silently diverge from CLI/LSP; **no parity test**. (Includes 3 concrete JS bugs.) | HIGH | Med |
| **P5** | **Coverage gate excludes 63% of `src/`** — all of render/parser/normalize. The core has no line floor. | HIGH | Med |
| **P6** | ~5 independent layout engines; text width estimated `chars×7` in **9+ places** that drift. | MED-HIGH | High |
| P7 | CLAUDE.md factual errors (above) | MED | **Low** |
| P8 | `DeterminismMode` is a **no-op stub** (empty body, arg discarded) presented as enforcement | MED | Low |
| P9 | Oracle is **advisory, not gating** (2/21 fixtures blocking, both trivial). Bonus: `branch-protection.sh` *claims* to require it, live `main` doesn't — verify. | MED | Med |
| P10 | **AWS/Azure/GCP/tupadr3 icon libs are stubs** — look supported, render as plain boxes | MED | High |
| P11 | Class relation labels float disconnected; parallel edges collapse; labels dropped | MED | Med |
| P12 | **JSON/YAML keys alphabetized** not document-order (serde_json missing `preserve_order`) | MED | **Low** |
| P13 | box_grid scene/SVG divergence (no `rebuild_scene` → validates stale geometry) | MED | Low |
| P14 | VS Code extension has **zero CI**; shaky dep pins | MED | Low |

(P15–P18 — `!definelong` unsupported, stale issue bodies, dead graph_layout items, missing
DOT/Sudoku — are in the structured doc.)

### My actual recommendation on sequencing

- **Wave 0 (a day, mostly trivial):** P7 doc fix, P12 one-line serde feature, P8 delete-or-implement,
  P14 CI for the extension, kill the dead graph_layout items, reconcile the oracle branch-protection
  lie. This clears the fog of "what's broken vs. just claimed."
- **Wave 1 (credibility):** P5 give the renderer *some* coverage floor, P9 decide if the oracle is a
  gate or a dashboard and stop implying otherwise, P3 graceful degradation.
- **Wave 2 (the anti-sprawl bets that decide how this ages):** **P1** un-monolith the parser into real
  modules, **P2** finish the typed-scene migration and delete the SVG-regex-reparse validator. These
  two are the whole ballgame for long-term maintainability.

Parity reality: **~60–65%** — broad, uneven depth, a few correctness bugs, stubbed icon libs.

---

## Part 2 — The straight, deeper version

The table tells you *what*. Here's *what's actually going on* — the parts that don't fit in a
severity column.

### First, the part nobody will tell you: this is good work

I went in expecting to find a Potemkin renderer. I didn't. There's a real Sugiyama layout engine
(rank assignment, cycle breaking, barycenter crossing-minimization, a 593-line channel router) with
20 of its own tests. The parser genuinely never panics on user input — zero `unwrap`/`expect`/`panic`
in all of `src/parser/`, which most hand-written parsers can't claim. Determinism is *actually*
maintained via BTreeMap discipline. Sequence diagrams are legitimately PlantUML-grade. ~1,794 tests
that assert real values, not `assert!(true)`. For 1,232 commits in 11 days, the *code* is disciplined.
Hold onto that, because the rest is critical.

### The actual disease is one architectural decision, not a hundred bugs

**The SVG string is the real intermediate representation, and validation works by regex-ing your own
output back apart.** Every renderer does `let mut out = String::new()` and `push_str`es raw SVG tags.
Then the validator — the thing that enforces "no labels outside viewport," that auto-expands the
viewBox, that inserts label background rects — does not look at any geometry model. It runs regexes
over the SVG text you just emitted (`extract_text_elements`, `parse_viewbox`, `extract_node_bboxes`)
to *recover* the geometry you already had in memory and threw away.

This is the root from which most of your other smells grow:

- It's why there's a "typed `RenderScene`" that only 6 of ~30 families populate and that **nothing
  authoritative consumes** — it's advisory, appended after the string-regex pass. The scene is the
  architecture you *want*; the string-regex is the architecture you *have*.
- It's why text width is estimated as `chars × 7px` in **nine-plus** independent places including the
  validator's own private copy — because layout and validation don't share a geometry model, they
  each re-guess.
- It's why box_grid's scene silently validates *stale* geometry (the SVG is post-processed after the
  scene is built, no `rebuild_scene`). The scene and the output have already diverged and nothing
  catches it because the scene isn't load-bearing.

Fix the IR and three of your "separate" findings collapse into one. That's P2, and it's the single
highest-value structural move available.

### Your specific fear — divergent codepaths — is real, and here's exactly where it lives

You're not paranoid. There are **two simultaneous half-finished migrations**, and a half-migration is
*definitionally* a divergent codepath:

1. **Typed scene migration**: ~6 families on the new path, ~24 on the old string-only path. Both
   alive. Same concern, two implementations, indefinitely.
2. **Shared layout engine adoption**: 3 families use `graph_layout/`; the rest (sequence, state,
   activity, salt, nwdiag, chart) each ship their *own* full layout engine. The "we have a real layout
   engine" story is true for ~10% of families and aspirational for the rest.

The worst possible state for sprawl isn't "old architecture" or "new architecture" — it's *both at
once, with each family rolling the dice on which one it uses*. That's where you are. Every new family
is a fork in the road where the author picks old-or-new ad hoc.

And it extends across the language boundary: the browser **reimplements `!include` resolution in 92
lines of JavaScript** and **reimplements the tokenizer in `puml-tokens.js`**, both diverging from the
~5,900-line Rust core, with **no parity test** — and meanwhile the WASM module *already exports* the
authoritative versions, which the site never calls (dead code). So the user-facing "does my diagram
work" surface runs different logic than the CLI, by construction.

### The risk allocation is inverted — and that's the dangerous part

Look at where the verification muscle is:

- The coverage gate **excludes 63% of `src/`** — all of `render/`, `parser/`, `normalize/`. The
  83/87% number is measured against the CLI and language service. **The hardest, most bug-prone code
  — layout and rendering — has no line-coverage floor at all.**
- The PlantUML oracle, the thing that would actually measure parity, is **advisory, not required**,
  passes at a soft ≥80% (and "WARN" at 50–79% still counts as success), and its committed report is a
  `{"skipped": true}` sentinel. Two of 21 fixtures are blocking, both trivial sequence diagrams.
- `DeterminismMode::Strict` — presented as a hard invariant — is an **empty function that discards its
  argument**.

So the gates are strongest where the code is easy and weakest where it's hard. The green checkmarks
are most reliable exactly where they matter least. A renderer regression can sail through CI; a CLI
flag typo cannot. That's backwards, and it's the kind of thing that feels safe right up until it isn't.

### The "60–65% parity" number is generous in practice

Breadth is real — 28 families render *something*. But two things make the lived experience worse than
the percentage:

1. **The parser is fail-loud and total**: one line it doesn't recognize aborts the *entire* diagram
   with a hard error. PlantUML renders what it can and shrugs at the rest. So a real-world `.puml` that's
   95% supported syntax + one `!definelong` or one exotic skinparam produces **nothing**, not 95% of a
   diagram. At ~60% syntax coverage, real files hit the 40% constantly.
2. **The icon libraries lie politely**: `!include <awslib14/...>` *resolves* (no error) and renders a
   plain stereotyped box instead of the AWS icon. It looks like it worked. Cloud-architecture diagrams —
   one of PlantUML's most popular uses — silently come out wrong rather than failing loudly.

P3 (graceful degradation) is mislabeled as medium priority in my own table. For real adoption it's
arguably the highest-impact behavioral change in the repo.

### The sneakiest risk is the narrative layer, not the code

The code is healthier than its documentation. CLAUDE.md — *the file every agent reads to orient* —
confidently describes a god-file that's 31 lines, a layout module at a path that's now a directory, a
"PEG grammar" that's a hand-rolled matcher backed by a dependency (`winnow`) with **zero uses**, an
open epic that's closed, and a coverage number that matches no file. The site ships a spec
(`studio-spa.md`) describing a TypeScript SPA **that was never built**.

This is the actual failure mode of high-velocity agent development: the code stays disciplined because
it's gated and tested; the prose rots because it isn't. And then your agents navigate by the prose.
That's how you get five agents and ten PRs on one layout bug (your own CLAUDE.md confesses this) —
they're coding to a map of a territory that moved. **The cheapest highest-leverage thing in this whole
audit is deleting and correcting fiction, not writing code.**

### The strategic fork you actually have to call

You can't have "broad PlantUML parity" *and* "a clean, well-architected layout engine" as simultaneous
in-progress goals — that's what produced the half-and-half. Pick:

- **A) Parity product**: embrace that families are bespoke, kill the aspirational shared-architecture
  framing, make the parser degrade gracefully, fix the icon stubs and class labels, and chase the
  oracle number. Accept per-family duplication as the cost of breadth.
- **B) Architecture-first**: freeze new families, *finish* the typed-scene migration, make the scene
  authoritative, delete the string-regex validator, migrate the existing families onto the shared
  engine, then resume breadth on a clean base.

Right now you're paying the cost of both and getting the benefit of neither. My honest recommendation:
**a short, ruthless Wave 0/1 (the cheap-truth cleanup — doc corrections, DeterminismMode, serde
feature, coverage honesty, oracle decision) to stop lying to yourselves, then commit hard to (B) for
the core families** (sequence/class/component/state/activity), because the string-IR is the thing that
will make every future bug more expensive, and it compounds. Breadth you can always add back on a sound
base; you cannot cheaply un-bake a string-as-IR after 50 families depend on its exact text format.

### One more, on velocity and bus factor

1,232 commits in 11 days, essentially one human plus agents. The velocity is the asset and the
liability: it's why the code is extensive and why the docs can't keep up. The structural fix isn't to
slow down — it's to make the **code self-describing** (real parser modules with privacy boundaries so
an agent can't leak a helper across 13K lines; one geometry model so there's one place to be right)
and to treat aspirational docs as bugs. The faster you go, the more the map has to be trustworthy,
because nobody's holding the whole thing in their head.

---

## Decision taken (2026-05-26)

After reading the above, the owner chose **option (B), aggressively**, and set the direction now
tracked in epic #1258 and the [migration strategy memory]:

- Kill `DeterminismMode` (done/in progress on the wave branch).
- Oracle: don't enforce, leave advisory.
- Coverage target 90%+; disabling the % gate while climbing is acceptable.
- **Pursue the typed-scene migration and the shared-layout/geometry migration first**, before
  anything else — families must stop reimplementing geometry from the ground up.
- Do the work on one mega-branch (`refactor/claude-wave-migrations`) and ship it as a single PR.
