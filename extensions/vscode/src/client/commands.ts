/**
 * commands.ts
 *
 * Registers all PUML commands for the extension:
 *   puml.preview.open      — open/refresh the live preview panel
 *   puml.lsp.restart       — restart the language server
 *   puml.check             — render the active file and surface diagnostics inline
 */
import * as vscode from 'vscode';
import { PumlLspClient } from './lspClient';
import { PumlPreviewPanel } from './previewPanel';
import { renderDocument } from './renderer';
import { PumlStatusBar } from './statusBar';

const DIAGNOSTIC_COLLECTION_NAME = 'puml';

export function registerPreviewCommands(
  context: vscode.ExtensionContext,
  lsp: PumlLspClient
): void {
  const diagCollection = vscode.languages.createDiagnosticCollection(DIAGNOSTIC_COLLECTION_NAME);
  context.subscriptions.push(diagCollection);

  // -------------------------------------------------------------------------
  // puml.preview.open — open live preview panel
  // -------------------------------------------------------------------------
  const openPreview = vscode.commands.registerCommand('puml.preview.open', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to preview.');
      return;
    }

    await lsp.start(context);
    await PumlPreviewPanel.show(editor.document, lsp, context);
  });

  // -------------------------------------------------------------------------
  // puml.lsp.restart — restart language server with exponential-backoff retry
  // -------------------------------------------------------------------------
  const restartLsp = vscode.commands.registerCommand('puml.lsp.restart', async () => {
    await lsp.restart(context);
    void vscode.window.showInformationMessage('puml-lsp restarted.');
  });

  // -------------------------------------------------------------------------
  // puml.check — render + push parser diagnostics as VS Code diagnostics
  // -------------------------------------------------------------------------
  const checkCmd = vscode.commands.registerCommand('puml.check', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to check.');
      return;
    }

    const document = editor.document;
    diagCollection.clear();

    await vscode.window.withProgress(
      {
        location: vscode.ProgressLocation.Window,
        title: 'PUML: Checking…',
        cancellable: false,
      },
      async () => {
        try {
          const result = await renderDocument(document, lsp, context);
          const vscDiags: vscode.Diagnostic[] = result.diagnostics.map((d) => {
            const range = new vscode.Range(0, 0, 0, 0);
            const severity =
              d.severity === 'error'
                ? vscode.DiagnosticSeverity.Error
                : d.severity === 'warning'
                  ? vscode.DiagnosticSeverity.Warning
                  : vscode.DiagnosticSeverity.Information;
            const diag = new vscode.Diagnostic(range, d.message, severity);
            diag.source = 'puml';
            return diag;
          });

          diagCollection.set(document.uri, vscDiags);

          const errCount = vscDiags.filter(
            (d) => d.severity === vscode.DiagnosticSeverity.Error
          ).length;
          const warnCount = vscDiags.filter(
            (d) => d.severity === vscode.DiagnosticSeverity.Warning
          ).length;

          PumlStatusBar.update({
            errorCount: errCount,
            warningCount: warnCount,
            family: result.family,
          });

          if (vscDiags.length === 0) {
            void vscode.window.showInformationMessage('PUML: No errors found.');
          } else {
            const summary = `PUML: ${errCount} error(s), ${warnCount} warning(s)`;
            const action = errCount > 0
              ? await vscode.window.showErrorMessage(summary, 'Show Problems')
              : await vscode.window.showWarningMessage(summary, 'Show Problems');
            if (action === 'Show Problems') {
              await vscode.commands.executeCommand('workbench.action.problems.focus');
            }
          }
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          void vscode.window.showErrorMessage(`PUML check failed: ${msg}`);
        }
      }
    );
  });

  context.subscriptions.push(openPreview, restartLsp, checkCmd);
}
