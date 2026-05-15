import * as path from 'node:path';
import * as vscode from 'vscode';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Trace,
} from 'vscode-languageclient/node';

export type RenderSvgResult = {
  svg: string;
  width: number;
  height: number;
  diagnostics: Array<{ message?: string }>;
};

export class PumlLspClient {
  private client: LanguageClient | undefined;

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
}

function defaultServerCommand(context: vscode.ExtensionContext): string {
  const isWindows = process.platform === 'win32';
  const bundled = path.join(context.extensionPath, 'bin', isWindows ? 'puml-lsp.exe' : 'puml-lsp');
  return bundled;
}
