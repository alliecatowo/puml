import * as vscode from 'vscode';

export class PumlPreviewPanel {
  private static panel: vscode.WebviewPanel | undefined;

  static async show(context: vscode.ExtensionContext, document: vscode.TextDocument): Promise<void> {
    if (!PumlPreviewPanel.panel) {
      PumlPreviewPanel.panel = vscode.window.createWebviewPanel(
        'pumlPreview',
        'PUML Preview',
        vscode.ViewColumn.Beside,
        { enableScripts: true }
      );

      PumlPreviewPanel.panel.onDidDispose(() => {
        PumlPreviewPanel.panel = undefined;
      });
    }

    PumlPreviewPanel.panel.title = `PUML Preview: ${document.fileName.split('/').pop() ?? 'Untitled'}`;
    PumlPreviewPanel.panel.webview.html = renderWebviewHtml(document.getText());

    const watcher = vscode.workspace.onDidChangeTextDocument((event) => {
      if (event.document.uri.toString() !== document.uri.toString()) {
        return;
      }

      if (PumlPreviewPanel.panel) {
        PumlPreviewPanel.panel.webview.html = renderWebviewHtml(event.document.getText());
      }
    });

    context.subscriptions.push(watcher);
    PumlPreviewPanel.panel.reveal(vscode.ViewColumn.Beside);
  }
}

function renderWebviewHtml(source: string): string {
  const escaped = source
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>PUML Preview</title>
  <style>
    body { font-family: sans-serif; padding: 16px; }
    pre { white-space: pre-wrap; background: #111; color: #ddd; padding: 12px; border-radius: 8px; }
    .banner { margin-bottom: 12px; color: #666; }
  </style>
</head>
<body>
  <div class="banner">WIP preview scaffold. Rendering engine hookup comes next.</div>
  <pre>${escaped}</pre>
</body>
</html>`;
}
