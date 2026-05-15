# Parity Roadmap

Date: 2026-05-15

This roadmap tracks high-impact sequence-diagram parity work relative to PlantUML behavior.

## Source Inputs

- Current parity research: `docs/parity-research-chunk-g-sequence.md`
- Contract decisions: `docs/decision-log.md`
- Coverage and test signals: `docs/coverage-status.md`, `tests/**`

## Priority Backlog

1. P1: Expand supported arrow syntax variants.
2. P1: Add `queue` participant role support.
3. P1: Implement `== separator ==` syntax.
4. P1: Make footbox toggles visibly affect SVG output.
5. P2: Improve group semantics and validation fidelity.
6. P2: Broaden autonumber format and restart parity.
7. P2: Improve found/lost and virtual endpoint rendering fidelity.
8. P3: Expand `skinparam` support in deterministic increments.
9. P3: Align preprocessor contract with implementation boundary.

## Milestone View

- Milestone 1 (compatibility unblockers): items 1-4
- Milestone 2 (semantic depth): items 5-7
- Milestone 3 (styling and preprocessing policy): items 8-9

## Definition of Done Per Item

- behavior covered by fixtures and tests
- stable snapshots for rendering-affecting changes
- CLI/docs contract updated in `README.md` and `docs/**`
- decision log updated for intentional boundary choices
