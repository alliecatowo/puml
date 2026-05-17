#!/usr/bin/env node
// Build the static site manifest by walking docs/examples/ and pairing
// every .puml source with its rendered .svg artifact.
//
// Output:
//   site/examples/<family>/<name>.{puml,svg}     copies of the corpus
//   site/assets/examples-index.json               manifest consumed by app.js
//
// Run:
//   node scripts/build-site.mjs

import { promises as fs } from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '..');
const examplesSrc = path.join(repoRoot, 'docs', 'examples');
const examplesDst = path.join(repoRoot, 'site', 'static', 'examples');
const manifestPath = path.join(repoRoot, 'site', 'static', 'examples-index.json');

const PRETTY_FAMILY = {
  activity: 'Activity',
  activity_new: 'Activity (new syntax)',
  activity_old: 'Activity (legacy)',
  archimate: 'ArchiMate',
  c4: 'C4',
  chart: 'Chart',
  chronology: 'Chronology',
  class: 'Class',
  component: 'Component',
  creole: 'Creole markup',
  deployment: 'Deployment',
  ditaa: 'Ditaa',
  ebnf: 'EBNF',
  gantt: 'Gantt',
  json: 'JSON',
  math: 'Math',
  mindmap: 'Mindmap',
  nwdiag: 'Network',
  object: 'Object',
  preprocessor: 'Preprocessor',
  regex: 'Regex',
  salt: 'Salt (UI)',
  sdl: 'SDL',
  sequence: 'Sequence',
  skinparams: 'Skinparams',
  state: 'State',
  themes: 'Themes',
  timing: 'Timing',
  usecase: 'Use case',
  wbs: 'WBS',
  yaml: 'YAML',
};

// Normalize source text so editor lookups are stable regardless of trailing
// whitespace differences between docs-examples authoring and editor input.
function normalizeSource(text) {
  return text
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .map((l) => l.replace(/\s+$/, ''))
    .join('\n')
    .replace(/\n+$/, '');
}

function hashSource(text) {
  return crypto.createHash('sha256').update(normalizeSource(text)).digest('hex').slice(0, 16);
}

async function walkFamily(family) {
  const dir = path.join(examplesSrc, family);
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const items = [];
  for (const e of entries) {
    if (!e.isFile() || !e.name.endsWith('.puml')) continue;
    const stem = e.name.slice(0, -'.puml'.length);
    const svgName = `${stem}.svg`;
    const svgPath = path.join(dir, svgName);
    let svgExists = true;
    try {
      await fs.access(svgPath);
    } catch {
      svgExists = false;
    }
    if (!svgExists) continue;
    const puml = await fs.readFile(path.join(dir, e.name), 'utf8');
    items.push({
      family,
      familyLabel: PRETTY_FAMILY[family] ?? family,
      name: stem,
      pumlPath: `examples/${family}/${e.name}`,
      svgPath: `examples/${family}/${svgName}`,
      lineCount: puml.split('\n').length,
      hash: hashSource(puml),
      title: prettyTitle(stem),
      preview: previewLine(puml),
    });
  }
  items.sort((a, b) => a.name.localeCompare(b.name));
  return items;
}

function prettyTitle(stem) {
  return stem
    .replace(/^\d+[_-]?/, '')
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, (c) => c.toUpperCase())
    .trim() || stem;
}

function previewLine(puml) {
  for (const line of puml.split('\n')) {
    const t = line.trim();
    if (!t) continue;
    if (t.startsWith('@start') || t.startsWith("'") || t.startsWith('/*') || t.startsWith('!')) continue;
    return t.length > 80 ? t.slice(0, 77) + '...' : t;
  }
  return '';
}

async function rimrafContents(dir) {
  try {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    for (const e of entries) {
      const p = path.join(dir, e.name);
      if (e.isDirectory()) {
        await fs.rm(p, { recursive: true, force: true });
      } else {
        await fs.unlink(p);
      }
    }
  } catch (err) {
    if (err.code !== 'ENOENT') throw err;
  }
}

async function copyExamples(items) {
  await fs.mkdir(examplesDst, { recursive: true });
  await rimrafContents(examplesDst);
  for (const it of items) {
    const dstPumlDir = path.join(examplesDst, it.family);
    await fs.mkdir(dstPumlDir, { recursive: true });
    await fs.copyFile(path.join(examplesSrc, it.family, `${it.name}.puml`), path.join(dstPumlDir, `${it.name}.puml`));
    await fs.copyFile(path.join(examplesSrc, it.family, `${it.name}.svg`), path.join(dstPumlDir, `${it.name}.svg`));
  }
}

async function main() {
  const families = (await fs.readdir(examplesSrc, { withFileTypes: true }))
    .filter((d) => d.isDirectory())
    .map((d) => d.name)
    .sort();

  const all = [];
  const byFamily = [];
  for (const fam of families) {
    const items = await walkFamily(fam);
    if (!items.length) continue;
    all.push(...items);
    byFamily.push({
      family: fam,
      label: PRETTY_FAMILY[fam] ?? fam,
      count: items.length,
    });
  }

  await copyExamples(all);

  const manifest = {
    generatedAt: new Date().toISOString(),
    totals: {
      examples: all.length,
      families: byFamily.length,
    },
    families: byFamily,
    examples: all,
  };

  await fs.mkdir(path.dirname(manifestPath), { recursive: true });
  await fs.writeFile(manifestPath, JSON.stringify(manifest, null, 2));

  console.log(`Wrote ${manifestPath}`);
  console.log(`  examples: ${all.length}`);
  console.log(`  families: ${byFamily.length}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
