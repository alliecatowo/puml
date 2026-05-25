#!/usr/bin/env python3
"""Check renderer-domain layering rules.

This is a CI-friendly guard for the renderer refactor contract:

    frontend -> preprocess -> parse/lower -> normalize -> build_scene -> validate_scene -> backend

The first enforced slice is deliberately narrow. Existing SVG compatibility APIs
remain in `src/api/render.rs`, while CLI/LSP/WASM and future internal callers
must consume artifact pages instead of bypassing the typed render contract.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from dataclasses import dataclass


RENDER_CORE_FORBIDDEN = re.compile(
    r"\bcrate::(?:api|ast|frontend|language_service|model|normalize|output|parser|preproc|registry|render|specialized)\b"
)
SVG_PAGE_API = re.compile(r"\brender_svg_pages_from_model\b")
DIRECT_RENDER_SVG = re.compile(r"\brender::render_[A-Za-z0-9_]+_svg\b")
RENDER_ARTIFACT_LITERAL = re.compile(r"(?:=|return)\s*RenderArtifact\s*\{")
RENDER_ARTIFACT_STATE_WRITE = re.compile(r"\.\s*(?:scene_availability|invariant_report)\s*=")

DEFAULT_ROOT = pathlib.Path(__file__).resolve().parents[1]

# Compatibility adapter allowlist. Keep comments here explicit so a future
# violation must justify why it belongs outside the artifact contract.
SVG_PAGE_API_ALLOWLIST = {
    "src/api/render.rs": "public SVG compatibility shims are defined here",
    "src/api/mod.rs": "public re-export for source compatibility",
    "src/lib.rs": "public re-export for source compatibility",
}

DIRECT_RENDER_SVG_ALLOWLIST = {
    "src/api/render.rs": "single adapter that converts normalized models into render artifacts",
}

RENDER_ARTIFACT_LITERAL_ALLOWLIST = {
    "src/output/contract.rs": "RenderArtifact constructors live in the output contract",
}

RENDER_ARTIFACT_STATE_WRITE_ALLOWLIST = {
    "src/output/contract.rs": "RenderArtifact constructors and lifecycle methods live in the output contract",
    "src/render/mod.rs": "RenderArtifact lifecycle methods own scene/validation state transitions",
}

SKIP_DIRS = {
    ".git",
    "target",
    "node_modules",
    "site/node_modules",
}


@dataclass(frozen=True, order=True)
class Violation:
    rule: str
    path: str
    line: int
    message: str


def iter_rust_files(root: pathlib.Path) -> list[pathlib.Path]:
    files: list[pathlib.Path] = []
    for path in root.rglob("*.rs"):
        rel = path.relative_to(root).as_posix()
        if any(rel == skip or rel.startswith(f"{skip}/") for skip in SKIP_DIRS):
            continue
        files.append(path)
    return sorted(files)


def line_matches(pattern: re.Pattern[str], text: str) -> list[int]:
    return [
        line_no
        for line_no, line in enumerate(text.splitlines(), start=1)
        if pattern.search(line)
    ]


def check_render_core_dependencies(root: pathlib.Path, path: pathlib.Path) -> list[Violation]:
    rel = path.relative_to(root).as_posix()
    if rel != "src/render_core.rs" and not rel.startswith("src/render_core/"):
        return []

    text = path.read_text(encoding="utf-8")
    return [
        Violation(
            "render-core-neutral",
            rel,
            line,
            "render_core must not import frontend/parser/model/api/render/output layers",
        )
        for line in line_matches(RENDER_CORE_FORBIDDEN, text)
    ]


def check_svg_page_api(root: pathlib.Path, path: pathlib.Path) -> list[Violation]:
    rel = path.relative_to(root).as_posix()
    text = path.read_text(encoding="utf-8")
    if not SVG_PAGE_API.search(text) or rel in SVG_PAGE_API_ALLOWLIST:
        return []
    return [
        Violation(
            "artifact-boundary",
            rel,
            line,
            "use render_artifact_pages_from_model and consume artifact.svg at the adapter edge",
        )
        for line in line_matches(SVG_PAGE_API, text)
    ]


def check_direct_render_svg(root: pathlib.Path, path: pathlib.Path) -> list[Violation]:
    rel = path.relative_to(root).as_posix()
    if not (
        rel.startswith("src/api/")
        or rel.startswith("src/bin/")
        or rel.startswith("src/cli_run/")
        or rel.startswith("crates/puml-wasm/src/")
    ):
        return []

    text = path.read_text(encoding="utf-8")
    if not DIRECT_RENDER_SVG.search(text) or rel in DIRECT_RENDER_SVG_ALLOWLIST:
        return []
    return [
        Violation(
            "svg-adapter-boundary",
            rel,
            line,
            "call the artifact API instead of a family-specific render_*_svg function",
        )
        for line in line_matches(DIRECT_RENDER_SVG, text)
    ]


def check_render_artifact_literals(root: pathlib.Path, path: pathlib.Path) -> list[Violation]:
    rel = path.relative_to(root).as_posix()
    text = path.read_text(encoding="utf-8")
    if not RENDER_ARTIFACT_LITERAL.search(text) or rel in RENDER_ARTIFACT_LITERAL_ALLOWLIST:
        return []
    return [
        Violation(
            "artifact-constructor-boundary",
            rel,
            line,
            "construct RenderArtifact through its constructors so scene availability stays explicit",
        )
        for line in line_matches(RENDER_ARTIFACT_LITERAL, text)
    ]


def check_render_artifact_state_writes(root: pathlib.Path, path: pathlib.Path) -> list[Violation]:
    rel = path.relative_to(root).as_posix()
    text = path.read_text(encoding="utf-8")
    if (
        not RENDER_ARTIFACT_STATE_WRITE.search(text)
        or rel in RENDER_ARTIFACT_STATE_WRITE_ALLOWLIST
    ):
        return []
    return [
        Violation(
            "artifact-state-boundary",
            rel,
            line,
            "update RenderArtifact scene/validation state through its lifecycle methods",
        )
        for line in line_matches(RENDER_ARTIFACT_STATE_WRITE, text)
    ]


def collect_violations(root: pathlib.Path) -> list[Violation]:
    violations: list[Violation] = []
    for path in iter_rust_files(root):
        violations.extend(check_render_core_dependencies(root, path))
        violations.extend(check_svg_page_api(root, path))
        violations.extend(check_direct_render_svg(root, path))
        violations.extend(check_render_artifact_literals(root, path))
        violations.extend(check_render_artifact_state_writes(root, path))
    return sorted(violations)


def render_report(violations: list[Violation], fail_on_violations: bool, fmt: str) -> str:
    mode = "enforced" if fail_on_violations else "warning-only"
    if fmt == "json":
        return json.dumps(
            {
                "schema": "puml.renderer_boundary_guard",
                "mode": mode,
                "violationCount": len(violations),
                "violations": [violation.__dict__ for violation in violations],
            },
            indent=2,
            sort_keys=True,
        )

    if not violations:
        return f"[renderer-boundary] {mode} guard: ok"

    lines = [
        f"[renderer-boundary] {mode} guard: {len(violations)} violation(s)",
    ]
    for violation in violations:
        lines.append(
            f"{violation.path}:{violation.line}: {violation.rule}: {violation.message}"
        )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=pathlib.Path, default=DEFAULT_ROOT)
    parser.add_argument("--format", choices=("text", "json"), default="text")
    parser.add_argument("--fail-on-violations", action="store_true")
    args = parser.parse_args(argv)

    root = args.root.resolve()
    violations = collect_violations(root)
    print(render_report(violations, args.fail_on_violations, args.format))
    if violations and args.fail_on_violations:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
