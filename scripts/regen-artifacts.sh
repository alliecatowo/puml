#!/usr/bin/env bash
# regen-artifacts.sh — Regenerate committed diagram artifacts after renderer changes.
#
# Usage:
#   scripts/regen-artifacts.sh [--force]
#
# Without --force the script skips regen when no renderer-affecting source files
# have changed (checked against the index / working tree).  With --force it always
# regens.  In CI the freshness check always uses --force so the gate is authoritative.
#
# Artifacts produced:
#   docs/diagrams/*.puml  → docs/diagrams/*.svg + docs/diagrams/*.png  (both committed)
#   docs/examples/**/*.puml → docs/examples/**/*.svg  (PNGs there are gitignored)
#
# Exit codes:
#   0  Regen complete (or skipped because nothing changed)
#   1  Binary not found or a render step failed

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Allow callers (e.g. CI using the release-ci profile) to override the binary
# path via the PUML_BIN environment variable.  Falls back to the standard
# release binary when not set.
PUML_BIN="${PUML_BIN:-${REPO_ROOT}/target/release/puml}"
FORCE=0

for arg in "$@"; do
  case "${arg}" in
    --force) FORCE=1 ;;
    *)
      echo "Usage: $0 [--force]" >&2
      exit 1
      ;;
  esac
done

# ---------------------------------------------------------------------------
# Skip guard — only regenerate when renderer-affecting files changed
# ---------------------------------------------------------------------------

RENDERER_PATHS=(
  "src/render/"
  "src/parser/"
  "src/normalize/"
  "src/theme.rs"
  "src/layout.rs"
  "src/lib.rs"
  "docs/diagrams/"
  "docs/examples/"
)

if [[ "${FORCE}" -eq 0 ]]; then
  # Check both staged and unstaged changes relative to HEAD
  changed_relevant=0
  for path in "${RENDERER_PATHS[@]}"; do
    if git -C "${REPO_ROOT}" diff --name-only HEAD -- "${path}" | grep -q .; then
      changed_relevant=1
      break
    fi
    if git -C "${REPO_ROOT}" diff --name-only --cached -- "${path}" | grep -q .; then
      changed_relevant=1
      break
    fi
  done

  if [[ "${changed_relevant}" -eq 0 ]]; then
    echo "[regen-artifacts] No renderer-affecting changes detected; skipping regen."
    echo "[regen-artifacts] Run with --force to regenerate unconditionally."
    exit 0
  fi
fi

# ---------------------------------------------------------------------------
# Ensure binary is present
# ---------------------------------------------------------------------------

if [[ ! -x "${PUML_BIN}" ]]; then
  echo "[regen-artifacts] Binary not found at ${PUML_BIN}; building with release-ci profile…"
  cargo build --profile release-ci --manifest-path "${REPO_ROOT}/Cargo.toml"
  # If PUML_BIN still points at the release path (default), redirect to where
  # the release-ci build actually lands so the script can proceed.
  if [[ "${PUML_BIN}" == "${REPO_ROOT}/target/release/puml" ]]; then
    PUML_BIN="${REPO_ROOT}/target/release-ci/puml"
  fi
fi

if [[ ! -x "${PUML_BIN}" ]]; then
  echo "[regen-artifacts] ERROR: Could not locate or build ${PUML_BIN}" >&2
  exit 1
fi

echo "[regen-artifacts] Using binary: ${PUML_BIN}"

# ---------------------------------------------------------------------------
# Regenerate docs/diagrams — SVG + PNG (both are committed there)
# ---------------------------------------------------------------------------

echo "[regen-artifacts] Regenerating docs/diagrams/ …"
find "${REPO_ROOT}/docs/diagrams" -name "*.puml" | sort | while read -r puml_file; do
  base="${puml_file%.puml}"
  echo "  → $(basename "${puml_file}")"
  "${PUML_BIN}" "${puml_file}" -o "${base}.svg"
  "${PUML_BIN}" --format png "${puml_file}" -o "${base}.png"
done

# ---------------------------------------------------------------------------
# Regenerate docs/examples — SVG only (PNGs are gitignored)
# ---------------------------------------------------------------------------

echo "[regen-artifacts] Regenerating docs/examples/ SVGs …"
find "${REPO_ROOT}/docs/examples" -name "*.puml" | sort | while read -r puml_file; do
  base="${puml_file%.puml}"
  echo "  → ${puml_file#"${REPO_ROOT}/"}"
  "${PUML_BIN}" "${puml_file}" -o "${base}.svg"
done

# ---------------------------------------------------------------------------
# Regenerate README markdown snippet diagrams if present
# ---------------------------------------------------------------------------

readme="${REPO_ROOT}/docs/examples/sequence/README.md"
if [[ -f "${readme}" ]]; then
  echo "[regen-artifacts] Regenerating markdown snippets from $(basename "${readme}") …"
  "${PUML_BIN}" --from-markdown --multi "${readme}"
fi

echo "[regen-artifacts] Done."
