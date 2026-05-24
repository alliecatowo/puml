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


def render_text(authored: list[RustFileSize], allowlisted: list[RustFileSize], threshold: int) -> str:
    lines = [
        f"[rust-file-size] warning-only guardrail: authored Rust files over {threshold} LOC",
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

    lines.append("[rust-file-size] status=warning-only; use --fail-on-violations when enforcement is enabled")
    return "\n".join(lines)


def render_json(authored: list[RustFileSize], allowlisted: list[RustFileSize], threshold: int) -> str:
    payload = {
        "schema": "puml.rust_file_size_guardrail",
        "schema_version": 1,
        "threshold": threshold,
        "mode": "warning-only",
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
        help="exit non-zero when authored files exceed the target; do not use in CI yet",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    root = args.root.resolve()
    authored, allowlisted = collect_over_limit(root, args.threshold)

    if args.format == "json":
        print(render_json(authored, allowlisted, args.threshold))
    else:
        print(render_text(authored, allowlisted, args.threshold))

    if args.fail_on_violations and authored:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
