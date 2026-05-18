/// Integration tests for URL include support (`!include https://...`).
///
/// These tests cover:
/// - `file://` URL shortcut resolution (no real network needed)
/// - HTTP fetch via a local httpmock server (no real network needed)
/// - Cache hit on second render produces byte-identical SVG with no second network call
/// - Redirects and oversized HTTP responses are rejected before caching
/// - `--no-url-includes` CLI flag produces E_INCLUDE_URL_DISABLED diagnostic
use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

// ---------------------------------------------------------------------------
// file:// URL resolution
// ---------------------------------------------------------------------------

/// A `!include file:///path/to/file.puml` should read from the local filesystem.
#[test]
fn include_file_url_resolves_from_local_filesystem() {
    let dir = tempdir().unwrap();
    let child = dir.path().join("child.puml");
    fs::write(&child, "A -> B : from_file_url\n").unwrap();

    let file_url = format!("file://{}", child.display());
    let src = format!("@startuml\n!include {file_url}\n@enduml\n");
    let main = dir.path().join("main.puml");
    fs::write(&main, &src).unwrap();

    Command::cargo_bin("puml")
        .unwrap()
        .arg("--check")
        .arg(&main)
        .assert()
        .success();
}

// ---------------------------------------------------------------------------
// HTTP fetch via local httpmock server
// ---------------------------------------------------------------------------

/// Helper: write a .puml fixture file that uses `!include <url>`.
fn write_include_fixture(dir: &std::path::Path, url: &str) -> std::path::PathBuf {
    let path = dir.join("diagram.puml");
    fs::write(&path, format!("@startuml\n!include {url}\n@enduml\n")).unwrap();
    path
}

#[test]
fn include_http_url_fetches_content_and_renders() {
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/diagram.puml");
        then.status(200)
            .header("content-type", "text/plain")
            .body("A -> B : fetched\n");
    });

    let url = server.url("/diagram.puml");
    let dir = tempdir().unwrap();

    // Clear any cache that might exist for this URL so we get a fresh fetch
    let cache_path = url_cache_path_for(&url);
    if let Some(ref p) = cache_path {
        let _ = fs::remove_file(p);
    }

    let fixture = write_include_fixture(dir.path(), &url);

    Command::cargo_bin("puml")
        .unwrap()
        .arg("--check")
        .arg(&fixture)
        .assert()
        .success();

    // Verify mock was hit
    _mock.assert_hits(1);
}

#[test]
fn include_http_url_cache_hit_no_second_network_call() {
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/cached.puml");
        then.status(200)
            .header("content-type", "text/plain")
            .body("A -> B : cached_content\n");
    });

    let url = server.url("/cached.puml");
    let dir = tempdir().unwrap();

    // Clear cache for fresh start
    let cache_path = url_cache_path_for(&url);
    if let Some(ref p) = cache_path {
        let _ = fs::remove_file(p);
    }

    let fixture = write_include_fixture(dir.path(), &url);
    let output1 = dir.path().join("out1.svg");
    let output2 = dir.path().join("out2.svg");

    // First render — should fetch from network
    Command::cargo_bin("puml")
        .unwrap()
        .args(["-o", output1.to_str().unwrap()])
        .arg(&fixture)
        .assert()
        .success();

    // Mock should have been called once
    _mock.assert_hits(1);

    let svg1 = fs::read_to_string(&output1).unwrap();

    // Second render — should serve from cache, not from network
    Command::cargo_bin("puml")
        .unwrap()
        .args(["-o", output2.to_str().unwrap()])
        .arg(&fixture)
        .assert()
        .success();

    // Still only 1 network call
    _mock.assert_hits(1);

    let svg2 = fs::read_to_string(&output2).unwrap();
    assert_eq!(
        svg1, svg2,
        "second render should be byte-identical to first"
    );
}

#[test]
fn include_http_404_produces_clear_diagnostic() {
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/missing.puml");
        then.status(404).body("Not Found");
    });

    let url = server.url("/missing.puml");
    let dir = tempdir().unwrap();

    // Clear cache
    let cache_path = url_cache_path_for(&url);
    if let Some(ref p) = cache_path {
        let _ = fs::remove_file(p);
    }

    let fixture = write_include_fixture(dir.path(), &url);

    Command::cargo_bin("puml")
        .unwrap()
        .arg("--check")
        .arg(&fixture)
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("E_INCLUDE_URL_FETCH").or(
                // ureq returns a non-2xx error that we surface
                predicate::str::contains("404")
                    .or(predicate::str::contains("fetch"))
                    .or(predicate::str::contains("HTTP")),
            ),
        );
}

#[test]
fn include_http_redirect_is_rejected_without_following_location() {
    let server = MockServer::start();

    let redirect_mock = server.mock(|when, then| {
        when.method(GET).path("/redirect.puml");
        then.status(302)
            .header("location", "/target.puml")
            .body("redirecting");
    });

    let target_mock = server.mock(|when, then| {
        when.method(GET).path("/target.puml");
        then.status(200).body("A -> B : should_not_fetch\n");
    });

    let url = server.url("/redirect.puml");
    let dir = tempdir().unwrap();

    let cache_path = url_cache_path_for(&url);
    if let Some(ref p) = cache_path {
        let _ = fs::remove_file(p);
    }

    let fixture = write_include_fixture(dir.path(), &url);

    Command::cargo_bin("puml")
        .unwrap()
        .arg("--check")
        .arg(&fixture)
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_INCLUDE_URL_REDIRECT"));

    redirect_mock.assert_hits(1);
    target_mock.assert_hits(0);
}

#[test]
fn include_http_response_larger_than_cap_is_rejected() {
    let server = MockServer::start();
    let body = "A".repeat(1024 * 1024 + 1);

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/large.puml");
        then.status(200)
            .header("content-type", "text/plain")
            .body(body);
    });

    let url = server.url("/large.puml");
    let dir = tempdir().unwrap();

    let cache_path = url_cache_path_for(&url);
    if let Some(ref p) = cache_path {
        let _ = fs::remove_file(p);
    }

    let fixture = write_include_fixture(dir.path(), &url);

    Command::cargo_bin("puml")
        .unwrap()
        .arg("--check")
        .arg(&fixture)
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_INCLUDE_URL_TOO_LARGE"));

    _mock.assert_hits(1);
}

// ---------------------------------------------------------------------------
// --no-url-includes flag
// ---------------------------------------------------------------------------

#[test]
fn no_url_includes_flag_produces_disabled_diagnostic() {
    let server = MockServer::start();

    let _mock = server.mock(|when, then| {
        when.method(GET).path("/lib.puml");
        then.status(200).body("A -> B : lib\n");
    });

    let url = server.url("/lib.puml");
    let dir = tempdir().unwrap();
    let fixture = write_include_fixture(dir.path(), &url);

    Command::cargo_bin("puml")
        .unwrap()
        .arg("--check")
        .arg("--no-url-includes")
        .arg(&fixture)
        .assert()
        .failure()
        .stderr(predicate::str::contains("E_INCLUDE_URL_DISABLED"));

    // Network should NOT have been called
    _mock.assert_hits(0);
}

// ---------------------------------------------------------------------------
// Helper to compute the cache path for a URL (mirrors parser logic)
// ---------------------------------------------------------------------------

fn url_cache_path_for(url: &str) -> Option<std::path::PathBuf> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hex::encode(hasher.finalize());

    let cache_base = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))?;

    Some(cache_base.join("puml").join("includes").join(hash))
}
