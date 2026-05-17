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

# 2. install Rust wasm target + wasm-pack (one time)
rustup target add wasm32-unknown-unknown
curl -fsSL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh

# 3. populate site/static/examples, mirror specs, and build the WASM renderer
node ../scripts/build-site.mjs
node ../scripts/mirror-specs.mjs
wasm-pack build --release --target web --out-dir ../site/static/wasm ../crates/puml-wasm

# 4. serve locally
cd site && zola serve

# 5. or build into site/public
cd site && zola build
```

## Deploy

CI: `.github/workflows/pages.yml` runs the steps above on every push to `main` that touches site content or the renderer, then uploads `site/public` to GitHub Pages.

## Editor architecture

`static/js/editor.js` loads CodeMirror through the shared import map declared in `templates/base.html`, wires the `.puml` `StreamLanguage` from `static/js/puml-lang.js`, and renders previews via the WASM bundle at `static/wasm/puml_wasm.js` (built from `crates/puml-wasm/`). See [`/developer/renderer/`](https://alliecatowo.github.io/puml/developer/renderer/) for the renderer bridge.
