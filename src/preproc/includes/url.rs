#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use std::fs;
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use std::io::Read;

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use sha2::{Digest, Sha256};

use crate::diagnostic::Diagnostic;
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use crate::preproc::{URL_INCLUDE_MAX_BYTES, URL_INCLUDE_TIMEOUT};

/// Resolve the on-disk cache path for a URL include.
/// Uses `~/.cache/puml/includes/<sha256-of-url>`.
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn url_cache_path(url: &str) -> Option<std::path::PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hex::encode(hasher.finalize());

    let cache_base = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))?;

    Some(cache_base.join("puml").join("includes").join(hash))
}

/// Fetch a URL include, using a local disk cache keyed by SHA-256 of the URL.
/// Returns the fetched content as a string.
/// Handles `file://` URLs by reading from the local filesystem directly.
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
pub(in crate::preproc) fn fetch_url_include(url: &str) -> Result<String, Diagnostic> {
    // Handle file:// URLs by stripping the scheme and reading from the local fs.
    if url.to_ascii_lowercase().starts_with("file://") {
        let path_str = &url["file://".len()..];
        return fs::read_to_string(path_str).map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!("failed to read file URL '{}': {e}", url),
            )
        });
    }

    // Check cache first.
    if let Some(cache_path) = url_cache_path(url) {
        if cache_path.exists() {
            return fs::read_to_string(&cache_path).map_err(|e| {
                Diagnostic::error_code(
                    "E_INCLUDE_URL_CACHE_READ",
                    format!("failed to read cache for '{}': {e}", url),
                )
            });
        }

        // Fetch via HTTP(S).
        let content = fetch_http_url_include(url)?;

        // Write to cache (best-effort; failures are non-fatal).
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&cache_path, &content);

        Ok(content)
    } else {
        // No cache path available; fetch directly without caching.
        fetch_http_url_include(url)
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn fetch_http_url_include(url: &str) -> Result<String, Diagnostic> {
    let response = ureq::builder()
        .redirects(0)
        .timeout_connect(URL_INCLUDE_TIMEOUT)
        .timeout_read(URL_INCLUDE_TIMEOUT)
        .timeout_write(URL_INCLUDE_TIMEOUT)
        .build()
        .get(url)
        .call()
        .map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!("failed to fetch '{}': {e}", url),
            )
        })?;

    if (300..400).contains(&response.status()) {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_URL_REDIRECT",
            format!(
                "redirects are not followed for URL include '{}': HTTP {} {}",
                url,
                response.status(),
                response.status_text()
            ),
        ));
    }

    if response.status() < 200 || response.status() >= 300 {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_URL_FETCH",
            format!(
                "HTTP {} fetching '{}': {}",
                response.status(),
                url,
                response.status_text()
            ),
        ));
    }

    read_limited_url_include_body(url, response)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn read_limited_url_include_body(
    url: &str,
    response: ureq::Response,
) -> Result<String, Diagnostic> {
    if let Some(length) = response
        .header("content-length")
        .and_then(|value| value.parse::<usize>().ok())
    {
        if length > URL_INCLUDE_MAX_BYTES {
            return Err(url_include_too_large(url, length));
        }
    }

    let mut bytes = Vec::new();
    let mut reader = response
        .into_reader()
        .take((URL_INCLUDE_MAX_BYTES + 1) as u64);
    reader.read_to_end(&mut bytes).map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_URL_FETCH",
            format!("failed to read response body from '{}': {e}", url),
        )
    })?;

    if bytes.len() > URL_INCLUDE_MAX_BYTES {
        return Err(url_include_too_large(url, bytes.len()));
    }

    String::from_utf8(bytes).map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_URL_FETCH",
            format!("failed to decode response body from '{}': {e}", url),
        )
    })
}

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn url_include_too_large(url: &str, bytes: usize) -> Diagnostic {
    Diagnostic::error_code(
        "E_INCLUDE_URL_TOO_LARGE",
        format!(
            "URL include '{}' is too large: {bytes} bytes exceeds the {URL_INCLUDE_MAX_BYTES} byte limit",
            url
        ),
    )
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "url-includes")))]
#[allow(dead_code)]
pub(in crate::preproc) fn fetch_url_include(url: &str) -> Result<String, Diagnostic> {
    Err(Diagnostic::error_code(
        "E_INCLUDE_URL_UNSUPPORTED",
        format!("URL includes are not supported in this build: {url}"),
    ))
}
