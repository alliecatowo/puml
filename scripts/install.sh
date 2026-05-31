#!/usr/bin/env sh
# install.sh — puml installer
#
# Downloads the latest (or a specific) signed puml release binary from GitHub,
# verifies its SHA-256 checksum, optionally verifies its cosign signature, and
# installs it to a user-writable prefix.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/alliecatowo/puml/main/scripts/install.sh | sh
#   curl -fsSL ... | sh -s -- --version v0.2.1
#   curl -fsSL ... | sh -s -- --prefix /opt/puml --dry-run
#
# Flags:
#   --version <tag>     Install a specific release tag (default: latest)
#   --prefix <dir>      Install binary under <dir>/bin (default: auto-detected)
#   --dry-run           Print what would be done without downloading or installing
#   --no-verify-sig     Skip cosign signature verification (still verifies SHA-256)
#   -h, --help          Show this help and exit
#
# Trust model:
#   1. SHA-256 checksum is always verified against the SHA256SUMS file published
#      to the GitHub release (downloaded over HTTPS).
#   2. cosign keyless signature is verified by default when cosign is on PATH.
#      Pass --no-verify-sig to skip (not recommended).
#   3. The binary is NOT executed during install; only `puml --version` is run
#      after installation as a self-test.
#
# Requirements: curl or wget, sha256sum or shasum, tar or unzip, optionally cosign.
#
# Exit codes: 0 success  1 fatal error  2 platform not supported

set -eu

# ── Constants ───────────────────────────────────────────────────────────────
REPO="alliecatowo/puml"
BINARY_NAME="puml"
GH_RELEASES="https://github.com/${REPO}/releases"
GH_API_LATEST="https://api.github.com/repos/${REPO}/releases/latest"

# ── Defaults ────────────────────────────────────────────────────────────────
INSTALL_VERSION=""      # empty = latest
INSTALL_PREFIX=""       # empty = auto-detect
DRY_RUN=0
VERIFY_SIG=1

# ── Helpers ─────────────────────────────────────────────────────────────────
say()  { printf '%s\n' "$*"; }
err()  { printf 'error: %s\n' "$*" >&2; exit 1; }
warn() { printf 'warning: %s\n' "$*" >&2; }

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    err "required command not found: $1 — please install it and re-run"
  fi
}

have_cmd() { command -v "$1" >/dev/null 2>&1; }

# Download a URL to a file; tries curl then wget.
download() {
  _url="$1"; _dest="$2"
  if have_cmd curl; then
    curl --proto '=https' --tlsv1.2 -fsSL "$_url" -o "$_dest"
  elif have_cmd wget; then
    wget --https-only -q "$_url" -O "$_dest"
  else
    err "neither curl nor wget is available"
  fi
}

# Print to stdout; used by download_stdout.
download_stdout() {
  _url="$1"
  if have_cmd curl; then
    curl --proto '=https' --tlsv1.2 -fsSL "$_url"
  elif have_cmd wget; then
    wget --https-only -q "$_url" -O -
  else
    err "neither curl nor wget is available"
  fi
}

# ── Platform detection ───────────────────────────────────────────────────────
detect_platform() {
  _os="$(uname -s)"
  _arch="$(uname -m)"

  case "$_os" in
    Linux)
      case "$_arch" in
        x86_64)  PLATFORM_TRIPLE="x86_64-unknown-linux-musl"  ;;
        aarch64) PLATFORM_TRIPLE="aarch64-unknown-linux-musl" ;;
        arm64)   PLATFORM_TRIPLE="aarch64-unknown-linux-musl" ;;
        *) err "unsupported Linux architecture: $_arch (supported: x86_64, aarch64)" ;;
      esac
      ARCHIVE_EXT="tar.gz"
      ;;
    Darwin)
      case "$_arch" in
        arm64)   PLATFORM_TRIPLE="aarch64-apple-darwin" ;;
        x86_64)  PLATFORM_TRIPLE="x86_64-apple-darwin"  ;;
        *) err "unsupported macOS architecture: $_arch (supported: arm64, x86_64)" ;;
      esac
      ARCHIVE_EXT="tar.gz"
      ;;
    MINGW*|MSYS*|CYGWIN*|Windows_NT)
      case "$_arch" in
        x86_64|AMD64) PLATFORM_TRIPLE="x86_64-pc-windows-msvc" ;;
        *) err "unsupported Windows architecture: $_arch (supported: x86_64)" ;;
      esac
      ARCHIVE_EXT="zip"
      ;;
    *)
      err "unsupported OS: $_os (supported: Linux, macOS, Windows)"
      ;;
  esac

  ARCHIVE_NAME="puml-${PLATFORM_TRIPLE}.${ARCHIVE_EXT}"
  LSP_ARCHIVE_NAME="puml-lsp-${PLATFORM_TRIPLE}.${ARCHIVE_EXT}"
}

# ── Prefix selection ─────────────────────────────────────────────────────────
detect_prefix() {
  if [ -n "$INSTALL_PREFIX" ]; then
    return
  fi

  # Prefer /usr/local/bin if writable (e.g. macOS with admin rights)
  if [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    INSTALL_PREFIX="/usr/local"
    return
  fi

  # Fall back to ~/.local (XDG-style, never requires sudo)
  INSTALL_PREFIX="${HOME}/.local"
}

# ── Parse arguments ──────────────────────────────────────────────────────────
parse_args() {
  while [ $# -gt 0 ]; do
    case "$1" in
      --version)
        [ $# -ge 2 ] || err "--version requires an argument"
        INSTALL_VERSION="$2"; shift 2 ;;
      --prefix)
        [ $# -ge 2 ] || err "--prefix requires an argument"
        INSTALL_PREFIX="$2"; shift 2 ;;
      --dry-run)
        DRY_RUN=1; shift ;;
      --no-verify-sig)
        VERIFY_SIG=0; shift ;;
      -h|--help)
        usage; exit 0 ;;
      *)
        err "unknown option: $1 (run with --help for usage)" ;;
    esac
  done
}

usage() {
  cat <<'EOF'
install.sh — puml installer

USAGE:
  curl -fsSL https://raw.githubusercontent.com/alliecatowo/puml/main/scripts/install.sh | sh
  curl -fsSL ... | sh -s -- [OPTIONS]

OPTIONS:
  --version <tag>     Install a specific release tag, e.g. v0.2.1 (default: latest)
  --prefix <dir>      Install to <dir>/bin (default: /usr/local or ~/.local)
  --dry-run           Show what would happen without downloading or installing
  --no-verify-sig     Skip cosign signature check (SHA-256 still verified)
  -h, --help          Show this message and exit

EXAMPLES:
  Install latest release:
    curl -fsSL https://raw.githubusercontent.com/alliecatowo/puml/main/scripts/install.sh | sh

  Install specific version to ~/.local:
    curl -fsSL ... | sh -s -- --version v0.2.1 --prefix ~/.local

  Inspect what would be installed:
    curl -fsSL ... | sh -s -- --dry-run
EOF
}

# ── Resolve latest tag via GitHub API ────────────────────────────────────────
resolve_version() {
  if [ -n "$INSTALL_VERSION" ]; then
    return
  fi
  say "Fetching latest release tag from GitHub..."
  INSTALL_VERSION="$(download_stdout "$GH_API_LATEST" \
    | grep '"tag_name"' \
    | head -1 \
    | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')"
  [ -n "$INSTALL_VERSION" ] || err "could not determine latest release tag"
  say "Latest release: $INSTALL_VERSION"
}

# ── Main ─────────────────────────────────────────────────────────────────────
main() {
  parse_args "$@"
  detect_platform
  detect_prefix

  BIN_DIR="${INSTALL_PREFIX}/bin"

  resolve_version

  BASE_URL="${GH_RELEASES}/download/${INSTALL_VERSION}"
  ARCHIVE_URL="${BASE_URL}/${ARCHIVE_NAME}"
  SHA256SUMS_URL="${BASE_URL}/SHA256SUMS"

  say ""
  say "  platform : ${PLATFORM_TRIPLE}"
  say "  version  : ${INSTALL_VERSION}"
  say "  archive  : ${ARCHIVE_NAME}"
  say "  dest     : ${BIN_DIR}/puml"
  say ""

  if [ "$DRY_RUN" -eq 1 ]; then
    say "[dry-run] Would download: ${ARCHIVE_URL}"
    say "[dry-run] Would verify SHA-256 from: ${SHA256SUMS_URL}"
    if [ "$VERIFY_SIG" -eq 1 ] && have_cmd cosign; then
      say "[dry-run] Would verify cosign bundle: ${ARCHIVE_URL}.cosign.bundle"
    fi
    say "[dry-run] Would install to: ${BIN_DIR}/puml"
    exit 0
  fi

  # ── Working directory ──────────────────────────────────────────────────────
  TMP_DIR="$(mktemp -d)"
  # shellcheck disable=SC2064
  trap "rm -rf '$TMP_DIR'" EXIT INT TERM

  # ── Download archive ───────────────────────────────────────────────────────
  say "Downloading ${ARCHIVE_NAME}..."
  download "$ARCHIVE_URL" "${TMP_DIR}/${ARCHIVE_NAME}"

  # ── Verify SHA-256 ─────────────────────────────────────────────────────────
  say "Downloading SHA256SUMS..."
  download "$SHA256SUMS_URL" "${TMP_DIR}/SHA256SUMS"

  say "Verifying SHA-256 checksum..."
  # Extract the expected hash for our archive only (avoid failures from missing files)
  EXPECTED_HASH="$(grep " ${ARCHIVE_NAME}$" "${TMP_DIR}/SHA256SUMS" | awk '{print $1}')"
  [ -n "$EXPECTED_HASH" ] || err "checksum entry for ${ARCHIVE_NAME} not found in SHA256SUMS"

  if have_cmd sha256sum; then
    ACTUAL_HASH="$(sha256sum "${TMP_DIR}/${ARCHIVE_NAME}" | awk '{print $1}')"
  elif have_cmd shasum; then
    ACTUAL_HASH="$(shasum -a 256 "${TMP_DIR}/${ARCHIVE_NAME}" | awk '{print $1}')"
  else
    err "no sha256sum or shasum found — cannot verify checksum"
  fi

  if [ "$EXPECTED_HASH" != "$ACTUAL_HASH" ]; then
    err "SHA-256 mismatch for ${ARCHIVE_NAME}
  expected: ${EXPECTED_HASH}
  actual:   ${ACTUAL_HASH}
  Refusing to install a corrupt or tampered archive."
  fi
  say "SHA-256 verified OK"

  # ── Verify cosign signature (optional) ────────────────────────────────────
  if [ "$VERIFY_SIG" -eq 1 ]; then
    if have_cmd cosign; then
      say "Downloading cosign bundle..."
      BUNDLE_URL="${ARCHIVE_URL}.cosign.bundle"
      download "$BUNDLE_URL" "${TMP_DIR}/${ARCHIVE_NAME}.cosign.bundle"

      say "Verifying cosign signature..."
      cosign verify-blob \
        --bundle "${TMP_DIR}/${ARCHIVE_NAME}.cosign.bundle" \
        --certificate-identity-regexp "https://github.com/${REPO}/" \
        --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
        "${TMP_DIR}/${ARCHIVE_NAME}"
      say "cosign signature verified OK"
    else
      warn "cosign not found on PATH — skipping signature verification"
      warn "Install cosign for full supply-chain verification: https://docs.sigstore.dev/cosign/installation/"
    fi
  else
    warn "Signature verification skipped (--no-verify-sig)"
  fi

  # ── Extract archive ────────────────────────────────────────────────────────
  say "Extracting archive..."
  mkdir -p "${TMP_DIR}/extracted"
  case "$ARCHIVE_EXT" in
    tar.gz)
      need_cmd tar
      tar -xzf "${TMP_DIR}/${ARCHIVE_NAME}" -C "${TMP_DIR}/extracted"
      ;;
    zip)
      if have_cmd unzip; then
        unzip -q "${TMP_DIR}/${ARCHIVE_NAME}" -d "${TMP_DIR}/extracted"
      elif have_cmd python3; then
        python3 -c "
import zipfile, sys
with zipfile.ZipFile('${TMP_DIR}/${ARCHIVE_NAME}') as z:
    z.extractall('${TMP_DIR}/extracted')
"
      else
        err "no unzip or python3 found — cannot extract zip archive"
      fi
      ;;
    *)
      err "unsupported archive format: $ARCHIVE_EXT"
      ;;
  esac

  # Locate the binary inside the extracted tree
  case "$ARCHIVE_EXT" in
    tar.gz) EXTRACTED_BIN="${TMP_DIR}/extracted/${BINARY_NAME}" ;;
    zip)    EXTRACTED_BIN="${TMP_DIR}/extracted/${BINARY_NAME}.exe" ;;
  esac

  [ -f "$EXTRACTED_BIN" ] || \
    EXTRACTED_BIN="$(find "${TMP_DIR}/extracted" -name "${BINARY_NAME}" -o -name "${BINARY_NAME}.exe" | head -1)"
  [ -f "$EXTRACTED_BIN" ] || err "could not locate ${BINARY_NAME} inside extracted archive"

  # ── Install binary ─────────────────────────────────────────────────────────
  if [ ! -d "$BIN_DIR" ]; then
    say "Creating ${BIN_DIR}..."
    mkdir -p "$BIN_DIR"
  fi

  INSTALL_PATH="${BIN_DIR}/${BINARY_NAME}"

  # On Windows (MSYS/Cygwin) add .exe suffix
  case "$ARCHIVE_EXT" in
    zip) INSTALL_PATH="${BIN_DIR}/${BINARY_NAME}.exe" ;;
  esac

  say "Installing to ${INSTALL_PATH}..."
  cp "$EXTRACTED_BIN" "$INSTALL_PATH"
  chmod 755 "$INSTALL_PATH"

  # ── Self-test ──────────────────────────────────────────────────────────────
  say "Running self-test: ${INSTALL_PATH} --version"
  "$INSTALL_PATH" --version

  # ── PATH hint ─────────────────────────────────────────────────────────────
  say ""
  say "puml ${INSTALL_VERSION} installed successfully!"
  say ""

  # Detect whether BIN_DIR is already on PATH
  case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *)
      say "Add ${BIN_DIR} to your PATH to use puml from any directory:"
      say ""
      say "  For bash/zsh, add to ~/.bashrc or ~/.zshrc:"
      say "    export PATH=\"${BIN_DIR}:\$PATH\""
      say ""
      say "  For fish:"
      say "    fish_add_path ${BIN_DIR}"
      say ""
      ;;
  esac

  say "Get started:"
  say "  puml --help"
  say "  puml hello.puml        # renders hello.svg"
  say "  puml --format png hello.puml"
}

main "$@"
