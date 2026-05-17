#!/usr/bin/env node
// Quick smoke test: load the puml-wasm bundle and prove the browser-facing
// frontend entry points work end-to-end.

import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { readFileSync } from 'node:fs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const CASES = [
  {
    name: 'plantuml sequence',
    frontend: 'plantuml',
    family: 'sequence',
    contains: 'hello',
    source: `@startuml
Alice -> Bob: hi
Bob --> Alice: hello
@enduml
`,
  },
  {
    name: 'picouml sequence',
    frontend: 'picouml',
    family: 'sequence',
    contains: 'request',
    source: `@startpicouml
Alice => Bob : request
Bob --> Alice : response
@endpicouml
`,
  },
  {
    name: 'mermaid class',
    frontend: 'mermaid',
    family: 'class',
    contains: 'User',
    source: `classDiagram
class User
User : +id
`,
  },
  {
    name: 'plantuml state non-sequence',
    frontend: 'plantuml',
    family: 'state',
    contains: 'Idle',
    source: `@startuml
[*] --> Idle
Idle --> Running : start
@enduml
`,
  },
];

const DIAGNOSTIC_CASE = {
  name: 'deterministic diagnostic',
  frontend: 'mermaid',
  source: `journey
section Unsupported
`,
};

export function defaultWasmRoot() {
  return path.resolve(__dirname, '..', 'site', 'static', 'wasm');
}

export async function runWasmSmoke(wasmRoot = defaultWasmRoot()) {
  const root = path.resolve(wasmRoot);
  const mod = await import(pathToFileURL(path.join(root, 'puml_wasm.js')).href);
  const wasmBytes = readFileSync(path.join(root, 'puml_wasm_bg.wasm'));
  await mod.default({ module_or_path: wasmBytes });

  const results = [];

  for (const testCase of CASES) {
    const json = renderJson(mod, testCase.source, testCase.frontend);
    const parsed = parseJson(json, testCase.name);
    const pages = Array.isArray(parsed.ok) ? parsed.ok : [];
    if (!pages.length) {
      throw new Error(`${testCase.name}: expected at least one rendered SVG page`);
    }
    for (const [index, svg] of pages.entries()) {
      if (!isNonEmptySvg(svg)) {
        throw new Error(`${testCase.name}: page ${index + 1} is not a non-empty SVG`);
      }
    }
    if (!pages.some((svg) => svg.includes(testCase.contains))) {
      throw new Error(`${testCase.name}: SVG did not contain expected text ${testCase.contains}`);
    }

    const family = mod.detect_family_with_frontend
      ? mod.detect_family_with_frontend(testCase.source, testCase.frontend)
      : mod.detect_family(testCase.source);
    if (family !== testCase.family) {
      throw new Error(`${testCase.name}: expected family ${testCase.family}, got ${family}`);
    }

    const byteLength = pages.reduce((sum, svg) => sum + svg.length, 0);
    results.push({
      name: testCase.name,
      ok: true,
      frontend: testCase.frontend,
      family,
      pages: pages.length,
      bytes: byteLength,
    });
  }

  const firstDiagnostic = diagnosticResult(mod, DIAGNOSTIC_CASE);
  const secondDiagnostic = diagnosticResult(mod, DIAGNOSTIC_CASE);
  if (JSON.stringify(firstDiagnostic) !== JSON.stringify(secondDiagnostic)) {
    throw new Error(`${DIAGNOSTIC_CASE.name}: diagnostic output was not deterministic`);
  }
  if (!firstDiagnostic.message) {
    throw new Error(`${DIAGNOSTIC_CASE.name}: expected diagnostic message`);
  }
  results.push({
    name: DIAGNOSTIC_CASE.name,
    ok: true,
    frontend: DIAGNOSTIC_CASE.frontend,
    diagnostic: firstDiagnostic,
  });

  return results;
}

function renderJson(mod, source, frontend) {
  if (mod.render_svgs_json_with_frontend) {
    return mod.render_svgs_json_with_frontend(source, frontend);
  }
  return mod.render_svgs_json(source);
}

function parseJson(json, label) {
  try {
    return JSON.parse(json);
  } catch (e) {
    throw new Error(`${label}: renderer returned invalid JSON: ${e.message}`);
  }
}

function isNonEmptySvg(svg) {
  return typeof svg === 'string' && svg.includes('<svg') && svg.includes('</svg>') && svg.length > 100;
}

function diagnosticResult(mod, testCase) {
  const parsed = parseJson(renderJson(mod, testCase.source, testCase.frontend), testCase.name);
  if (!parsed.error) {
    throw new Error(`${testCase.name}: expected an error diagnostic`);
  }
  const error = parsed.error;
  return {
    code: error.code || '',
    severity: error.severity || 'error',
    line: error.line || null,
    column: error.column || null,
    message: error.message || '',
  };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const wasmRoot = process.argv[2] || defaultWasmRoot();
  try {
    const results = await runWasmSmoke(wasmRoot);
    for (const result of results) {
      if (result.diagnostic) {
        console.log(`OK: ${result.name} (${result.diagnostic.severity}: ${result.diagnostic.message})`);
      } else {
        console.log(
          `OK: ${result.name} frontend=${result.frontend} family=${result.family} pages=${result.pages} bytes=${result.bytes}`,
        );
      }
    }
  } catch (e) {
    console.error(`wasm smoke failed: ${e.message || e}`);
    process.exit(1);
  }
}
