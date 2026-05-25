#!/usr/bin/env python3
"""Validate and summarize the executable PlantUML feature manifest."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any


ISSUE_RE = re.compile(r"^#\d+$")
ALLOWED_STATUS = {"supported", "partial", "unsupported", "known_visual_risk"}
ALLOWED_ORACLE = {"promoted-blocking", "promoted-report", "eligible", "not-applicable"}
ALLOWED_UPSTREAM = {"expected-match", "expected-drift", "not-upstream-compatible"}
REQUIRED_ENTRY_FIELDS = {
    "id",
    "family",
    "construct",
    "expected_status",
    "fixtures",
    "issues",
    "oracle_status",
    "visual_gate",
}


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def load_manifest(path: Path) -> dict[str, Any]:
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError(f"{path}: invalid JSON: {exc}") from exc
    if not isinstance(data, dict):
        raise ValueError(f"{path}: top-level value must be an object")
    return data


def validate_manifest(data: dict[str, Any], root: Path) -> dict[str, Any]:
    errors: list[str] = []
    entries = data.get("entries")
    if data.get("schema_version") != "1.0":
        errors.append("schema_version must be 1.0")
    if not isinstance(entries, list):
        errors.append("entries must be an array")
        entries = []
    if len(entries) < 50:
        errors.append(f"manifest must contain at least 50 entries, found {len(entries)}")

    seen_ids: set[str] = set()
    family_counts: Counter[str] = Counter()
    status_counts: Counter[str] = Counter()
    oracle_counts: Counter[str] = Counter()
    issue_counts: Counter[str] = Counter()
    fixtures_by_family: dict[str, set[str]] = defaultdict(set)

    for index, entry in enumerate(entries, start=1):
        if not isinstance(entry, dict):
            errors.append(f"entry {index}: must be an object")
            continue
        missing = sorted(REQUIRED_ENTRY_FIELDS - set(entry))
        if missing:
            errors.append(f"entry {index}: missing required fields {missing}")
            continue

        entry_id = entry["id"]
        if not isinstance(entry_id, str) or not entry_id:
            errors.append(f"entry {index}: id must be a non-empty string")
        elif entry_id in seen_ids:
            errors.append(f"entry {entry_id}: duplicate id")
        else:
            seen_ids.add(entry_id)

        family = entry["family"]
        if not isinstance(family, str) or not family:
            errors.append(f"entry {entry_id}: family must be a non-empty string")
            family = "<invalid>"
        else:
            family_counts[family] += 1

        construct = entry["construct"]
        if not isinstance(construct, str) or not construct.strip():
            errors.append(f"entry {entry_id}: construct must be a non-empty string")

        status = entry["expected_status"]
        if status not in ALLOWED_STATUS:
            errors.append(f"entry {entry_id}: invalid expected_status {status!r}")
        else:
            status_counts[status] += 1

        oracle = entry["oracle_status"]
        if oracle not in ALLOWED_ORACLE:
            errors.append(f"entry {entry_id}: invalid oracle_status {oracle!r}")
        else:
            oracle_counts[oracle] += 1

        fixtures = entry["fixtures"]
        if not isinstance(fixtures, list) or not fixtures:
            errors.append(f"entry {entry_id}: fixtures must be a non-empty array")
        else:
            for fixture in fixtures:
                if not isinstance(fixture, str) or not fixture:
                    errors.append(f"entry {entry_id}: fixture must be a non-empty string")
                    continue
                path = Path(fixture)
                if path.is_absolute() or ".." in path.parts:
                    errors.append(f"entry {entry_id}: fixture must be repo-relative: {fixture}")
                    continue
                if not (root / path).is_file():
                    errors.append(f"entry {entry_id}: fixture does not exist: {fixture}")
                fixtures_by_family[family].add(fixture)

        issues = entry["issues"]
        if not isinstance(issues, list) or not issues:
            errors.append(f"entry {entry_id}: issues must be a non-empty array")
        else:
            for issue in issues:
                if not isinstance(issue, str) or ISSUE_RE.match(issue) is None:
                    errors.append(f"entry {entry_id}: malformed issue ref {issue!r}")
                else:
                    issue_counts[issue] += 1

        if status == "unsupported" and not entry.get("diagnostics_policy"):
            errors.append(f"entry {entry_id}: unsupported entries need diagnostics_policy")

        if status == "known_visual_risk" and "#594" not in issues:
            errors.append(f"entry {entry_id}: known visual risk entries must reference #594")

        if oracle.startswith("promoted-"):
            render_command = entry.get("render_command")
            upstream = entry.get("upstream_compatibility")
            if upstream not in ALLOWED_UPSTREAM:
                errors.append(
                    f"entry {entry_id}: oracle-promoted entries need valid upstream_compatibility"
                )
            if not isinstance(render_command, str) or not render_command.strip():
                errors.append(f"entry {entry_id}: oracle-promoted entries need render_command")
            else:
                first_fixture = fixtures[0] if isinstance(fixtures, list) and fixtures else ""
                if isinstance(first_fixture, str) and first_fixture not in render_command:
                    errors.append(
                        f"entry {entry_id}: render_command must name the primary fixture"
                    )

    required = data.get("required_families", [])
    if not isinstance(required, list) or not required:
        errors.append("required_families must be a non-empty array")
        required = []
    for family in required:
        if family_counts.get(family, 0) == 0:
            errors.append(f"required family has no manifest entries: {family}")

    summary = {
        "entries": len(entries),
        "families": dict(sorted(family_counts.items())),
        "statuses": dict(sorted(status_counts.items())),
        "oracle_statuses": dict(sorted(oracle_counts.items())),
        "issues": dict(sorted(issue_counts.items())),
        "fixture_counts_by_family": {
            family: len(fixtures) for family, fixtures in sorted(fixtures_by_family.items())
        },
        "errors": errors,
    }
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        default="tests/plantuml_feature_manifest.json",
        help="repo-relative or absolute manifest path",
    )
    parser.add_argument("--json", action="store_true", help="emit machine-readable summary")
    args = parser.parse_args()

    root = repo_root()
    manifest = Path(args.manifest)
    if not manifest.is_absolute():
        manifest = root / manifest

    try:
        summary = validate_manifest(load_manifest(manifest), root)
    except ValueError as exc:
        print(str(exc), file=sys.stderr)
        return 2

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(f"entries: {summary['entries']}")
        print("families:")
        for family, count in summary["families"].items():
            print(f"  {family}: {count}")
        print("statuses:")
        for status, count in summary["statuses"].items():
            print(f"  {status}: {count}")
        if summary["errors"]:
            print("errors:")
            for error in summary["errors"]:
                print(f"  - {error}")

    return 1 if summary["errors"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
