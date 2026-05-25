/**
 * renderer.ts
 *
 * Thin abstraction over rendering strategies: LSP-first, CLI subprocess fallback.
 * Used by preview, export, and check commands so they all go through one path.
 */
import * as cp from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import * as vscode from 'vscode';
import { ExportResult, PumlLspClient, RenderSvgResult } from './lspClient';

export interface RenderResult {
  svg: string;
  diagnostics: Array<{ message: string; severity: 'error' | 'warning' | 'info' }>;
  family?: string;
}

/** Render a document via LSP if available, otherwise fall back to CLI subprocess. */
export async function renderDocument(
  document: vscode.TextDocument,
  lsp: PumlLspClient,
  context: vscode.ExtensionContext
): Promise<RenderResult> {
  if (lsp.isRunning()) {
    try {
      const raw = await lsp.renderSvg(document.uri.toString());
      return normaliseLspResult(raw);
    } catch (_err) {
      // LSP error — fall through to CLI
    }
  }

  return renderViaCli(document, context);
}

/** Render via `puml` CLI subprocess — writes source to a temp file, reads SVG stdout. */
export async function renderViaCli(
  document: vscode.TextDocument,
  _context: vscode.ExtensionContext
): Promise<RenderResult> {
  const config = vscode.workspace.getConfiguration('puml');
  const cliBin = resolvePumlBin(config.get<string>('cli.path', ''));

  // Write current document text to a temp file (handles unsaved edits).
  const tmp = path.join(os.tmpdir(), `puml-preview-${Date.now()}.puml`);
  fs.writeFileSync(tmp, document.getText(), 'utf8');

  try {
    const svg = await execPuml(cliBin, ['--format', 'svg', tmp, '-o', '-']);
    return { svg, diagnostics: [] };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      svg: '',
      diagnostics: [{ message: msg, severity: 'error' }],
    };
  } finally {
    try {
      fs.unlinkSync(tmp);
    } catch {
      // best-effort cleanup
    }
  }
}

/** Export to SVG file via CLI. Returns the output path. */
export async function exportSvg(
  document: vscode.TextDocument,
  outputPath: string,
  context: vscode.ExtensionContext,
  lsp?: PumlLspClient
): Promise<void> {
  if (lsp?.isRunning()) {
    try {
      const result = await lsp.exportDocument(document.uri.toString(), 'svg');
      if (writeLspExportResult(result, outputPath)) {
        return;
      }
    } catch {
      // Fall back to the CLI export path below.
    }
  }

  const config = vscode.workspace.getConfiguration('puml');
  const cliBin = resolvePumlBin(config.get<string>('cli.path', ''));

  const tmp = path.join(os.tmpdir(), `puml-export-${Date.now()}.puml`);
  fs.writeFileSync(tmp, document.getText(), 'utf8');
  try {
    await execPuml(cliBin, ['--format', 'svg', tmp, '-o', outputPath]);
  } finally {
    try {
      fs.unlinkSync(tmp);
    } catch {
      // best-effort cleanup
    }
  }
}

/** Export to PNG file via CLI. Returns the output path. */
export async function exportPng(
  document: vscode.TextDocument,
  outputPath: string,
  context: vscode.ExtensionContext,
  lsp?: PumlLspClient
): Promise<void> {
  if (lsp?.isRunning()) {
    try {
      const result = await lsp.exportDocument(document.uri.toString(), 'png');
      if (writeLspExportResult(result, outputPath)) {
        return;
      }
    } catch {
      // Fall back to the CLI export path below.
    }
  }

  const config = vscode.workspace.getConfiguration('puml');
  const cliBin = resolvePumlBin(config.get<string>('cli.path', ''));

  const tmp = path.join(os.tmpdir(), `puml-export-${Date.now()}.puml`);
  fs.writeFileSync(tmp, document.getText(), 'utf8');
  try {
    await execPuml(cliBin, ['--format', 'png', tmp, '-o', outputPath]);
  } finally {
    try {
      fs.unlinkSync(tmp);
    } catch {
      // best-effort cleanup
    }
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function normaliseLspResult(raw: RenderSvgResult): RenderResult {
  return {
    svg: raw.svg,
    diagnostics: raw.diagnostics.map((d) => ({
      message: d.message ?? '(unknown diagnostic)',
      severity: normaliseSeverity(d.severity),
    })),
    family: undefined,
  };
}

function writeLspExportResult(result: ExportResult, outputPath: string): boolean {
  if (result.diagnostics.some((diag) => normaliseSeverity(diag.severity) === 'error')) {
    return false;
  }

  const page = result.pages?.[0];
  const content = result.content ?? page?.content;
  if (typeof content === 'string') {
    fs.writeFileSync(outputPath, content, 'utf8');
    return true;
  }

  const contentBase64 = result.contentBase64 ?? page?.contentBase64;
  if (typeof contentBase64 === 'string') {
    fs.writeFileSync(outputPath, Buffer.from(contentBase64, 'base64'));
    return true;
  }

  return false;
}

function normaliseSeverity(raw: string | undefined): 'error' | 'warning' | 'info' {
  return raw === 'warning' ? 'warning' : raw === 'info' ? 'info' : 'error';
}

function resolvePumlBin(configured: string): string {
  if (configured && configured.trim().length > 0) {
    return configured.trim();
  }
  const isWindows = process.platform === 'win32';
  return isWindows ? 'puml.exe' : 'puml';
}

function execPuml(bin: string, args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = [];
    const errChunks: Buffer[] = [];

    const child = cp.spawn(bin, args, { stdio: ['ignore', 'pipe', 'pipe'] });

    child.stdout.on('data', (chunk: Buffer) => chunks.push(chunk));
    child.stderr.on('data', (chunk: Buffer) => errChunks.push(chunk));

    child.on('error', (err) => {
      reject(new Error(`puml binary not found or failed to start: ${err.message}`));
    });

    child.on('close', (code) => {
      if (code === 0) {
        resolve(Buffer.concat(chunks).toString('utf8'));
      } else {
        const stderr = Buffer.concat(errChunks).toString('utf8').trim();
        reject(new Error(stderr || `puml exited with code ${code}`));
      }
    });
  });
}
