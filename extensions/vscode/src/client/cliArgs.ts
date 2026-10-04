/**
 * cliArgs.ts
 *
 * Pure helpers (no `vscode` import) for building `puml` CLI argument lists.
 */
import * as path from 'node:path';

/**
 * Arguments that point the CLI's include sandbox at the document's own
 * directory. The CLI fallback renders from a temp copy of the buffer, so
 * without this `!include` would resolve against the temp directory (and fail).
 * Only file-backed documents have a meaningful directory; untitled and other
 * virtual documents get no include root.
 */
export function includeRootArgs(scheme: string, fsPath: string): string[] {
  if (scheme !== 'file' || !fsPath) {
    return [];
  }
  return ['--include-root', path.dirname(fsPath)];
}
