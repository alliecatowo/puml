#!/usr/bin/env python3
"""Gate promoted oracle fixtures against explicit category expectations."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Dict, Iterable, List


VALID_CATEGORIES = {"match", "drift", "puml-only", "jar-only", "both-fail"}
CATEGORY_ALIASES = {"parse-fail": "both-fail"}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", help="oracle_report.json path")
    parser.add_argument("--manifest", required=True, help="promoted fixture manifest JSON")
    parser.add_argument(
        "--write",
        action="store_true",
        help="annotate the report with promoted_gate results",
    )
    parser.add_argument(
        "--validate-manifest-only",
        action="store_true",
        help="validate the promoted fixture manifest without requiring an oracle report",
    )
    parser.add_argument(
        "--repo-root",
        type=Path,
        help="repository root used to verify promoted fixture paths exist",
    )
    return parser.parse_args()


def read_json(path: Path) -> Dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return value


def normalize_category(category: str) -> str:
    return CATEGORY_ALIASES.get(category, category)


def normalize_allowed(raw: Any) -> List[str]:
    if raw is None:
        values: Iterable[Any] = ["match"]
    elif isinstance(raw, str):
        values = [raw]
    elif isinstance(raw, list):
        values = raw
    else:
        raise ValueError("allowed category value must be a string or list")

    allowed = []
    for value in values:
        category = normalize_category(str(value))
        if category not in VALID_CATEGORIES:
            raise ValueError(f"unknown oracle category in promoted manifest: {value}")
        allowed.append(category)
    return sorted(set(allowed))


def promoted_entries(manifest: Dict[str, Any]) -> List[Dict[str, Any]]:
    raw_entries = manifest.get("promoted_fixtures", manifest.get("fixtures", []))
    if not isinstance(raw_entries, list):
        raise ValueError("promoted manifest must contain a fixture list")

    entries = []
    for raw in raw_entries:
        if isinstance(raw, str):
            entry = {"path": raw, "allowed_categories": ["match"]}
        elif isinstance(raw, dict):
            entry = dict(raw)
        else:
            raise ValueError("promoted fixture entries must be strings or objects")

        path = str(entry.get("path", "")).strip()
        if not path:
            raise ValueError("promoted fixture entry is missing path")
        allowed = normalize_allowed(
            entry.get("allowed_categories", entry.get("expected_category"))
        )
        entries.append(
            {
                "path": path,
                "allowed_categories": allowed,
                "reason": entry.get("reason"),
            }
        )
    return entries


def validate_manifest(manifest: Dict[str, Any], repo_root: Path | None = None) -> List[Dict[str, Any]]:
    entries = promoted_entries(manifest)
    seen = set()
    errors = []

    for entry in entries:
        path = entry["path"]
        if path in seen:
            errors.append({"path": path, "error": "duplicate promoted fixture path"})
        seen.add(path)

        fixture_path = Path(path)
        if fixture_path.is_absolute() or ".." in fixture_path.parts:
            errors.append({"path": path, "error": "promoted fixture paths must be repo-relative"})
            continue

        if repo_root is not None and not (repo_root / fixture_path).is_file():
            errors.append({"path": path, "error": "promoted fixture path does not exist"})

    return errors


def evaluate(report: Dict[str, Any], manifest: Dict[str, Any]) -> Dict[str, Any]:
    if report.get("skipped"):
        return {
            "schema_version": "1.0",
            "status": "skipped",
            "manifest": manifest.get("name", "promoted oracle fixtures"),
            "total": 0,
            "passed": 0,
            "violations": [],
        }

    fixtures_by_path = {
        str(fixture.get("path", "")): str(fixture.get("category", ""))
        for fixture in report.get("fixtures", [])
    }

    promoted = promoted_entries(manifest)
    checks = []
    violations = []
    for entry in promoted:
        path = entry["path"]
        actual = fixtures_by_path.get(path, "missing")
        allowed = entry["allowed_categories"]
        passed = actual in allowed
        check = {
            "path": path,
            "actual_category": actual,
            "allowed_categories": allowed,
            "passed": passed,
        }
        if entry.get("reason"):
            check["reason"] = entry["reason"]
        checks.append(check)

        if not passed:
            violation = dict(check)
            if actual == "both-fail":
                violation["regression_kind"] = "parse-fail"
            elif actual == "missing":
                violation["regression_kind"] = "missing"
            elif actual not in VALID_CATEGORIES:
                violation["regression_kind"] = "unknown-category"
            else:
                violation["regression_kind"] = actual
            violations.append(violation)

    return {
        "schema_version": "1.0",
        "status": "fail" if violations else "pass",
        "manifest": manifest.get("name", "promoted oracle fixtures"),
        "total": len(checks),
        "passed": len([check for check in checks if check["passed"]]),
        "violations": violations,
        "checks": checks,
    }


def main() -> int:
    args = parse_args()
    manifest_path = Path(args.manifest)

    manifest = read_json(manifest_path)
    manifest_errors = validate_manifest(manifest, args.repo_root)
    if manifest_errors:
        print(
            json.dumps(
                {
                    "schema_version": "1.0",
                    "status": "fail",
                    "manifest": manifest.get("name", "promoted oracle fixtures"),
                    "errors": manifest_errors,
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 3

    if args.validate_manifest_only:
        print(
            json.dumps(
                {
                    "schema_version": "1.0",
                    "status": "pass",
                    "manifest": manifest.get("name", "promoted oracle fixtures"),
                    "total": len(promoted_entries(manifest)),
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0

    if not args.report:
        raise ValueError("--report is required unless --validate-manifest-only is set")

    report_path = Path(args.report)
    report = read_json(report_path)
    gate = evaluate(report, manifest)

    if args.write:
        report["promoted_gate"] = gate
        report_path.write_text(json.dumps(report, separators=(",", ":")) + "\n", encoding="utf-8")

    print(json.dumps(gate, indent=2, sort_keys=True))
    return 3 if gate["status"] == "fail" else 0


if __name__ == "__main__":
    raise SystemExit(main())
