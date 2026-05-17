#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
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

const markdownFencePage = readBuiltPage(markdownFencePagePath, 'Markdown fences');
const statePage = readBuiltPage(statePagePath, 'State guide');

assert(
  existsSync(scriptPath),
  `missing hydrated preview script: ${scriptPath}`,
  'hydrated preview script exists',
  { scriptPath },
);
const script = readFileSync(scriptPath, 'utf8');

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
  ['creates toggle buttons', 'puml-fence-toggle'],
  ['initializes collapsed state', "aria-expanded', 'false'"],
  ['links toggles to panels', 'aria-controls'],
  ['calls the frontend-aware WASM renderer', 'getEngine().render(source, { frontend: lang })'],
  ['inserts rendered SVG pages', 'renderSvgs(panel, result.svgs)'],
]) {
  assertScriptContains(script, needle, `hydrator is missing expected behavior: ${name}`);
}

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
