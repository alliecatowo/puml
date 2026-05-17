#!/usr/bin/env python3
"""Deterministic differential oracle smoke checks against PlantUML SVG output."""

from __future__ import annotations

import argparse
import json
import re
import shlex
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_VERSION = "1.1.0"
DEFAULT_OUTPUT = ROOT / "docs" / "benchmarks" / "oracle_smoke_latest.json"
SVG_TAG_RE = re.compile(r"<\s*([a-zA-Z][a-zA-Z0-9:_-]*)\b")
VIEWBOX_RE = re.compile(r'viewBox\s*=\s*"([^"]+)"', re.IGNORECASE)

FIXTURES: List[Dict[str, Any]] = [
    {
        "fixture": "basic/hello.puml",
        "category": "sequence-core",
        "support_status": "implemented",
        "expected_oracle_category": "match",
        "drift_reason": "baseline sequence render should stay close to the reference SVG shape",
        "plantuml_reference": "https://plantuml.com/sequence-diagram",
        "expect_tokens": ["Alice", "Bob", "hello"],
    },
    {
        "fixture": "participants/valid_aliases.puml",
        "category": "sequence-core",
        "support_status": "implemented",
        "expected_oracle_category": "match",
        "drift_reason": "participant aliases are part of the implemented sequence subset",
        "plantuml_reference": "https://plantuml.com/sequence-diagram",
        "expect_tokens": ["User", "API", "request"],
    },
    {
        "fixture": "groups/valid_ref_and_else_rendering.puml",
        "category": "sequence-fragments",
        "support_status": "implemented",
        "expected_oracle_category": "match",
        "drift_reason": "fragment labels and branch text should survive both renderers",
        "plantuml_reference": "https://plantuml.com/sequence-diagram",
        "expect_tokens": ["ref", "else"],
    },
    {
        "fixture": "notes/valid_multiline_blocks.puml",
        "category": "sequence-notes",
        "support_status": "implemented",
        "expected_oracle_category": "match",
        "drift_reason": "multiline notes are supported but remain useful text-presence sentinels",
        "plantuml_reference": "https://plantuml.com/sequence-diagram",
        "expect_tokens": ["note"],
    },
    {
        "fixture": "styling/valid_skinparam_unsupported.puml",
        "category": "styling-partial",
        "support_status": "partial",
        "expected_oracle_category": "drift",
        "drift_reason": "unsupported skinparam keys are accepted as deterministic warnings rather than full PlantUML styling",
        "plantuml_reference": "https://plantuml.com/skinparam",
        "expect_tokens": ["Alice", "Bob"],
    },
    {
        "fixture": "errors/invalid_preproc_dynamic_invoke.puml",
        "category": "preprocessor-advanced",
        "support_status": "partial",
        "expected_oracle_category": "jar-only",
        "drift_reason": "dynamic invocation remains outside the deterministic preprocessor subset",
        "plantuml_reference": "https://plantuml.com/preprocessing",
        "expect_tokens": [],
    },
    {
        "fixture": "families/valid_salt_login_form.puml",
        "category": "family-partial",
        "support_status": "partial",
        "expected_oracle_category": "drift",
        "drift_reason": "Salt widget breadth is intentionally narrower than the Java PlantUML reference",
        "plantuml_reference": "https://plantuml.com/salt",
        "expect_tokens": ["Login"],
    },
    {
        "fixture": "families/valid_chart_bar_quarterly.puml",
        "category": "family-partial",
        "support_status": "partial",
        "expected_oracle_category": "drift",
        "drift_reason": "chart axis, legend, and style semantics remain fixture-backed partial parity",
        "plantuml_reference": "https://plantuml.com/chart-diagram",
        "expect_tokens": [],
    },
    {
        "fixture": "families/valid_mindmap_orientation.puml",
        "category": "family-partial",
        "support_status": "partial",
        "expected_oracle_category": "drift",
        "drift_reason": "mindmap orientation metadata is deterministic but not full PlantUML layout parity",
        "plantuml_reference": "https://plantuml.com/mindmap-diagram",
        "expect_tokens": [],
    },
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--quick", action="store_true", help="run a reduced fixture set")
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="report output path",
    )
    parser.add_argument(
        "--oracle-command",
        default="plantuml -tsvg -pipe",
        help="oracle render command executed with fixture source on stdin",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="exit non-zero when any smoke comparison fails",
    )
    parser.add_argument(
        "--dry",
        "--dry-run",
        dest="dry",
        action="store_true",
        help="write report metadata without executing render commands",
    )
    parser.add_argument("--quiet", action="store_true", help="suppress pass summary")
    return parser.parse_args()


def run_local_render(src: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["cargo", "run", "--quiet", "--", "-"],
        cwd=ROOT,
        input=src,
        capture_output=True,
        text=True,
        check=False,
    )


def run_oracle_render(command: str, src: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        shlex.split(command),
        cwd=ROOT,
        input=src,
        capture_output=True,
        text=True,
        check=False,
    )


def parse_viewbox(svg: str) -> Dict[str, float] | None:
    match = VIEWBOX_RE.search(svg)
    if match is None:
        return None
    parts = match.group(1).replace(",", " ").split()
    if len(parts) != 4:
        return None
    try:
        x, y, w, h = (float(p) for p in parts)
    except ValueError:
        return None
    return {"x": x, "y": y, "width": w, "height": h}


def svg_tag_histogram(svg: str) -> Dict[str, int]:
    histogram: Dict[str, int] = {}
    for raw in SVG_TAG_RE.findall(svg):
        tag = raw.lower().split(":")[-1]
        if tag.startswith("/"):
            continue
        histogram[tag] = histogram.get(tag, 0) + 1
    return histogram


def normalize_svg(svg: str) -> str:
    return svg.rstrip("\r\n")


def classification_for(fixture: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "category": fixture["category"],
        "support_status": fixture["support_status"],
        "expected_oracle_category": fixture["expected_oracle_category"],
        "drift_reason": fixture["drift_reason"],
        "plantuml_reference": fixture["plantuml_reference"],
    }


def count_by(fixtures: List[Dict[str, Any]], field: str) -> Dict[str, int]:
    counts: Dict[str, int] = {}
    for row in fixtures:
        value = str(row["classification"][field])
        counts[value] = counts.get(value, 0) + 1
    return dict(sorted(counts.items()))


def evaluate_fixture(fixture: Dict[str, Any], args: argparse.Namespace) -> Dict[str, Any]:
    rel = fixture["fixture"]
    src_path = ROOT / "tests" / "fixtures" / rel
    src = src_path.read_text(encoding="utf-8")
    classification = classification_for(fixture)

    if args.dry:
        return {
            "fixture": rel,
            "classification": classification,
            "local": {"attempted": False, "exit_code": None, "stderr": "", "svg_bytes": None},
            "oracle": {
                "attempted": False,
                "exit_code": None,
                "stderr": "",
                "svg_bytes": None,
                "command": args.oracle_command,
            },
            "comparison": {
                "state": "not-run",
                "passed": None,
                "notes": ["dry run: local and oracle render commands were not executed"],
                "token_checks": [],
                "local_viewbox": None,
                "oracle_viewbox": None,
                "local_tags": {},
                "oracle_tags": {},
            },
        }

    local = run_local_render(src)
    oracle = run_oracle_render(args.oracle_command, src)

    local_svg = normalize_svg(local.stdout) if local.returncode == 0 else ""
    oracle_svg = normalize_svg(oracle.stdout) if oracle.returncode == 0 else ""
    local_viewbox = parse_viewbox(local_svg) if local.returncode == 0 else None
    oracle_viewbox = parse_viewbox(oracle_svg) if oracle.returncode == 0 else None

    token_checks: List[Dict[str, Any]] = []
    for token in fixture.get("expect_tokens", []):
        in_local = token in local_svg
        in_oracle = token in oracle_svg
        token_checks.append({"token": token, "local": in_local, "oracle": in_oracle})

    notes: List[str] = []
    if local.returncode != 0:
        notes.append(f"local renderer failed with exit={local.returncode}")
    if oracle.returncode != 0:
        notes.append(f"oracle renderer failed with exit={oracle.returncode}")
    if local.returncode == 0 and local_viewbox is None:
        notes.append("local renderer output missing valid viewBox")
    if oracle.returncode == 0 and oracle_viewbox is None:
        notes.append("oracle renderer output missing valid viewBox")
    if any(not (check["local"] and check["oracle"]) for check in token_checks):
        notes.append("expected semantic token missing in one or both SVG outputs")

    passed = (
        local.returncode == 0
        and oracle.returncode == 0
        and local_viewbox is not None
        and oracle_viewbox is not None
        and all(check["local"] and check["oracle"] for check in token_checks)
    )

    if passed:
        notes.append("smoke parity checks passed")

    return {
        "fixture": rel,
        "classification": classification,
        "local": {
            "attempted": True,
            "exit_code": local.returncode,
            "stderr": local.stderr.strip(),
            "svg_bytes": len(local_svg.encode("utf-8")) if local.returncode == 0 else None,
        },
        "oracle": {
            "attempted": True,
            "exit_code": oracle.returncode,
            "stderr": oracle.stderr.strip(),
            "svg_bytes": len(oracle_svg.encode("utf-8")) if oracle.returncode == 0 else None,
            "command": args.oracle_command,
        },
        "comparison": {
            "state": "passed" if passed else "failed",
            "passed": passed,
            "notes": notes,
            "token_checks": token_checks,
            "local_viewbox": local_viewbox,
            "oracle_viewbox": oracle_viewbox,
            "local_tags": svg_tag_histogram(local_svg) if local.returncode == 0 else {},
            "oracle_tags": svg_tag_histogram(oracle_svg) if oracle.returncode == 0 else {},
        },
    }


def main() -> int:
    args = parse_args()
    selected = FIXTURES[:2] if args.quick else FIXTURES

    fixtures = [evaluate_fixture(fix, args) for fix in selected]
    if args.dry:
        passed = 0
        failed = 0
        not_run = len(fixtures)
    else:
        passed = sum(1 for row in fixtures if row["comparison"]["passed"])
        failed = len(fixtures) - passed
        not_run = 0

    report = {
        "schema_version": SCHEMA_VERSION,
        "generated_at_utc": datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z"),
        "tool": {
            "name": "puml",
            "runner": "cargo run --quiet -- -",
            "cwd": str(ROOT),
            "quick_mode": args.quick,
            "dry_run": args.dry,
        },
        "oracle": {
            "interface_version": "1",
            "mode": "metadata-dry-run" if args.dry else "plantuml-smoke",
            "enabled": not args.dry,
            "command": args.oracle_command,
            "comparison_only": True,
            "runtime_dependency": False,
            "build_dependency": False,
            "normal_cargo_test_uses_oracle": False,
            "deterministic_controls": [
                "fixed fixture corpus ordering",
                "fixture-backed expected oracle categories",
                "exact token-presence checks",
                "viewBox presence checks",
                "structured JSON report schema",
            ],
        },
        "summary": {
            "total": len(fixtures),
            "passed": passed,
            "failed": failed,
            "not_run": not_run,
            "by_fixture_category": count_by(fixtures, "category"),
            "by_support_status": count_by(fixtures, "support_status"),
            "by_expected_oracle_category": count_by(fixtures, "expected_oracle_category"),
        },
        "fixtures": fixtures,
    }

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    if not args.quiet:
        print(
            f"oracle smoke wrote {out_path} (total={len(fixtures)}, passed={passed}, failed={failed})"
        )

    if args.strict and failed > 0:
        print(
            f"[oracle-smoke] differential oracle failures: {failed}",
            file=sys.stderr,
        )
        return 5
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
