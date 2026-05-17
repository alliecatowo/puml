# `site/` &mdash; puml docs + studio

Static site for [https://alliecatowo.github.io/puml/](https://alliecatowo.github.io/puml/).

- Generator: [Zola](https://www.getzola.org/) (Rust-native, picked to stay in-ecosystem).
- Content: Markdown in `content/`, with custom Tera templates in `templates/`.
- Styling: SCSS in `sass/style.scss`, compiled by Zola.
- Examples: every `.puml` / `.svg` pair under `../docs/examples/` is copied into `static/examples/` and indexed by `static/examples-index.json`.
- Specs: every file in `../docs/specs/` is mirrored into `content/developer/specs/` with Zola-friendly frontmatter.

## Local build

```bash
# 1. install Zola (one time)
curl -fsSL https://github.com/getzola/zola/releases/download/v0.19.2/zola-v0.19.2-x86_64-unknown-linux-gnu.tar.gz \
  | sudo tar -xz -C /usr/local/bin/

# 2. populate site/static/examples and site/content/developer/specs
node ../scripts/build-site.mjs
node ../scripts/mirror-specs.mjs

# 3. serve locally
cd site && zola serve

# 4. or build into site/public
cd site && zola build
```

## Deploy

CI: `.github/workflows/pages.yml` runs the three steps above on every push to `main` that touches site content, then uploads `site/public` to GitHub Pages.

## Editor architecture

`static/js/editor.js` programs against a `RenderEngine` interface. v1 is `ManifestLookupEngine` (matches source hash against the baked corpus). v2 will be a `WasmEngine` that posts messages to a Web Worker loading `puml-wasm`. See [`/developer/wasm-roadmap/`](https://alliecatowo.github.io/puml/developer/wasm-roadmap/).
