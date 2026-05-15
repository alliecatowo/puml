#!/usr/bin/env python3
"""Render a fixture corpus and validate SVG bounds sanity heuristics."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Tuple

ROOT = Path(__file__).resolve().parents[1]

FIXTURE_CORPUS = [
    "basic/hello.puml",
    "groups/valid_ref_and_else_rendering.puml",
    "groups/valid_overflow_long_blocks.puml",
    "notes/valid_multiline_blocks.puml",
    "notes/valid_note_across_multi.puml",
    "structure/valid_separator_delay_divider_spacer.puml",
    "e2e/participant_kinds.puml",
    "overflow/overflow_notes_refs_groups.puml",
    "participants/valid_aliases.puml",
    "autonumber/valid_with_format.puml",
    "lifecycle/valid_shortcuts_expansion.puml",
]

SVG_RE = re.compile(r"<svg\b[^>]*>", re.IGNORECASE)
RECT_RE = re.compile(r"<rect\b[^>]*>", re.IGNORECASE)
TEXT_RE = re.compile(r"<text\b[^>]*>", re.IGNORECASE)
ATTR_RE = re.compile(r"([A-Za-z_:][A-Za-z0-9_.:-]*)\s*=\s*\"([^\"]*)\"")
NUMBER_RE = re.compile(r"^-?(?:\d+|\d*\.\d+)$")


@dataclass
class Bounds:
    min_x: float
    min_y: float
    max_x: float
    max_y: float


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--margin", type=float, default=2.0, help="accepted out-of-bounds margin")
    parser.add_argument("--quiet", action="store_true", help="suppress pass output")
    return parser.parse_args()


def parse_attrs(tag: str) -> Dict[str, str]:
    return {k: v for (k, v) in ATTR_RE.findall(tag)}


def parse_number(value: str) -> Optional[float]:
    if NUMBER_RE.match(value.strip()):
        return float(value)
    return None


def parse_viewbox(svg_open_tag: str) -> Bounds:
    attrs = parse_attrs(svg_open_tag)
    raw = attrs.get("viewBox")
    if raw is None:
        raise ValueError("missing viewBox")
    parts = raw.replace(",", " ").split()
    if len(parts) != 4:
        raise ValueError(f"invalid viewBox: {raw}")
    x, y, w, h = (float(p) for p in parts)
    if w < 0 or h < 0:
        raise ValueError(f"negative viewBox dimensions: {raw}")
    return Bounds(min_x=x, min_y=y, max_x=x + w, max_y=y + h)


def check_range(name: str, val: float, low: float, high: float, margin: float) -> Optional[str]:
    if val < low - margin or val > high + margin:
        return f"{name}={val} outside [{low - margin}, {high + margin}]"
    return None


def validate_svg(svg: str, fixture_name: str, margin: float) -> List[str]:
    errors: List[str] = []

    root_match = SVG_RE.search(svg)
    if root_match is None:
        return [f"{fixture_name}: missing <svg> root"]

    try:
        bounds = parse_viewbox(root_match.group(0))
    except ValueError as exc:
        return [f"{fixture_name}: {exc}"]

    for rect_tag in RECT_RE.findall(svg):
        attrs = parse_attrs(rect_tag)
        x = parse_number(attrs.get("x", "0"))
        y = parse_number(attrs.get("y", "0"))
        w = parse_number(attrs.get("width", ""))
        h = parse_number(attrs.get("height", ""))

        if w is not None and w < 0:
            errors.append(f"{fixture_name}: rect has negative width ({w})")
        if h is not None and h < 0:
            errors.append(f"{fixture_name}: rect has negative height ({h})")

        if x is not None and y is not None and w is not None and h is not None:
            for err in [
                check_range("rect.x", x, bounds.min_x, bounds.max_x, margin),
                check_range("rect.y", y, bounds.min_y, bounds.max_y, margin),
                check_range("rect.x+width", x + w, bounds.min_x, bounds.max_x, margin),
                check_range("rect.y+height", y + h, bounds.min_y, bounds.max_y, margin),
            ]:
                if err:
                    errors.append(f"{fixture_name}: {err}")

    for text_tag in TEXT_RE.findall(svg):
        attrs = parse_attrs(text_tag)
        if "text-anchor" not in attrs:
            continue
        x = parse_number(attrs.get("x", ""))
        y = parse_number(attrs.get("y", ""))
        if x is None or y is None:
            continue

        for err in [
            check_range("text.x", x, bounds.min_x, bounds.max_x, margin),
            check_range("text.y", y, bounds.min_y, bounds.max_y, margin),
        ]:
            if err:
                errors.append(f"{fixture_name}: {err}")

    return errors


def render_fixture(rel_path: str) -> Tuple[str, str]:
    abs_path = ROOT / "tests" / "fixtures" / rel_path
    src = abs_path.read_text(encoding="utf-8")
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--", "-"],
        cwd=ROOT,
        input=src,
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"render failed for {rel_path} (exit {proc.returncode})\n"
            f"stderr:\n{proc.stderr.strip()}\nstdout:\n{proc.stdout.strip()}"
        )
    return rel_path, proc.stdout


def main() -> int:
    args = parse_args()
    all_errors: List[str] = []

    for rel_path in FIXTURE_CORPUS:
        try:
            fixture_name, svg = render_fixture(rel_path)
        except RuntimeError as exc:
            all_errors.append(str(exc))
            continue

        all_errors.extend(validate_svg(svg, fixture_name, args.margin))

    if all_errors:
        print("SVG bounds audit failed:", file=sys.stderr)
        for err in all_errors:
            print(f"- {err}", file=sys.stderr)
        return 1

    if not args.quiet:
        print(
            f"SVG bounds audit passed for {len(FIXTURE_CORPUS)} fixtures "
            f"(margin={args.margin})."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
