# URL Include Policy

`puml` treats remote includes as a compatibility feature and a trust boundary.
PlantUML-compatible command-line rendering enables URL includes by default, while
embedded/editor surfaces avoid surprise network access.

## Surface Defaults

| Surface | URL include behavior |
|---|---|
| CLI / native library with default features | Enabled by default for `!include https://...`, `!includeurl`, URL `!include_many`, and URL `!import`; pass `--no-url-includes` or set `ParsePipelineOptions::no_url_includes = true` to reject remote targets. |
| LSP (`puml-lsp`) | Disabled for diagnostics, hover, semantic features, and preview commands. Opening a document should never fetch a remote URL as an editor side effect. |
| WASM / browser studio | Unsupported because the WASM crate builds without URL include dependencies and has no filesystem resolver. URL targets return deterministic diagnostics. |
| Agent / automation surfaces | Prefer explicit policy: use the CLI default when you are intentionally checking PlantUML compatibility, and use `--no-url-includes` for untrusted inputs or no-network audit runs. |

## Cache And Diagnostics

HTTP(S) URL includes are cached under `$XDG_CACHE_HOME/puml/includes/<sha256>`
or `$HOME/.cache/puml/includes/<sha256>` when `XDG_CACHE_HOME` is not set.
The cache keeps repeated renders byte-stable and avoids repeated network calls
for the same URL.

Disabling URL includes produces `E_INCLUDE_URL_DISABLED`. Fetch, status, cache,
or unsupported-build failures use `E_INCLUDE_URL_FETCH`,
`E_INCLUDE_URL_CACHE_READ`, or `E_INCLUDE_URL_UNSUPPORTED` as appropriate.

## Why This Split Exists

PlantUML compatibility requires accepting remote includes in the native renderer.
Editor and browser contexts have different expectations: simply opening a file
should not perform network IO, and the browser build cannot use the native cache
or filesystem resolver. Keeping the policy explicit lets compatibility and
safety coexist instead of pretending one default fits every surface.
