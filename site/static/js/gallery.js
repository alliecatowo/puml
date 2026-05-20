// Full gallery with family filter chips, search, and lazy-loaded SVG thumbs.

import { loadManifest, siteBaseUrl, assetUrl } from './manifest.js';

const state = {
  manifest: null,
  family: 'all',
  query: '',
};

async function init() {
  const base = siteBaseUrl();
  state.base = base;
  try {
    state.manifest = await loadManifest(base);
  } catch (e) {
    const grid = document.getElementById('gallery-grid');
    if (grid) grid.innerHTML = '<div class="empty">Failed to load manifest: ' + escapeHtml(e.message) + '</div>';
    return;
  }

  // Filter from ?family=... or ?open=... query
  const params = new URLSearchParams(window.location.search);
  if (params.get('family')) state.family = params.get('family');
  const open = params.get('open'); // e.g. "sequence/01_basic"

  renderChips();
  document.getElementById('gallery-search').addEventListener('input', (e) => {
    state.query = e.target.value.trim().toLowerCase();
    render();
  });
  render();

  if (open) {
    // Defer to next tick so cards exist.
    setTimeout(() => {
      const el = document.querySelector(`[data-card-id="${cssEscape(open)}"]`);
      if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }, 50);
  }
}

function renderChips() {
  const wrap = document.getElementById('gallery-chips');
  if (!wrap) return;
  wrap.innerHTML = '';

  const fams = [{ family: 'all', label: 'All', count: state.manifest.totals.examples }]
    .concat(state.manifest.families);

  for (const f of fams) {
    const chip = document.createElement('button');
    chip.className = 'chip' + (state.family === f.family ? ' active' : '');
    chip.type = 'button';
    chip.textContent = `${f.label}${f.count != null ? ` (${f.count})` : ''}`;
    chip.addEventListener('click', () => {
      state.family = f.family;
      const url = new URL(window.location.href);
      if (f.family === 'all') url.searchParams.delete('family');
      else url.searchParams.set('family', f.family);
      window.history.replaceState({}, '', url);
      renderChips();
      render();
    });
    wrap.appendChild(chip);
  }
}

function render() {
  const grid = document.getElementById('gallery-grid');
  const count = document.getElementById('gallery-count');
  if (!grid) return;

  const items = state.manifest.examples.filter((it) => {
    if (state.family !== 'all' && it.family !== state.family) return false;
    if (!state.query) return true;
    const blob = `${it.family} ${it.familyLabel} ${it.name} ${it.title} ${it.preview}`.toLowerCase();
    return state.query.split(/\s+/).every((tok) => blob.includes(tok));
  });

  count.textContent = `${items.length.toLocaleString()} example${items.length === 1 ? '' : 's'}` +
    (state.family !== 'all' ? ` in ${prettyFamily(state.family)}` : '') +
    (state.query ? ` matching “${state.query}”` : '');

  if (!items.length) {
    grid.innerHTML = '<div class="empty">No examples match. Try a different family or clearer search.</div>';
    return;
  }

  grid.innerHTML = '';
  for (const it of items) {
    const id = `${it.family}/${it.name}`;
    const card = document.createElement('a');
    card.className = 'card';
    card.dataset.cardId = id;
    const editUrl = state.base.replace(/\/$/, '') + `/editor/?open=${encodeURIComponent(id)}`;
    card.href = editUrl;
    card.title = 'Open in editor';
    card.innerHTML = `
      <div class="thumb"><img loading="lazy" alt="${escapeHtml(it.title)} preview" src="${assetUrl(state.base, it.svgPath)}" /></div>
      <div class="meta">
        <span class="title">${escapeHtml(it.title)}</span>
        <span class="sub"><span class="tag">${escapeHtml(it.familyLabel)}</span><span>${it.lineCount} lines</span></span>
        ${it.preview ? `<span class="preview">${escapeHtml(it.preview)}</span>` : ''}
      </div>`;
    grid.appendChild(card);
  }
}

function prettyFamily(f) {
  const m = state.manifest.families.find((x) => x.family === f);
  return m ? m.label : f;
}

function escapeHtml(s) {
  return String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

function cssEscape(s) {
  return String(s ?? '')
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"');
}

init();
