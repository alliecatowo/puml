/**
 * runTests.js
 *
 * Lightweight integration-style test runner that exercises the extension
 * modules without a full VS Code process. Tests are vanilla Node.js — no
 * framework dependency — so they run quickly in CI with just `node ./tests/runTests.js`.
 *
 * @vscode/test-electron smoke suite lives in tests/suite/ and is invoked
 * separately when a display is available (see package.json "test:electron").
 * This file covers the parts unit-testable in isolation: renderer logic,
 * HTML output shape, LSP binary resolution logic, and smoke-contract invariants.
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
// 2. Source contract invariants
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
  // The fallback can be expressed as either a return or a const assignment.
  const hasReturn = lspSrc.includes("return isWindows ? 'puml-lsp.exe' : 'puml-lsp';");
  const hasConst = lspSrc.includes("const fallback = isWindows ? 'puml-lsp.exe' : 'puml-lsp'");
  assert.ok(hasReturn || hasConst, 'PATH fallback for puml-lsp missing in lspClient');
});

test('lspClient exposes isRunning()', () => {
  assert.ok(lspSrc.includes('isRunning()'), 'isRunning() method missing');
});

// ---------------------------------------------------------------------------
// 3. LSP binary resolution (bundled → PATH three-tier strategy)
// ---------------------------------------------------------------------------
console.log('-- lsp binary resolution --');

test('lspClient exposes resolveLspBinary (bundled-binary strategy)', () => {
  assert.ok(
    lspSrc.includes('export function resolveLspBinary'),
    'resolveLspBinary not exported'
  );
});

test('lspClient logs binary path to output channel', () => {
  assert.ok(
    lspSrc.includes('getOutputChannel().appendLine'),
    'output channel logging missing in lspClient'
  );
});

test('lspClient exposes getOutputChannel()', () => {
  assert.ok(lspSrc.includes('export function getOutputChannel'), 'getOutputChannel not exported');
});

// ---------------------------------------------------------------------------
// 4. Output channel + LSP restart/backoff
// ---------------------------------------------------------------------------
console.log('-- output channel + restart backoff --');

test('lspClient has exponential-backoff restart (MAX_RESTART_ATTEMPTS)', () => {
  assert.ok(lspSrc.includes('MAX_RESTART_ATTEMPTS'), 'MAX_RESTART_ATTEMPTS constant missing');
  assert.ok(lspSrc.includes('RESTART_BACKOFF_BASE_MS'), 'RESTART_BACKOFF_BASE_MS constant missing');
  assert.ok(lspSrc.includes('Math.pow(2, this.restartAttempts)'), 'exponential-backoff formula missing');
});

test('lspClient has manualRestart that resets backoff counter', () => {
  assert.ok(lspSrc.includes('async manualRestart'), 'manualRestart method missing');
  assert.ok(
    lspSrc.includes('this.restartAttempts = 0'),
    'backoff counter reset missing in manualRestart'
  );
});

test('commands.ts calls manualRestart for puml.lsp.restart', () => {
  assert.ok(
    commandsSrc.includes('lsp.manualRestart'),
    'puml.lsp.restart should use manualRestart (not restart) to reset backoff'
  );
});

test('commands.ts registers puml.lsp.showOutput to reveal output channel', () => {
  assert.ok(
    commandsSrc.includes("'puml.lsp.showOutput'"),
    'puml.lsp.showOutput command missing'
  );
  assert.ok(
    commandsSrc.includes('getOutputChannel'),
    'output channel not used in commands.ts'
  );
});

// ---------------------------------------------------------------------------
// 5. Live-preview wiring
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

test('previewPanel reads debounce delay from configuration (puml.preview.debounceMs)', () => {
  assert.ok(
    previewSrc.includes("'preview.debounceMs'") || previewSrc.includes('"preview.debounceMs"'),
    'puml.preview.debounceMs config key not referenced'
  );
  assert.ok(
    previewSrc.includes('getDebounceMs'),
    'getDebounceMs helper not found'
  );
});

// ---------------------------------------------------------------------------
// 6. Renderer module
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
  assert.ok(
    lspSrc.includes("command: 'puml.languageService'"),
    'puml.languageService command missing'
  );
  assert.ok(
    lspSrc.includes('LanguageServiceSurfaceResult'),
    'language-service surface type missing'
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
// 7. Export commands
// ---------------------------------------------------------------------------
console.log('-- export commands --');

test('exportCommands registers puml.export.svg', () => {
  assert.ok(exportSrc.includes("'puml.export.svg'"), "puml.export.svg not registered");
});

test('exportCommands registers puml.export.png', () => {
  assert.ok(exportSrc.includes("'puml.export.png'"), "puml.export.png not registered");
});

test('exportCommands registers puml.export.source', () => {
  assert.ok(exportSrc.includes("'puml.export.source'"), "puml.export.source not registered");
});

test('exportCommands registers puml.export.json', () => {
  assert.ok(exportSrc.includes("'puml.export.json'"), "puml.export.json not registered");
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

test('exportCommands export.source writes document text directly', () => {
  assert.ok(exportSrc.includes('document.getText()'), 'getText() call missing in exportCommands');
  assert.ok(exportSrc.includes('fs.writeFileSync'), 'writeFileSync missing in exportCommands');
});

test('exportCommands export.json includes family and diagnostics', () => {
  assert.ok(exportSrc.includes("family:"), "family field missing in JSON export payload");
  assert.ok(exportSrc.includes("diagnostics:"), "diagnostics field missing in JSON export payload");
});

// ---------------------------------------------------------------------------
// 8. Check and Show commands
// ---------------------------------------------------------------------------
console.log('-- check and show commands --');

test('commands.ts registers puml.check', () => {
  assert.ok(commandsSrc.includes("'puml.check'"), "puml.check not registered");
});

test('commands.ts registers puml.showSvg', () => {
  assert.ok(commandsSrc.includes("'puml.showSvg'"), "puml.showSvg not registered");
});

test('commands.ts showSvg opens SVG source in a text document', () => {
  assert.ok(
    commandsSrc.includes('openTextDocument'),
    'showSvg command should open SVG in a text document'
  );
});

test('commands.ts registers puml.showDiagnostics', () => {
  assert.ok(commandsSrc.includes("'puml.showDiagnostics'"), "puml.showDiagnostics not registered");
});

test('commands.ts showDiagnostics focuses Problems panel', () => {
  assert.ok(
    commandsSrc.includes('workbench.action.problems.focus'),
    'showDiagnostics should focus the Problems panel'
  );
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
// 9. Status bar
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
// 10. Extension entry point
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
// 11. package.json command declarations
// ---------------------------------------------------------------------------
console.log('-- package.json --');

const pkg = JSON.parse(
  fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8')
);
const commandIds = pkg.contributes.commands.map((c) => c.command);

test('package.json declares puml.preview.open', () => {
  assert.ok(commandIds.includes('puml.preview.open'), 'puml.preview.open missing');
});

test('package.json declares puml.showSvg', () => {
  assert.ok(commandIds.includes('puml.showSvg'), 'puml.showSvg missing');
});

test('package.json declares puml.showDiagnostics', () => {
  assert.ok(commandIds.includes('puml.showDiagnostics'), 'puml.showDiagnostics missing');
});

test('package.json declares puml.export.svg', () => {
  assert.ok(commandIds.includes('puml.export.svg'), 'puml.export.svg missing');
});

test('package.json declares puml.export.png', () => {
  assert.ok(commandIds.includes('puml.export.png'), 'puml.export.png missing');
});

test('package.json declares puml.export.source', () => {
  assert.ok(commandIds.includes('puml.export.source'), 'puml.export.source missing');
});

test('package.json declares puml.export.json', () => {
  assert.ok(commandIds.includes('puml.export.json'), 'puml.export.json missing');
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

test('package.json declares puml.lsp.showOutput', () => {
  assert.ok(commandIds.includes('puml.lsp.showOutput'), 'puml.lsp.showOutput missing');
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

test('package.json has editor/context entries for all puml commands (>= 9)', () => {
  const menus = pkg.contributes.menus;
  assert.ok(
    menus['editor/context'] && menus['editor/context'].length >= 9,
    `editor/context entries missing or incomplete (got ${menus['editor/context']?.length ?? 0}, expected >= 9)`
  );
});

// ---------------------------------------------------------------------------
// 12. Activation events
// ---------------------------------------------------------------------------
console.log('-- activation events --');

test('package.json activation includes puml.showSvg', () => {
  assert.ok(
    pkg.activationEvents.includes('onCommand:puml.showSvg'),
    'puml.showSvg activation event missing'
  );
});

test('package.json activation includes puml.export.source', () => {
  assert.ok(
    pkg.activationEvents.includes('onCommand:puml.export.source'),
    'puml.export.source activation event missing'
  );
});

test('package.json activation includes puml.export.json', () => {
  assert.ok(
    pkg.activationEvents.includes('onCommand:puml.export.json'),
    'puml.export.json activation event missing'
  );
});

test('package.json activation includes puml.lsp.showOutput', () => {
  assert.ok(
    pkg.activationEvents.includes('onCommand:puml.lsp.showOutput'),
    'puml.lsp.showOutput activation event missing'
  );
});

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------
console.log(`\n${passed + failed} tests: ${passed} passed, ${failed} failed\n`);

if (failed > 0) {
  process.exit(1);
}
