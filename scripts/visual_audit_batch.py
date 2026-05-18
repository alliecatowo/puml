#!/usr/bin/env python3
"""
visual_audit_batch.py — Slice the PNG corpus for an AI visual audit agent.

Reads target/audit_corpus/manifest.json and prints a tab/pipe-separated table
of PNG paths an audit agent should ingest. Caps output with --max to avoid
overwhelming an agent's context window.

Usage:
    python3 scripts/visual_audit_batch.py [--filter REGEX] [--max N] [--status ok|failed|all]

Output columns (pipe-separated):
    PATH | family | source_size_bytes | render_status

Example:
    python3 scripts/visual_audit_batch.py --filter sequence --max 50
    python3 scripts/visual_audit_batch.py --status failed
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST_PATH = REPO_ROOT / "target" / "audit_corpus" / "manifest.json"


def _family_from_source(source_rel: str) -> str:
    """
    Derive the diagram family from the source path.
    E.g. docs/examples/sequence/01_basic.puml -> sequence
         tests/fixtures/families/valid_activity.puml -> activity
         tests/fixtures/basic/hello.puml -> basic
    """
    parts = Path(source_rel).parts
    # Try to find a meaningful family segment
    # For examples: docs/examples/<family>/...
    # For fixtures: tests/fixtures/families/valid_<family>_...
    # For basic: tests/fixtures/basic/...
    if "examples" in parts:
        idx = list(parts).index("examples")
        if idx + 1 < len(parts) - 1:
            return parts[idx + 1]
    if "families" in parts:
        # Extract family from filename like valid_sequence_foo.puml
        stem = Path(source_rel).stem
        # strip leading valid_/invalid_
        stem = re.sub(r"^(valid|invalid)_", "", stem)
        # take first segment before underscore
        return stem.split("_")[0]
    if "basic" in parts:
        return "basic"
    if "visual_baselines" in parts:
        idx = list(parts).index("visual_baselines")
        if idx + 1 < len(parts) - 1:
            return parts[idx + 1]
    # fallback: parent directory name
    return Path(source_rel).parent.name or "unknown"


def _source_size(source_rel: str) -> int:
    """Return size of the source file in bytes, or 0 if inaccessible."""
    try:
        return (REPO_ROOT / source_rel).stat().st_size
    except OSError:
        return 0


def main():
    parser = argparse.ArgumentParser(
        description="Print a slice of the PNG corpus for AI visual audit.",
    )
    parser.add_argument(
        "--filter", metavar="REGEX",
        help="Only include entries whose source path matches this regex.",
    )
    parser.add_argument(
        "--max", type=int, default=None, metavar="N",
        help="Cap output at N entries (prevents context window overload).",
    )
    parser.add_argument(
        "--status", choices=["ok", "skipped", "failed", "rendered", "all"], default="rendered",
        help="Which render statuses to include (default: rendered = ok + skipped).",
    )
    parser.add_argument(
        "--family", metavar="FAMILY",
        help="Only include entries from a specific diagram family.",
    )
    args = parser.parse_args()

    if not MANIFEST_PATH.exists():
        print(
            f"ERROR: manifest not found at {MANIFEST_PATH}\n"
            "Run `python3 scripts/render_corpus.py` first.",
            file=sys.stderr,
        )
        sys.exit(1)

    with open(MANIFEST_PATH) as fh:
        manifest = json.load(fh)

    entries = manifest.get("entries", [])

    # Filter by status
    if args.status == "rendered":
        # rendered = ok + skipped (both have a valid PNG on disk)
        entries = [e for e in entries if e.get("status") in ("ok", "skipped")]
    elif args.status == "all":
        entries = [e for e in entries if e.get("status") in ("ok", "skipped", "failed")]
    else:
        entries = [e for e in entries if e.get("status") == args.status]

    # Filter by regex
    if args.filter:
        pat = re.compile(args.filter)
        entries = [e for e in entries if pat.search(e.get("source", ""))]

    # Filter by family
    if args.family:
        entries = [
            e for e in entries
            if _family_from_source(e.get("source", "")) == args.family
        ]

    # Cap
    if args.max is not None:
        entries = entries[: args.max]

    if not entries:
        print("# No entries match the given filters.", file=sys.stderr)
        sys.exit(0)

    # Print header
    print("PATH | family | source_size_bytes | render_status")

    for e in entries:
        source_rel = e.get("source", "")
        png_rel = e.get("png", "")
        png_abs = str(REPO_ROOT / png_rel) if png_rel else ""
        family = _family_from_source(source_rel)
        src_size = _source_size(source_rel)
        status = e.get("status", "unknown")
        print(f"{png_abs} | {family} | {src_size} | {status}")

    # Summary to stderr so it doesn't pollute stdout piping
    print(f"\n# {len(entries)} entries listed", file=sys.stderr)


if __name__ == "__main__":
    main()
