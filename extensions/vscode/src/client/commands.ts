import * as vscode from 'vscode';
import { PumlPreviewPanel } from './previewPanel';

export function registerPreviewCommands(context: vscode.ExtensionContext): void {
  const disposable = vscode.commands.registerCommand('puml.preview.open', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to preview.');
      return;
    }

    await PumlPreviewPanel.show(context, editor.document);
  });

  context.subscriptions.push(disposable);
}
