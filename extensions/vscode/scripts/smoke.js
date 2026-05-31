const fs = require('node:fs');
const path = require('node:path');

const dist = path.join(__dirname, '..', 'dist', 'extension.js');
if (!fs.existsSync(dist)) {
  throw new Error('Missing dist/extension.js after build');
}

const srcPreview = fs.readFileSync(path.join(__dirname, '..', 'src', 'client', 'previewPanel.ts'), 'utf8');
const srcLspClient = fs.readFileSync(path.join(__dirname, '..', 'src', 'client', 'lspClient.ts'), 'utf8');
const srcCommands = fs.readFileSync(path.join(__dirname, '..', 'src', 'client', 'commands.ts'), 'utf8');
const srcExport = fs.readFileSync(path.join(__dirname, '..', 'src', 'client', 'exportCommands.ts'), 'utf8');
const pkg = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'package.json'), 'utf8'));
const commandIds = pkg.contributes.commands.map((c) => c.command);

// --- Preview panel ---
if (!srcPreview.includes('puml.renderSvg')) {
  throw new Error('Preview panel contract marker missing: puml.renderSvg');
}
if (srcPreview.includes('parseModel(')) {
  throw new Error('Found private parser code in preview panel; scaffold must stay LSP-backed');
}
if (!srcPreview.includes('getDebounceMs')) {
  throw new Error('Preview panel must read debounce from config via getDebounceMs()');
}

// --- LSP client ---
if (!srcLspClient.includes('fs.existsSync')) {
  throw new Error('LSP client contract drift: expected bundled-binary existence guard');
}
const hasLspPathReturn = srcLspClient.includes("return isWindows ? 'puml-lsp.exe' : 'puml-lsp';");
const hasLspPathConst = srcLspClient.includes("const fallback = isWindows ? 'puml-lsp.exe' : 'puml-lsp'");
if (!hasLspPathReturn && !hasLspPathConst) {
  throw new Error('LSP client contract drift: expected PATH fallback for puml-lsp');
}
if (!srcLspClient.includes("command: 'puml.renderScene'")) {
  throw new Error('LSP client contract drift: expected renderScene workspace command');
}
if (!srcLspClient.includes("command: 'puml.export'")) {
  throw new Error('LSP client contract drift: expected export workspace command');
}
if (!srcLspClient.includes("command: 'puml.languageService'")) {
  throw new Error('LSP client contract drift: expected languageService workspace command');
}
if (!srcLspClient.includes('MAX_RESTART_ATTEMPTS')) {
  throw new Error('LSP client contract drift: expected exponential-backoff restart (MAX_RESTART_ATTEMPTS)');
}
if (!srcLspClient.includes('export function getOutputChannel')) {
  throw new Error('LSP client contract drift: expected exported getOutputChannel()');
}

// --- Commands ---
if (!srcCommands.includes("'puml.showSvg'")) {
  throw new Error('Commands contract drift: puml.showSvg missing');
}
if (!srcCommands.includes("'puml.showDiagnostics'")) {
  throw new Error('Commands contract drift: puml.showDiagnostics missing');
}
if (!srcCommands.includes("'puml.lsp.showOutput'")) {
  throw new Error('Commands contract drift: puml.lsp.showOutput missing');
}
if (!srcCommands.includes('lsp.manualRestart')) {
  throw new Error('Commands contract drift: puml.lsp.restart must use manualRestart()');
}

// --- Export commands ---
if (!srcExport.includes("'puml.export.source'")) {
  throw new Error('Export commands contract drift: puml.export.source missing');
}
if (!srcExport.includes("'puml.export.json'")) {
  throw new Error('Export commands contract drift: puml.export.json missing');
}

// --- package.json command declarations ---
const requiredCommands = [
  'puml.preview.open', 'puml.showSvg', 'puml.showDiagnostics',
  'puml.export.svg', 'puml.export.png', 'puml.export.source', 'puml.export.json',
  'puml.check', 'puml.renderScene', 'puml.lsp.restart', 'puml.lsp.showOutput',
];
for (const cmd of requiredCommands) {
  if (!commandIds.includes(cmd)) {
    throw new Error(`package.json missing command declaration: ${cmd}`);
  }
}

console.log('[vscode-smoke] build artifact exists and preview is LSP-backed');
console.log('[vscode-smoke] all command declarations verified');
console.log('[vscode-smoke] output channel + backoff + binary resolution contracts verified');
