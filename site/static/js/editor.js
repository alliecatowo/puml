// Editor page bootstrap.
// Loads CodeMirror 6 from esm.sh (via the import map declared in base.html so
// every @codemirror/* package is a single shared module instance), wires the
// puml StreamLanguage, and renders previews with the WASM renderer built from
// the puml-wasm crate.

import { EditorView, basicSetup } from 'codemirror';
import { EditorState, Compartment } from '@codemirror/state';
import { keymap } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { syntaxHighlighting, HighlightStyle } from '@codemirror/language';
import { tags as t } from '@lezer/highlight';

import { pumlLanguage } from './puml-lang.js';
import { loadManifest, siteBaseUrl, assetUrl } from './manifest.js';

const DEFAULT_SOURCE = `@startuml
title Sign-in handshake

actor User
participant "Web App" as Web
participant API
database Sessions

User -> Web: POST /login
activate Web

Web -> API: validate(credentials)
activate API
API -> Sessions: create session
Sessions --> API: sessionId
API --> Web: 200 OK + sessionId
deactivate API

Web --> User: Set-Cookie: sid
deactivate Web
@enduml
`;

const STORAGE_KEY = 'puml-editor.source';

// Map CodeMirror highlight tags (from the StreamLanguage token names) to our
// custom CSS classes. Each class is defined in sass/style.scss.
const highlightStyle = HighlightStyle.define([
  { tag: t.comment,      class: 'tok-comment' },
  { tag: t.meta,         class: 'tok-directive' },
  { tag: t.keyword,      class: 'tok-keyword' },
  { tag: t.atom,         class: 'tok-lifecycle' },
  { tag: t.typeName,     class: 'tok-stereo' },
  { tag: t.operator,     class: 'tok-arrow' },
  { tag: t.string,       class: 'tok-string' },
  { tag: t.number,       class: 'tok-number' },
  { tag: t.literal,      class: 'tok-color' },
  { tag: t.bracket,      class: 'tok-bracket' },
  { tag: t.variableName, color: 'inherit' },
]);

// Single shared engine: dynamic-imports the wasm-bindgen JS shim, initializes
// the .wasm binary, then exposes render(source) used by the editor on each
// keystroke (debounced) and Cmd/Ctrl+Enter.
class WasmRenderer {
  constructor(base) {
    this.base = base;
    this.ready = null;
    this.module = null;
  }

  describe() { return 'Renderer: in-browser WASM'; }

  async init() {
    if (this.ready) return this.ready;
    this.ready = (async () => {
      const jsUrl = assetUrl(this.base, 'wasm/puml_wasm.js');
      const wasmUrl = assetUrl(this.base, 'wasm/puml_wasm_bg.wasm');
      const mod = await import(jsUrl);
      await mod.default({ module_or_path: wasmUrl });
      this.module = mod;
    })();
    return this.ready;
  }

  async render(source) {
    await this.init();
    const json = this.module.render_svgs_json(source);
    let parsed;
    try {
      parsed = JSON.parse(json);
    } catch (e) {
      return { ok: false, diagnostics: [{ severity: 'error', message: `Renderer returned invalid JSON: ${e.message}` }] };
    }
    if (parsed.error) {
      return {
        ok: false,
        diagnostics: [{
          severity: parsed.error.severity || 'error',
          message: parsed.error.message || 'Render failed.',
          line: parsed.error.line,
          column: parsed.error.column,
        }],
      };
    }
    const pages = Array.isArray(parsed.ok) ? parsed.ok : [];
    if (!pages.length) {
      return { ok: false, diagnostics: [{ severity: 'error', message: 'Renderer returned no pages.' }] };
    }
    return { ok: true, svgs: pages };
  }
}

let view;
let engine;
let manifest;
let base;
const langCompartment = new Compartment();

async function init() {
  base = siteBaseUrl();
  engine = new WasmRenderer(base);
  setStatus('preview', 'Renderer: loading WASM…', 'warn');

  // Load the example picker manifest in parallel with the WASM init.
  try {
    manifest = await loadManifest(base);
  } catch (e) {
    // Examples are nice-to-have; the renderer doesn't depend on them.
    setStatus('editor', `Could not load example list: ${e.message}`, 'warn', true);
    manifest = { examples: [], families: [] };
  }

  // Populate example picker grouped by family.
  const picker = document.getElementById('example-picker');
  if (picker && manifest.examples.length) {
    const familyMap = new Map();
    for (const ex of manifest.examples) {
      if (!familyMap.has(ex.family)) familyMap.set(ex.family, []);
      familyMap.get(ex.family).push(ex);
    }
    for (const [, items] of [...familyMap.entries()].sort()) {
      const og = document.createElement('optgroup');
      og.label = items[0].familyLabel;
      for (const it of items) {
        const opt = document.createElement('option');
        opt.value = `${it.family}/${it.name}`;
        opt.textContent = it.title;
        og.appendChild(opt);
      }
      picker.appendChild(og);
    }
    picker.addEventListener('change', async (e) => {
      const id = e.target.value;
      if (!id) return;
      await openExampleById(id);
      e.target.value = '';
    });
  }

  document.getElementById('reset-btn').addEventListener('click', () => {
    setSource(DEFAULT_SOURCE);
    render();
  });

  document.getElementById('copy-btn').addEventListener('click', async () => {
    const text = view.state.doc.toString();
    try {
      await navigator.clipboard.writeText(text);
      setStatus('editor', `Copied ${text.length} chars to clipboard.`, 'ok', true);
    } catch {
      setStatus('editor', 'Clipboard unavailable.', 'warn', true);
    }
  });

  document.getElementById('render-btn').addEventListener('click', render);
  document.getElementById('download-btn').addEventListener('click', downloadSvg);

  // Build the editor.
  const initial = localStorage.getItem(STORAGE_KEY) || DEFAULT_SOURCE;
  view = new EditorView({
    parent: document.getElementById('editor-host'),
    state: EditorState.create({
      doc: initial,
      extensions: [
        basicSetup,
        history(),
        keymap.of([indentWithTab, ...defaultKeymap, ...historyKeymap, {
          key: 'Mod-Enter', run: () => { render(); return true; },
        }, {
          key: 'Mod-s', preventDefault: true, run: () => { downloadSvg(); return true; },
        }]),
        langCompartment.of(pumlLanguage),
        syntaxHighlighting(highlightStyle),
        EditorView.theme({}, { dark: true }),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) {
            const text = u.state.doc.toString();
            try { localStorage.setItem(STORAGE_KEY, text); } catch {}
            scheduleAutoRender();
          }
        }),
      ],
    }),
  });

  setStatus('editor', 'Ready. Type, or load an example. Cmd/Ctrl+Enter to render.', 'ok');

  // Warm the renderer in the background while the user reads the page; the
  // first render will then be near-instant.
  engine.init().then(() => {
    setStatus('preview', engine.describe(), 'ok');
  }).catch((e) => {
    setStatus('preview', `Renderer failed to initialize: ${e.message}`, 'bad');
  });

  // Open via ?open=family/name; otherwise render the current source.
  const params = new URLSearchParams(window.location.search);
  const open = params.get('open');
  if (open) {
    await openExampleById(open);
  } else {
    render();
  }
}

let renderTimer = null;
function scheduleAutoRender() {
  if (renderTimer) clearTimeout(renderTimer);
  renderTimer = setTimeout(render, 400);
}

function setSource(text) {
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: text },
  });
}

async function openExampleById(id) {
  if (!manifest || !manifest.examples) return;
  const [family, name] = id.split('/');
  const ex = manifest.examples.find((e) => e.family === family && e.name === name);
  if (!ex) {
    setStatus('editor', `Unknown example: ${id}`, 'bad', true);
    return;
  }
  try {
    const res = await fetch(assetUrl(base, ex.pumlPath));
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const text = await res.text();
    setSource(text);
    setStatus('editor', `Loaded ${ex.familyLabel} / ${ex.title}.`, 'ok');
    render();
  } catch (e) {
    setStatus('editor', `Failed to load ${id}: ${e.message}`, 'bad', true);
  }
}

async function render() {
  if (!view || !engine) return;
  const source = view.state.doc.toString();
  const previewHost = document.getElementById('preview-host');
  let result;
  try {
    result = await engine.render(source);
  } catch (e) {
    result = { ok: false, diagnostics: [{ severity: 'error', message: e.message || String(e) }] };
  }
  if (result.ok) {
    previewHost.innerHTML = result.svgs.join('\n');
    const pages = result.svgs.length;
    setStatus('preview', pages > 1
      ? `Rendered ${pages} pages.`
      : `Rendered.`, 'ok');
  } else {
    const diag = result.diagnostics?.[0];
    const where = diag?.line ? ` (line ${diag.line}${diag.column ? `, col ${diag.column}` : ''})` : '';
    previewHost.innerHTML = `
      <div class="preview-placeholder">
        <span class="pill">render error</span>
        <p>${escapeHtml(diag?.message ?? 'Render failed.')}${escapeHtml(where)}</p>
      </div>`;
    setStatus('preview', `Render error${where}.`, 'bad');
  }
}

function downloadSvg() {
  const host = document.getElementById('preview-host');
  const svg = host.querySelector('svg');
  if (!svg) {
    setStatus('preview', 'Nothing to download yet.', 'warn', true);
    return;
  }
  const blob = new Blob([new XMLSerializer().serializeToString(svg)], { type: 'image/svg+xml' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'diagram.svg';
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function setStatus(which, message, kind = 'ok', revertAfter = false) {
  const bar = document.getElementById(`${which}-status`);
  const txt = document.getElementById(`${which}-status-text`);
  if (!bar || !txt) return;
  bar.classList.remove('ok', 'warn', 'bad');
  if (kind) bar.classList.add(kind);
  txt.textContent = message;
  if (revertAfter) {
    setTimeout(() => {
      if (which === 'preview') txt.textContent = engine ? engine.describe() : '';
      if (which === 'editor') txt.textContent = 'Ready. Type, or load an example. Cmd/Ctrl+Enter to render.';
    }, 2500);
  }
}

function escapeHtml(s) {
  return String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

init().catch((err) => {
  setStatus('editor', `Editor failed to start: ${err.message || err}`, 'bad');
  // Also surface in the preview pane so users see something went wrong even
  // if they don't notice the status bar.
  const host = document.getElementById('preview-host');
  if (host) {
    host.innerHTML = `
      <div class="preview-placeholder">
        <span class="pill">startup error</span>
        <p>${escapeHtml(err.message || String(err))}</p>
      </div>`;
  }
  console.error(err);
});
