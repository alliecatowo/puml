#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import vm from 'node:vm';
import { runWasmSmoke } from '../../scripts/wasm-smoke.mjs';

const args = process.argv.slice(2);
const requireWasm = args.includes('--require-wasm');
const liveWasm = requireWasm || args.includes('--live-wasm');
const siteArg = args.find((arg) => !arg.startsWith('--')) || 'site';
const siteRoot = resolve(siteArg);
const publicRoot = join(siteRoot, 'public');
const markdownFencePagePath = join(publicRoot, 'guide', 'markdown-fences', 'index.html');
const statePagePath = join(publicRoot, 'guide', 'state', 'index.html');
const scriptPath = join(publicRoot, 'js', 'inline-fence-preview.js');
const stylesheetPath = join(publicRoot, 'style.css');
const wasmJsPath = join(publicRoot, 'wasm', 'puml_wasm.js');
const wasmPath = join(publicRoot, 'wasm', 'puml_wasm_bg.wasm');
const reportPath = resolve('target', 'site-smoke', 'inline-fence-preview.json');
const checks = [];

function fail(message) {
  writeReport(false, message);
  console.error(`inline fence preview smoke failed: ${message}`);
  process.exit(1);
}

function pass(name, detail = {}) {
  checks.push({ name, ok: true, ...detail });
}

function assert(condition, message, name = message, detail = {}) {
  if (!condition) fail(message);
  pass(name, detail);
}

function writeReport(ok, message = '') {
  mkdirSync(resolve('target', 'site-smoke'), { recursive: true });
  writeFileSync(
    reportPath,
    JSON.stringify(
      {
        ok,
        message,
        siteRoot,
        publicRoot,
        requireWasm,
        liveWasm,
        checks,
      },
      null,
      2,
    ),
  );
}

function readBuiltPage(pagePath, label) {
  assert(
    existsSync(pagePath),
    `missing representative ${label} page: ${pagePath}`,
    `${label}: page exists`,
    { pagePath },
  );
  return readFileSync(pagePath, 'utf8');
}

function fenceCount(page, lang) {
  const escaped = lang.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const re = new RegExp(`(?:data-lang=["']${escaped}["']|language-${escaped}\\b)`, 'g');
  return page.match(re)?.length ?? 0;
}

function assertScriptContains(script, needle, message) {
  assert(script.includes(needle), message, `script contains ${needle}`, { needle });
}

function assertStylesheetContains(stylesheet, needle, message) {
  assertStylesheetContainsAny(stylesheet, [needle], message, `stylesheet contains ${needle}`);
}

function assertStylesheetContainsAny(stylesheet, needles, message, name = `stylesheet contains ${needles[0]}`) {
  const compactStylesheet = stylesheet.replace(/\s+/g, '');
  const compactNeedles = needles.map((needle) => needle.replace(/\s+/g, ''));
  assert(
    compactNeedles.some((needle) => compactStylesheet.includes(needle)),
    message,
    name,
    { needles },
  );
}

class SmokeClassList {
  constructor(element) {
    this.element = element;
    this.names = new Set();
  }

  add(...names) {
    for (const name of names) this.names.add(name);
    this.sync();
  }

  remove(...names) {
    for (const name of names) this.names.delete(name);
    this.sync();
  }

  contains(name) {
    return this.names.has(name);
  }

  [Symbol.iterator]() {
    return this.names[Symbol.iterator]();
  }

  sync() {
    this.element._className = [...this.names].join(' ');
  }

  setFromString(value) {
    this.names = new Set(String(value || '').split(/\s+/).filter(Boolean));
    this.sync();
  }
}

class SmokeElement {
  constructor(tagName) {
    this.tagName = tagName.toUpperCase();
    this.children = [];
    this.parentNode = null;
    this.dataset = {};
    this.attributes = new Map();
    this.listeners = new Map();
    this.classList = new SmokeClassList(this);
    this._className = '';
    this._textContent = '';
    this.hidden = false;
  }

  get className() {
    return this._className;
  }

  set className(value) {
    this.classList.setFromString(value);
  }

  get textContent() {
    if (this.children.length) return this.children.map((child) => child.textContent).join('');
    return this._textContent;
  }

  set textContent(value) {
    this.children = [];
    this._textContent = String(value ?? '');
  }

  get innerHTML() {
    return this._textContent;
  }

  set innerHTML(value) {
    this.children = [];
    this._textContent = String(value ?? '');
  }

  set id(value) {
    this.setAttribute('id', value);
  }

  get id() {
    return this.getAttribute('id') || '';
  }

  append(...nodes) {
    for (const node of nodes) this.appendChild(node);
  }

  appendChild(node) {
    if (node.parentNode) {
      const existingIndex = node.parentNode.children.indexOf(node);
      if (existingIndex !== -1) node.parentNode.children.splice(existingIndex, 1);
    }
    node.parentNode = this;
    this.children.push(node);
    return node;
  }

  insertBefore(node, before) {
    if (node.parentNode) {
      const existingIndex = node.parentNode.children.indexOf(node);
      if (existingIndex !== -1) node.parentNode.children.splice(existingIndex, 1);
    }
    node.parentNode = this;
    const index = this.children.indexOf(before);
    if (index === -1) {
      this.children.push(node);
    } else {
      this.children.splice(index, 0, node);
    }
    return node;
  }

  replaceChildren(...nodes) {
    this.children = [];
    this._textContent = '';
    this.append(...nodes);
  }

  setAttribute(name, value) {
    this.attributes.set(name, String(value));
    if (name.startsWith('data-')) this.dataset[toDatasetKey(name.slice(5))] = String(value);
  }

  getAttribute(name) {
    return this.attributes.get(name) ?? null;
  }

  removeAttribute(name) {
    this.attributes.delete(name);
    if (name.startsWith('data-')) delete this.dataset[toDatasetKey(name.slice(5))];
  }

  addEventListener(type, listener) {
    this.listeners.set(type, listener);
  }

  async click() {
    const listener = this.listeners.get('click');
    if (listener) await listener({ currentTarget: this, preventDefault() {} });
  }

  closest(selector) {
    let node = this;
    while (node) {
      if (matchesSelector(node, selector)) return node;
      node = node.parentNode;
    }
    return null;
  }

  querySelector(selector) {
    if (selector === 'span:last-child') {
      const spans = allDescendants(this).filter((node) => node.tagName === 'SPAN');
      return spans.at(-1) || null;
    }
    return allDescendants(this).find((node) => matchesSelector(node, selector)) || null;
  }

  querySelectorAll(selector) {
    return allDescendants(this).filter((node) => matchesSelector(node, selector));
  }
}

class SmokeDocument extends SmokeElement {
  constructor() {
    super('#document');
    this.readyState = 'complete';
  }

  createElement(tagName) {
    return new SmokeElement(tagName);
  }

  addEventListener() {}
}

function allDescendants(root) {
  const nodes = [];
  for (const child of root.children) {
    nodes.push(child, ...allDescendants(child));
  }
  return nodes;
}

function matchesSelector(node, selector) {
  if (selector === 'code') return node.tagName === 'CODE';
  if (selector === 'button') return node.tagName === 'BUTTON';
  if (selector === '.prose pre') return node.tagName === 'PRE' && Boolean(node.closest('.prose'));
  if (selector.startsWith('.')) return node.classList.contains(selector.slice(1));
  if (selector === '[data-puml-fence-preview]') return Object.hasOwn(node.dataset, 'pumlFencePreview');
  return false;
}

function toDatasetKey(name) {
  return name.replace(/-([a-z])/g, (_, char) => char.toUpperCase());
}

async function assertInteractiveHydration(script) {
  const document = new SmokeDocument();
  const prose = document.createElement('main');
  prose.className = 'prose';
  const pre = document.createElement('pre');
  const code = document.createElement('code');
  code.className = 'language-picouml';
  code.textContent = '@startpicouml\nAlice => Bob : request\n@endpicouml\n';
  pre.appendChild(code);
  prose.appendChild(pre);
  document.appendChild(prose);

  const renderCalls = [];
  class SmokeRenderer {
    constructor(base) {
      this.base = base;
    }

    async render(source, options) {
      renderCalls.push({ source, options, base: this.base });
      return { ok: true, svgs: ['<svg role="img"><text>request</text></svg>'] };
    }
  }

  const context = vm.createContext({
    document,
    siteBaseUrl: () => '/docs/',
    WasmRenderer: SmokeRenderer,
    diagnosticLabel: (diag) => diag?.message || 'Render failed.',
  });
  const executable = script
    .replace(/import[^\n]+;\n/g, '')
    .replace('export function hydrateInlineFencePreviews', 'function hydrateInlineFencePreviews');
  vm.runInContext(`${executable}\nthis.hydrateInlineFencePreviews = hydrateInlineFencePreviews;`, context);
  context.hydrateInlineFencePreviews(document);

  const wrapper = document.querySelector('.puml-fence-preview');
  assert(wrapper, 'hydrator did not wrap a supported Markdown fence', 'interactive hydration: wrapper created');
  assert(wrapper.dataset.lang === 'picouml', 'hydrator did not preserve the fence language', 'interactive hydration: language captured');

  const button = wrapper.querySelector('button');
  const panel = wrapper.querySelector('.puml-fence-render');
  assert(button && panel, 'hydrator did not create a graph toggle and render panel', 'interactive hydration: controls created');
  assert(panel.hidden, 'render panel should start collapsed', 'interactive hydration: starts collapsed');
  assert(button.getAttribute('aria-controls') === panel.id, 'graph toggle is not linked to its render panel', 'interactive hydration: aria-controls linked');

  await button.click();

  assert(!panel.hidden, 'clicking the graph toggle did not reveal the render panel', 'interactive hydration: click opens panel');
  assert(renderCalls.length === 1, `expected one renderer call, saw ${renderCalls.length}`, 'interactive hydration: renderer called once');
  assert(renderCalls[0].options.frontend === 'picouml', 'renderer was not called with the code fence language', 'interactive hydration: frontend passed');
  assert(renderCalls[0].source.includes('Alice => Bob'), 'renderer did not receive the fence source', 'interactive hydration: source passed');
  assert(button.dataset.renderState === 'ready', 'toggle did not report ready after a successful render', 'interactive hydration: ready state');
  assert(wrapper.dataset.rendered === 'true', 'wrapper was not marked as rendered after success', 'interactive hydration: rendered marker');
  assert(panel.querySelector('.puml-fence-pages')?.innerHTML.includes('<svg'), 'rendered SVG was not inserted into the panel', 'interactive hydration: SVG inserted');

  await button.click();
  assert(panel.hidden, 'second click did not collapse the render panel', 'interactive hydration: click closes panel');
}

function assertSyntaxHighlightingWithPreWrappedMarkup(script) {
  const document = new SmokeDocument();
  const prose = document.createElement('main');
  prose.className = 'prose';
  const pre = document.createElement('pre');
  pre.dataset.lang = 'puml';
  const code = document.createElement('code');
  code.textContent = '@startuml\nAlice -> Bob : hello\n@enduml\n';

  const staleSpan = document.createElement('span');
  staleSpan.className = 'zola-stale-token';
  staleSpan.textContent = code.textContent;
  code.appendChild(staleSpan);
  pre.appendChild(code);
  prose.appendChild(pre);
  document.appendChild(prose);

  const context = vm.createContext({
    document,
    PUML_HIGHLIGHT_LANGS: new Set(['puml', 'plantuml', 'picouml']),
    highlightPumlToHtml: (source) => `<span class="tok-directive">@startuml</span>\n${source}`,
    siteBaseUrl: () => '/docs/',
    WasmRenderer: class {
      async render() {
        return { ok: true, svgs: ['<svg role="img"></svg>'] };
      }
    },
    diagnosticLabel: (diag) => diag?.message || 'Render failed.',
  });
  const executable = script
    .replace(/import[^\n]+;\n/g, '')
    .replace('export function hydrateInlineFencePreviews', 'function hydrateInlineFencePreviews');
  vm.runInContext(`${executable}\nthis.hydrateInlineFencePreviews = hydrateInlineFencePreviews;`, context);
  context.hydrateInlineFencePreviews(document);

  assert(
    code.dataset.pumlHighlighted === 'true',
    'supported PUML fences with pre-existing child markup were not highlighted',
    'interactive highlighting: marks fence as highlighted',
  );
  assert(
    code.innerHTML.includes('tok-directive'),
    'supported PUML fences with pre-existing child markup did not receive PUML token markup',
    'interactive highlighting: replaces stale child markup',
  );
}

const markdownFencePage = readBuiltPage(markdownFencePagePath, 'Markdown fences');
const statePage = readBuiltPage(statePagePath, 'State guide');

assert(
  existsSync(scriptPath),
  `missing hydrated preview script: ${scriptPath}`,
  'hydrated preview script exists',
  { scriptPath },
);
const script = readFileSync(scriptPath, 'utf8');
assert(
  existsSync(stylesheetPath),
  `missing site stylesheet: ${stylesheetPath}`,
  'site stylesheet exists',
  { stylesheetPath },
);
const stylesheet = readFileSync(stylesheetPath, 'utf8');

for (const [label, page] of [
  ['Markdown fences', markdownFencePage],
  ['State guide', statePage],
]) {
  assert(
    page.includes('inline-fence-preview.js'),
    `${label} page does not include the inline fence preview module hook`,
    `${label}: includes preview module`,
  );
}

const pumlFenceTotal = fenceCount(markdownFencePage, 'puml') + fenceCount(statePage, 'puml');
assert(
  pumlFenceTotal >= 2,
  `expected at least two generated puml graph fences across representative pages, found ${pumlFenceTotal}`,
  'generated puml graph fences',
  { count: pumlFenceTotal },
);

const mermaidFenceTotal = fenceCount(statePage, 'mermaid');
assert(
  mermaidFenceTotal >= 1 && statePage.includes('stateDiagram-v2'),
  'state guide does not contain the generated Mermaid graph block used to smoke static graph rendering',
  'generated Mermaid graph block',
  { count: mermaidFenceTotal },
);

for (const lang of ['puml', 'plantuml', 'picouml']) {
  assert(
    script.includes(`'${lang}'`),
    `hydrator does not list supported language ${lang}`,
    `hydrator supports ${lang}`,
    { lang },
  );
}

for (const [name, needle] of [
  ['marks hydrated wrappers', 'data-puml-fence-preview'],
  ['creates compact graph bubble buttons', 'puml-fence-bubble'],
  ['shows fence language labels', 'puml-fence-lang'],
  ['initializes collapsed state', "aria-expanded', 'false'"],
  ['sets accessible graph labels', "aria-label', `Show rendered ${lang} graph`"],
  ['initializes unpressed toggle state', "aria-pressed', 'false'"],
  ['marks loading render state', "button.dataset.renderState = 'loading'"],
  ['marks ready render state', "button.dataset.renderState = 'ready'"],
  ['marks failed render state', "button.dataset.renderState = 'error'"],
  ['marks successfully rendered wrappers', "wrapper.dataset.rendered = 'true'"],
  ['links toggles to panels', 'aria-controls'],
  ['calls the frontend-aware WASM renderer', 'getEngine().render(source, { frontend: lang })'],
  ['inserts rendered SVG pages', 'renderSvgs(panel, result.svgs)'],
]) {
  assertScriptContains(script, needle, `hydrator is missing expected behavior: ${name}`);
}

await assertInteractiveHydration(script);
assertSyntaxHighlightingWithPreWrappedMarkup(script);

for (const [name, needle] of [
  ['styles graph bubble', '.puml-fence-bubble'],
  ['keeps graph button compact', 'white-space: nowrap'],
  ['stacks preview below source on smaller screens', '.puml-fence-preview.is-open .puml-fence-body { grid-template-columns: 1fr'],
]) {
  assertStylesheetContains(
    stylesheet,
    needle,
    `stylesheet is missing expected inline graph preview behavior: ${name}`,
  );
}

assertStylesheetContainsAny(
  stylesheet,
  [
    'grid-template-columns: minmax(0, 1fr) minmax(300px, 0.92fr)',
    'grid-template-columns: minmax(0, 1fr) minmax(300px, .92fr)',
  ],
  'stylesheet is missing expected inline graph preview behavior: lays out open preview beside source on desktop',
  'stylesheet contains desktop side-by-side preview grid',
);

if (requireWasm) {
  assert(existsSync(wasmJsPath), `missing built WASM JS bundle: ${wasmJsPath}`, 'WASM JS bundle exists', { wasmJsPath });
  assert(existsSync(wasmPath), `missing built WASM binary: ${wasmPath}`, 'WASM binary exists', { wasmPath });
}

if (liveWasm) {
  assert(existsSync(wasmJsPath), `missing built WASM JS bundle for live smoke: ${wasmJsPath}`, 'live WASM JS bundle exists', { wasmJsPath });
  assert(existsSync(wasmPath), `missing built WASM binary for live smoke: ${wasmPath}`, 'live WASM binary exists', { wasmPath });

  try {
    const results = await runWasmSmoke(join(publicRoot, 'wasm'));
    for (const result of results) {
      const { name, ...detail } = result;
      pass(`live WASM renderer: ${name}`, detail);
    }
  } catch (e) {
    fail(`live WASM renderer failed: ${e.message || e}`);
  }
}

writeReport(true);
console.log(`OK: inline fence preview hydration hook and graph blocks present (${reportPath})`);
