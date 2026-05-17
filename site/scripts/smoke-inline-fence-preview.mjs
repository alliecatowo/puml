#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const siteRoot = resolve(process.argv[2] || 'site');
const publicRoot = join(siteRoot, 'public');
const pagePath = join(publicRoot, 'guide', 'markdown-fences', 'index.html');
const scriptPath = join(publicRoot, 'js', 'inline-fence-preview.js');

function fail(message) {
  console.error(`inline fence preview smoke failed: ${message}`);
  process.exit(1);
}

if (!existsSync(pagePath)) fail(`missing representative page: ${pagePath}`);
if (!existsSync(scriptPath)) fail(`missing hydrated preview script: ${scriptPath}`);

const page = readFileSync(pagePath, 'utf8');
const script = readFileSync(scriptPath, 'utf8');

if (!page.includes('inline-fence-preview.js')) {
  fail('representative page does not include the inline fence preview module hook');
}

if (!/(data-lang=["']puml["']|language-puml)/.test(page)) {
  fail('representative page does not contain a puml code fence for hydration');
}

for (const lang of ['puml', 'plantuml', 'picouml']) {
  if (!script.includes(`'${lang}'`)) {
    fail(`hydrator does not list supported language ${lang}`);
  }
}

if (!script.includes('data-puml-fence-preview')) {
  fail('hydrator does not mark wrapped fences with data-puml-fence-preview');
}

console.log('OK: inline fence preview hydration hook present on guide/markdown-fences');
