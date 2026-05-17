import { siteBaseUrl } from './manifest.js';
import { WasmRenderer, diagnosticLabel } from './wasm-renderer.js';

const SUPPORTED_LANGS = new Set(['puml', 'pumlx', 'plantuml', 'uml', 'puml-sequence', 'uml-sequence', 'picouml', 'mermaid']);

let engine;
let nextPreviewId = 1;

function getEngine() {
  if (!engine) engine = new WasmRenderer(siteBaseUrl());
  return engine;
}

function fenceLanguage(pre, code) {
  const raw =
    pre.dataset.lang ||
    code?.dataset.lang ||
    [...(code?.classList || [])].find((name) => name.startsWith('language-'))?.slice('language-'.length) ||
    [...pre.classList].find((name) => name.startsWith('language-'))?.slice('language-'.length) ||
    '';
  return raw.trim().split(/\s+/)[0].toLowerCase();
}

function hydrateFence(pre) {
  if (pre.closest('[data-puml-fence-preview]')) return;
  const code = pre.querySelector('code');
  const lang = fenceLanguage(pre, code);
  if (!SUPPORTED_LANGS.has(lang)) return;

  const source = code ? code.textContent : pre.textContent;
  const panelId = `puml-fence-render-${nextPreviewId++}`;

  const wrapper = document.createElement('div');
  wrapper.className = 'puml-fence-preview';
  wrapper.setAttribute('data-puml-fence-preview', '');
  wrapper.dataset.lang = lang;

  const toolbar = document.createElement('div');
  toolbar.className = 'puml-fence-toolbar';

  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'puml-fence-toggle';
  button.setAttribute('aria-controls', panelId);
  button.setAttribute('aria-expanded', 'false');
  button.title = `Toggle rendered ${lang} preview`;
  button.textContent = 'Preview';

  const body = document.createElement('div');
  body.className = 'puml-fence-body';

  const panel = document.createElement('div');
  panel.id = panelId;
  panel.className = 'puml-fence-render';
  panel.hidden = true;
  panel.setAttribute('role', 'region');
  panel.setAttribute('aria-live', 'polite');
  panel.setAttribute('aria-label', `Rendered ${lang} diagram preview`);

  pre.parentNode.insertBefore(wrapper, pre);
  toolbar.appendChild(button);
  body.append(pre, panel);
  wrapper.append(toolbar, body);

  let hasRendered = false;
  let isRendering = false;

  button.addEventListener('click', async () => {
    const expanded = button.getAttribute('aria-expanded') === 'true';
    if (expanded) {
      wrapper.classList.remove('is-open');
      panel.hidden = true;
      button.setAttribute('aria-expanded', 'false');
      button.textContent = 'Preview';
      return;
    }

    wrapper.classList.add('is-open');
    panel.hidden = false;
    button.setAttribute('aria-expanded', 'true');
    button.textContent = 'Hide preview';

    if (!hasRendered && !isRendering) {
      isRendering = true;
      renderLoading(panel);
      try {
        const result = await getEngine().render(source, { frontend: lang });
        if (result.ok) {
          renderSvgs(panel, result.svgs);
          hasRendered = true;
        } else {
          renderDiagnostic(panel, result.diagnostics?.[0]);
        }
      } catch (e) {
        renderDiagnostic(panel, { severity: 'error', message: e.message || String(e) });
      } finally {
        isRendering = false;
      }
    }
  });
}

function renderLoading(panel) {
  panel.setAttribute('aria-busy', 'true');
  const message = document.createElement('div');
  message.className = 'puml-fence-message';
  message.textContent = 'Rendering preview...';
  panel.replaceChildren(message);
}

function renderSvgs(panel, svgs) {
  panel.removeAttribute('aria-busy');
  const pages = document.createElement('div');
  pages.className = 'puml-fence-pages';
  pages.innerHTML = svgs.join('\n');
  panel.replaceChildren(pages);
}

function renderDiagnostic(panel, diag) {
  panel.removeAttribute('aria-busy');
  const box = document.createElement('div');
  box.className = 'puml-fence-diagnostic';
  box.setAttribute('role', 'status');

  const label = document.createElement('span');
  label.className = 'pill';
  label.textContent = diag?.severity || 'error';

  const message = document.createElement('p');
  message.textContent = diagnosticLabel(diag);

  box.append(label, message);
  panel.replaceChildren(box);
}

export function hydrateInlineFencePreviews(root = document) {
  root.querySelectorAll('.prose pre').forEach(hydrateFence);
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => hydrateInlineFencePreviews(), { once: true });
} else {
  hydrateInlineFencePreviews();
}
