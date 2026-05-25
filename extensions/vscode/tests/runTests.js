/**
 * runTests.js
 *
 * Lightweight integration-style test runner that exercises the extension
 * modules without a full VS Code process. Tests are vanilla Node.js — no
 * framework dependency — so they run quickly in CI with just `node ./tests/runTests.js`.
 *
 * Actual @vscode/test-electron tests require a display; add those in a
 * follow-up wave once CI has an Xvfb step. This file covers the parts that
 * can be unit-tested in isolation: renderer logic, HTML output shape, and
 * smoke-contract invariants.
 */
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');

let passed = 0;
let failed = 0;

function test(name, fn) {
  try {
    fn();
    console.log(`  ✓ ${name}`);
    passed++;
  } catch (err) {
    console.error(`  ✗ ${name}`);
    console.error(`    ${err.message}`);
    failed++;
  }
}

// ---------------------------------------------------------------------------
// 1. Smoke: dist/extension.js was built
// ---------------------------------------------------------------------------
console.log('\n[puml-vscode] tests\n');
console.log('-- build artifact --');

test('dist/extension.js exists after compile', () => {
  const dist = path.join(__dirname, '..', 'dist', 'extension.js');
  assert.ok(fs.existsSync(dist), `Missing ${dist}`);
});

// ---------------------------------------------------------------------------
// 2. Source contract invariants (same as smoke.js, kept here for test suite)
// ---------------------------------------------------------------------------
console.log('-- source contracts --');

const previewSrc = fs.readFileSync(
  path.join(__dirname, '..', 'src', 'client', 'previewPanel.ts'),
  'utf8'
);
const lspSrc = fs.readFileSync(
  path.join(__dirname, '..', 'src', 'client', 'lspClient.ts'),
  'utf8'
);
const rendererSrc = fs.readFileSync(
  path.join(__dirname, '..', 'src', 'client', 'renderer.ts'),
  'utf8'
);
const commandsSrc = fs.readFileSync(
  path.join(__dirname, '..', 'src', 'client', 'commands.ts'),
  'utf8'
);
const exportSrc = fs.readFileSync(
  path.join(__dirname, '..', 'src', 'client', 'exportCommands.ts'),
  'utf8'
);
const statusBarSrc = fs.readFileSync(
  path.join(__dirname, '..', 'src', 'client', 'statusBar.ts'),
  'utf8'
);
const extensionSrc = fs.readFileSync(
  path.join(__dirname, '..', 'src', 'extension.ts'),
  'utf8'
);

test('previewPanel contains puml.renderSvg marker (LSP contract)', () => {
  assert.ok(previewSrc.includes('puml.renderSvg'), 'puml.renderSvg marker missing');
});

test('previewPanel does not contain private parseModel() call', () => {
  assert.ok(!previewSrc.includes('parseModel('), 'Private parser found in preview');
});

test('lspClient contains bundled-binary existence guard', () => {
  assert.ok(lspSrc.includes('fs.existsSync'), 'bundled-binary guard missing');
});

test('lspClient contains PATH fallback for puml-lsp', () => {
  assert.ok(
    lspSrc.includes("return isWindows ? 'puml-lsp.exe' : 'puml-lsp';"),
    'PATH fallback missing'
  );
});

test('lspClient exposes isRunning()', () => {
  assert.ok(lspSrc.includes('isRunning()'), 'isRunning() method missing');
});

// ---------------------------------------------------------------------------
// 3. Live-preview wiring
// ---------------------------------------------------------------------------
console.log('-- live preview --');

test('previewPanel registers onDidChangeTextDocument listener (debounce)', () => {
  assert.ok(
    previewSrc.includes('onDidChangeTextDocument'),
    'onDidChangeTextDocument not found'
  );
});

test('previewPanel registers onDidSaveTextDocument listener (immediate refresh)', () => {
  assert.ok(
    previewSrc.includes('onDidSaveTextDocument'),
    'onDidSaveTextDocument not found'
  );
});

test('previewPanel has stale-response guard (seq counter)', () => {
  assert.ok(previewSrc.includes('this.seq'), 'sequence counter not found');
  assert.ok(previewSrc.includes('mySeq !== this.seq'), 'stale-guard check not found');
});

test('previewPanel shows loading state during render', () => {
  assert.ok(previewSrc.includes('loadingHtml'), 'loadingHtml not found');
});

// ---------------------------------------------------------------------------
// 4. Renderer module
// ---------------------------------------------------------------------------
console.log('-- renderer --');

test('renderer has LSP-first path', () => {
  assert.ok(rendererSrc.includes('lsp.isRunning()'), 'LSP-first check missing');
  assert.ok(rendererSrc.includes('lsp.renderSvg'), 'LSP renderSvg call missing');
});

test('lspClient exposes renderScene/export/explainDiagnostic workspace commands', () => {
  assert.ok(lspSrc.includes("command: 'puml.renderScene'"), 'puml.renderScene command missing');
  assert.ok(lspSrc.includes("command: 'puml.export'"), 'puml.export command missing');
  assert.ok(
    lspSrc.includes("command: 'puml.explainDiagnostic'"),
    'puml.explainDiagnostic command missing'
  );
});

test('renderer has CLI subprocess fallback', () => {
  assert.ok(rendererSrc.includes('cp.spawn'), 'CLI subprocess spawn missing');
});

test('renderer exports exportSvg and exportPng', () => {
  assert.ok(rendererSrc.includes('export async function exportSvg'), 'exportSvg missing');
  assert.ok(rendererSrc.includes('export async function exportPng'), 'exportPng missing');
});

test('renderer can persist LSP export content before CLI fallback', () => {
  assert.ok(rendererSrc.includes('writeLspExportResult'), 'LSP export writer missing');
  assert.ok(rendererSrc.includes('contentBase64'), 'base64 export path missing');
});

test('renderer writes temp file and cleans up', () => {
  assert.ok(rendererSrc.includes('os.tmpdir()'), 'tmpdir() not found');
  assert.ok(rendererSrc.includes('fs.unlinkSync'), 'cleanup not found');
});

// ---------------------------------------------------------------------------
// 5. Export commands
// ---------------------------------------------------------------------------
console.log('-- export commands --');

test('exportCommands registers puml.export.svg', () => {
  assert.ok(exportSrc.includes("'puml.export.svg'"), "puml.export.svg not registered");
});

test('exportCommands registers puml.export.png', () => {
  assert.ok(exportSrc.includes("'puml.export.png'"), "puml.export.png not registered");
});

test('exportCommands guards against non-puml file', () => {
  assert.ok(
    exportSrc.includes("languageId !== 'puml'"),
    'language guard missing in exportCommands'
  );
});

test('exportCommands uses showSaveDialog for path selection', () => {
  assert.ok(exportSrc.includes('showSaveDialog'), 'showSaveDialog not found');
});

// ---------------------------------------------------------------------------
// 6. Check command
// ---------------------------------------------------------------------------
console.log('-- check command --');

test('commands.ts registers puml.check', () => {
  assert.ok(commandsSrc.includes("'puml.check'"), "puml.check not registered");
});

test('commands.ts registers puml.renderScene JSON inspector', () => {
  assert.ok(commandsSrc.includes("'puml.renderScene'"), "puml.renderScene not registered");
  assert.ok(commandsSrc.includes('openTextDocument'), 'renderScene command should open JSON output');
});

test('check command creates VS Code diagnostic collection', () => {
  assert.ok(
    commandsSrc.includes('createDiagnosticCollection'),
    'createDiagnosticCollection missing'
  );
});

test('check command updates status bar', () => {
  assert.ok(
    commandsSrc.includes('PumlStatusBar.update'),
    'PumlStatusBar.update call missing in check command'
  );
});

// ---------------------------------------------------------------------------
// 7. Status bar
// ---------------------------------------------------------------------------
console.log('-- status bar --');

test('statusBar registers and shows on puml files', () => {
  assert.ok(
    statusBarSrc.includes('onDidChangeActiveTextEditor'),
    'active editor listener missing'
  );
  assert.ok(
    statusBarSrc.includes("languageId === 'puml'"),
    'language guard missing in status bar'
  );
});

test('statusBar item clicks open preview', () => {
  assert.ok(
    statusBarSrc.includes("'puml.preview.open'"),
    "status bar command is not puml.preview.open"
  );
});

test('statusBar update() method accepts errorCount + warningCount', () => {
  assert.ok(statusBarSrc.includes('errorCount'), 'errorCount parameter missing');
  assert.ok(statusBarSrc.includes('warningCount'), 'warningCount parameter missing');
});

// ---------------------------------------------------------------------------
// 8. Extension entry point
// ---------------------------------------------------------------------------
console.log('-- extension entry --');

test('extension.ts registers export commands', () => {
  assert.ok(
    extensionSrc.includes('registerExportCommands'),
    'registerExportCommands not called from activate'
  );
});

test('extension.ts registers status bar', () => {
  assert.ok(
    extensionSrc.includes('PumlStatusBar.register'),
    'PumlStatusBar.register not called from activate'
  );
});

// ---------------------------------------------------------------------------
// 9. package.json command declarations
// ---------------------------------------------------------------------------
console.log('-- package.json --');

const pkg = JSON.parse(
  fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8')
);
const commandIds = pkg.contributes.commands.map((c) => c.command);

test('package.json declares puml.preview.open', () => {
  assert.ok(commandIds.includes('puml.preview.open'), 'puml.preview.open missing');
});

test('package.json declares puml.export.svg', () => {
  assert.ok(commandIds.includes('puml.export.svg'), 'puml.export.svg missing');
});

test('package.json declares puml.export.png', () => {
  assert.ok(commandIds.includes('puml.export.png'), 'puml.export.png missing');
});

test('package.json declares puml.check', () => {
  assert.ok(commandIds.includes('puml.check'), 'puml.check missing');
});

test('package.json declares puml.renderScene', () => {
  assert.ok(commandIds.includes('puml.renderScene'), 'puml.renderScene missing');
});

test('package.json declares puml.lsp.restart', () => {
  assert.ok(commandIds.includes('puml.lsp.restart'), 'puml.lsp.restart missing');
});

test('package.json has puml.cli.path configuration', () => {
  const props = pkg.contributes.configuration.properties;
  assert.ok('puml.cli.path' in props, 'puml.cli.path config missing');
});

test('package.json has puml.preview.debounceMs configuration', () => {
  const props = pkg.contributes.configuration.properties;
  assert.ok('puml.preview.debounceMs' in props, 'puml.preview.debounceMs config missing');
});

test('package.json has editor/title menu entry for preview', () => {
  const menus = pkg.contributes.menus;
  assert.ok(
    menus['editor/title'] && menus['editor/title'].length > 0,
    'editor/title menu missing'
  );
});

test('package.json has editor/context entries for puml commands', () => {
  const menus = pkg.contributes.menus;
  assert.ok(
    menus['editor/context'] && menus['editor/context'].length >= 5,
    'editor/context entries missing or incomplete'
  );
});

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------
console.log(`\n${passed + failed} tests: ${passed} passed, ${failed} failed\n`);

if (failed > 0) {
  process.exit(1);
}
