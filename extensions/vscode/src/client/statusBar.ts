/**
 * statusBar.ts
 *
 * Persistent status bar item that shows:
 *   - Current diagram family (e.g. "sequence", "class") when a .puml file is active.
 *   - Diagnostic count badge (errors / warnings) updated after each render.
 *   - Clicking opens the preview panel.
 *
 * Call `PumlStatusBar.update()` from the preview refresh path to keep counts in sync.
 */
import * as vscode from 'vscode';

export class PumlStatusBar {
  private static item: vscode.StatusBarItem | undefined;
  private static diagnosticCount = 0;
  private static family: string | undefined;

  static register(context: vscode.ExtensionContext): void {
    if (PumlStatusBar.item) {
      return;
    }

    const item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    item.command = 'puml.preview.open';
    item.tooltip = 'PUML: Open preview panel';
    PumlStatusBar.item = item;
    context.subscriptions.push(item);

    // Show/hide based on active editor language.
    const refresh = () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && editor.document.languageId === 'puml') {
        PumlStatusBar.renderLabel();
        item.show();
      } else {
        item.hide();
      }
    };

    context.subscriptions.push(
      vscode.window.onDidChangeActiveTextEditor(refresh),
      vscode.window.onDidChangeVisibleTextEditors(refresh)
    );

    refresh();
  }

  /** Update after a render pass. Pass 0 errors/warnings + optional family string. */
  static update(opts: {
    errorCount: number;
    warningCount: number;
    family?: string;
  }): void {
    PumlStatusBar.diagnosticCount = opts.errorCount + opts.warningCount;
    PumlStatusBar.family = opts.family;
    PumlStatusBar.renderLabel();
  }

  private static renderLabel(): void {
    if (!PumlStatusBar.item) {
      return;
    }

    const familyPart = PumlStatusBar.family ? ` [${PumlStatusBar.family}]` : '';
    const diagCount = PumlStatusBar.diagnosticCount;
    const diagPart = diagCount > 0 ? ` $(error) ${diagCount}` : ' $(check)';

    PumlStatusBar.item.text = `$(graph)${familyPart}${diagPart}`;
  }
}
