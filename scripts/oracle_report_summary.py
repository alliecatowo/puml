#!/usr/bin/env python3
"""Publish a durable, human-readable summary for JAR-backed oracle reports."""

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path
from typing import Any, Dict, Iterable, List


DRIFT_CATEGORIES = {"drift", "puml-only", "jar-only", "both-fail"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, help="oracle_report.json path")
    parser.add_argument("--markdown", required=True, help="markdown summary output path")
    parser.add_argument("--json", required=True, help="machine-readable summary output path")
    parser.add_argument(
        "--pages-dir",
        help="optional static report directory containing index.html plus JSON/Markdown files",
    )
    return parser.parse_args()


def read_report(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def family_for(path: str) -> str:
    parts = path.split("/")
    if len(parts) >= 3 and parts[0] == "tests" and parts[1] == "fixtures":
        return parts[2]
    if len(parts) >= 3 and parts[0] == "docs" and parts[1] == "examples":
        return parts[2] if len(parts) > 3 else "examples-root"
    return parts[0] if parts else "unknown"


def count_by(rows: Iterable[Dict[str, Any]], key: str) -> Dict[str, int]:
    counts: Dict[str, int] = {}
    for row in rows:
        value = str(row[key])
        counts[value] = counts.get(value, 0) + 1
    return dict(sorted(counts.items()))


def top_drift_families(fixtures: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    grouped: Dict[str, Dict[str, Any]] = {}
    for fixture in fixtures:
        category = str(fixture.get("category", "unknown"))
        if category not in DRIFT_CATEGORIES:
            continue

        path = str(fixture.get("path", ""))
        family = family_for(path)
        entry = grouped.setdefault(
            family,
            {
                "family": family,
                "count": 0,
                "categories": {},
                "representative_fixtures": [],
            },
        )
        entry["count"] += 1
        entry["categories"][category] = entry["categories"].get(category, 0) + 1
        if len(entry["representative_fixtures"]) < 5:
            entry["representative_fixtures"].append(path)

    rows = sorted(grouped.values(), key=lambda row: (-row["count"], row["family"]))
    for row in rows:
        row["categories"] = dict(sorted(row["categories"].items()))
    return rows


def summarize(report: Dict[str, Any]) -> Dict[str, Any]:
    summary = report.get("summary", {})
    total = int(summary.get("total", 0) or 0)
    match = int(summary.get("match", 0) or 0)
    drift = int(summary.get("drift", 0) or 0)
    puml_only = int(summary.get("puml_only", 0) or 0)
    jar_only = int(summary.get("jar_only", 0) or 0)
    both_fail = int(summary.get("both_fail", 0) or 0)
    match_pct = int((match * 100) / total) if total else 0
    promoted_gate = report.get("promoted_gate") or {}
    promoted_failed = promoted_gate.get("status") == "fail"

    if report.get("skipped"):
        gate_status = "skipped"
    elif promoted_failed:
        gate_status = "fail"
    elif match_pct >= 80:
        gate_status = "pass"
    elif match_pct >= 50:
        gate_status = "advisory"
    else:
        gate_status = "fail"

    fixtures = list(report.get("fixtures", []))
    return {
        "schema_version": "1.0",
        "source_schema_version": report.get("schema_version"),
        "source_timestamp": report.get("timestamp"),
        "skipped": bool(report.get("skipped", False)),
        "skip_reason": report.get("reason"),
        "jar_version": report.get("jar_version"),
        "gate_status": gate_status,
        "match_pct": match_pct,
        "fixture_count": total,
        "outcome_counts": {
            "pass": match,
            "advisory": drift,
            "fail": puml_only + jar_only + both_fail,
        },
        "category_counts": {
            "match": match,
            "drift": drift,
            "puml_only": puml_only,
            "jar_only": jar_only,
            "both_fail": both_fail,
        },
        "promoted_gate": promoted_gate,
        "top_drift_families": top_drift_families(fixtures),
    }


def markdown_for(summary: Dict[str, Any]) -> str:
    if summary["skipped"]:
        reason = summary.get("skip_reason") or "oracle comparison did not run"
        return "\n".join(
            [
                "# Oracle Conformance Report",
                "",
                "Status: skipped",
                "",
                f"Reason: {reason}",
                "",
                "This is a comparison-only report. A skipped report is not parity evidence.",
                "",
            ]
        )

    counts = summary["category_counts"]
    outcomes = summary["outcome_counts"]
    promoted_gate = summary.get("promoted_gate") or {}
    lines = [
        "# Oracle Conformance Report",
        "",
        f"Status: {summary['gate_status']}",
        f"PlantUML JAR: {summary.get('jar_version') or 'unknown'}",
        f"Source timestamp: {summary.get('source_timestamp') or 'unknown'}",
        "",
        "This report compares puml SVG output with a pinned Java PlantUML JAR.",
        "It is conformance evidence, not a pixel-perfect parity claim.",
        "",
        "## Summary",
        "",
        f"- Fixtures: {summary['fixture_count']}",
        f"- Match percentage: {summary['match_pct']}%",
        f"- Pass fixtures: {outcomes['pass']}",
        f"- Advisory drift fixtures: {outcomes['advisory']}",
        f"- Render-failure attention fixtures: {outcomes['fail']}",
        f"- Promoted fixture gate: {promoted_gate.get('status', 'not-run')}",
        "",
        "| Category | Count |",
        "|---|---:|",
        f"| match | {counts['match']} |",
        f"| drift | {counts['drift']} |",
        f"| puml-only | {counts['puml_only']} |",
        f"| jar-only | {counts['jar_only']} |",
        f"| both-fail | {counts['both_fail']} |",
        "",
        "## Top Drift Families",
        "",
    ]

    if promoted_gate.get("violations"):
        lines.extend(
            [
                "## Promoted Fixture Violations",
                "",
                "| Fixture | Actual category | Allowed categories |",
                "|---|---|---|",
            ]
        )
        for violation in promoted_gate["violations"]:
            allowed = ", ".join(violation.get("allowed_categories", []))
            lines.append(
                f"| {violation.get('path', '')} | {violation.get('actual_category', '')} | {allowed} |"
            )
        lines.append("")

    if summary["top_drift_families"]:
        lines.extend(["| Family | Count | Categories | Representative fixtures |", "|---|---:|---|---|"])
        for row in summary["top_drift_families"]:
            categories = ", ".join(
                f"{name}: {count}" for name, count in row["categories"].items()
            )
            fixtures = "<br>".join(row["representative_fixtures"])
            lines.append(
                f"| {row['family']} | {row['count']} | {categories} | {fixtures} |"
            )
    else:
        lines.append("No drift families were reported.")

    lines.extend(
        [
            "",
            "## Threshold Semantics",
            "",
            "- 80% or higher match: pass.",
            "- 50% to 79% match: advisory; CI records the report and keeps the run green.",
            "- Below 50% match: blocking failure.",
            "",
        ]
    )
    return "\n".join(lines)


def html_for(markdown: str, summary: Dict[str, Any]) -> str:
    title = "Oracle Conformance Report"
    escaped_lines = []
    for line in markdown.splitlines():
        escaped = html.escape(line)
        if line.startswith("# "):
            escaped_lines.append(f"<h1>{html.escape(line[2:])}</h1>")
        elif line.startswith("## "):
            escaped_lines.append(f"<h2>{html.escape(line[3:])}</h2>")
        elif line.startswith("- "):
            escaped_lines.append(f"<p>{escaped}</p>")
        elif line.startswith("|"):
            escaped_lines.append(f"<pre>{escaped}</pre>")
        elif line:
            escaped_lines.append(f"<p>{escaped}</p>")
        else:
            escaped_lines.append("")

    summary_json = html.escape(json.dumps(summary, indent=2))
    return "\n".join(
        [
            "<!doctype html>",
            '<html lang="en">',
            "<head>",
            '<meta charset="utf-8">',
            '<meta name="viewport" content="width=device-width, initial-scale=1">',
            f"<title>{title}</title>",
            "<style>",
            "body{font-family:system-ui,sans-serif;max-width:960px;margin:2rem auto;padding:0 1rem;line-height:1.5}",
            "pre{white-space:pre-wrap;background:#f6f8fa;padding:.75rem;border:1px solid #d0d7de;overflow:auto}",
            "h1,h2{line-height:1.2}",
            "</style>",
            "</head>",
            "<body>",
            *escaped_lines,
            "<h2>Machine Summary</h2>",
            f"<pre>{summary_json}</pre>",
            "</body>",
            "</html>",
            "",
        ]
    )


def write_outputs(args: argparse.Namespace, report: Dict[str, Any]) -> None:
    summary = summarize(report)
    markdown = markdown_for(summary)

    md_path = Path(args.markdown)
    json_path = Path(args.json)
    md_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.parent.mkdir(parents=True, exist_ok=True)
    md_path.write_text(markdown, encoding="utf-8")
    json_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")

    if args.pages_dir:
        pages_dir = Path(args.pages_dir)
        pages_dir.mkdir(parents=True, exist_ok=True)
        (pages_dir / "index.html").write_text(html_for(markdown, summary), encoding="utf-8")
        (pages_dir / "oracle_report.md").write_text(markdown, encoding="utf-8")
        (pages_dir / "oracle_report_summary.json").write_text(
            json.dumps(summary, indent=2) + "\n",
            encoding="utf-8",
        )
        (pages_dir / "oracle_report.json").write_text(
            json.dumps(report, indent=2) + "\n",
            encoding="utf-8",
        )


def main() -> int:
    args = parse_args()
    write_outputs(args, read_report(Path(args.input)))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
