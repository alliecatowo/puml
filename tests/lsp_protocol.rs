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

fn execute_command_request(id: i64, command: &str, arguments: Vec<Value>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "workspace/executeCommand",
        "params": {
            "command": command,
            "arguments": arguments
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
fn workspace_commands_route_render_scene_export_and_explain_diagnostic() {
    let uri = "file:///commands.puml";
    let messages = run_lsp(vec![
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        open_doc_message(uri, "@startuml\nAlice -> Bob: hi\n@enduml\n"),
        execute_command_request(2, "puml.renderScene", vec![json!(uri)]),
        execute_command_request(3, "puml.export", vec![json!(uri), json!({"format": "svg"})]),
        execute_command_request(
            4,
            "puml.explainDiagnostic",
            vec![json!({
                "code": "E_ARROW_INVALID",
                "message": "[E_ARROW_INVALID] invalid arrow syntax",
                "range": {
                    "start": {"line": 1, "character": 6},
                    "end": {"line": 1, "character": 8}
                }
            })],
        ),
        execute_command_request(5, "puml.languageService", vec![]),
        json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":null}),
        json!({"jsonrpc":"2.0","method":"exit","params":null}),
    ]);

    let scene = request_result(&messages, 2);
    assert_eq!(scene["schema"], "puml.renderScene");
    assert_eq!(scene["schemaVersion"], 1);
    assert_eq!(scene["model"]["kind"], "Sequence");
    assert_eq!(scene["scene"]["kind"], "Sequence");
    assert_eq!(scene["scene"]["pageCount"], 1);
    assert_eq!(scene["scene"]["pages"][0]["participants"][0]["id"], "Alice");
    assert_eq!(scene["diagnostics"], json!([]));

    let exported = request_result(&messages, 3);
    assert_eq!(exported["schema"], "puml.export");
    assert_eq!(exported["format"], "svg");
    assert_eq!(exported["mediaType"], "image/svg+xml");
    assert_eq!(exported["encoding"], "utf-8");
    assert!(exported["content"]
        .as_str()
        .expect("svg content")
        .contains("<svg"));
    assert_eq!(exported["pages"][0]["name"], "diagram-1.svg");
    assert_eq!(exported["diagnostics"], json!([]));

    let explained = request_result(&messages, 4);
    assert_eq!(explained["schema"], "puml.explainDiagnostic");
    assert_eq!(explained["diagnostic"]["code"], "E_ARROW_INVALID");
    assert!(explained["explanation"]["summary"]
        .as_str()
        .expect("summary")
        .contains("arrow"));
    assert_eq!(explained["diagnostics"], json!([]));

    let surface = request_result(&messages, 5);
    assert_eq!(surface["schema"], "puml.languageService");
    assert!(surface["completion"]["items"]
        .as_array()
        .expect("completion items")
        .iter()
        .any(|item| item["label"] == "component"));
}

#[test]
fn workspace_command_json_shapes_are_deterministic() {
    let uri = "file:///deterministic-commands.puml";
    let run = || {
        run_lsp(vec![
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            open_doc_message(
                uri,
                "@startuml\nparticipant Alice\nAlice -> Bob: hi\n@enduml\n",
            ),
            execute_command_request(2, "puml.renderScene", vec![json!(uri)]),
            execute_command_request(
                3,
                "puml.export",
                vec![json!(uri), json!({"format": "html"})],
            ),
            json!({"jsonrpc":"2.0","id":4,"method":"shutdown","params":null}),
            json!({"jsonrpc":"2.0","method":"exit","params":null}),
        ])
    };

    let first = run();
    let second = run();
    assert_eq!(request_result(&first, 2), request_result(&second, 2));
    assert_eq!(request_result(&first, 3), request_result(&second, 3));
    assert_eq!(
        request_result(&first, 3)
            .as_object()
            .expect("export response")
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            "content",
            "contentBase64",
            "diagnostics",
            "encoding",
            "format",
            "mediaType",
            "model",
            "pages",
            "scene",
            "schema",
            "schemaVersion"
        ]
    );
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
    assert!(labels.contains(&"!theme"));
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
    assert!(labels.contains(&"<|--"));
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
    assert!(labels.contains(&"fork"));
    assert!(labels.contains(&"while"));
    assert!(labels.contains(&"endif"));
    assert!(hover_markdown(request_result(&messages, 3)).contains("Start an activity diagram flow"));
}

#[test]
fn skinparam_hover_exposes_supported_value_type_annotations() {
    let messages = lsp_round_trip_for_hover_and_completion(
        "file:///style.puml",
        "@startuml\nskinparam ArrowColor #334155\nA -> B\n@enduml\n",
        1,
        12,
    );

    let hover = hover_markdown(request_result(&messages, 3));
    assert!(hover.contains("`ArrowColor`"));
    assert!(hover.contains("Value type: color"));
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
