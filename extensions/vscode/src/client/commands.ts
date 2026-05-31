/**
 * commands.ts
 *
 * Registers all PUML commands for the extension:
 *   puml.preview.open      — open/refresh the live preview panel
 *   puml.showSvg           — open rendered SVG source in a side editor
 *   puml.showDiagnostics   — render, show diagnostics panel, and focus Problems
 *   puml.lsp.restart       — restart the language server (manual, resets backoff)
 *   puml.lsp.showOutput    — reveal the PUML output channel
 *   puml.check             — render the active file and surface diagnostics inline
 *   puml.renderScene       — open render scene JSON in a side editor
 */
import * as vscode from 'vscode';
import { PumlLspClient, getOutputChannel } from './lspClient';
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
  // puml.showSvg — render active file and open SVG source in a side editor
  // -------------------------------------------------------------------------
  const showSvgCmd = vscode.commands.registerCommand('puml.showSvg', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to view SVG.');
      return;
    }

    await lsp.start(context);
    await vscode.window.withProgress(
      { location: vscode.ProgressLocation.Window, title: 'PUML: Rendering SVG…', cancellable: false },
      async () => {
        try {
          const result = await renderDocument(editor.document, lsp, context);
          if (!result.svg || result.svg.trim().length === 0) {
            void vscode.window.showWarningMessage('PUML: No SVG output — check diagnostics.');
            return;
          }
          const svgDoc = await vscode.workspace.openTextDocument({
            language: 'xml',
            content: result.svg,
          });
          await vscode.window.showTextDocument(svgDoc, vscode.ViewColumn.Beside, true);
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          void vscode.window.showErrorMessage(`PUML show SVG failed: ${msg}`);
        }
      }
    );
  });

  // -------------------------------------------------------------------------
  // puml.showDiagnostics — render and focus the Problems panel
  // -------------------------------------------------------------------------
  const showDiagnosticsCmd = vscode.commands.registerCommand('puml.showDiagnostics', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to show diagnostics.');
      return;
    }

    const document = editor.document;
    diagCollection.clear();

    await vscode.window.withProgress(
      { location: vscode.ProgressLocation.Window, title: 'PUML: Running diagnostics…', cancellable: false },
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

          const errCount = vscDiags.filter((d) => d.severity === vscode.DiagnosticSeverity.Error).length;
          const warnCount = vscDiags.filter((d) => d.severity === vscode.DiagnosticSeverity.Warning).length;

          PumlStatusBar.update({ errorCount: errCount, warningCount: warnCount, family: result.family });

          await vscode.commands.executeCommand('workbench.action.problems.focus');

          if (vscDiags.length === 0) {
            void vscode.window.showInformationMessage('PUML: No diagnostics — diagram is valid.');
          }
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          void vscode.window.showErrorMessage(`PUML diagnostics failed: ${msg}`);
        }
      }
    );
  });

  // -------------------------------------------------------------------------
  // puml.lsp.restart — manual restart; resets backoff counter
  // -------------------------------------------------------------------------
  const restartLsp = vscode.commands.registerCommand('puml.lsp.restart', async () => {
    await lsp.manualRestart(context);
    void vscode.window.showInformationMessage('puml-lsp restarted.');
  });

  // -------------------------------------------------------------------------
  // puml.lsp.showOutput — reveal the PUML output channel
  // -------------------------------------------------------------------------
  const showOutputCmd = vscode.commands.registerCommand('puml.lsp.showOutput', () => {
    getOutputChannel().show();
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

  const renderSceneCmd = vscode.commands.registerCommand('puml.renderScene', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== 'puml') {
      void vscode.window.showWarningMessage('Open a .puml document to inspect render scene JSON.');
      return;
    }

    await lsp.start(context);
    try {
      const result = await lsp.renderScene(editor.document.uri.toString(), { frontend: 'auto' });
      const sceneDoc = await vscode.workspace.openTextDocument({
        language: 'json',
        content: JSON.stringify(result, null, 2),
      });
      await vscode.window.showTextDocument(sceneDoc, vscode.ViewColumn.Beside, true);
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      void vscode.window.showErrorMessage(`PUML render scene failed: ${msg}`);
    }
  });

  context.subscriptions.push(
    openPreview,
    showSvgCmd,
    showDiagnosticsCmd,
    restartLsp,
    showOutputCmd,
    checkCmd,
    renderSceneCmd,
  );
}
