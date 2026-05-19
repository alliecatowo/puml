import { siteBaseUrl } from './manifest.js';
import { highlightPumlToHtml, PUML_HIGHLIGHT_LANGS } from './puml-tokens.js';
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
  applySyntaxHighlighting(code, lang);

  const source = code ? code.textContent : pre.textContent;
  const panelId = `puml-fence-render-${nextPreviewId++}`;

  const wrapper = document.createElement('div');
  wrapper.className = 'puml-fence-preview';
  wrapper.setAttribute('data-puml-fence-preview', '');
  wrapper.dataset.lang = lang;

  const toolbar = document.createElement('div');
  toolbar.className = 'puml-fence-toolbar';

  const language = document.createElement('span');
  language.className = 'puml-fence-lang';
  language.textContent = lang;

  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'puml-fence-toggle puml-fence-bubble';
  button.setAttribute('aria-controls', panelId);
  button.setAttribute('aria-expanded', 'false');
  button.setAttribute('aria-label', `Show rendered ${lang} graph`);
  button.setAttribute('aria-pressed', 'false');
  button.dataset.renderState = 'idle';
  button.title = `Show rendered ${lang} graph`;
  button.innerHTML = '<svg aria-hidden="true" viewBox="0 0 16 16"><path d="M2.5 11.5h3v2h-3zM6.5 6.5h3v7h-3zM10.5 2.5h3v11h-3z"/></svg><span>Graph</span>';

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
  toolbar.appendChild(language);
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
      button.setAttribute('aria-label', `Show rendered ${lang} graph`);
      button.setAttribute('aria-pressed', 'false');
      button.title = `Show rendered ${lang} graph`;
      setButtonLabel(button, 'Graph');
      return;
    }

    wrapper.classList.add('is-open');
    panel.hidden = false;
    button.setAttribute('aria-expanded', 'true');
    button.setAttribute('aria-label', `Hide rendered ${lang} graph`);
    button.setAttribute('aria-pressed', 'true');
    button.title = `Hide rendered ${lang} graph`;
    setButtonLabel(button, 'Hide');

    if (!hasRendered && !isRendering) {
      isRendering = true;
      button.dataset.renderState = 'loading';
      setButtonLabel(button, 'Loading');
      renderLoading(panel);
      try {
        const result = await getEngine().render(source, { frontend: lang });
        if (result.ok) {
          renderSvgs(panel, result.svgs);
          hasRendered = true;
          button.dataset.renderState = 'ready';
          wrapper.dataset.rendered = 'true';
        } else {
          renderDiagnostic(panel, result.diagnostics?.[0]);
          button.dataset.renderState = 'error';
        }
      } catch (e) {
        renderDiagnostic(panel, { severity: 'error', message: e.message || String(e) });
        button.dataset.renderState = 'error';
      } finally {
        isRendering = false;
        if (button.getAttribute('aria-expanded') === 'true') {
          setButtonLabel(button, button.dataset.renderState === 'loading' ? 'Graph' : 'Hide');
        }
      }
    }
  });
}

function applySyntaxHighlighting(code, lang) {
  if (
    !code ||
    typeof PUML_HIGHLIGHT_LANGS === 'undefined' ||
    typeof highlightPumlToHtml !== 'function' ||
    !PUML_HIGHLIGHT_LANGS.has(lang) ||
    code.children.length > 0
  ) return;
  code.innerHTML = highlightPumlToHtml(code.textContent || '');
}

function setButtonLabel(button, label) {
  const text = button.querySelector('span:last-child');
  if (text) text.textContent = label;
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
