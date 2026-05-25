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

export class PumlLspClient {
  private client: LanguageClient | undefined;

  isRunning(): boolean {
    return this.client !== undefined;
  }

  async start(context: vscode.ExtensionContext): Promise<void> {
    if (this.client) {
      return;
    }

    const config = vscode.workspace.getConfiguration('puml');
    const configuredPath = config.get<string>('lsp.path')?.trim();
    const command = configuredPath && configuredPath.length > 0 ? configuredPath : defaultServerCommand(context);

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
    };

    this.client = new LanguageClient('puml-lsp', 'puml-lsp', serverOptions, clientOptions);

    const trace = config.get<string>('lsp.trace', 'off');
    this.client.setTrace(trace === 'messages' ? Trace.Messages : trace === 'verbose' ? Trace.Verbose : Trace.Off);

    await this.client.start();
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
    await this.client.stop();
    this.client = undefined;
  }

  async restart(context: vscode.ExtensionContext): Promise<void> {
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

function defaultServerCommand(context: vscode.ExtensionContext): string {
  const isWindows = process.platform === 'win32';
  const bundled = path.join(context.extensionPath, 'bin', isWindows ? 'puml-lsp.exe' : 'puml-lsp');
  if (fs.existsSync(bundled)) {
    return bundled;
  }
  return isWindows ? 'puml-lsp.exe' : 'puml-lsp';
}
