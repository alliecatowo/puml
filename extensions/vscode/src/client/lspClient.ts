import * as path from 'node:path';
import * as fs from 'node:fs';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Trace,
} from 'vscode-languageclient/node';

export type RenderSvgResult = {
  schema?: string;
  svg: string;
  svgs?: string[];
  width: number;
  height: number;
  diagnostics: Array<{ message?: string; severity?: string; code?: string }>;
  model?: unknown;
  scene?: unknown;
};

export type RenderSceneResult = {
  schema?: string;
  schemaVersion?: number;
  model?: unknown;
  scene?: unknown;
  diagnostics: Array<{ message?: string; severity?: string; code?: string }>;
};

export type ExportResult = {
  schema?: string;
  schemaVersion?: number;
  format?: string;
  mediaType?: string;
  encoding?: string;
  content?: string | null;
  contentBase64?: string | null;
  pages?: Array<{ name?: string; content?: string; contentBase64?: string }>;
  diagnostics: Array<{ message?: string; severity?: string; code?: string }>;
};

export type ExplainDiagnosticResult = {
  schema?: string;
  schemaVersion?: number;
  diagnostic?: unknown;
  explanation?: { summary?: string; action?: string };
  diagnostics: Array<{ message?: string; severity?: string; code?: string }>;
};

export type LanguageServiceSurfaceResult = {
  schema?: string;
  schemaVersion?: number;
  families?: unknown[];
  graphElements?: unknown[];
  completion?: { items?: unknown[] };
  syntax?: unknown;
  semanticTokens?: unknown;
  diagnostics?: Array<{ message?: string; severity?: string; code?: string }>;
};

/** Shared output channel for all PUML extension logging. */
let _outputChannel: vscode.OutputChannel | undefined;

export function getOutputChannel(): vscode.OutputChannel {
  if (!_outputChannel) {
    _outputChannel = vscode.window.createOutputChannel('PUML');
  }
  return _outputChannel;
}

const MAX_RESTART_ATTEMPTS = 5;
const RESTART_BACKOFF_BASE_MS = 1000;

export class PumlLspClient {
  private client: LanguageClient | undefined;
  private restartAttempts = 0;

  isRunning(): boolean {
    return this.client !== undefined;
  }

  async start(context: vscode.ExtensionContext): Promise<void> {
    if (this.client) {
      return;
    }

    const config = vscode.workspace.getConfiguration('puml');
    const configuredPath = config.get<string>('lsp.path')?.trim();
    const command = configuredPath && configuredPath.length > 0
      ? configuredPath
      : resolveLspBinary(context);

    const out = getOutputChannel();
    out.appendLine(`[puml-lsp] starting: ${command}`);

    const serverOptions: ServerOptions = {
      command,
      args: [],
      transport: 0,
    };

    const clientOptions: LanguageClientOptions = {
      documentSelector: [{ language: 'puml' }],
      synchronize: {
        configurationSection: 'puml',
      },
      outputChannel: out,
    };

    this.client = new LanguageClient('puml-lsp', 'puml-lsp', serverOptions, clientOptions);

    const trace = config.get<string>('lsp.trace', 'off');
    this.client.setTrace(trace === 'messages' ? Trace.Messages : trace === 'verbose' ? Trace.Verbose : Trace.Off);

    try {
      await this.client.start();
      this.restartAttempts = 0;
      out.appendLine('[puml-lsp] started successfully');
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      out.appendLine(`[puml-lsp] start failed: ${msg}`);
      this.client = undefined;
      throw err;
    }

    context.subscriptions.push({
      dispose: () => {
        void this.stop();
      },
    });
  }

  async stop(): Promise<void> {
    if (!this.client) {
      return;
    }
    getOutputChannel().appendLine('[puml-lsp] stopping');
    await this.client.stop();
    this.client = undefined;
  }

  /**
   * Restart with exponential backoff. If the server has already failed
   * MAX_RESTART_ATTEMPTS times consecutively, gives up and surfaces an error.
   */
  async restart(context: vscode.ExtensionContext): Promise<void> {
    await this.stop();

    if (this.restartAttempts >= MAX_RESTART_ATTEMPTS) {
      const out = getOutputChannel();
      out.appendLine(
        `[puml-lsp] giving up after ${MAX_RESTART_ATTEMPTS} restart attempts. ` +
        'Check the PUML output channel for details. You can manually retry via "PUML: Restart Language Server".'
      );
      void vscode.window.showErrorMessage(
        `puml-lsp failed to start after ${MAX_RESTART_ATTEMPTS} attempts. Check the PUML output channel.`,
        'Show Output'
      ).then((action) => {
        if (action === 'Show Output') {
          getOutputChannel().show();
        }
      });
      return;
    }

    const delayMs = RESTART_BACKOFF_BASE_MS * Math.pow(2, this.restartAttempts);
    this.restartAttempts++;

    getOutputChannel().appendLine(`[puml-lsp] restart attempt ${this.restartAttempts} in ${delayMs}ms`);

    await new Promise<void>((resolve) => setTimeout(resolve, delayMs));
    await this.start(context);
  }

  /**
   * Manual restart requested by the user — resets the backoff counter so the
   * next failure sequence starts fresh.
   */
  async manualRestart(context: vscode.ExtensionContext): Promise<void> {
    this.restartAttempts = 0;
    await this.stop();
    await this.start(context);
  }

  async renderSvg(uri: string): Promise<RenderSvgResult> {
    if (!this.client) {
      throw new Error('puml-lsp client is not started');
    }

    const out = await this.client.sendRequest<RenderSvgResult>('workspace/executeCommand', {
      command: 'puml.renderSvg',
      arguments: [uri],
    });

    return out;
  }

  async renderScene(uri: string, options: Record<string, unknown> = {}): Promise<RenderSceneResult> {
    if (!this.client) {
      throw new Error('puml-lsp client is not started');
    }

    return this.client.sendRequest<RenderSceneResult>('workspace/executeCommand', {
      command: 'puml.renderScene',
      arguments: [uri, options],
    });
  }

  async exportDocument(
    uri: string,
    format: 'svg' | 'png',
    options: Record<string, unknown> = {}
  ): Promise<ExportResult> {
    if (!this.client) {
      throw new Error('puml-lsp client is not started');
    }

    return this.client.sendRequest<ExportResult>('workspace/executeCommand', {
      command: 'puml.export',
      arguments: [uri, { ...options, format }],
    });
  }

  async explainDiagnostic(diagnostic: unknown): Promise<ExplainDiagnosticResult> {
    if (!this.client) {
      throw new Error('puml-lsp client is not started');
    }

    return this.client.sendRequest<ExplainDiagnosticResult>('workspace/executeCommand', {
      command: 'puml.explainDiagnostic',
      arguments: [diagnostic],
    });
  }

  async languageServiceSurface(): Promise<LanguageServiceSurfaceResult> {
    if (!this.client) {
      throw new Error('puml-lsp client is not started');
    }

    return this.client.sendRequest<LanguageServiceSurfaceResult>('workspace/executeCommand', {
      command: 'puml.languageService',
      arguments: [],
    });
  }
}

/**
 * Resolve the puml-lsp binary using a three-tier strategy:
 *   1. Extension-bundled binary at <extensionPath>/bin/puml-lsp[.exe]  (preferred)
 *   2. `puml-lsp` / `puml-lsp.exe` from PATH                           (fallback)
 *
 * The configuredPath override (puml.lsp.path setting) is applied in
 * PumlLspClient.start() before this function is ever called.
 */
export function resolveLspBinary(context: vscode.ExtensionContext): string {
  const isWindows = process.platform === 'win32';
  const bundled = path.join(context.extensionPath, 'bin', isWindows ? 'puml-lsp.exe' : 'puml-lsp');
  if (fs.existsSync(bundled)) {
    getOutputChannel().appendLine(`[puml-lsp] using bundled binary: ${bundled}`);
    return bundled;
  }
  const fallback = isWindows ? 'puml-lsp.exe' : 'puml-lsp';
  getOutputChannel().appendLine(
    `[puml-lsp] bundled binary not found at ${bundled}; falling back to PATH: ${fallback}`
  );
  return fallback;
}
