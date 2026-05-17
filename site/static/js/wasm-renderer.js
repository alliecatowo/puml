import { assetUrl } from './manifest.js';

// Shared browser renderer for the editor and inline docs previews. It loads
// only local site assets produced from crates/puml-wasm; no remote service is
// involved in rendering diagrams.
export class WasmRenderer {
  constructor(base) {
    this.base = base;
    this.ready = null;
    this.module = null;
  }

  describe() { return 'Renderer: in-browser WASM'; }

  async init() {
    if (this.ready) return this.ready;
    this.ready = (async () => {
      const jsUrl = assetUrl(this.base, 'wasm/puml_wasm.js');
      const wasmUrl = assetUrl(this.base, 'wasm/puml_wasm_bg.wasm');
      const mod = await import(jsUrl);
      await mod.default({ module_or_path: wasmUrl });
      this.module = mod;
    })();
    return this.ready;
  }

  async render(source) {
    await this.init();
    const json = this.module.render_svgs_json(source);
    let parsed;
    try {
      parsed = JSON.parse(json);
    } catch (e) {
      return { ok: false, diagnostics: [{ severity: 'error', message: `Renderer returned invalid JSON: ${e.message}` }] };
    }
    if (parsed.error) {
      return {
        ok: false,
        diagnostics: [{
          severity: parsed.error.severity || 'error',
          message: parsed.error.message || 'Render failed.',
          line: parsed.error.line,
          column: parsed.error.column,
        }],
      };
    }
    const pages = Array.isArray(parsed.ok) ? parsed.ok : [];
    if (!pages.length) {
      return { ok: false, diagnostics: [{ severity: 'error', message: 'Renderer returned no pages.' }] };
    }
    return { ok: true, svgs: pages };
  }
}

export function diagnosticLabel(diag) {
  const where = diag?.line ? `line ${diag.line}${diag.column ? `, col ${diag.column}` : ''}` : '';
  const message = diag?.message || 'Render failed.';
  return where ? `${where}: ${message}` : message;
}
