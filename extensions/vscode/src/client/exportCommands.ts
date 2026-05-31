/**
 * exportCommands.ts
 *
 * Implements puml.export.svg, puml.export.png, puml.export.source,
 * and puml.export.json:
 *   - Prompts user for a save path.
 *   - Renders via CLI subprocess or LSP (SVG/PNG export uses the CLI fallback
 *     so the result lands in a proper file; source/json write directly).
 *   - Shows progress notification during render.
 */
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { PumlLspClient } from './lspClient';
import { exportPng, exportSvg, renderDocument } from './renderer';

export function registerExportCommands(
  context: vscode.ExtensionContext,
  lsp: PumlLspClient
): void {
  const exportSvgCmd = vscode.commands.registerCommand('puml.export.svg', async () => {
    await runExport(context, lsp, 'svg');
  });

  const exportPngCmd = vscode.commands.registerCommand('puml.export.png', async () => {
    await runExport(context, lsp, 'png');
  });

  // puml.export.source — save the current editor buffer as a .puml file
  const exportSourceCmd = vscode.commands.registerCommand('puml.export.source', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to export source.');
      return;
    }

    const document = editor.document;
    const defaultName = path.basename(document.fileName, path.extname(document.fileName));
    const defaultUri = vscode.Uri.file(
      path.join(path.dirname(document.fileName), `${defaultName}.puml`)
    );

    const target = await vscode.window.showSaveDialog({
      defaultUri,
      filters: { 'PUML Source': ['puml', 'plantuml', 'iuml'] },
      saveLabel: 'Export Source',
    });

    if (!target) {
      return;
    }

    try {
      fs.writeFileSync(target.fsPath, document.getText(), 'utf8');
      const open = await vscode.window.showInformationMessage(
        `Exported source: ${target.fsPath}`,
        'Reveal in Explorer'
      );
      if (open === 'Reveal in Explorer') {
        await vscode.commands.executeCommand('revealFileInOS', target);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      void vscode.window.showErrorMessage(`PUML export source failed: ${msg}`);
    }
  });

  // puml.export.json — export compile result as JSON (model + diagnostics)
  const exportJsonCmd = vscode.commands.registerCommand('puml.export.json', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to export JSON.');
      return;
    }

    const document = editor.document;
    const defaultName = path.basename(document.fileName, path.extname(document.fileName));
    const defaultUri = vscode.Uri.file(
      path.join(path.dirname(document.fileName), `${defaultName}.json`)
    );

    const target = await vscode.window.showSaveDialog({
      defaultUri,
      filters: { 'JSON': ['json'] },
      saveLabel: 'Export JSON',
    });

    if (!target) {
      return;
    }

    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Notification,
        title: 'PUML: Exporting JSON…',
        cancellable: false,
      },
      async () => {
        try {
          await lsp.start(context);
          const result = await renderDocument(document, lsp, context);
          const payload = {
            family: result.family ?? null,
            diagnostics: result.diagnostics,
          };
          fs.writeFileSync(target.fsPath, JSON.stringify(payload, null, 2), 'utf8');
          const open = await vscode.window.showInformationMessage(
            `Exported JSON: ${target.fsPath}`,
            'Reveal in Explorer'
          );
          if (open === 'Reveal in Explorer') {
            await vscode.commands.executeCommand('revealFileInOS', target);
          }
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          void vscode.window.showErrorMessage(`PUML export JSON failed: ${msg}`);
        }
      }
    );
  });

  context.subscriptions.push(exportSvgCmd, exportPngCmd, exportSourceCmd, exportJsonCmd);
}

async function runExport(
  context: vscode.ExtensionContext,
  lsp: PumlLspClient,
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
          await exportSvg(document, target.fsPath, context, lsp);
        } else {
          await exportPng(document, target.fsPath, context, lsp);
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
