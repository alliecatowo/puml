# CI integration

## GitHub Actions: lint on PR

Validate all `.puml` files on every pull request without writing output.

```yaml
name: Lint diagrams
on: [pull_request]

jobs:
  puml-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Install puml
        run: cargo install puml --bin puml

      - name: Check all diagrams
        run: |
          find . -name '*.puml' -not -path './target/*' \
            -exec puml --check {} +
```

## GitHub Actions: render and commit SVGs

Render all `.puml` files and commit the resulting SVGs on push to main.

```yaml
name: Render diagrams
on:
  push:
    branches: [main]
    paths: ['docs/**/*.puml', '**/*.puml']

jobs:
  render:
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - name: Install puml
        run: cargo install puml --bin puml

      - name: Render diagrams
        run: |
          find . -name '*.puml' -not -path './target/*' | while read f; do
            puml "$f"
          done

      - name: Commit rendered SVGs
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add '*.svg'
          git diff --staged --quiet || git commit -m "chore: render diagrams [skip ci]"
          git push
```

## GitHub Actions: lint Markdown fences

Check all fenced `puml` blocks embedded in Markdown files.

```yaml
- name: Check Markdown diagram fences
  run: |
    find . -name '*.md' -not -path './target/*' \
      -exec puml --from-markdown --check {} +
```

## GitLab CI

```yaml
puml-lint:
  image: rust:latest
  before_script:
    - cargo install puml --bin puml
  script:
    - find . -name '*.puml' -not -path './target/*' -exec puml --check {} +
  rules:
    - if: '$CI_PIPELINE_SOURCE == "merge_request_event"'
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Validation error (diagram is invalid) |
| 2 | I/O error |
| 3 | Internal error |

Use `--diagnostics json` for machine-readable output in scripts.
