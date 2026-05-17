// Tiny shared loader for the examples manifest, with caching across pages.

let _cache = null;

export async function loadManifest(baseUrl = '') {
  if (_cache) return _cache;
  const url = baseUrl.replace(/\/$/, '') + '/examples-index.json';
  const res = await fetch(url, { cache: 'no-cache' });
  if (!res.ok) throw new Error(`Failed to load manifest at ${url}: ${res.status}`);
  _cache = await res.json();
  return _cache;
}

// Normalize a chunk of `.puml` source the same way the build script does, so
// editor input lookups against manifest hashes are stable.
export function normalizeSource(text) {
  return text
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .map((l) => l.replace(/\s+$/, ''))
    .join('\n')
    .replace(/\n+$/, '');
}

// FNV-1a-ish via SubtleCrypto SHA-256; returns hex truncated to 16 chars to
// match the build-script's hash slice.
export async function hashSource(text) {
  const enc = new TextEncoder().encode(normalizeSource(text));
  const buf = await crypto.subtle.digest('SHA-256', enc);
  const bytes = new Uint8Array(buf);
  let hex = '';
  for (let i = 0; i < 8; i++) {
    hex += bytes[i].toString(16).padStart(2, '0');
  }
  return hex;
}

// Build absolute URL for a manifest-relative path (e.g. "examples/sequence/01_basic.svg").
export function assetUrl(baseUrl, relPath) {
  return baseUrl.replace(/\/$/, '') + '/' + relPath.replace(/^\//, '');
}

// Best-effort sniff of the base URL the site is served from. Works for both
// project pages (/puml/) and a root deploy (/).
export function siteBaseUrl() {
  const meta = document.querySelector('meta[name="site-base"]');
  if (meta && meta.content) return meta.content;
  // Fall back to dirname of the current document.
  const p = window.location.pathname;
  // Strip trailing filename if any.
  const dir = p.endsWith('/') ? p : p.replace(/[^/]*$/, '');
  return dir.replace(/\/(editor|gallery|guide|developer)\/?.*$/, '/');
}
