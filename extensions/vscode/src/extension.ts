import * as vscode from 'vscode';
import { registerPreviewCommands } from './client/commands';
import { PumlLspClient } from './client/lspClient';

const lspClient = new PumlLspClient();

export function activate(context: vscode.ExtensionContext): void {
  registerPreviewCommands(context, lspClient);

  if (vscode.workspace.getConfiguration('puml').get<boolean>('lsp.enabled', true)) {
    void lspClient.start(context);
  }
}

export async function deactivate(): Promise<void> {
  await lspClient.stop();
}
