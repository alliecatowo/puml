use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};

fn frame(message: Value) -> Vec<u8> {
    let body = serde_json::to_vec(&message).expect("json body");
    let mut framed = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    framed.extend(body);
    framed
}

fn run_lsp(messages: Vec<Value>) -> Vec<Value> {
    let mut child = Command::new(cargo_bin("puml-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn puml-lsp");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for message in messages {
            stdin.write_all(&frame(message)).expect("write lsp frame");
        }
    }

    let output = child.wait_with_output().expect("lsp output");
    assert!(
        output.status.success(),
        "puml-lsp failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    parse_frames(&output.stdout)
}

fn parse_frames(mut bytes: &[u8]) -> Vec<Value> {
    let mut messages = Vec::new();
    while !bytes.is_empty() {
        let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
            break;
        };
        let header = std::str::from_utf8(&bytes[..header_end]).expect("utf8 header");
        let len = header
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length:"))
            .expect("content length")
            .trim()
            .parse::<usize>()
            .expect("length number");
        let body_start = header_end + 4;
        let body_end = body_start + len;
        messages.push(serde_json::from_slice(&bytes[body_start..body_end]).expect("json frame"));
        bytes = &bytes[body_end..];
    }
    messages
}

fn request_result(messages: &[Value], id: i64) -> &Value {
    messages
        .iter()
        .find(|message| message.get("id").and_then(Value::as_i64) == Some(id))
        .and_then(|message| message.get("result"))
        .unwrap_or_else(|| panic!("missing result for id {id}: {messages:#?}"))
}

fn open_doc_message(uri: &str, text: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": uri,
                "languageId": "puml",
                "version": 1,
                "text": text
            }
        }
    })
}

fn completion_request(id: i64, uri: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 0}
        }
    })
}

fn hover_request(id: i64, uri: &str, line: u64, character: u64) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "textDocument/hover",
        "params": {
            "textDocument": {"uri": uri},
            "position": {"line": line, "character": character}
        }
    })
}

fn lsp_round_trip_for_hover_and_completion(
    uri: &str,
    text: &str,
    hover_line: u64,
    hover_character: u64,
) -> Vec<Value> {
    run_lsp(vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        open_doc_message(uri, text),
        completion_request(2, uri),
        hover_request(3, uri, hover_line, hover_character),
        json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ])
}

fn completion_labels(result: &Value) -> Vec<&str> {
    result["items"]
        .as_array()
        .expect("completion items")
        .iter()
        .filter_map(|item| item["label"].as_str())
        .collect()
}

fn hover_markdown(result: &Value) -> &str {
    result["contents"]["value"]
        .as_str()
        .expect("hover markdown")
}

#[test]
fn sequence_completion_and_hover_are_static_but_available() {
    let messages = lsp_round_trip_for_hover_and_completion(
        "file:///sequence.puml",
        "@startuml\nA --> B: hi\n@enduml\n",
        1,
        3,
    );

    let labels = completion_labels(request_result(&messages, 2));
    assert!(labels.contains(&"participant"));
    assert!(labels.contains(&"-->>"));
    assert!(hover_markdown(request_result(&messages, 3)).contains("Dashed message arrow"));
}

#[test]
fn class_completion_and_hover_cover_current_static_keyword_surface() {
    let messages = lsp_round_trip_for_hover_and_completion(
        "file:///class.puml",
        "@startuml\nclass User\n@enduml\n",
        1,
        1,
    );

    let labels = completion_labels(request_result(&messages, 2));
    assert!(labels.contains(&"class"));
    assert!(labels.contains(&"interface"));
    assert!(hover_markdown(request_result(&messages, 3)).contains("Declare a class node"));
}

#[test]
fn activity_completion_and_hover_cover_current_static_keyword_surface() {
    let messages = lsp_round_trip_for_hover_and_completion(
        "file:///activity.puml",
        "@startuml\nstart\n:Work;\nstop\n@enduml\n",
        1,
        1,
    );

    let labels = completion_labels(request_result(&messages, 2));
    assert!(labels.contains(&"start"));
    assert!(labels.contains(&"endif"));
    assert!(hover_markdown(request_result(&messages, 3)).contains("Start an activity diagram flow"));
}

#[test]
fn gantt_json_and_yaml_do_not_have_family_specific_completion_or_hover_claims_yet() {
    let cases = [
        (
            "file:///gantt.puml",
            "@startgantt\nProject starts 2026-05-18\n@endgantt\n",
            1,
            1,
            "`Project`",
        ),
        (
            "file:///json.puml",
            "@startjson\n{\"name\":\"puml\"}\n@endjson\n",
            1,
            3,
            "`name`",
        ),
        (
            "file:///yaml.puml",
            "@startyaml\nname: puml\n@endyaml\n",
            1,
            1,
            "`name`",
        ),
    ];

    for (uri, source, hover_line, hover_character, expected_hover) in cases {
        let messages =
            lsp_round_trip_for_hover_and_completion(uri, source, hover_line, hover_character);
        let labels = completion_labels(request_result(&messages, 2));
        assert!(
            !labels.contains(&"Project starts"),
            "gantt-specific completions should not be advertised yet"
        );
        assert!(
            !labels.contains(&"json path"),
            "JSON-specific completions should not be advertised yet"
        );
        assert!(
            !labels.contains(&"yaml path"),
            "YAML-specific completions should not be advertised yet"
        );
        assert_eq!(hover_markdown(request_result(&messages, 3)), expected_hover);
    }
}
