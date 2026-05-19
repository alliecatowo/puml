/**
 * exportCommands.ts
 *
 * Implements puml.export.svg and puml.export.png:
 *   - Prompts user for a save path.
 *   - Renders via CLI subprocess (export always uses CLI so the result is a
 *     proper file, not an in-memory SVG string from the LSP).
 *   - Shows progress notification during render.
 */
import * as path from 'node:path';
import * as vscode from 'vscode';
import { exportPng, exportSvg } from './renderer';

export function registerExportCommands(context: vscode.ExtensionContext): void {
  const exportSvgCmd = vscode.commands.registerCommand('puml.export.svg', async () => {
    await runExport(context, 'svg');
  });

  const exportPngCmd = vscode.commands.registerCommand('puml.export.png', async () => {
    await runExport(context, 'png');
  });

  context.subscriptions.push(exportSvgCmd, exportPngCmd);
}

async function runExport(
  context: vscode.ExtensionContext,
  format: 'svg' | 'png'
): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== 'puml') {
    void vscode.window.showWarningMessage('Open a .puml document to export.');
    return;
  }

  const document = editor.document;
  const defaultName = path.basename(document.fileName, path.extname(document.fileName));
  const defaultUri = vscode.Uri.file(
    path.join(path.dirname(document.fileName), `${defaultName}.${format}`)
  );

  const target = await vscode.window.showSaveDialog({
    defaultUri,
    filters: {
      [format === 'svg' ? 'SVG Image' : 'PNG Image']: [format],
    },
    saveLabel: `Export as ${format.toUpperCase()}`,
  });

  if (!target) {
    return; // user cancelled
  }

  await vscode.window.withProgress(
    {
      location: vscode.ProgressLocation.Notification,
      title: `PUML: Exporting as ${format.toUpperCase()}…`,
      cancellable: false,
    },
    async () => {
      try {
        if (format === 'svg') {
          await exportSvg(document, target.fsPath, context);
        } else {
          await exportPng(document, target.fsPath, context);
        }
        const open = await vscode.window.showInformationMessage(
          `Exported: ${target.fsPath}`,
          'Reveal in Explorer'
        );
        if (open === 'Reveal in Explorer') {
          await vscode.commands.executeCommand('revealFileInOS', target);
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        void vscode.window.showErrorMessage(`PUML export failed: ${msg}`);
      }
    }
  );
}
