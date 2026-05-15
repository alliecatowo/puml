import * as vscode from 'vscode';
import { registerPreviewCommands } from './client/commands';

export function activate(context: vscode.ExtensionContext): void {
  registerPreviewCommands(context);
}

export function deactivate(): void {
  // no-op
}
