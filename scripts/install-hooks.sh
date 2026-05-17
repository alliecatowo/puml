#!/usr/bin/env bash
# scripts/install-hooks.sh — install or uninstall lefthook git hooks for puml
#
# Usage:
#   ./scripts/install-hooks.sh            # install hooks
#   ./scripts/install-hooks.sh --uninstall  # remove hooks

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

UNINSTALL=false
for arg in "$@"; do
  case "$arg" in
    --uninstall) UNINSTALL=true ;;
    --help|-h)
      echo "Usage: $0 [--uninstall]"
      echo "  (no flags)   Install lefthook git hooks"
      echo "  --uninstall  Remove lefthook git hooks"
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      echo "Run '$0 --help' for usage." >&2
      exit 1
      ;;
  esac
done

# ── uninstall path ───────────────────────────────────────────────────────────
if $UNINSTALL; then
  if ! command -v lefthook &>/dev/null; then
    echo "lefthook is not installed; removing hook files manually."
    rm -f .git/hooks/pre-commit .git/hooks/pre-push
    echo "Hooks removed."
  else
    lefthook uninstall
    echo "lefthook hooks uninstalled."
  fi
  exit 0
fi

# ── install path ─────────────────────────────────────────────────────────────
if ! command -v lefthook &>/dev/null; then
  echo ""
  echo "lefthook is not installed. Install it with one of the following:"
  echo ""
  echo "  # via cargo (works anywhere Rust toolchain is present):"
  echo "  cargo install lefthook"
  echo ""
  echo "  # via the official install script (Linux/macOS):"
  echo "  curl -sSfL https://raw.githubusercontent.com/evilmartians/lefthook/master/install.sh | sh"
  echo ""
  echo "  # via brew (macOS/Linux):"
  echo "  brew install lefthook"
  echo ""
  echo "  # via apt (Ubuntu/Debian, requires the evilmartians apt repo or snap):"
  echo "  snap install lefthook"
  echo ""
  echo "After installing lefthook, re-run: ./scripts/install-hooks.sh"
  exit 1
fi

lefthook install
echo ""
echo "lefthook hooks installed."
echo "  pre-commit : cargo fmt --check"
echo "  pre-push   : cargo clippy --all-targets -- -D warnings"
echo "  pre-push   : cargo test --lib --quiet"
echo ""
echo "To uninstall: ./scripts/install-hooks.sh --uninstall"
