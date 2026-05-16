# Diagram Families Architecture Spec

Date: 2026-05-15  
Status: Approved for implementation scaffolding

## Purpose

Define a shared intermediate-representation (IR) boundary and family layout-engine interface so multiple diagram families can be added without rewriting parser/CLI contracts or regressing existing sequence behavior.

This spec is decision-complete for implementers for the first parity-program scaffolding phase.

## Scope

In scope:
- Public family routing contract in core API.
- Shared IR envelope between parser/normalizer and family layout engines.
- Family layout-engine interface and deterministic behavior contract.
- Error boundary for unsupported families during staged parity rollout.

Out of scope (current scaffold slice):
- Additional family-specific semantics not yet implemented in each active lane.
- Concrete class/state/activity renderers.
- Plugin/editor protocol changes.

## Architecture Overview

Pipeline target:

1. `parse` input into AST `Document`.
2. Route by `DiagramFamily`.
3. Build family IR at the boundary.
4. Run family-specific layout engine.
5. Render scene(s) into SVG.

Current rollout state:
- Sequence family: stable baseline path with active parity hardening.
- Additional families: routed by family-aware APIs and currently in progressive parity lanes; unsupported families return deterministic diagnostics until implemented.

## Shared IR Boundary

### IR Envelope

All family pipelines must conform to one envelope shape:
- `family`: selected `DiagramFamily`.
- `source_fingerprint`: stable identifier derived from source bytes (for cache keys and deterministic tests).
- `page_units`: ordered list of family page units to layout.
- `diagnostics`: structured warnings/errors attached before render.

### IR Requirements

- Deterministic: identical source + options => byte-stable IR payload ordering.
- Serializable: IR must be representable as stable JSON for debugging (`--dump` follow-up phase).
- Family-local internals: each family may define private event/node payloads, but envelope ordering and metadata rules are shared.
- Span-preserving: IR nodes/events that originate from source constructs must retain source spans for diagnostics.

## Family Layout-Engine Interface

Each family layout engine must expose the same conceptual interface:

- Input:
  - family IR page units
  - layout options
- Output:
  - ordered scene/page list
  - non-fatal diagnostics (warnings)
  - fatal diagnostic on unrecoverable family-specific layout error

Contract rules:
- No parser re-entry from layout engine.
- No source mutation.
- Stable ordering for pages and visual elements.
- Deterministic floating-point/rounding policy per engine (documented with fixtures).

## Public API Contract (Scaffolding Phase)

Core library must provide:
- A public `DiagramFamily` enum for explicit routing.
- Family-aware render entrypoints.
- Backward-compatible sequence-default helpers.

Behavior rules:
- Existing `render_source_to_svg` and `render_source_to_svgs` preserve current sequence behavior.
- New family-aware APIs route sequence normally.
- Non-sequence families return deterministic `Diagnostic::error` with explicit unsupported-family text.


## Canonical Examples Corpus (Top-Layer Parity Artifact)

`docs/examples/` is the canonical parity demonstration corpus and is treated as a top-layer artifact above fixture granularity.

Structure contract:
- `docs/examples/<family>/` directory per `DiagramFamily`.
- Each canonical example is a required source/render pair: `NNN_slug.puml` + `NNN_slug.svg`.
- Example IDs (`NNN`) are stable references used by parity matrix rows and release notes.

Governance contract:
- Every new feature/primitive must add at least one canonical example pair in the relevant family directory.
- Behavior-changing renderer/normalizer work must refresh impacted SVG pairs in the same PR.
- Unsupported behavior is tracked in error fixtures, not in canonical acceptance pairs.

Docs-as-tests policy:
- Canonical examples are executable test vectors.
- CI parity gate target: render each corpus `.puml`, compare output to checked-in `.svg`, fail on unexpected drift.
- Parity matrix cells must cite corpus IDs as primary evidence, with fixture/test IDs as secondary evidence.

## Determinism Contract

For every family-aware stub before full implementation:
- Same input/source must produce identical `Result` shape and identical error text.
- Rejections for unsupported families are stable and not data-dependent beyond family name.

## Rollout Plan Hooks

This scaffolding unblocks incremental family implementation slices:
1. Parser family detection hardening.
2. Family IR builders per family.
3. Family layout engines.
4. Renderer specialization where needed.
5. CLI family flags / auto-routing policy.

## Acceptance Criteria

Scaffolding phase is complete when:
- `DiagramFamily` is available in public API.
- Family-aware render routing stubs compile and are covered by tests.
- Sequence behavior stays unchanged through existing tests.
- Unsupported families are deterministically rejected with explicit diagnostics.
- Parity roadmap contains family-by-family execution slices.
