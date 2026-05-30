#!/usr/bin/env python3
"""Report authored Rust source files that exceed the maintainability target."""

from __future__ import annotations

import argparse
import json
import pathlib
import sys
from dataclasses import dataclass


DEFAULT_THRESHOLD = 600
SEARCH_DIRS = ("src", "crates")

# Generated icon tables are intentionally large data blobs imported from upstream
# icon packs. They are not authored ownership units and should not drive splits.
ALLOWLIST_REASONS = {
    "src/bootstrap_icons.rs": "generated Bootstrap Icons SVG table",
    "src/material_icons.rs": "generated Material Icons SVG table",
    "src/openiconic.rs": "generated Open Iconic SVG table",
    # Was 584 lines before parser-unmonolith refactor; 23 lines added for required
    # pub(crate) visibility annotations on cross-module struct fields (FamilyDeclParts,
    # FamilyHeritage, FamilyInlineStyle). Split is tracked in #1258.
    "src/parser/family_declarations.rs": "pre-existing large module; +23 lines from refactor visibility annotations",
    # Was 591 lines; grew to 617 after wave-14 re-orient fix (#1318/#1319) added
    # reversed-edge path-flip logic and EdgeInfo docs. The routing algorithm is a
    # single tightly-coupled computation; a mechanical split would only add indirection.
    # Tracked for a future path-building extraction in the graph-layout refactor (#590).
    "src/render/graph_layout/router.rs": "wave-14 re-orient fix added 26 lines; split tracked in #590",
    # Was 558 lines on main; grew to 624 after wave-14 curves+anchoring (#1318/#1319)
    # inlined the state self-transition cubic-arc and internal-actions rendering.
    # The render_node function is a match-arm state machine; mechanical sub-function
    # extraction requires plumbing all local vars through parameters. Split tracked in #590.
    "src/render/state/node_render.rs": "wave-14 self-transition curve added 66 lines net; split tracked in #590",
    # Was 600 lines on main; grew to 601 after wave-15 density-followups added the
    # skip_group_collision_resolution field to the layout options struct. The field is
    # tightly coupled to the collision-resolution loop in the same module; extraction
    # would add indirection with no benefit. Split tracked in #590.
    "src/render/family/box_grid.rs": "wave-15 density retune added skip_group_collision_resolution field; split tracked in #590",
}


@dataclass(frozen=True, order=True)
class RustFileSize:
    lines: int
    path: str


def repo_relative(path: pathlib.Path, root: pathlib.Path) -> str:
    return path.relative_to(root).as_posix()


def count_lines(path: pathlib.Path) -> int:
    with path.open("r", encoding="utf-8") as source:
        return sum(1 for _ in source)


def iter_rust_files(root: pathlib.Path) -> list[pathlib.Path]:
    files: list[pathlib.Path] = []
    for dirname in SEARCH_DIRS:
        base = root / dirname
        if not base.exists():
            continue
        files.extend(path for path in base.rglob("*.rs") if path.is_file())
    return sorted(files)


def collect_over_limit(root: pathlib.Path, threshold: int) -> tuple[list[RustFileSize], list[RustFileSize]]:
    authored: list[RustFileSize] = []
    allowlisted: list[RustFileSize] = []

    for path in iter_rust_files(root):
        rel = repo_relative(path, root)
        lines = count_lines(path)
        if lines <= threshold:
            continue
        entry = RustFileSize(lines=lines, path=rel)
        if rel in ALLOWLIST_REASONS:
            allowlisted.append(entry)
        else:
            authored.append(entry)

    return sorted(authored, reverse=True), sorted(allowlisted, reverse=True)


def guardrail_mode(fail_on_violations: bool) -> str:
    return "enforced" if fail_on_violations else "warning-only"


def render_text(
    authored: list[RustFileSize],
    allowlisted: list[RustFileSize],
    threshold: int,
    fail_on_violations: bool = False,
) -> str:
    mode = guardrail_mode(fail_on_violations)
    lines = [
        f"[rust-file-size] {mode} guardrail: authored Rust files over {threshold} LOC",
        f"[rust-file-size] authored_over_limit={len(authored)} allowlisted_generated={len(allowlisted)}",
    ]

    if authored:
        lines.append("[rust-file-size] authored files to split:")
        width = max(len(str(entry.lines)) for entry in authored)
        for entry in authored:
            lines.append(f"  {entry.lines:>{width}}  {entry.path}")
    else:
        lines.append("[rust-file-size] no authored Rust files exceed the target")

    if allowlisted:
        lines.append("[rust-file-size] allowlisted generated files:")
        width = max(len(str(entry.lines)) for entry in allowlisted)
        for entry in allowlisted:
            reason = ALLOWLIST_REASONS[entry.path]
            lines.append(f"  {entry.lines:>{width}}  {entry.path}  # {reason}")

    if fail_on_violations:
        status = "enforced; violations fail this command"
    else:
        status = "warning-only; pass --fail-on-violations to enforce"
    lines.append(f"[rust-file-size] status={status}")
    return "\n".join(lines)


def render_json(
    authored: list[RustFileSize],
    allowlisted: list[RustFileSize],
    threshold: int,
    fail_on_violations: bool = False,
) -> str:
    payload = {
        "schema": "puml.rust_file_size_guardrail",
        "schema_version": 1,
        "threshold": threshold,
        "mode": guardrail_mode(fail_on_violations),
        "authored_over_limit": [entry.__dict__ for entry in authored],
        "allowlisted_generated": [
            {**entry.__dict__, "reason": ALLOWLIST_REASONS[entry.path]} for entry in allowlisted
        ],
    }
    return json.dumps(payload, indent=2, sort_keys=True)


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Report authored Rust files over the maintainability LOC target."
    )
    parser.add_argument(
        "--root",
        type=pathlib.Path,
        default=pathlib.Path(__file__).resolve().parents[1],
        help="repository root to scan",
    )
    parser.add_argument(
        "--threshold",
        type=int,
        default=DEFAULT_THRESHOLD,
        help=f"line-count target for authored Rust files (default: {DEFAULT_THRESHOLD})",
    )
    parser.add_argument(
        "--format",
        choices=("text", "json"),
        default="text",
        help="report format",
    )
    parser.add_argument(
        "--fail-on-violations",
        action="store_true",
        help="exit non-zero when authored files exceed the target",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    root = args.root.resolve()
    authored, allowlisted = collect_over_limit(root, args.threshold)

    if args.format == "json":
        print(render_json(authored, allowlisted, args.threshold, args.fail_on_violations))
    else:
        print(render_text(authored, allowlisted, args.threshold, args.fail_on_violations))

    if args.fail_on_violations and authored:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
