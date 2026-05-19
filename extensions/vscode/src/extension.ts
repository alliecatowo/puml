import * as vscode from 'vscode';
import { registerPreviewCommands } from './client/commands';
import { registerExportCommands } from './client/exportCommands';
import { PumlLspClient } from './client/lspClient';
import { PumlStatusBar } from './client/statusBar';

const lspClient = new PumlLspClient();

export function activate(context: vscode.ExtensionContext): void {
  // Core preview + check commands.
  registerPreviewCommands(context, lspClient);

  // Export commands (SVG / PNG).
  registerExportCommands(context);

  // Status bar item (shows diagram family + diagnostic count).
  PumlStatusBar.register(context);

  // Start the language server if enabled.
  if (vscode.workspace.getConfiguration('puml').get<boolean>('lsp.enabled', true)) {
    void lspClient.start(context);
  }
}

export async function deactivate(): Promise<void> {
  await lspClient.stop();
}
