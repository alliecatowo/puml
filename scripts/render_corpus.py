#!/usr/bin/env python3
"""
render_corpus.py — Render the full PUML example corpus to PNG for visual audit.

Every AI visual audit must ingest raster (PNG) images, not SVG text. This script
walks all known source directories, renders each .puml/.txt file via
`target/release/puml --format png`, and emits a manifest.json for downstream
audit tooling.

Usage:
    python3 scripts/render_corpus.py [--force] [--filter REGEX] [--workers N] [--dpi N]
"""

import argparse
import json
import multiprocessing
import os
import re
import subprocess
import sys
import time
from pathlib import Path


# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

# Repo root: two levels up from this script (scripts/ -> repo root)
REPO_ROOT = Path(__file__).resolve().parent.parent

# Source directories to walk (only those that exist are processed)
SOURCE_DIRS = [
    REPO_ROOT / "docs" / "examples",
    REPO_ROOT / "tests" / "visual_baselines",
    REPO_ROOT / "tests" / "fixtures" / "families",
    REPO_ROOT / "tests" / "fixtures" / "basic",
    REPO_ROOT / "stdlib" / "examples",
]

SOURCE_EXTENSIONS = {".puml", ".txt"}

OUTPUT_ROOT = REPO_ROOT / "target" / "audit_corpus" / "png"
MANIFEST_PATH = REPO_ROOT / "target" / "audit_corpus" / "manifest.json"

PUML_BINARY = REPO_ROOT / "target" / "release" / "puml"

PNG_MAGIC = b"\x89PNG"


# ---------------------------------------------------------------------------
# Worker function (runs in subprocess pool)
# ---------------------------------------------------------------------------

def _render_one(args):
    """
    Render a single source file to PNG.

    args: (source_path_str, out_path_str, dpi, force)

    Returns a manifest entry dict.
    """
    source_str, out_str, dpi, force = args
    source = Path(source_str)
    out = Path(out_str)

    entry = {
        "source": str(source.relative_to(REPO_ROOT)),
        "png": str(out.relative_to(REPO_ROOT)),
        "status": "pending",
        "size_bytes": None,
        "render_time_s": None,
        "warnings": [],
        "stderr": None,
    }

    # mtime-based skip (unless --force)
    if not force and out.exists():
        try:
            if out.stat().st_mtime >= source.stat().st_mtime:
                # Verify it is a real PNG before declaring up-to-date
                with open(out, "rb") as fh:
                    magic = fh.read(4)
                if magic == PNG_MAGIC:
                    entry["status"] = "skipped"
                    entry["size_bytes"] = out.stat().st_size
                    return entry
        except OSError:
            pass  # fall through to re-render

    out.parent.mkdir(parents=True, exist_ok=True)

    cmd = [
        str(PUML_BINARY),
        "--format", "png",
        "--dpi", str(dpi),
        str(source),
        "-o", str(out),
    ]

    t0 = time.monotonic()
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60,
        )
    except subprocess.TimeoutExpired:
        entry["status"] = "failed"
        entry["stderr"] = "render timed out after 60s"
        return entry
    except Exception as exc:
        entry["status"] = "failed"
        entry["stderr"] = f"subprocess error: {exc}"
        return entry

    elapsed = time.monotonic() - t0
    entry["render_time_s"] = round(elapsed, 4)

    stderr_text = (result.stderr or "").strip()

    if result.returncode != 0:
        entry["status"] = "failed"
        entry["stderr"] = stderr_text or f"exit code {result.returncode}"
        # Clean up empty/corrupt output if present
        if out.exists():
            try:
                out.unlink()
            except OSError:
                pass
        return entry

    # Verify output exists and has PNG magic bytes
    if not out.exists():
        entry["status"] = "failed"
        entry["stderr"] = "output file not created (exit 0 but no file)"
        return entry

    with open(out, "rb") as fh:
        magic = fh.read(4)
    if magic != PNG_MAGIC:
        entry["status"] = "failed"
        entry["stderr"] = f"output is not PNG (magic: {magic.hex()})"
        return entry

    entry["status"] = "ok"
    entry["size_bytes"] = out.stat().st_size

    # Collect any warnings from stderr (non-empty stderr with exit 0)
    if stderr_text:
        entry["warnings"] = [line for line in stderr_text.splitlines() if line.strip()]

    return entry


# ---------------------------------------------------------------------------
# Corpus discovery
# ---------------------------------------------------------------------------

def discover_sources(filter_re=None):
    """Return list of (source_path, out_path) pairs for all corpus sources."""
    pattern = re.compile(filter_re) if filter_re else None
    pairs = []

    for src_dir in SOURCE_DIRS:
        if not src_dir.exists():
            continue
        for src in sorted(src_dir.rglob("*")):
            if src.suffix.lower() not in SOURCE_EXTENSIONS:
                continue
            if not src.is_file():
                continue
            rel = src.relative_to(REPO_ROOT)
            if pattern and not pattern.search(str(rel)):
                continue
            # Mirror source tree under output root, appending .png
            out = OUTPUT_ROOT / rel.with_suffix(rel.suffix + ".png")
            pairs.append((src, out))

    return pairs


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Render PUML corpus to PNG for visual audit agents.",
    )
    parser.add_argument(
        "--force", action="store_true",
        help="Re-render even if output PNG is up-to-date.",
    )
    parser.add_argument(
        "--filter", metavar="REGEX",
        help="Only render sources whose relative path matches this regex.",
    )
    parser.add_argument(
        "--workers", type=int, default=None,
        help="Worker process count (default: CPU count).",
    )
    parser.add_argument(
        "--dpi", type=float, default=96.0,
        help="Rasterization DPI (default: 96).",
    )
    parser.add_argument(
        "--quiet", "-q", action="store_true",
        help="Suppress per-file progress output.",
    )
    args = parser.parse_args()

    # Validate binary
    if not PUML_BINARY.exists():
        print(
            f"ERROR: puml binary not found at {PUML_BINARY}\n"
            "Run `cargo build --release` first.",
            file=sys.stderr,
        )
        sys.exit(2)

    # Discover corpus
    pairs = discover_sources(filter_re=args.filter)
    if not pairs:
        print("No source files found matching the given filter.", file=sys.stderr)
        sys.exit(0)

    total = len(pairs)
    print(f"Corpus: {total} source files", file=sys.stderr)

    # Build worker args
    worker_args = [
        (str(src), str(out), args.dpi, args.force)
        for src, out in pairs
    ]

    workers = args.workers or multiprocessing.cpu_count()
    workers = min(workers, total)

    t_start = time.monotonic()

    # Run renders in parallel
    manifest_entries = []
    done = 0
    with multiprocessing.Pool(processes=workers) as pool:
        for entry in pool.imap_unordered(_render_one, worker_args):
            done += 1
            manifest_entries.append(entry)
            if not args.quiet:
                status_sym = {
                    "ok": "OK",
                    "skipped": "--",
                    "failed": "FAIL",
                    "pending": "??",
                }.get(entry["status"], entry["status"])
                print(
                    f"[{done:4d}/{total}] {status_sym:4s}  {entry['source']}",
                    file=sys.stderr,
                )

    elapsed_total = time.monotonic() - t_start

    # Sort manifest for determinism
    manifest_entries.sort(key=lambda e: e["source"])

    # Summary stats
    ok_count = sum(1 for e in manifest_entries if e["status"] == "ok")
    skipped_count = sum(1 for e in manifest_entries if e["status"] == "skipped")
    failed_count = sum(1 for e in manifest_entries if e["status"] == "failed")
    warn_count = sum(1 for e in manifest_entries if e.get("warnings"))
    total_bytes = sum(
        e["size_bytes"] for e in manifest_entries
        if e["size_bytes"] is not None
    )

    manifest = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "corpus_root": str(REPO_ROOT),
        "total": total,
        "ok": ok_count,
        "skipped": skipped_count,
        "failed": failed_count,
        "warned": warn_count,
        "total_size_bytes": total_bytes,
        "elapsed_seconds": round(elapsed_total, 2),
        "dpi": args.dpi,
        "entries": manifest_entries,
    }

    MANIFEST_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(MANIFEST_PATH, "w") as fh:
        json.dump(manifest, fh, indent=2)
        fh.write("\n")

    print(
        f"\nDone in {elapsed_total:.1f}s: "
        f"{ok_count} rendered, {skipped_count} skipped, {failed_count} failed, "
        f"{warn_count} with warnings. "
        f"Total PNG size: {total_bytes / 1024:.0f} KB",
        file=sys.stderr,
    )
    print(f"Manifest: {MANIFEST_PATH}", file=sys.stderr)

    if failed_count:
        print(
            f"\n{failed_count} FAILED renders (these are visual bugs):",
            file=sys.stderr,
        )
        for e in manifest_entries:
            if e["status"] == "failed":
                print(f"  {e['source']}: {e['stderr']}", file=sys.stderr)

    # Exit 0 even with failures — failures are informational for audit
    sys.exit(0)


if __name__ == "__main__":
    main()
