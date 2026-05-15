import * as vscode from 'vscode';
import { RenderSvgResult } from './lspClient';

export class PumlPreviewPanel {
  private static panel: vscode.WebviewPanel | undefined;

  static async show(document: vscode.TextDocument, payload: RenderSvgResult): Promise<void> {
    if (!PumlPreviewPanel.panel) {
      PumlPreviewPanel.panel = vscode.window.createWebviewPanel(
        'pumlPreview',
        'PUML Preview',
        vscode.ViewColumn.Beside,
        { enableScripts: false }
      );

      PumlPreviewPanel.panel.onDidDispose(() => {
        PumlPreviewPanel.panel = undefined;
      });
    }

    PumlPreviewPanel.panel.title = `PUML Preview: ${document.fileName.split('/').pop() ?? 'Untitled'}`;
    PumlPreviewPanel.panel.webview.html = renderWebviewHtml(payload);
    PumlPreviewPanel.panel.reveal(vscode.ViewColumn.Beside);
  }
}

function renderWebviewHtml(payload: RenderSvgResult): string {
  const safeSvg = payload.svg && payload.svg.trim().length > 0 ? payload.svg : '<svg xmlns="http://www.w3.org/2000/svg"><text x="8" y="20">No preview output.</text></svg>';
  const diag = payload.diagnostics
    .map((d) => d.message)
    .filter((d): d is string => Boolean(d))
    .map(escapeHtml);

  const diagnosticsHtml = diag.length
    ? `<ul>${diag.map((m) => `<li>${m}</li>`).join('')}</ul>`
    : '<p>No diagnostics.</p>';

  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8" />
<meta name="viewport" content="width=device-width, initial-scale=1.0" />
<title>PUML Preview</title>
<style>
  body { margin: 0; font-family: system-ui, sans-serif; background: #fafafa; color: #111; }
  .meta { padding: 10px 14px; border-bottom: 1px solid #ddd; background: #fff; }
  .canvas { padding: 12px; overflow: auto; }
  .diag { padding: 10px 14px; border-top: 1px solid #ddd; background: #fff; font-size: 12px; }
  .canvas svg { max-width: 100%; height: auto; }
</style>
</head>
<body>
  <div class="meta">Rendered via <code>puml-lsp</code> command <code>puml.renderSvg</code>.</div>
  <div class="canvas">${safeSvg}</div>
  <div class="diag"><strong>Diagnostics</strong>${diagnosticsHtml}</div>
</body>
</html>`;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}
