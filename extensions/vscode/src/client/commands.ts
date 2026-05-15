import * as vscode from 'vscode';
import { PumlLspClient } from './lspClient';
import { PumlPreviewPanel } from './previewPanel';

export function registerPreviewCommands(
  context: vscode.ExtensionContext,
  lsp: PumlLspClient
): void {
  const openPreview = vscode.commands.registerCommand('puml.preview.open', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to preview.');
      return;
    }

    await lsp.start(context);
    const uri = editor.document.uri.toString();
    const result = await lsp.renderSvg(uri);
    await PumlPreviewPanel.show(editor.document, result);
  });

  const restartLsp = vscode.commands.registerCommand('puml.lsp.restart', async () => {
    await lsp.restart(context);
    void vscode.window.showInformationMessage('puml-lsp restarted.');
  });

  context.subscriptions.push(openPreview, restartLsp);
}
