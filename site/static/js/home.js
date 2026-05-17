// Homepage: populate stat strip, hero preview, and featured gallery from the manifest.

import { loadManifest, siteBaseUrl, assetUrl } from './manifest.js';

const FEATURED = [
  'sequence/01_basic',
  'class/01_basic',
  'state/01_basic',
  'activity_new/01_basic',
  'usecase/01_basic',
  'component/01_basic',
  'mindmap/01_basic',
  'gantt/01_basic',
  'chart/01_basic',
];

async function init() {
  const base = siteBaseUrl();
  let manifest;
  try {
    manifest = await loadManifest(base);
  } catch (e) {
    console.error('home: failed to load manifest', e);
    return;
  }

  // Stat strip.
  for (const el of document.querySelectorAll('[data-stat]')) {
    const key = el.dataset.stat;
    if (key === 'examples') el.textContent = manifest.totals.examples.toLocaleString();
    if (key === 'families') el.textContent = manifest.totals.families.toLocaleString();
  }

  // Hero preview &mdash; inline the sequence/01_basic SVG.
  const heroRender = document.querySelector('[data-hero-render]');
  if (heroRender) {
    const hero = manifest.examples.find((e) => e.family === 'sequence' && e.name === '01_basic')
              || manifest.examples[0];
    if (hero) {
      try {
        const svgRes = await fetch(assetUrl(base, hero.svgPath));
        if (svgRes.ok) {
          heroRender.innerHTML = await svgRes.text();
        }
      } catch (e) {
        console.warn('home: hero render fetch failed', e);
      }
    }
  }

  // Featured gallery row.
  const grid = document.querySelector('[data-featured-gallery]');
  if (grid) {
    grid.innerHTML = '';
    const lookup = new Map(manifest.examples.map((e) => [`${e.family}/${e.name}`, e]));
    const picks = FEATURED.map((k) => lookup.get(k)).filter(Boolean);
    if (!picks.length) {
      grid.innerHTML = '<div class="empty">No examples available.</div>';
      return;
    }
    for (const item of picks) {
      const card = document.createElement('a');
      card.className = 'card';
      card.href = base.replace(/\/$/, '') + `/gallery/?open=${encodeURIComponent(item.family + '/' + item.name)}`;
      card.innerHTML = `
        <div class="thumb"><img loading="lazy" alt="${escapeHtml(item.title)} preview" src="${assetUrl(base, item.svgPath)}" /></div>
        <div class="meta">
          <span class="title">${escapeHtml(item.title)}</span>
          <span class="sub"><span class="tag">${escapeHtml(item.familyLabel)}</span></span>
          ${item.preview ? `<span class="preview">${escapeHtml(item.preview)}</span>` : ''}
        </div>`;
      grid.appendChild(card);
    }
  }
}

function escapeHtml(s) {
  return String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

init();
