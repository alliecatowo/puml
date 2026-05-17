// Editor page bootstrap.
// Loads CodeMirror 6 from esm.sh, wires the puml StreamLanguage, and renders
// previews via a pluggable RenderEngine. v1 = manifest lookup (matches source
// against the 248 baked examples). v2 (planned) = WASM worker, drop-in
// replacement behind the same interface.

import { EditorView, basicSetup } from 'https://esm.sh/codemirror@6.0.1';
import { EditorState, Compartment } from 'https://esm.sh/@codemirror/state@6.4.1';
import { keymap } from 'https://esm.sh/@codemirror/view@6.26.3';
import { defaultKeymap, history, historyKeymap, indentWithTab } from 'https://esm.sh/@codemirror/commands@6.6.0';
import { syntaxHighlighting, HighlightStyle } from 'https://esm.sh/@codemirror/language@6.10.2';
import { tags as t } from 'https://esm.sh/@lezer/highlight@1.2.0';

import { pumlLanguage } from './puml-lang.js';
import { loadManifest, normalizeSource, hashSource, siteBaseUrl, assetUrl } from './manifest.js';

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

const STORAGE_KEY = 'puml-editor.source.v1';

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

class ManifestLookupEngine {
  constructor(manifest, base) {
    this.manifest = manifest;
    this.base = base;
    this.byHash = new Map(manifest.examples.map((e) => [e.hash, e]));
  }

  describe() { return 'Renderer: manifest lookup (v1)'; }

  async render(source) {
    const h = await hashSource(source);
    const hit = this.byHash.get(h);
    if (!hit) {
      return {
        ok: false,
        diagnostics: [{
          severity: 'info',
          message: 'No baked example matches this source. Live in-browser rendering is on the WASM roadmap; for now, try one of the baked examples from the picker on the left or browse the gallery.',
        }],
        suggestions: this.suggest(source),
      };
    }
    const svgRes = await fetch(assetUrl(this.base, hit.svgPath));
    if (!svgRes.ok) {
      return { ok: false, diagnostics: [{ severity: 'error', message: `Failed to load ${hit.svgPath}: ${svgRes.status}` }] };
    }
    return { ok: true, svg: await svgRes.text(), match: hit };
  }

  suggest(source) {
    const sigSource = signature(source);
    const scored = this.manifest.examples
      .map((e) => ({ e, score: similarity(sigSource, e) }))
      .sort((a, b) => b.score - a.score)
      .slice(0, 4)
      .map(({ e }) => e);
    return scored;
  }
}

function signature(src) {
  const lines = src.toLowerCase().split('\n').map((l) => l.trim()).filter(Boolean);
  return {
    family: detectFamily(lines),
    tokens: new Set(lines.flatMap((l) => l.split(/[^a-z0-9_]+/).filter(Boolean))),
  };
}

function similarity(sig, ex) {
  let score = 0;
  if (ex.family === sig.family) score += 5;
  const exTokens = new Set((ex.title + ' ' + ex.preview).toLowerCase().split(/\s+/));
  for (const t of exTokens) if (sig.tokens.has(t)) score += 1;
  return score;
}

function detectFamily(lines) {
  for (const l of lines) {
    if (l.startsWith('@start')) {
      const m = l.match(/^@start(\w*)/);
      if (m && m[1]) return m[1];
    }
    if (l.startsWith('sequencediagram')) return 'sequence';
    if (l.startsWith('classdiagram')) return 'class';
    if (l.startsWith('statediagram')) return 'state';
    if (l.startsWith('flowchart') || l.startsWith('graph ')) return 'activity';
  }
  return 'sequence';
}

let view;
let engine;
let manifest;
let base;
const langCompartment = new Compartment();

async function init() {
  base = siteBaseUrl();
  try {
    manifest = await loadManifest(base);
  } catch (e) {
    setStatus('editor', `Failed to load manifest: ${e.message}`, 'bad');
    return;
  }
  engine = new ManifestLookupEngine(manifest, base);
  setStatus('preview', engine.describe(), 'ok');

  // Populate example picker.
  const picker = document.getElementById('example-picker');
  // Group options by family for usability.
  const familyMap = new Map();
  for (const ex of manifest.examples) {
    if (!familyMap.has(ex.family)) familyMap.set(ex.family, []);
    familyMap.get(ex.family).push(ex);
  }
  for (const [fam, items] of [...familyMap.entries()].sort()) {
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
    await openExampleById(id, { fromPicker: true });
    e.target.value = '';
  });

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

  // Build editor.
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

  // Open via ?open=family/name.
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

async function openExampleById(id, opts = {}) {
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
  const source = view.state.doc.toString();
  const previewHost = document.getElementById('preview-host');
  const result = await engine.render(source);
  if (result.ok) {
    previewHost.innerHTML = result.svg;
    setStatus('preview', `Rendered ${result.match.familyLabel} / ${result.match.title} (manifest match).`, 'ok');
  } else {
    const sugs = (result.suggestions || []).slice(0, 4);
    const sugHtml = sugs.length
      ? `<div style="margin-top:10px; font-size:12px;">
           <div style="margin-bottom:6px; color:#4a5285;">Closest baked examples:</div>
           ${sugs.map((s) => `<div><a href="?open=${encodeURIComponent(s.family + '/' + s.name)}">${escapeHtml(s.familyLabel)} / ${escapeHtml(s.title)}</a></div>`).join('')}
         </div>` : '';
    previewHost.innerHTML = `
      <div class="preview-placeholder">
        <span class="pill">no live render yet</span>
        <p>${escapeHtml(result.diagnostics?.[0]?.message ?? 'No render available for this source.')}</p>
        ${sugHtml}
      </div>`;
    setStatus('preview', 'Source does not match any baked example yet. Live render coming with WASM.', 'warn');
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

init();
