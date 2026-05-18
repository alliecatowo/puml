# URL Include Policy

`puml` treats URL-addressed includes as a compatibility feature and a trust
boundary. PlantUML-compatible command-line rendering enables URL includes by
default, while embedded/editor surfaces avoid surprise network access or local
file reads.

## Surface Defaults

| Surface | URL include behavior |
|---|---|
| CLI / native library with default features | Enabled by default for `!include https://...`, `!include http://...`, `!include file://...`, `!includeurl`, URL `!include_many`, and URL `!import`; pass `--no-url-includes` or set `ParsePipelineOptions::no_url_includes = true` to reject URL targets. |
| LSP (`puml-lsp`) | Disabled for diagnostics, hover, semantic features, and preview commands. Opening a document should never fetch a remote URL or read a `file://` target as an editor side effect. |
| WASM / browser studio | Unsupported because the WASM crate builds without URL include dependencies and has no filesystem resolver. URL targets return deterministic diagnostics. |
| Agent / automation surfaces | Disabled by default for the bundled MCP tools. Pass `allow_url_includes: true` in a tool call only when URL-based fetching or local `file://` reads are intentional. Use the native CLI default when you are deliberately checking PlantUML compatibility outside an agent tool surface. |

## Cache And Diagnostics

HTTP(S) URL includes are cached under `$XDG_CACHE_HOME/puml/includes/<sha256>`
or `$HOME/.cache/puml/includes/<sha256>` when `XDG_CACHE_HOME` is not set.
The cache keeps repeated renders byte-stable and avoids repeated network calls
for the same URL. Cache entries do not currently expire automatically; remove
the matching cache file or the `puml/includes` cache directory to force a fresh
fetch.

Network fetches use a 10 second connect/read/write timeout, do not follow HTTP
redirects, and reject response bodies larger than 1 MiB before caching them.
Redirects fail with `E_INCLUDE_URL_REDIRECT`; oversized responses fail with
`E_INCLUDE_URL_TOO_LARGE`.

`file://` URL includes are not cached; when URL includes are enabled they read
directly from the local filesystem path named by the URL.

Disabling URL includes produces `E_INCLUDE_URL_DISABLED` for HTTP(S) and
`file://` targets. Fetch, file-read, status, cache, or unsupported-build
failures use `E_INCLUDE_URL_FETCH`, `E_INCLUDE_URL_CACHE_READ`, or
`E_INCLUDE_URL_UNSUPPORTED` as appropriate.

## Why This Split Exists

PlantUML compatibility requires accepting URL includes in the native renderer.
Editor and browser contexts have different expectations: simply opening a file
should not perform network IO or read arbitrary local `file://` targets, and the
browser build cannot use the native cache or filesystem resolver. Keeping the
policy explicit lets compatibility and safety coexist instead of pretending one
default fits every surface.
