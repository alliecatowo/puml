#!/usr/bin/env node
// Quick smoke test: load the puml-wasm bundle from site/static/wasm and
// render a tiny sequence diagram, prove the binary works end-to-end.

import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { readFileSync } from 'node:fs';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const wasmRoot = path.resolve(__dirname, '..', 'site', 'static', 'wasm');

const mod = await import(path.join(wasmRoot, 'puml_wasm.js'));
const wasmBytes = readFileSync(path.join(wasmRoot, 'puml_wasm_bg.wasm'));
await mod.default({ module_or_path: wasmBytes });

const SOURCE = `@startuml
Alice -> Bob: hi
Bob --> Alice: hello
@enduml
`;

const svg = mod.render_svg(SOURCE);
if (!svg.includes('<svg')) {
  console.error('render_svg did not return an <svg> document');
  process.exit(1);
}
console.log(`OK: render_svg returned ${svg.length} bytes`);

const family = mod.detect_family(SOURCE);
console.log(`OK: detect_family = ${family}`);

const jsonOut = mod.render_svgs_json('not a real diagram');
const parsed = JSON.parse(jsonOut);
if (!parsed.error) {
  console.error('render_svgs_json on bad input should have returned { error }');
  process.exit(1);
}
console.log(`OK: render_svgs_json error path = ${parsed.error.message?.slice(0, 60)}`);
