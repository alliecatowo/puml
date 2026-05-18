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

// ---------------------------------------------------------------------------
// !include pre-processor
// ---------------------------------------------------------------------------
// Base URL for resolving relative include paths (e.g. `!include path/to/file.puml`).
// Falls back to the site's own examples directory which hosts a bundled stdlib.
const INCLUDE_BASE_URL = (() => {
  try {
    // Prefer an explicit override stored by the page (not currently set, but
    // allows future configuration without changing this module).
    return window.__PUML_INCLUDE_BASE__ ||
      new URL('examples/', window.location.origin + (window.__PUML_BASE__ || '/')).toString();
  } catch {
    return 'https://alliecatowo.github.io/puml/examples/';
  }
})();

// Matches !include / !includeurl / !includesub / !include_many lines.
// Group 1: directive keyword (include, includeurl, includesub, include_many)
// Group 2: the path/url token
const INCLUDE_LINE_RE = /^(\s*)!(include(?:url|sub|_many)?)\s+([^\s!][^\s]*)(\s*)$/i;

/**
 * Recursively resolve !include / !includeurl directives by fetching the
 * referenced content and inlining it.  Returns the expanded source string.
 *
 * @param {string} source  PlantUML source that may contain !include lines.
 * @param {number} maxDepth  Maximum recursion depth (default 8).
 * @returns {Promise<{text: string, count: number, errors: string[]}>}
 */
async function resolveIncludes(source, maxDepth = 8) {
  let count = 0;
  const errors = [];

  async function expand(src, depth) {
    if (depth <= 0) return src;
    const lines = src.split('\n');
    const out = [];
    for (const line of lines) {
      const m = line.match(INCLUDE_LINE_RE);
      if (!m) { out.push(line); continue; }
      // m[2] = directive, m[3] = target path/url
      let target = m[3];
      // Strip angle-bracket or double-quote stdlib notation: <C4.puml> or "C4.puml"
      target = target.replace(/^[<"](.+)[>"]$/, '$1');

      let url = target;
      if (!/^https?:\/\//i.test(url) && !url.startsWith('data:')) {
        try {
          url = new URL(target, INCLUDE_BASE_URL).toString();
        } catch {
          url = INCLUDE_BASE_URL + target;
        }
      }

      try {
        const resp = await fetch(url, { mode: 'cors' });
        if (!resp.ok) {
          const msg = `include failed: ${url} (HTTP ${resp.status})`;
          errors.push(msg);
          out.push(`' ${msg}`);
          continue;
        }
        const text = await resp.text();
        // Strip @startuml / @enduml wrappers so the content merges cleanly.
        const stripped = text
          .replace(/^\s*@start\w+[^\n]*\n?/im, '')
          .replace(/\n?\s*@end\w+\s*$/im, '');
        count++;
        const inner = await expand(stripped, depth - 1);
        out.push(`' --- begin include ${target} ---`);
        out.push(inner);
        out.push(`' --- end include ${target} ---`);
      } catch (e) {
        let msg = `include failed: ${url} (${e.message})`;
        // Provide a friendlier hint for CORS failures (TypeError with no message
        // or "Failed to fetch" are both typical for CORS-blocked requests).
        if (!e.message || /failed to fetch|networkerror|cors/i.test(e.message)) {
          msg = `include failed: CORS blocked — ${url} — try a proxy or paste content inline`;
        }
        errors.push(msg);
        out.push(`' ${msg}`);
      }
    }
    return out.join('\n');
  }

  const text = await expand(source, maxDepth);
  return { text, count, errors };
}

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

// Track the source of the currently-selected example so that Reset can restore
// it rather than always falling back to DEFAULT_SOURCE.
let currentExampleSource = null;

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
      // Do NOT reset e.target.value to '' after selection; the native <select>
      // already shows the chosen option's text, which is the desired UX.
      // Resetting it caused the label to snap back to "Load example…"
      // immediately after the user picked an item (bug 3).
      await openExampleById(id);
    });
  }

  document.getElementById('reset-btn').addEventListener('click', () => {
    // Restore to the currently-selected example when one is active; otherwise
    // fall back to the built-in default (bug 2).
    setSource(currentExampleSource ?? DEFAULT_SOURCE);
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

// Debounced auto-render: wait 400 ms after the last keystroke before
// calling render().  If a render is already in flight (renderInFlight is
// true) we don't cancel the timer — we just let the in-flight render
// finish, and the timer queued here will fire afterwards.
let renderTimer = null;
let renderInFlight = false;

function scheduleAutoRender() {
  if (renderTimer) clearTimeout(renderTimer);
  renderTimer = setTimeout(async () => {
    renderTimer = null;
    // If another render is already in progress, re-queue and let it finish
    // first so we always render the latest source without cancelling work.
    if (renderInFlight) {
      scheduleAutoRender();
      return;
    }
    renderInFlight = true;
    try {
      await render();
    } finally {
      renderInFlight = false;
    }
  }, 400);
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
    // Record source so Reset can restore the example (bug 2).
    currentExampleSource = text;
    setSource(text);
    setStatus('editor', `Loaded ${ex.familyLabel} / ${ex.title}.`, 'ok');
    render();
  } catch (e) {
    setStatus('editor', `Failed to load ${id}: ${e.message}`, 'bad', true);
  }
}

async function render() {
  if (!view || !engine) return;
  const rawSource = view.state.doc.toString();
  const previewHost = document.getElementById('preview-host');

  // --- JS-side !include pre-processor ---
  // Detect whether there are any include directives before paying the async cost.
  const hasIncludes = /^!include/im.test(rawSource);
  let source = rawSource;
  let includeCount = 0;
  let includeErrors = [];
  if (hasIncludes) {
    const includeCount_ = (rawSource.match(/^!include/gim) || []).length;
    setStatus('preview', `Resolving ${includeCount_} include(s)…`, 'warn');
    try {
      const resolved = await resolveIncludes(rawSource);
      source = resolved.text;
      includeCount = resolved.count;
      includeErrors = resolved.errors;
    } catch (e) {
      // Should not normally throw, but guard defensively.
      includeErrors.push(`resolveIncludes error: ${e.message}`);
    }
  }

  let result;
  try {
    const frontend = document.getElementById('frontend-picker')?.value || 'auto';
    result = await engine.render(source, { frontend });
  } catch (e) {
    result = { ok: false, diagnostics: [{ severity: 'error', message: e.message || String(e) }] };
  }

  // Build diagnostics for any include resolution errors (shown alongside WASM diags).
  const includeDiags = includeErrors.map((msg) => ({ severity: 'warn', message: msg }));

  if (result.ok) {
    previewHost.innerHTML = result.svgs.join('\n');
    const pages = result.svgs.length;
    let statusMsg = pages > 1 ? `Rendered ${pages} pages.` : 'Rendered.';
    if (includeCount > 0) {
      statusMsg += ` ${includeCount} include(s) resolved.`;
    }
    setStatus('preview', statusMsg, 'ok');
    showDiagnosticsPanel(includeDiags);
    // Show the include-count pill when at least one include resolved.
    showIncludePill(includeCount, includeErrors);
  } else {
    const diag = result.diagnostics?.[0];
    // If the WASM still sees an E_INCLUDE_NOT_SUPPORTED_WASM it means the JS
    // pre-processor couldn't resolve the include (e.g. non-URL relative path
    // with no matching base).  Show a clear, actionable message instead of the
    // old "paste inline" banner — the user now knows fetch is happening.
    if (diag?.message?.includes('E_INCLUDE_NOT_SUPPORTED_WASM')) {
      const allDiags = [
        ...includeDiags,
        { severity: 'warn', message: 'Some !include paths could not be resolved by the browser pre-processor. Try an absolute URL (https://…) or paste the content inline.' },
      ];
      const partialSvgs = result.svgs?.length ? result.svgs : [];
      previewHost.innerHTML = partialSvgs.length ? partialSvgs.join('\n') : `
        <div class="preview-placeholder">
          <span class="pill">partial render</span>
          <p>Some includes could not be fetched. See the diagnostics panel for details.</p>
        </div>`;
      setStatus('preview', 'Some includes unresolved — see diagnostics.', 'warn');
      showDiagnosticsPanel(allDiags);
      showIncludePill(includeCount, includeErrors);
    } else {
      previewHost.innerHTML = `
        <div class="preview-placeholder">
          <span class="pill">render error</span>
          <p>${escapeHtml(diagnosticLabel(diag))}</p>
        </div>`;
      setStatus('preview', diag?.line ? `Render error at line ${diag.line}.` : 'Render error.', 'bad');
      showDiagnosticsPanel([...includeDiags, ...(result.diagnostics || [])]);
      showIncludePill(includeCount, includeErrors);
    }
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

// ---------- Diagnostics sidebar panel ----------

/**
 * Populate (or clear) the diagnostics panel that sits between the editor and
 * the status bar.  Each diagnostic gets a clickable "line N, col M" link that
 * moves the CodeMirror cursor to the reported position.
 *
 * @param {Array<{severity?: string, message?: string, line?: number, column?: number}>} diags
 */
function showDiagnosticsPanel(diags) {
  const panel = document.getElementById('diagnostics-panel');
  if (!panel) return;
  if (!diags || diags.length === 0) {
    panel.hidden = true;
    panel.innerHTML = '';
    return;
  }
  panel.hidden = false;
  panel.innerHTML = diags.map((d) => {
    const sev = d.severity || 'error';
    const msg = escapeHtml(d.message || 'Render failed.');
    let locHtml = '';
    if (d.line) {
      const col = d.column || 1;
      // The link text and aria-label describe the position; clicking it jumps
      // the CM6 cursor to that line/col without scrolling the page.
      locHtml = `<button class="diag-loc" data-line="${d.line}" data-col="${col}" title="Jump to line ${d.line}, col ${col}" aria-label="Jump to line ${d.line}, col ${col}">line ${d.line}${d.column ? `, col ${col}` : ''}</button>`;
    }
    return `<div class="diag-row diag-${sev}">${locHtml}<span class="diag-msg">${msg}</span></div>`;
  }).join('');

  // Wire up jump-to-line buttons.
  panel.querySelectorAll('.diag-loc').forEach((btn) => {
    btn.addEventListener('click', () => {
      if (!view) return;
      const line = parseInt(btn.dataset.line, 10);
      const col = parseInt(btn.dataset.col, 10) || 1;
      jumpToPosition(line, col);
    });
  });
}

/**
 * Move the CodeMirror 6 cursor to the given 1-based line and column, and
 * scroll the line into view.
 */
function jumpToPosition(line, col) {
  if (!view) return;
  const doc = view.state.doc;
  // Clamp to document bounds.
  const lineObj = doc.line(Math.min(line, doc.lines));
  const pos = Math.min(lineObj.from + (col - 1), lineObj.to);
  view.dispatch({
    selection: { anchor: pos },
    scrollIntoView: true,
  });
  view.focus();
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

/**
 * Update (or hide) the include-resolved pill in the preview status bar.
 * @param {number} count  Number of includes successfully fetched.
 * @param {string[]} errors  List of error messages from failed includes.
 */
function showIncludePill(count, errors) {
  const bar = document.getElementById('preview-status');
  if (!bar) return;
  // Remove any existing pill first.
  bar.querySelector('.include-pill')?.remove();
  if (count === 0 && errors.length === 0) return;

  const pill = document.createElement('span');
  pill.className = 'include-pill';
  const hasErrors = errors.length > 0;
  if (count > 0 && !hasErrors) {
    pill.textContent = `🔗 ${count} include(s) resolved`;
    pill.title = `${count} !include directive(s) were fetched and inlined by the browser pre-processor.`;
    pill.classList.add('include-pill-ok');
  } else if (count > 0 && hasErrors) {
    pill.textContent = `🔗 ${count} ok / ${errors.length} failed`;
    pill.title = errors.join('\n');
    pill.classList.add('include-pill-warn');
  } else {
    pill.textContent = `⚠️ ${errors.length} include(s) failed`;
    pill.title = errors.join('\n');
    pill.classList.add('include-pill-warn');
  }
  // Make the pill expandable: clicking shows a tooltip-style popover with URLs.
  pill.setAttribute('role', 'button');
  pill.setAttribute('tabindex', '0');
  pill.addEventListener('click', () => toggleIncludeDetail(pill, count, errors));
  pill.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleIncludeDetail(pill, count, errors); }
  });
  bar.appendChild(pill);
}

function toggleIncludeDetail(pill, count, errors) {
  const existing = pill.querySelector('.include-pill-detail');
  if (existing) { existing.remove(); return; }
  const detail = document.createElement('span');
  detail.className = 'include-pill-detail';
  const lines = [];
  if (count > 0) lines.push(`${count} resolved`);
  errors.forEach((e) => lines.push(`✗ ${e}`));
  detail.textContent = lines.join(' | ');
  pill.appendChild(detail);
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
