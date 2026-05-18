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
import { WasmRenderer, diagnosticLabel } from './wasm-renderer.js';

// Use a minimal sequence diagram as the default; the previous multi-actor
// sign-in flow triggered E_FAMILY_MIXED because it mixed deployment syntax
// into a component diagram context during WASM parsing.
const DEFAULT_SOURCE = `@startuml
title Welcome to puml
Alice -> Bob: Hello
Bob --> Alice: World
@enduml
`;

const STORAGE_KEY = 'puml-editor.source';
const FRONTEND_STORAGE_KEY = 'puml-editor.frontend';

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
  const frontendPicker = document.getElementById('frontend-picker');
  if (frontendPicker) {
    const savedFrontend = localStorage.getItem(FRONTEND_STORAGE_KEY);
    const queryFrontend = new URLSearchParams(window.location.search).get('frontend')
      || new URLSearchParams(window.location.search).get('dialect');
    frontendPicker.value = queryFrontend || savedFrontend || 'auto';
    frontendPicker.addEventListener('change', () => {
      try { localStorage.setItem(FRONTEND_STORAGE_KEY, frontendPicker.value); } catch {}
      render();
    });
  }

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
    const frontend = document.getElementById('frontend-picker')?.value || 'auto';
    result = await engine.render(source, { frontend });
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
    previewHost.innerHTML = `
      <div class="preview-placeholder">
        <span class="pill">render error</span>
        <p>${escapeHtml(diagnosticLabel(diag))}</p>
      </div>`;
    setStatus('preview', diag?.line ? `Render error at line ${diag.line}.` : 'Render error.', 'bad');
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
