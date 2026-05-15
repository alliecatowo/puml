#!/usr/bin/env python3
"""Benchmark trend + gate evaluation helper.

This script keeps benchmark gate logic testable outside shell glue.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys


def load_json(path: pathlib.Path) -> dict:
    return json.loads(path.read_text())


def maybe_load_json(path: pathlib.Path) -> dict | None:
    if not path.exists() or path.stat().st_size == 0:
        return None
    return load_json(path)


def build_rows(current: dict, previous: dict | None) -> list[dict]:
    prev_means: dict[str, float] = {}
    if previous:
        for item in previous.get("scenarios", []):
            prev_means[item["name"]] = float(item["mean_ms"])

    rows: list[dict] = []
    for item in current.get("scenarios", []):
        name = item["name"]
        curr = float(item["mean_ms"])
        prev_val = prev_means.get(name)
        delta_ms = None if prev_val is None else round(curr - prev_val, 3)
        delta_pct = None
        if prev_val not in (None, 0.0):
            delta_pct = round(((curr - prev_val) / prev_val) * 100.0, 3)
        rows.append(
            {
                "name": name,
                "current_mean_ms": round(curr, 3),
                "previous_mean_ms": None if prev_val is None else round(prev_val, 3),
                "delta_ms": delta_ms,
                "delta_pct": delta_pct,
            }
        )

    rows.sort(key=lambda r: r["name"])
    return rows


def command_trend(args: argparse.Namespace) -> int:
    current = load_json(args.current)
    previous = maybe_load_json(args.previous)

    baseline_mode_match = previous is not None and previous.get("mode") == args.mode
    if previous and not baseline_mode_match:
        previous = None

    rows = build_rows(current, previous)

    trend = {
        "timestamp_utc": args.timestamp_utc,
        "mode": args.mode,
        "source": str(args.current),
        "baseline_source": str(args.previous),
        "binary": {
            "path": current.get("binary"),
            "size_bytes": args.binary_bytes,
            "limit_bytes": args.binary_limit_bytes,
            "within_limit": args.binary_bytes <= args.binary_limit_bytes,
        },
        "gates": {
            "absolute_mean_ms_limit": args.abs_limit,
            "regression_pct_limit": args.regression_limit_pct,
            "regression_min_delta_ms": args.regression_min_delta_ms,
        },
        "scenarios": rows,
        "baseline": {
            "timestamp_utc": None if previous is None else previous.get("timestamp_utc"),
            "available": previous is not None,
            "mode_match": baseline_mode_match,
        },
        "environment": {
            "host": args.host,
            "os": args.os_name,
            "kernel": args.kernel,
            "arch": args.arch,
            "rustc": args.rustc,
            "timing_tool": args.timing_tool,
        },
        "plantuml_oracle": {
            "status": "todo",
            "notes": "No-Java baseline keeps oracle placeholders only.",
        },
    }

    args.output_json.write_text(json.dumps(trend, indent=2, sort_keys=True) + "\n")

    lines = [
        "# Benchmark Trend",
        "",
        f"- Timestamp (UTC): `{args.timestamp_utc}`",
        f"- Mode: `{args.mode}`",
        f"- Baseline source: `{args.previous}`",
        f"- Baseline mode match: `{str(baseline_mode_match).lower()}`",
        f"- Baseline timestamp (UTC): `{trend['baseline']['timestamp_utc'] or 'none'}`",
        f"- Binary: `{args.binary_bytes}` bytes (limit `{args.binary_limit_bytes}`)",
        (
            f"- Regression gate: delta > `{args.regression_limit_pct:.3f}%` "
            f"and `>{args.regression_min_delta_ms:.3f}ms`"
        ),
        "",
        "| Scenario | Current Mean (ms) | Previous Mean (ms) | Delta (ms) | Delta (%) |",
        "|---|---:|---:|---:|---:|",
    ]

    for row in rows:
        prev_mean = "n/a" if row["previous_mean_ms"] is None else f"{row['previous_mean_ms']:.3f}"
        delta_ms = "n/a" if row["delta_ms"] is None else f"{row['delta_ms']:.3f}"
        delta_pct = "n/a" if row["delta_pct"] is None else f"{row['delta_pct']:.3f}"
        lines.append(
            f"| `{row['name']}` | {row['current_mean_ms']:.3f} | {prev_mean} | {delta_ms} | {delta_pct} |"
        )

    lines.extend(
        [
            "",
            "## PlantUML Oracle",
            "- Status: `todo`",
            "- Notes: no-Java baseline keeps oracle placeholders only.",
        ]
    )
    args.output_md.write_text("\n".join(lines) + "\n")
    return 0


def eval_failures(
    current: dict,
    previous: dict | None,
    mode: str,
    abs_limit: float,
    regression_limit_pct: float,
    regression_min_delta_ms: float,
    binary_bytes: int,
    binary_limit_bytes: int,
) -> list[str]:
    failures: list[str] = []

    if binary_bytes > binary_limit_bytes:
        failures.append(f"binary size {binary_bytes}B exceeds {binary_limit_bytes}B")

    prev_map: dict[str, float] = {}
    can_compare_regression = False
    if previous and previous.get("mode") == mode:
        can_compare_regression = True
        for item in previous.get("scenarios", []):
            prev_map[item["name"]] = float(item["mean_ms"])

    for item in current.get("scenarios", []):
        name = item["name"]
        curr = float(item["mean_ms"])
        if curr > abs_limit:
            failures.append(
                f"{name}: mean {curr:.3f}ms exceeds absolute limit {abs_limit:.3f}ms"
            )

        if can_compare_regression:
            prev = prev_map.get(name)
            if prev is not None and prev > 0:
                delta_ms = curr - prev
                delta_pct = ((curr - prev) / prev) * 100.0
                if delta_pct > regression_limit_pct and delta_ms > regression_min_delta_ms:
                    failures.append(
                        (
                            f"{name}: regression {delta_pct:.3f}% exceeds limit "
                            f"{regression_limit_pct:.3f}% and delta {delta_ms:.3f}ms "
                            f"exceeds floor {regression_min_delta_ms:.3f}ms "
                            f"(current {curr:.3f}ms vs previous {prev:.3f}ms)"
                        )
                    )

    return failures


def command_failures(args: argparse.Namespace) -> int:
    current = load_json(args.current)
    previous = maybe_load_json(args.previous)

    failures = eval_failures(
        current=current,
        previous=previous,
        mode=args.mode,
        abs_limit=args.abs_limit,
        regression_limit_pct=args.regression_limit_pct,
        regression_min_delta_ms=args.regression_min_delta_ms,
        binary_bytes=args.binary_bytes,
        binary_limit_bytes=args.binary_limit_bytes,
    )

    for line in failures:
        print(line)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Benchmark gate helper")
    sub = parser.add_subparsers(dest="cmd", required=True)

    trend = sub.add_parser("trend", help="write trend json+markdown")
    trend.add_argument("--current", type=pathlib.Path, required=True)
    trend.add_argument("--previous", type=pathlib.Path, required=True)
    trend.add_argument("--output-json", type=pathlib.Path, required=True)
    trend.add_argument("--output-md", type=pathlib.Path, required=True)
    trend.add_argument("--timestamp-utc", required=True)
    trend.add_argument("--mode", required=True)
    trend.add_argument("--abs-limit", type=float, required=True)
    trend.add_argument("--regression-limit-pct", type=float, required=True)
    trend.add_argument("--regression-min-delta-ms", type=float, required=True)
    trend.add_argument("--binary-bytes", type=int, required=True)
    trend.add_argument("--binary-limit-bytes", type=int, required=True)
    trend.add_argument("--host", required=True)
    trend.add_argument("--os-name", required=True)
    trend.add_argument("--kernel", required=True)
    trend.add_argument("--arch", required=True)
    trend.add_argument("--rustc", required=True)
    trend.add_argument("--timing-tool", required=True)
    trend.set_defaults(func=command_trend)

    failures = sub.add_parser("failures", help="print gate failures")
    failures.add_argument("--current", type=pathlib.Path, required=True)
    failures.add_argument("--previous", type=pathlib.Path, required=True)
    failures.add_argument("--mode", required=True)
    failures.add_argument("--abs-limit", type=float, required=True)
    failures.add_argument("--regression-limit-pct", type=float, required=True)
    failures.add_argument("--regression-min-delta-ms", type=float, required=True)
    failures.add_argument("--binary-bytes", type=int, required=True)
    failures.add_argument("--binary-limit-bytes", type=int, required=True)
    failures.set_defaults(func=command_failures)

    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())
