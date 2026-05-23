#!/usr/bin/env python3
"""Classify changed files for CI gate routing."""

from __future__ import annotations

import argparse
from pathlib import Path


DOCS_SITE_FILES = {
    "README.md",
    "CONTRIBUTING.md",
    "LICENSE",
}

SITE_SMOKE_FILES = {
    "scripts/build-site.mjs",
    "scripts/mirror-specs.mjs",
    "scripts/site-smoke.sh",
    "site/scripts/smoke-inline-fence-preview.mjs",
}

WASM_SITE_FILES = {
    ".github/actions/install-wasm-pack/action.yml",
    "scripts/wasm-smoke.mjs",
    "site/static/js/inline-fence-preview.js",
    "site/static/js/wasm-renderer.js",
}

FULL_GATE_EXACT = {
    "Cargo.toml",
    "Cargo.lock",
    "scripts/check-all.sh",
    "scripts/bench.sh",
    "scripts/bench_gate.py",
    "scripts/ci-classify-changes.py",
    ".github/workflows/pr-gate.yml",
}

FULL_GATE_PREFIXES = (
    "src/",
    "tests/",
    "stdlib/",
)

WASM_CHECK_EXACT = {
    "Cargo.toml",
    "Cargo.lock",
    "scripts/ci-classify-changes.py",
    ".github/workflows/pr-gate.yml",
}

WASM_CHECK_PREFIXES = (
    "src/",
    "crates/puml-wasm/",
)

DOCS_EXAMPLES_DRIFT_EXACT = {
    "Cargo.toml",
    "Cargo.lock",
    "scripts/ci-classify-changes.py",
    "scripts/render_check.py",
    ".github/workflows/pr-gate.yml",
}

DOCS_EXAMPLES_DRIFT_PREFIXES = (
    "docs/examples/",
    "src/",
    "stdlib/",
)

ARTIFACT_REGEN_EXACT = {
    "Cargo.toml",
    "Cargo.lock",
    "scripts/ci-classify-changes.py",
    "scripts/regen-artifacts.sh",
    ".github/workflows/pr-gate.yml",
}

ARTIFACT_REGEN_PREFIXES = (
    "src/",
    "stdlib/",
    "docs/examples/",
    "docs/diagrams/",
)

# Paths that trigger the full oracle conformance run (oracle.yml).
ORACLE_EXACT = {
    "Cargo.toml",
    "Cargo.lock",
    "tests/oracle_promoted_fixtures.json",
    "scripts/oracle.sh",
    "scripts/oracle_report_summary.py",
    "scripts/oracle_promoted_gate.py",
    "scripts/ci-classify-changes.py",
    ".github/workflows/oracle.yml",
}

ORACLE_PREFIXES = (
    "src/",
    "crates/",
    "tests/fixtures/",
    "tests/oracle_smoke.rs",
    "docs/examples/",
    "stdlib/",
)

# Paths that additionally trigger the oracle_smoke sentinel test run.
ORACLE_SMOKE_EXACT = {
    "tests/oracle_smoke.rs",
    "scripts/oracle.sh",
    "scripts/oracle_report_summary.py",
    "scripts/ci-classify-changes.py",
    ".github/workflows/oracle.yml",
    ".github/workflows/differential-oracle-smoke.yml",
    "scripts/differential_oracle_smoke.py",
}

# Paths that trigger the differential-oracle-smoke workflow specifically.
DIFF_ORACLE_SMOKE_EXACT = {
    "scripts/differential_oracle_smoke.py",
    "scripts/ci-classify-changes.py",
    ".github/workflows/differential-oracle-smoke.yml",
}


def is_markdown(path: str) -> bool:
    return path.endswith(".md")


def is_site_path(path: str) -> bool:
    return path == "site" or path.startswith("site/")


def classify(paths: list[str]) -> dict[str, bool]:
    run_full_gate = False
    docs_examples_changed = False
    run_docs_examples_drift = False
    run_artifact_regen = False
    run_wasm_check = False
    run_site_smoke = False
    run_wasm_site_smoke = False
    run_oracle = False
    run_oracle_smoke = False
    run_diff_oracle_smoke = False

    for path in paths:
        if not path:
            continue

        if path in WASM_CHECK_EXACT or path.startswith(WASM_CHECK_PREFIXES):
            run_wasm_check = True

        if path in DOCS_EXAMPLES_DRIFT_EXACT or path.startswith(DOCS_EXAMPLES_DRIFT_PREFIXES):
            run_docs_examples_drift = True

        if path in ARTIFACT_REGEN_EXACT or path.startswith(ARTIFACT_REGEN_PREFIXES):
            run_artifact_regen = True

        if path in ORACLE_EXACT or path.startswith(ORACLE_PREFIXES):
            run_oracle = True

        if path in ORACLE_SMOKE_EXACT:
            run_oracle_smoke = True

        if path in DIFF_ORACLE_SMOKE_EXACT:
            run_diff_oracle_smoke = True

        if path.startswith("docs/examples/"):
            run_full_gate = True
            docs_examples_changed = True
            run_site_smoke = True
            continue

        if path.startswith("crates/puml-wasm/"):
            run_full_gate = True
            run_site_smoke = True
            run_wasm_site_smoke = True
            continue

        if path in WASM_SITE_FILES:
            run_site_smoke = True
            run_wasm_site_smoke = True
            continue

        if is_site_path(path) or path in SITE_SMOKE_FILES or path.startswith("docs/specs/"):
            run_site_smoke = True
            continue

        if path in DOCS_SITE_FILES or path.startswith("docs/") or is_markdown(path):
            run_site_smoke = True
            continue

        if path in FULL_GATE_EXACT or path.startswith(FULL_GATE_PREFIXES):
            run_full_gate = True
            continue

        run_full_gate = True

    if not paths:
        run_full_gate = True
        run_docs_examples_drift = True
        run_artifact_regen = True
        run_wasm_check = True
        run_site_smoke = True
        run_oracle = True
        run_oracle_smoke = True
        run_diff_oracle_smoke = True

    return {
        "run_full_gate": run_full_gate,
        "docs_examples_changed": docs_examples_changed,
        "run_docs_examples_drift": run_docs_examples_drift,
        "run_artifact_regen": run_artifact_regen,
        "run_wasm_check": run_wasm_check,
        "run_site_smoke": run_site_smoke,
        "run_wasm_site_smoke": run_wasm_site_smoke,
        "run_oracle": run_oracle,
        "run_oracle_smoke": run_oracle_smoke,
        "run_diff_oracle_smoke": run_diff_oracle_smoke,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--changed-files",
        type=Path,
        help="newline-delimited changed file list; stdin is used when omitted",
    )
    parser.add_argument(
        "--github-output",
        type=Path,
        help="append key=value outputs for GitHub Actions",
    )
    return parser.parse_args()


def read_paths(path: Path | None) -> list[str]:
    text = path.read_text(encoding="utf-8") if path else input_stream()
    return [line.strip() for line in text.splitlines() if line.strip()]


def input_stream() -> str:
    import sys

    return sys.stdin.read()


def bool_text(value: bool) -> str:
    return "true" if value else "false"


def main() -> int:
    args = parse_args()
    outputs = classify(read_paths(args.changed_files))
    lines = [f"{key}={bool_text(value)}" for key, value in outputs.items()]

    for line in lines:
        print(line)

    if args.github_output:
        with args.github_output.open("a", encoding="utf-8") as handle:
            for line in lines:
                handle.write(f"{line}\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
