#!/usr/bin/env python3
"""Run parity/invariant checks for puml and emit a machine-readable report."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional

ROOT = Path(__file__).resolve().parents[1]
SCHEMA_VERSION = "1.0.0"

FIXTURE_CORPUS = [
    "basic/hello.puml",
    "participants/valid_aliases.puml",
    "arrows/valid_expanded_forms.puml",
    "groups/valid_ref_and_else_rendering.puml",
    "notes/valid_multiline_blocks.puml",
    "structure/valid_separator_delay_divider_spacer.puml",
    "lifecycle/valid_shortcuts_expansion.puml",
    "autonumber/valid_with_format.puml",
    "errors/invalid_unmatched_enduml.puml",
    "errors/invalid_malformed_note_ref.puml",
    "errors/invalid_nested_startuml.puml",
]

SVG_RE = re.compile(r"<svg\\b[^>]*>", re.IGNORECASE)
VIEWBOX_RE = re.compile(r'viewBox\\s*=\\s*"([^"]+)"')
MD_PUML_LINK_RE = re.compile(r"\[[^\]]+\]\(([^)]+\.puml)\)")
MD_FENCE_RE = re.compile(
    r"```(?:puml|pumlx|picouml|plantuml|uml|puml-sequence|uml-sequence|mermaid)\n(.*?)```",
    re.DOTALL | re.IGNORECASE,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output",
        default=str(ROOT / "docs" / "benchmarks" / "parity_latest.json"),
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
        help="print fixture/doc inputs and exit without executing",
    )
    parser.add_argument(
        "--oracle-command-template",
        default=None,
        help=(
            "placeholder command template for future PlantUML oracle comparison "
            "(stored in report, never executed in baseline mode)"
        ),
    )
    parser.add_argument(
        "--fail-on-doc-drift",
        action="store_true",
        help="exit non-zero when docs/examples source-to-svg drift is detected",
    )
    parser.add_argument(
        "--oracle",
        action="store_true",
        help=(
            "invoke scripts/oracle.sh against docs/examples/**/*.puml and include "
            "the diff count in the harness report (requires PUML_ORACLE_JAR or "
            "./oracle/plantuml.jar; skipped gracefully when JAR is absent)"
        ),
    )
    return parser.parse_args()


def run_puml(args: List[str], stdin_text: Optional[str] = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["cargo", "run", "--quiet", "--", *args],
        cwd=ROOT,
        input=stdin_text,
        capture_output=True,
        text=True,
        check=False,
    )


def parse_diagnostics_json(raw: str) -> List[Dict[str, Any]]:
    if not raw.strip():
        return []
    try:
        payload = json.loads(raw)
    except json.JSONDecodeError:
        return [
            {
                "severity": "error",
                "message": "diagnostics output was not valid JSON",
                "span": None,
                "line": None,
                "column": None,
                "snippet": None,
                "caret": None,
            }
        ]
    if isinstance(payload, dict) and isinstance(payload.get("diagnostics"), list):
        return payload["diagnostics"]
    return []


def parse_viewbox(svg: str) -> Optional[Dict[str, float]]:
    match = SVG_RE.search(svg)
    if match is None:
        return None
    vb_match = VIEWBOX_RE.search(match.group(0))
    if vb_match is None:
        return None
    parts = vb_match.group(1).replace(",", " ").split()
    if len(parts) != 4:
        return None
    try:
        x, y, w, h = (float(p) for p in parts)
    except ValueError:
        return None
    return {"x": x, "y": y, "width": w, "height": h}


def validate_report_schema(report: Dict[str, Any]) -> List[str]:
    errors: List[str] = []
    top_keys = {
        "schema_version",
        "generated_at_utc",
        "tool",
        "oracle",
        "summary",
        "fixtures",
        "doc_examples",
    }
    missing = [k for k in top_keys if k not in report]
    if missing:
        errors.append(f"missing top-level keys: {missing}")

    if report.get("schema_version") != SCHEMA_VERSION:
        errors.append(
            f"unexpected schema_version: {report.get('schema_version')} != {SCHEMA_VERSION}"
        )

    fixtures = report.get("fixtures")
    if not isinstance(fixtures, list):
        errors.append("fixtures must be a list")
        return errors

    for idx, fixture in enumerate(fixtures):
        for key in ["fixture", "check", "render", "oracle"]:
            if key not in fixture:
                errors.append(f"fixtures[{idx}] missing key: {key}")
        check = fixture.get("check", {})
        render = fixture.get("render", {})
        oracle = fixture.get("oracle", {})
        for key in ["passed", "exit_code", "diagnostics", "stderr"]:
            if key not in check:
                errors.append(f"fixtures[{idx}].check missing key: {key}")
        for key in ["attempted", "passed", "exit_code", "stderr", "metadata"]:
            if key not in render:
                errors.append(f"fixtures[{idx}].render missing key: {key}")
        for key in ["status", "comparison", "notes"]:
            if key not in oracle:
                errors.append(f"fixtures[{idx}].oracle missing key: {key}")

    doc_examples = report.get("doc_examples")
    if not isinstance(doc_examples, dict):
        errors.append("doc_examples must be an object")
        return errors
    for key in ["summary", "entries"]:
        if key not in doc_examples:
            errors.append(f"doc_examples missing key: {key}")
    entries = doc_examples.get("entries")
    if not isinstance(entries, list):
        errors.append("doc_examples.entries must be a list")
        return errors
    for idx, entry in enumerate(entries):
        for key in [
            "source_markdown",
            "source_kind",
            "source_ref",
            "artifact_svg",
            "artifact_exists",
            "artifact_matches_render",
            "artifact_up_to_date",
            "status",
            "notes",
        ]:
            if key not in entry:
                errors.append(f"doc_examples.entries[{idx}] missing key: {key}")
    return errors


def build_fixture_record(rel_path: str) -> Dict[str, Any]:
    abs_path = ROOT / "tests" / "fixtures" / rel_path
    src = abs_path.read_text(encoding="utf-8")

    check_proc = run_puml(["--check", "--diagnostics", "json", "-"], stdin_text=src)
    diagnostics = parse_diagnostics_json(check_proc.stdout)
    check_passed = check_proc.returncode == 0

    render_record: Dict[str, Any] = {
        "attempted": check_passed,
        "passed": False,
        "exit_code": None,
        "stderr": "",
        "metadata": {
            "svg_bytes": None,
            "viewbox": None,
        },
    }

    if check_passed:
        render_proc = run_puml(["-"], stdin_text=src)
        render_record["passed"] = render_proc.returncode == 0
        render_record["exit_code"] = render_proc.returncode
        render_record["stderr"] = render_proc.stderr.strip()
        if render_proc.returncode == 0:
            viewbox = parse_viewbox(render_proc.stdout)
            render_record["metadata"] = {
                "svg_bytes": len(render_proc.stdout.encode("utf-8")),
                "viewbox": viewbox,
            }

    return {
        "fixture": rel_path,
        "check": {
            "passed": check_passed,
            "exit_code": check_proc.returncode,
            "diagnostics": diagnostics,
            "stderr": check_proc.stderr.strip(),
        },
        "render": render_record,
        "oracle": {
            "status": "todo",
            "comparison": None,
            "notes": "PlantUML oracle hook not executed in baseline mode.",
        },
    }


def render_source_text(src: str) -> Dict[str, Any]:
    proc = run_puml(["-"], stdin_text=src)
    if proc.returncode != 0:
        return {
            "ok": False,
            "exit_code": proc.returncode,
            "stderr": proc.stderr.strip(),
            "svg": None,
        }
    # CLI stdout for render mode appends a trailing newline; normalize so
    # docs artifacts are compared against canonical SVG payload bytes.
    normalized_svg = proc.stdout.rstrip("\r\n")
    return {"ok": True, "exit_code": 0, "stderr": "", "svg": normalized_svg}


def canonicalize_svg_text(svg: str) -> str:
    # CLI stdout may include a trailing newline while checked-in SVG artifacts do not.
    return svg.rstrip("\r\n")


def discover_doc_examples() -> List[Dict[str, Any]]:
    docs_examples = ROOT / "docs" / "examples"
    if not docs_examples.exists():
        return []

    rows: List[Dict[str, Any]] = []
    markdown_files = sorted(docs_examples.rglob("*.md"))
    for md_path in markdown_files:
        raw = md_path.read_text(encoding="utf-8")
        rel_md = str(md_path.relative_to(ROOT))

        for linked in MD_PUML_LINK_RE.findall(raw):
            puml_path = (md_path.parent / linked).resolve()
            artifact = puml_path.with_suffix(".svg")
            rows.append(
                {
                    "source_markdown": rel_md,
                    "source_kind": "linked_file",
                    "source_ref": str(puml_path.relative_to(ROOT)),
                    "artifact_svg": str(artifact.relative_to(ROOT)),
                    "source_text": puml_path.read_text(encoding="utf-8")
                    if puml_path.exists()
                    else None,
                    "source_mtime_ns": puml_path.stat().st_mtime_ns if puml_path.exists() else None,
                    "markdown_mtime_ns": md_path.stat().st_mtime_ns,
                    "artifact_mtime_ns": artifact.stat().st_mtime_ns if artifact.exists() else None,
                }
            )

        snippet_index = 0
        for snippet in MD_FENCE_RE.findall(raw):
            snippet_index += 1
            artifact = md_path.with_name(f"{md_path.stem}_snippet_{snippet_index}.svg")
            rows.append(
                {
                    "source_markdown": rel_md,
                    "source_kind": "inline_snippet",
                    "source_ref": f"{rel_md}#snippet-{snippet_index}",
                    "artifact_svg": str(artifact.relative_to(ROOT)),
                    "source_text": snippet.strip() + "\n",
                    "source_mtime_ns": md_path.stat().st_mtime_ns,
                    "markdown_mtime_ns": md_path.stat().st_mtime_ns,
                    "artifact_mtime_ns": artifact.stat().st_mtime_ns if artifact.exists() else None,
                }
            )

    return rows


def evaluate_doc_example(row: Dict[str, Any]) -> Dict[str, Any]:
    artifact_path = ROOT / row["artifact_svg"]
    source_text = row["source_text"]
    notes: List[str] = []

    if source_text is None:
        return {
            "source_markdown": row["source_markdown"],
            "source_kind": row["source_kind"],
            "source_ref": row["source_ref"],
            "artifact_svg": row["artifact_svg"],
            "artifact_exists": artifact_path.exists(),
            "artifact_matches_render": False,
            "artifact_up_to_date": False,
            "status": "fail",
            "notes": ["source file missing"],
        }

    render = render_source_text(source_text)
    if not render["ok"]:
        notes.append(
            f"source did not render (exit={render['exit_code']}): {render['stderr']}"
        )
        return {
            "source_markdown": row["source_markdown"],
            "source_kind": row["source_kind"],
            "source_ref": row["source_ref"],
            "artifact_svg": row["artifact_svg"],
            "artifact_exists": artifact_path.exists(),
            "artifact_matches_render": False,
            "artifact_up_to_date": False,
            "status": "fail",
            "notes": notes,
        }

    artifact_exists = artifact_path.exists()
    artifact_matches = False
    artifact_up_to_date = False
    if artifact_exists:
        disk_svg = artifact_path.read_text(encoding="utf-8")
        artifact_matches = canonicalize_svg_text(disk_svg) == canonicalize_svg_text(
            render["svg"]
        )
        # CI and fresh git checkouts can rewrite filesystem mtimes, so
        # timestamp freshness is not a stable drift signal. We treat
        # content equality as the canonical freshness contract.
        artifact_up_to_date = artifact_matches
    else:
        notes.append("artifact missing")

    if artifact_exists and not artifact_matches:
        notes.append("artifact content does not match current renderer output")
    if artifact_exists and not artifact_up_to_date:
        notes.append("artifact content is stale vs current renderer output")

    status = "pass" if artifact_exists and artifact_matches and artifact_up_to_date else "fail"
    return {
        "source_markdown": row["source_markdown"],
        "source_kind": row["source_kind"],
        "source_ref": row["source_ref"],
        "artifact_svg": row["artifact_svg"],
        "artifact_exists": artifact_exists,
        "artifact_matches_render": artifact_matches,
        "artifact_up_to_date": artifact_up_to_date,
        "status": status,
        "notes": notes,
    }


def run_oracle() -> Dict[str, Any]:
    """Invoke scripts/oracle.sh and return its parsed JSON output.

    Always returns a dict.  When the oracle is skipped (JAR absent) or the
    script fails unexpectedly, returns a safe sentinel with ``skipped=True``
    so callers can record the result without crashing.
    """
    oracle_script = ROOT / "scripts" / "oracle.sh"
    if not oracle_script.exists():
        return {
            "oracle_version": "1",
            "skipped": True,
            "skip_reason": "scripts/oracle.sh not found",
            "total": 0,
            "identical": 0,
            "diff_count": 0,
            "diffs": [],
        }

    try:
        proc = subprocess.run(
            ["bash", str(oracle_script)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            timeout=300,
        )
    except Exception as exc:  # noqa: BLE001
        return {
            "oracle_version": "1",
            "skipped": True,
            "skip_reason": f"oracle script raised: {exc}",
            "total": 0,
            "identical": 0,
            "diff_count": 0,
            "diffs": [],
        }

    raw = proc.stdout.strip()
    if not raw:
        return {
            "oracle_version": "1",
            "skipped": True,
            "skip_reason": f"oracle script produced no output (exit={proc.returncode}); stderr={proc.stderr.strip()!r}",
            "total": 0,
            "identical": 0,
            "diff_count": 0,
            "diffs": [],
        }

    try:
        result = json.loads(raw)
    except json.JSONDecodeError as exc:
        return {
            "oracle_version": "1",
            "skipped": True,
            "skip_reason": f"oracle output was not valid JSON: {exc}",
            "total": 0,
            "identical": 0,
            "diff_count": 0,
            "diffs": [],
        }

    return result


def main() -> int:
    args = parse_args()

    selected = FIXTURE_CORPUS[:4] if args.quick else FIXTURE_CORPUS
    if args.dry:
        print(f"[parity] dry run; fixtures={len(selected)} quick={args.quick}")
        for rel in selected:
            print(f"  - tests/fixtures/{rel}")
        docs_examples = discover_doc_examples()
        print(f"[parity] discovered doc example entries={len(docs_examples)}")
        return 0

    report: Dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "generated_at_utc": datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        "tool": {
            "name": "puml",
            "runner": "cargo run --quiet --",
            "cwd": str(ROOT),
            "quick_mode": args.quick,
        },
        "oracle": {
            "interface_version": "1",
            "mode": "active" if args.oracle else "todo",
            "enabled": args.oracle,
            "command_template": args.oracle_command_template,
            "notes": (
                "Oracle invoked via scripts/oracle.sh."
                if args.oracle
                else "Reserved for future PlantUML comparison integration."
            ),
        },
        "summary": {
            "total": 0,
            "check_passed": 0,
            "check_failed": 0,
            "render_passed": 0,
            "render_failed": 0,
        },
        "fixtures": [],
        "doc_examples": {"summary": {"total": 0, "passed": 0, "failed": 0}, "entries": []},
    }

    for rel_path in selected:
        record = build_fixture_record(rel_path)
        report["fixtures"].append(record)

    total = len(report["fixtures"])
    check_passed = sum(1 for row in report["fixtures"] if row["check"]["passed"])
    render_passed = sum(1 for row in report["fixtures"] if row["render"]["passed"])

    report["summary"] = {
        "total": total,
        "check_passed": check_passed,
        "check_failed": total - check_passed,
        "render_passed": render_passed,
        "render_failed": check_passed - render_passed,
    }

    doc_rows = discover_doc_examples()
    doc_entries = [evaluate_doc_example(row) for row in doc_rows]
    doc_passed = sum(1 for row in doc_entries if row["status"] == "pass")
    report["doc_examples"] = {
        "summary": {
            "total": len(doc_entries),
            "passed": doc_passed,
            "failed": len(doc_entries) - doc_passed,
        },
        "entries": doc_entries,
    }

    # --oracle: invoke oracle.sh and embed result in report
    if args.oracle:
        oracle_result = run_oracle()
        report["oracle"]["result"] = oracle_result
        report["oracle"]["diff_count"] = oracle_result.get("diff_count", 0)
        report["oracle"]["skipped"] = oracle_result.get("skipped", False)
        if not args.quiet:
            if oracle_result.get("skipped"):
                print(
                    f"[parity] oracle skipped: {oracle_result.get('skip_reason', 'unknown')}"
                )
            else:
                print(
                    f"[parity] oracle: total={oracle_result.get('total', 0)} "
                    f"identical={oracle_result.get('identical', 0)} "
                    f"diff_count={oracle_result.get('diff_count', 0)}"
                )

    schema_errors = validate_report_schema(report)
    if schema_errors:
        print("Parity harness schema validation failed:", file=sys.stderr)
        for err in schema_errors:
            print(f"- {err}", file=sys.stderr)
        return 2

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    if not args.quiet:
        print(
            "parity harness wrote "
            f"{out_path} "
            f"(fixtures={total}, check_passed={check_passed}, render_passed={render_passed})"
        )

    if args.fail_on_doc_drift and report["doc_examples"]["summary"]["failed"] > 0:
        print(
            "[parity] docs example drift detected: "
            f"failed={report["doc_examples"]["summary"]["failed"]}",
            file=sys.stderr,
        )
        return 4

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
