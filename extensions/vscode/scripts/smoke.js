const fs = require('node:fs');
const path = require('node:path');

const dist = path.join(__dirname, '..', 'dist', 'extension.js');
if (!fs.existsSync(dist)) {
  throw new Error('Missing dist/extension.js after build');
}

const srcPreview = fs.readFileSync(path.join(__dirname, '..', 'src', 'client', 'previewPanel.ts'), 'utf8');
if (!srcPreview.includes('puml.renderSvg')) {
  throw new Error('Preview panel contract marker missing: puml.renderSvg');
}
if (srcPreview.includes('parseModel(')) {
  throw new Error('Found private parser code in preview panel; scaffold must stay LSP-backed');
}

console.log('[vscode-smoke] build artifact exists and preview is LSP-backed');
