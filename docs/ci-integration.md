# CI integration

`puml` is designed to behave like a compiler in CI: install one binary, validate source
files with `--check`, and optionally render deterministic artifacts for review.

This page focuses on copyable recipes that match the current CLI.

---

## CI strategy

Use one of these modes:

| Mode | What it does | Best for |
|---|---|---|
| Lint only | `puml --check` | Fast PR gates and pre-commit hooks |
| Lint Markdown | `puml --from-markdown --check` | Docs that keep diagrams in fences |
| Render artifacts | `puml file.puml` or `--format png/pdf` | Review previews and published docs |
| Structured reports | `--diagnostics json` or `--lint-report json` | Bots, annotations, dashboards |

Always exclude build output directories when scanning a repository:

```bash
find . -name '*.puml' -not -path './target/*' -exec puml --check {} +
```

---

## Install in CI

### Cargo install

```bash
cargo install puml --bin puml
```

This is portable wherever a Rust toolchain is available. Cache Cargo registry and build
outputs if install time matters.

### Pinned Git install

```bash
cargo install --git https://github.com/alliecatowo/puml --rev <commit-sha> --bin puml
```

Use this for reproducible builds from repository source.

### Release asset

The current release workflow publishes Linux x86_64 and macOS x86_64 binary assets.
For Ubuntu runners:

```bash
curl -L -o puml \
  https://github.com/alliecatowo/puml/releases/latest/download/puml-linux-x86_64
chmod +x puml
sudo mv puml /usr/local/bin/puml
puml --version
```

For production CI, prefer a tagged URL instead of `latest`.

---

## GitHub Actions: lint diagrams

```yaml
name: Lint diagrams
on: [pull_request]

jobs:
  puml:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          shared-key: puml-cli

      - name: Install puml
        run: cargo install puml --bin puml

      - name: Check diagrams
        run: |
          find . -name '*.puml' -not -path './target/*' \
            -exec puml --check {} +
```

---

## GitHub Actions: fast binary install

```yaml
name: Lint diagrams
on: [pull_request]

jobs:
  puml:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install puml
        run: |
          curl -L -o puml \
            https://github.com/alliecatowo/puml/releases/latest/download/puml-linux-x86_64
          chmod +x puml
          sudo mv puml /usr/local/bin/puml
          puml --version

      - name: Check diagrams
        run: |
          find . -name '*.puml' -not -path './target/*' \
            -exec puml --check {} +
```

Pin the URL to a tag once you have selected a version:

```text
https://github.com/alliecatowo/puml/releases/download/v0.1.0/puml-linux-x86_64
```

---

## GitHub Actions: Markdown fences

```yaml
- name: Check Markdown diagram fences
  run: |
    find . -name '*.md' -not -path './target/*' \
      -exec puml --from-markdown --check {} +
```

This validates supported fenced blocks such as `puml`, `plantuml`, `picouml`, `uml`,
and selected `mermaid` fences.

---

## GitHub Actions: JSON diagnostics

Use JSON diagnostics when another tool will parse results:

```yaml
- name: Check diagrams with JSON diagnostics
  run: |
    set -o pipefail
    find . -name '*.puml' -not -path './target/*' \
      -exec puml --diagnostics json --check {} + \
      | tee puml-diagnostics.json
```

For top-level batch linting with a summary report:

```yaml
- name: Batch lint report
  run: |
    puml --check --lint-glob 'docs/**/*.puml' --lint-report json \
      > puml-lint-report.json
```

---

## GitHub Actions: render SVG artifacts

Render SVGs on every PR and upload them for review:

```yaml
name: Render diagram previews
on: [pull_request]

jobs:
  render:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Install puml
        run: cargo install puml --bin puml

      - name: Render SVG previews
        run: |
          mkdir -p /tmp/puml-svg
          find . -name '*.puml' -not -path './target/*' | while read -r f; do
            out="/tmp/puml-svg/${f#./}"
            mkdir -p "$(dirname "$out")"
            puml "$f" -o "${out%.puml}.svg"
          done

      - uses: actions/upload-artifact@v4
        with:
          name: puml-svg-previews
          path: /tmp/puml-svg/**/*.svg
```

---

## GitHub Actions: render PNG or PDF previews

```yaml
- name: Render PNG previews
  run: |
    mkdir -p /tmp/puml-png
    find . -name '*.puml' -not -path './target/*' | while read -r f; do
      out="/tmp/puml-png/${f#./}"
      mkdir -p "$(dirname "$out")"
      puml --format png --dpi 192 "$f" -o "${out%.puml}.png"
    done

- uses: actions/upload-artifact@v4
  with:
    name: puml-png-previews
    path: /tmp/puml-png/**/*.png
```

For PDF, change `--format png` and the extension to `pdf`.

---

## GitLab CI

```yaml
puml-lint:
  stage: test
  image: rust:1.88-slim
  script:
    - cargo install puml --bin puml
    - find . -name '*.puml' -not -path './target/*' -exec puml --check {} +
  cache:
    key: puml-cargo
    paths:
      - /usr/local/cargo/registry
      - target
  rules:
    - if: '$CI_PIPELINE_SOURCE == "merge_request_event"'
```

With the Linux release asset:

```yaml
puml-lint:
  stage: test
  image: debian:bookworm-slim
  before_script:
    - apt-get update -qq && apt-get install -y curl ca-certificates
    - curl -L -o /usr/local/bin/puml https://github.com/alliecatowo/puml/releases/latest/download/puml-linux-x86_64
    - chmod +x /usr/local/bin/puml
    - puml --version
  script:
    - find . -name '*.puml' -not -path './target/*' -exec puml --check {} +
```

---

## pre-commit framework

Add this to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: puml-check
        name: Validate puml diagrams
        language: system
        entry: puml --check
        files: '\.puml$'
        pass_filenames: true
```

Install the hook:

```bash
pre-commit install
```

`puml` must already be on your `PATH`.

---

## Manual git pre-commit hook

```bash
cat > .git/hooks/pre-commit <<'EOF_HOOK'
#!/usr/bin/env bash
set -euo pipefail

staged=$(git diff --cached --name-only --diff-filter=ACMR | grep '\.puml$' || true)
if [ -z "$staged" ]; then
  exit 0
fi

printf '%s\n' "$staged" | xargs puml --check
EOF_HOOK
chmod +x .git/hooks/pre-commit
```

This checks only staged `.puml` files.

---

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Validation or diagnostics failure |
| 2 | I/O error |
| 3 | Internal error |

Treat code `3` as a `puml` bug and report it with the source input and command.

---

## CI tips

- Pin versions in production CI instead of using `latest`.
- Use `--from-markdown --check` for docs repositories that keep diagrams in Markdown.
- Upload rendered SVG or PNG artifacts on PRs when reviewers need visual output.
- Use `--allow-url-includes` only in workflows that intentionally permit network reads.
- Use `--include-root` when rendering stdin input that contains relative includes.
- Keep generated SVG/PDF/PNG artifacts in git only when your docs publishing workflow
  expects checked-in render outputs.
