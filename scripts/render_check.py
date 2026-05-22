#!/usr/bin/env python3
"""Render every .puml under docs/examples/ and report failures.

Replaces scripts/parity_harness.py.  Simpler schema, same CI gate behavior.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_VERSION = "2.0.0"

SVG_RE = re.compile(r"<svg\b[^>]*>", re.IGNORECASE)
VIEWBOX_RE = re.compile(r'viewBox\s*=\s*"([^"]+)"')
MD_FENCE_RE = re.compile(
    r"```(?:puml|pumlx|picouml|plantuml|uml|puml-sequence|uml-sequence|mermaid)\n(.*?)```",
    re.DOTALL | re.IGNORECASE,
)
DOC_SOURCE_SUFFIXES = {".puml", ".plantuml", ".picouml"}
DOC_EXAMPLE_EXCLUSIONS = {
    "docs/examples/component/04_deployment_style.puml": (
        "mixed component/deployment compatibility example; parser currently "
        "rejects deployment nodes inside a component diagram, so the legacy "
        "SVG is not treated as a regenerated docs artifact"
    ),
    "docs/examples/sequence/15_large_diagram.puml": (
        "mixed sequence/component/deployment compatibility example; parser "
        "currently rejects deployment nodes inside a component diagram, so "
        "the legacy SVG is not treated as a regenerated docs artifact"
    ),
    "docs/examples/themes/07_no_theme_default.puml": (
        "theme fallback compatibility source that currently parses as a "
        "specialized family before sequence participants are accepted; the "
        "legacy SVG is not treated as a regenerated docs artifact"
    ),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        default=str(ROOT / "docs" / "benchmarks" / "render_check_latest.json"),
        help="report JSON output path",
    )
    parser.add_argument("--quiet", action="store_true", help="suppress pass summary")
    parser.add_argument(
        "--quick",
        action="store_true",
        help="run reduced corpus for fast local validation",
    )
    parser.add_argument(
        "--dry",
        action="store_true",
        help="print discovered inputs and exit without rendering",
    )
    parser.add_argument(
        "--fail-on-doc-drift",
        action="store_true",
        help="exit non-zero when docs/examples source-to-svg drift is detected",
    )
    return parser.parse_args()


def puml_exe_name() -> str:
    return "puml.exe" if os.name == "nt" else "puml"


def cargo_debug_dir() -> Path:
    target_dir = os.environ.get("CARGO_TARGET_DIR")
    if target_dir:
        path = Path(target_dir)
        if not path.is_absolute():
            path = ROOT / path
        return path / "debug"
    return ROOT / "target" / "debug"


def ensure_puml_binary() -> Path:
    proc = subprocess.run(
        ["cargo", "build", "--quiet", "--bin", "puml"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            "failed to build puml binary for docs render check: "
            f"{proc.stderr.strip()}"
        )
    puml_bin = cargo_debug_dir() / puml_exe_name()
    if not puml_bin.exists():
        raise RuntimeError(f"puml binary was not produced at expected path: {puml_bin}")
    return puml_bin


def run_puml(args: List[str], stdin_text: Optional[str] = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["cargo", "run", "--quiet", "--bin", "puml", "--", *args],
        cwd=ROOT,
        input=stdin_text,
        capture_output=True,
        text=True,
        check=False,
    )


def render_source_text(src: str, puml_bin: Optional[Path] = None) -> Dict[str, Any]:
    if puml_bin is None:
        proc = run_puml(["-"], stdin_text=src)
    else:
        try:
            proc = subprocess.run(
                [str(puml_bin), "-"],
                cwd=ROOT,
                input=src,
                capture_output=True,
                text=True,
                check=False,
            )
        except FileNotFoundError:
            # Alternate target dirs or concurrent cleanup can stale this path;
            # rerun through Cargo so the check still returns useful output.
            proc = run_puml(["-"], stdin_text=src)
    if proc.returncode != 0:
        return {
            "ok": False,
            "exit_code": proc.returncode,
            "stderr": proc.stderr.strip(),
            "svg": None,
        }
    normalized_svg = proc.stdout.rstrip("\r\n")
    return {"ok": True, "exit_code": 0, "stderr": "", "svg": normalized_svg}


def render_source_file(source_ref: str, puml_bin: Path) -> Dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="puml-render-check-") as tmp:
        out_path = Path(tmp) / "artifact.svg"
        try:
            proc = subprocess.run(
                [str(puml_bin), source_ref, "-o", str(out_path)],
                cwd=ROOT,
                capture_output=True,
                text=True,
                check=False,
            )
        except FileNotFoundError:
            # Alternate target dirs or concurrent cleanup can stale this path;
            # rerun through Cargo so the check still returns useful output.
            proc = run_puml([source_ref, "-o", str(out_path)])
        if proc.returncode != 0:
            return {
                "ok": False,
                "exit_code": proc.returncode,
                "stderr": proc.stderr.strip(),
                "svg": None,
            }
        return {
            "ok": True,
            "exit_code": 0,
            "stderr": "",
            "svg": out_path.read_text(encoding="utf-8"),
        }


def canonicalize_svg(svg: str) -> str:
    return svg.rstrip("\r\n")


def git_tracked_doc_sources(docs_examples: Path) -> List[Path]:
    proc = subprocess.run(
        [
            "git",
            "ls-files",
            "--",
            "docs/examples/*.puml",
            "docs/examples/**/*.puml",
            "docs/examples/*.plantuml",
            "docs/examples/**/*.plantuml",
            "docs/examples/*.picouml",
            "docs/examples/**/*.picouml",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode == 0:
        files = [
            ROOT / line
            for line in proc.stdout.splitlines()
            if Path(line).suffix in DOC_SOURCE_SUFFIXES
        ]
        return sorted(set(files))

    return sorted(
        path
        for path in docs_examples.rglob("*")
        if path.is_file() and path.suffix in DOC_SOURCE_SUFFIXES
    )


def discover_entries() -> List[Dict[str, Any]]:
    """Discover all docs/examples source files (and fenced snippets in READMEs)."""
    docs_examples = ROOT / "docs" / "examples"
    if not docs_examples.exists():
        return []

    rows: List[Dict[str, Any]] = []

    # .puml / .plantuml / .picouml source files
    for source_path in git_tracked_doc_sources(docs_examples):
        artifact = source_path.with_suffix(".svg")
        rel_source = str(source_path.relative_to(ROOT))
        rel_artifact = str(artifact.relative_to(ROOT))
        rows.append(
            {
                "source_kind": "source_file",
                "source_ref": rel_source,
                "artifact_svg": rel_artifact,
                "source_text": source_path.read_text(encoding="utf-8"),
                "excluded": rel_source in DOC_EXAMPLE_EXCLUSIONS,
                "exclusion_reason": DOC_EXAMPLE_EXCLUSIONS.get(rel_source),
            }
        )

    # Fenced code snippets inside docs/examples/**/*.md
    for md_path in sorted(docs_examples.rglob("*.md")):
        raw = md_path.read_text(encoding="utf-8")
        rel_md = str(md_path.relative_to(ROOT))
        snippet_index = 0
        for snippet in MD_FENCE_RE.findall(raw):
            snippet_index += 1
            artifact = md_path.with_name(f"{md_path.stem}_snippet_{snippet_index}.svg")
            rows.append(
                {
                    "source_kind": "inline_snippet",
                    "source_ref": f"{rel_md}#snippet-{snippet_index}",
                    "artifact_svg": str(artifact.relative_to(ROOT)),
                    "source_text": snippet.strip() + "\n",
                    "excluded": False,
                    "exclusion_reason": None,
                }
            )

    return rows


def evaluate_entry(row: Dict[str, Any], puml_bin: Optional[Path]) -> Dict[str, Any]:
    """Render one entry; return a flat result dict."""
    artifact_path = ROOT / row["artifact_svg"]

    if row["excluded"]:
        return {
            "source_kind": row["source_kind"],
            "source_ref": row["source_ref"],
            "artifact_svg": row["artifact_svg"],
            "artifact_exists": artifact_path.exists(),
            "artifact_up_to_date": False,
            "excluded": True,
            "exclusion_reason": row["exclusion_reason"],
            "status": "excluded",
            "notes": [row["exclusion_reason"]],
        }

    # Render
    if puml_bin is not None and row["source_kind"] == "source_file":
        render = render_source_file(row["source_ref"], puml_bin)
    else:
        render = render_source_text(row["source_text"], puml_bin=puml_bin)

    notes: List[str] = []
    if not render["ok"]:
        notes.append(f"render failed (exit={render['exit_code']}): {render['stderr']}")
        return {
            "source_kind": row["source_kind"],
            "source_ref": row["source_ref"],
            "artifact_svg": row["artifact_svg"],
            "artifact_exists": artifact_path.exists(),
            "artifact_up_to_date": False,
            "excluded": False,
            "exclusion_reason": None,
            "status": "fail",
            "notes": notes,
        }

    artifact_exists = artifact_path.exists()
    artifact_up_to_date = False
    if artifact_exists:
        disk_svg = artifact_path.read_text(encoding="utf-8")
        artifact_up_to_date = canonicalize_svg(disk_svg) == canonicalize_svg(render["svg"])
    else:
        notes.append("artifact missing")

    if artifact_exists and not artifact_up_to_date:
        notes.append("artifact content does not match current renderer output")

    status = "pass" if artifact_exists and artifact_up_to_date else "fail"
    return {
        "source_kind": row["source_kind"],
        "source_ref": row["source_ref"],
        "artifact_svg": row["artifact_svg"],
        "artifact_exists": artifact_exists,
        "artifact_up_to_date": artifact_up_to_date,
        "excluded": False,
        "exclusion_reason": None,
        "status": status,
        "notes": notes,
    }


def main() -> int:
    args = parse_args()

    rows = discover_entries()

    if args.quick:
        rows = rows[:10]

    if args.dry:
        print(f"[render_check] dry run; entries={len(rows)} quick={args.quick}")
        for row in rows:
            print(f"  - {row['source_ref']}")
        return 0

    puml_bin = ensure_puml_binary() if rows else None
    entries = [evaluate_entry(row, puml_bin) for row in rows]

    total = len(entries)
    passed = sum(1 for e in entries if e["status"] == "pass")
    excluded = sum(1 for e in entries if e["status"] == "excluded")
    failed = total - passed - excluded

    report: Dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "summary": {
            "total": total,
            "passed": passed,
            "excluded": excluded,
            "failed": failed,
        },
        "entries": entries,
    }

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    if not args.quiet:
        print(
            f"[render_check] wrote {out_path} "
            f"(total={total}, passed={passed}, excluded={excluded}, failed={failed})"
        )

    if args.fail_on_doc_drift and failed > 0:
        print(
            f"[render_check] docs example drift detected: failed={failed}",
            file=sys.stderr,
        )
        return 4

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
