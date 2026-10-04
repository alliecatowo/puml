//! Security regression tests for the agent-pack MCP server
//! (`agent-pack/bin/puml-mcp`): include_root confinement, allow_url_includes
//! wiring and output_path safety.
//!
//! The server is copied into a throwaway workspace so its ROOT (and anything
//! it writes) is the temp dir, never this checkout.

#![cfg(unix)]

use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Workspace {
    _tmp: tempfile::TempDir,
    root: PathBuf,
}

fn workspace() -> Workspace {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().join("ws");
    let bin_dir = root.join("agent-pack").join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("agent-pack/bin/puml-mcp"),
        bin_dir.join("puml-mcp"),
    )
    .unwrap();
    // A file outside the workspace that must stay unreachable.
    fs::write(tmp.path().join("outside.puml"), "A -> B : leaked\n").unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"keep\"\n").unwrap();
    let root = root.canonicalize().unwrap();
    Workspace { _tmp: tmp, root }
}

/// Call a tool through the legacy single-line protocol. None if python3 is missing.
fn call(ws: &Workspace, tool: &str, params: Value) -> Option<Value> {
    let mut child = match Command::new("python3")
        .arg(ws.root.join("agent-pack/bin/puml-mcp"))
        .env("PUML_MCP_PUML_BIN", env!("CARGO_BIN_EXE_puml"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return None,
    };
    let req = json!({"tool": tool, "params": params}).to_string();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("{req}\n").as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let line = String::from_utf8_lossy(&out.stdout);
    Some(serde_json::from_str(line.lines().next().unwrap_or("{}")).expect("json response"))
}

macro_rules! call_or_skip {
    ($ws:expr, $tool:expr, $params:expr) => {
        match call(&$ws, $tool, $params) {
            Some(v) => v,
            None => return,
        }
    };
}

fn text_of(v: &Value) -> String {
    v.to_string()
}

#[test]
fn include_root_outside_workspace_is_rejected() {
    let ws = workspace();
    let outside = ws.root.parent().unwrap();
    let resp = call_or_skip!(
        ws,
        "puml_render_svg",
        json!({
            "text": "@startuml\n!include outside.puml\n@enduml\n",
            "include_root": outside.to_string_lossy(),
        })
    );
    assert_eq!(resp["ok"], false, "{resp}");
    assert!(
        text_of(&resp).contains("path escapes workspace root"),
        "{resp}"
    );

    // Relative traversal is rejected as well.
    let resp = call_or_skip!(
        ws,
        "puml_check",
        json!({"text": "@startuml\nA -> B\n@enduml\n", "include_root": ".."})
    );
    assert_eq!(resp["ok"], false, "{resp}");
}

#[test]
fn include_root_inside_workspace_works_and_stays_confined() {
    let ws = workspace();
    fs::create_dir_all(ws.root.join("inc")).unwrap();
    fs::write(ws.root.join("inc/part.puml"), "A -> B : from-include\n").unwrap();
    let resp = call_or_skip!(
        ws,
        "puml_render_svg",
        json!({
            "text": "@startuml\n!include part.puml\n@enduml\n",
            "include_root": "inc",
        })
    );
    assert_eq!(resp["ok"], true, "{resp}");
    assert!(resp["svg"].as_str().unwrap().contains("from-include"));

    // An include escaping the (inside-workspace) root is refused by the CLI.
    fs::write(ws.root.join("top.puml"), "A -> B : top\n").unwrap();
    let resp = call_or_skip!(
        ws,
        "puml_check",
        json!({
            "text": "@startuml\n!include ../top.puml\n@enduml\n",
            "include_root": "inc",
        })
    );
    assert_eq!(resp["ok"], false, "{resp}");
    assert!(text_of(&resp).contains("E_INCLUDE_ESCAPE"), "{resp}");
}

#[test]
fn allow_url_includes_flag_is_forwarded() {
    let ws = workspace();
    let src = "@startuml\n!include http://127.0.0.1:9/never.puml\n@enduml\n";

    let resp = call_or_skip!(ws, "puml_check", json!({"text": src}));
    assert_eq!(resp["ok"], false);
    assert!(text_of(&resp).contains("E_INCLUDE_URL_DISABLED"), "{resp}");

    let resp = call_or_skip!(
        ws,
        "puml_check",
        json!({"text": src, "allow_url_includes": false})
    );
    assert!(text_of(&resp).contains("E_INCLUDE_URL_DISABLED"), "{resp}");

    // When true the include is attempted (and fails on the closed port),
    // instead of being rejected as disabled.
    let resp = call_or_skip!(
        ws,
        "puml_check",
        json!({"text": src, "allow_url_includes": true})
    );
    assert_eq!(resp["ok"], false);
    let body = text_of(&resp);
    assert!(!body.contains("E_INCLUDE_URL_DISABLED"), "{resp}");
    assert!(body.contains("E_INCLUDE_URL_FETCH"), "{resp}");
}

#[test]
fn file_url_includes_are_confined_even_when_urls_allowed() {
    let ws = workspace();
    let outside = ws.root.parent().unwrap().join("outside.puml");
    fs::create_dir_all(ws.root.join("inc")).unwrap();
    let resp = call_or_skip!(
        ws,
        "puml_check",
        json!({
            "text": format!("@startuml\n!include file://{}\n@enduml\n", outside.display()),
            "include_root": "inc",
            "allow_url_includes": true,
        })
    );
    assert_eq!(resp["ok"], false, "{resp}");
    assert!(text_of(&resp).contains("E_INCLUDE_ESCAPE"), "{resp}");
}

#[test]
fn output_path_must_stay_in_workspace_and_be_a_generated_artifact() {
    let ws = workspace();
    let src = "@startuml\nA -> B\n@enduml\n";
    let keep = fs::read_to_string(ws.root.join("Cargo.toml")).unwrap();

    for bad in [
        "../escape.svg",
        "/tmp/abs-escape.svg",
        "Cargo.toml",
        "agent-pack/bin/puml-mcp",
        ".git/config",
        ".git/hooks/pre-commit.svg",
        "out.sh",
    ] {
        for tool in ["puml_render_file", "puml_render_png"] {
            let resp = call_or_skip!(ws, tool, json!({"text": src, "output_path": bad}));
            assert_eq!(resp["ok"], false, "{tool} {bad}: {resp}");
            assert!(resp.get("error").is_some(), "{tool} {bad}: {resp}");
        }
    }
    assert_eq!(
        fs::read_to_string(ws.root.join("Cargo.toml")).unwrap(),
        keep
    );
    assert!(fs::read_to_string(ws.root.join("agent-pack/bin/puml-mcp"))
        .unwrap()
        .starts_with("#!/usr/bin/env python3"));
    assert!(!ws.root.join(".git").exists());

    let resp = call_or_skip!(
        ws,
        "puml_render_file",
        json!({"text": src, "output_path": "out/diagram.svg"})
    );
    assert_eq!(resp["ok"], true, "{resp}");
    assert!(ws.root.join("out/diagram.svg").is_file());
}

#[test]
fn source_path_must_stay_in_workspace() {
    let ws = workspace();
    let resp = call_or_skip!(ws, "puml_check", json!({"path": "../outside.puml"}));
    assert_eq!(resp["ok"], false, "{resp}");
    assert!(
        text_of(&resp).contains("path escapes workspace root"),
        "{resp}"
    );
}
