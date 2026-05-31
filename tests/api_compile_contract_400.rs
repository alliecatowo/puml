//! Contract tests for the stable compile API and worker protocol (issue #400).
//!
//! These tests lock the JSON shape of [`CompileResult`] and the worker
//! protocol message types. Any change to field names, types, or the `ok`/
//! `family`/`svg_pages`/`diagnostics`/`model_summary`/`semantic_tokens`/
//! `symbols`/`language_service` structure must be deliberate and reflected
//! here.
//!
//! # What this covers
//!
//! - [`puml::compile`] returns a fully-typed, serde-serialisable struct.
//! - The `ok` flag is `true` for valid source and `false` for parse errors.
//! - `family` is set to the detected diagram family string.
//! - `svg_pages` is non-empty for valid source and empty when `ok` is `false`.
//! - `diagnostics`, `semantic_tokens`, and `symbols` have the correct shape.
//! - `language_service` surface flags are all `true` by default.
//! - Worker protocol round-trips (request → dispatch → response) produce the
//!   expected `op` tag and result.
//! - JSON serialisation of all types is stable (no renames, no removals).

use puml::worker::{
    dispatch, CompileRequest, CompletionRequest, DiagnosticsRequest, HoverRequest, RenderRequest,
    SemanticTokensRequest, WorkerRequest, WorkerRequestPayload, WorkerResponsePayload,
};
use puml::{compile, CompileResult, DiagnosticDto, ModelSummary, SpanDto};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const SEQUENCE_SOURCE: &str = "@startuml\nAlice -> Bob: hello\n@enduml\n";
const INVALID_SOURCE: &str = "@startuml\n!!!invalid syntax!!!\n@enduml\n";

// ---------------------------------------------------------------------------
// compile() API contract
// ---------------------------------------------------------------------------

#[test]
fn compile_returns_typed_struct_not_just_json() {
    let result: CompileResult = compile(SEQUENCE_SOURCE).expect("compile must not fail");

    // Top-level shape
    assert!(result.ok, "valid source must produce ok=true");
    assert_eq!(result.family, "sequence");
    assert!(
        !result.svg_pages.is_empty(),
        "must have at least one SVG page"
    );
    assert!(result.svg_pages[0].contains("<svg"), "page must be SVG");

    // Model summary
    assert_eq!(result.model_summary.kind, "sequence");
    assert!(
        result.model_summary.node_count >= 2,
        "at least Alice and Bob must be counted"
    );
    assert!(
        result.model_summary.edge_count >= 1,
        "at least one message/event must be counted"
    );

    // Language service surface
    assert!(result.language_service.hover);
    assert!(result.language_service.completion);
    assert!(result.language_service.diagnostics);
    assert!(result.language_service.semantic_tokens);
    assert!(result.language_service.document_symbols);
    assert!(result.language_service.formatting);
    assert!(result.language_service.definition);
    assert!(result.language_service.references);
    assert!(result.language_service.rename);
}

#[test]
fn compile_ok_is_false_for_invalid_source() {
    let result = compile(INVALID_SOURCE).expect("compile itself must not return Err");

    assert!(!result.ok, "invalid source must produce ok=false");
    assert!(
        result.svg_pages.is_empty(),
        "no SVG pages must be produced for invalid source"
    );
    assert!(
        !result.diagnostics.is_empty(),
        "at least one diagnostic must be present"
    );
    let has_error = result.diagnostics.iter().any(|d| d.severity == "error");
    assert!(has_error, "at least one error-severity diagnostic required");
}

#[test]
fn compile_diagnostics_have_stable_field_shape() {
    let result = compile(INVALID_SOURCE).expect("compile must not Err");

    for diag in &result.diagnostics {
        // Required string fields must be non-empty
        assert!(!diag.severity.is_empty(), "severity must be set");
        assert!(!diag.category.is_empty(), "category must be set");
        assert!(!diag.message.is_empty(), "message must be set");

        // severity is one of the two allowed values
        assert!(
            diag.severity == "error" || diag.severity == "warning",
            "severity must be 'error' or 'warning', got '{}'",
            diag.severity
        );
    }
}

#[test]
fn compile_semantic_tokens_have_stable_field_shape() {
    let result = compile(SEQUENCE_SOURCE).expect("compile must not Err");

    // There may be zero tokens for simple source, but if present they must
    // conform to the contract.
    for token in &result.semantic_tokens {
        assert!(token.end >= token.start, "end must be >= start");
        assert!(
            token.kind == "keyword" || token.kind == "operator",
            "token kind must be 'keyword' or 'operator', got '{}'",
            token.kind
        );
    }
}

#[test]
fn compile_symbols_have_stable_field_shape() {
    let result = compile(SEQUENCE_SOURCE).expect("compile must not Err");

    // Alice is a participant; Alice -> Bob message is a message symbol.
    assert!(
        !result.symbols.is_empty(),
        "at least one symbol must be present for sequence diagram"
    );
    for sym in &result.symbols {
        assert!(!sym.name.is_empty(), "symbol name must be non-empty");
        assert!(!sym.kind.is_empty(), "symbol kind must be non-empty");
        assert!(
            sym.span.end >= sym.span.start,
            "symbol span end must be >= start"
        );
    }
    let participant = result
        .symbols
        .iter()
        .find(|s| s.kind == "participant" && s.name == "Bob");
    // Bob appears via the message even without an explicit participant
    // declaration; it may or may not be in the symbol list depending on how
    // document_symbols handles implicit participants.
    let message = result.symbols.iter().find(|s| s.kind == "message");
    assert!(
        message.is_some(),
        "at least one message symbol must be present"
    );
    let _ = participant; // presence is optional
}

// ---------------------------------------------------------------------------
// JSON serialisation shape contract
// ---------------------------------------------------------------------------

#[test]
fn compile_result_serialises_to_expected_json_keys() {
    let result = compile(SEQUENCE_SOURCE).expect("compile must not Err");
    let json_str = serde_json::to_string(&result).expect("must serialise");
    let json: serde_json::Value = serde_json::from_str(&json_str).expect("must deserialise");

    // Top-level keys
    let obj = json.as_object().expect("must be an object");
    assert!(obj.contains_key("ok"), "missing key: ok");
    assert!(obj.contains_key("family"), "missing key: family");
    assert!(obj.contains_key("svg_pages"), "missing key: svg_pages");
    assert!(obj.contains_key("diagnostics"), "missing key: diagnostics");
    assert!(
        obj.contains_key("model_summary"),
        "missing key: model_summary"
    );
    assert!(
        obj.contains_key("semantic_tokens"),
        "missing key: semantic_tokens"
    );
    assert!(obj.contains_key("symbols"), "missing key: symbols");
    assert!(
        obj.contains_key("language_service"),
        "missing key: language_service"
    );

    // model_summary sub-keys
    let summary = &json["model_summary"];
    let summary_obj = summary
        .as_object()
        .expect("model_summary must be an object");
    assert!(
        summary_obj.contains_key("kind"),
        "missing key: model_summary.kind"
    );
    assert!(
        summary_obj.contains_key("warning_count"),
        "missing key: model_summary.warning_count"
    );
    assert!(
        summary_obj.contains_key("node_count"),
        "missing key: model_summary.node_count"
    );
    assert!(
        summary_obj.contains_key("edge_count"),
        "missing key: model_summary.edge_count"
    );

    // language_service sub-keys
    let ls = &json["language_service"];
    let ls_obj = ls.as_object().expect("language_service must be an object");
    for key in &[
        "hover",
        "completion",
        "diagnostics",
        "semantic_tokens",
        "document_symbols",
        "formatting",
        "definition",
        "references",
        "rename",
    ] {
        assert!(
            ls_obj.contains_key(*key),
            "missing language_service key: {}",
            key
        );
    }
}

#[test]
fn compile_result_round_trips_through_json() {
    let original = compile(SEQUENCE_SOURCE).expect("compile must not Err");
    let json_str = serde_json::to_string(&original).expect("must serialise");
    let recovered: CompileResult = serde_json::from_str(&json_str).expect("must deserialise back");

    assert_eq!(original.ok, recovered.ok);
    assert_eq!(original.family, recovered.family);
    assert_eq!(original.svg_pages.len(), recovered.svg_pages.len());
    assert_eq!(original.diagnostics.len(), recovered.diagnostics.len());
    assert_eq!(
        original.model_summary.node_count,
        recovered.model_summary.node_count
    );
    assert_eq!(original.symbols.len(), recovered.symbols.len());
    assert_eq!(
        original.semantic_tokens.len(),
        recovered.semantic_tokens.len()
    );
}

// ---------------------------------------------------------------------------
// Worker protocol contract
// ---------------------------------------------------------------------------

#[test]
fn worker_compile_request_dispatches_and_returns_compile_payload() {
    let req = WorkerRequest {
        id: "req-001".to_string(),
        version: 1,
        payload: WorkerRequestPayload::Compile(CompileRequest {
            source: SEQUENCE_SOURCE.to_string(),
            frontend: None,
        }),
    };

    let resp = dispatch(req);

    assert!(resp.ok, "dispatch must succeed");
    assert_eq!(resp.id, "req-001");
    assert_eq!(resp.version, 1);

    match resp.payload.expect("payload must be present") {
        WorkerResponsePayload::Compile(result) => {
            assert!(result.ok);
            assert_eq!(result.family, "sequence");
            assert!(!result.svg_pages.is_empty());
        }
        other => panic!("expected Compile payload, got {:?}", other),
    }
}

#[test]
fn worker_render_request_dispatches_and_returns_svg_pages() {
    let req = WorkerRequest {
        id: "req-002".to_string(),
        version: 1,
        payload: WorkerRequestPayload::Render(RenderRequest {
            source: SEQUENCE_SOURCE.to_string(),
            format: "svg".to_string(),
        }),
    };

    let resp = dispatch(req);

    assert!(resp.ok);
    match resp.payload.expect("payload must be present") {
        WorkerResponsePayload::Render(r) => {
            assert!(!r.svg_pages.is_empty(), "must have SVG pages");
            assert!(r.svg_pages[0].contains("<svg"));
        }
        other => panic!("expected Render payload, got {:?}", other),
    }
}

#[test]
fn worker_hover_request_returns_hover_payload() {
    let req = WorkerRequest {
        id: "req-003".to_string(),
        version: 1,
        payload: WorkerRequestPayload::Hover(HoverRequest {
            source: SEQUENCE_SOURCE.to_string(),
            line: 1,
            column: 1,
        }),
    };

    let resp = dispatch(req);

    assert!(resp.ok);
    match resp.payload.expect("payload must be present") {
        WorkerResponsePayload::Hover(_) => {}
        other => panic!("expected Hover payload, got {:?}", other),
    }
}

#[test]
fn worker_diagnostics_request_returns_diagnostics_payload() {
    let req = WorkerRequest {
        id: "req-004".to_string(),
        version: 1,
        payload: WorkerRequestPayload::Diagnostics(DiagnosticsRequest {
            source: INVALID_SOURCE.to_string(),
        }),
    };

    let resp = dispatch(req);

    assert!(resp.ok, "dispatch must succeed even for invalid source");
    match resp.payload.expect("payload must be present") {
        WorkerResponsePayload::Diagnostics(d) => {
            assert!(
                !d.diagnostics.is_empty(),
                "must have diagnostics for invalid source"
            );
        }
        other => panic!("expected Diagnostics payload, got {:?}", other),
    }
}

#[test]
fn worker_semantic_tokens_request_returns_tokens_payload() {
    let req = WorkerRequest {
        id: "req-005".to_string(),
        version: 1,
        payload: WorkerRequestPayload::SemanticTokens(SemanticTokensRequest {
            source: SEQUENCE_SOURCE.to_string(),
        }),
    };

    let resp = dispatch(req);

    assert!(resp.ok);
    match resp.payload.expect("payload must be present") {
        WorkerResponsePayload::SemanticTokens(_) => {}
        other => panic!("expected SemanticTokens payload, got {:?}", other),
    }
}

#[test]
fn worker_completion_request_returns_completion_payload() {
    let req = WorkerRequest {
        id: "req-006".to_string(),
        version: 1,
        payload: WorkerRequestPayload::Completion(CompletionRequest {
            source: SEQUENCE_SOURCE.to_string(),
            line: 1,
            column: 1,
        }),
    };

    let resp = dispatch(req);

    assert!(resp.ok);
    match resp.payload.expect("payload must be present") {
        WorkerResponsePayload::Completion(c) => {
            assert!(!c.items.is_empty(), "completion must return items");
        }
        other => panic!("expected Completion payload, got {:?}", other),
    }
}

#[test]
fn worker_request_json_has_stable_envelope_shape() {
    let req = WorkerRequest::compile("req-007", SEQUENCE_SOURCE);
    let json_str = serde_json::to_string(&req).expect("must serialise");
    let json: serde_json::Value = serde_json::from_str(&json_str).expect("must deserialise");

    let obj = json.as_object().expect("must be an object");
    assert!(obj.contains_key("id"), "missing key: id");
    assert!(obj.contains_key("version"), "missing key: version");
    assert!(obj.contains_key("payload"), "missing key: payload");
    assert_eq!(json["id"], "req-007");
    assert_eq!(json["version"], 1);
}

#[test]
fn worker_response_error_constructor_sets_ok_false() {
    use puml::worker::WorkerResponse;
    let resp = WorkerResponse::err("req-err", "something broke");
    assert!(!resp.ok);
    assert_eq!(resp.error.as_deref(), Some("something broke"));
    assert!(resp.payload.is_none());
}

#[test]
fn worker_response_serialises_with_op_tag() {
    let req = WorkerRequest::compile("req-008", SEQUENCE_SOURCE);
    let resp = dispatch(req);
    let json_str = serde_json::to_string(&resp).expect("must serialise");
    let json: serde_json::Value = serde_json::from_str(&json_str).expect("must deserialise");

    assert_eq!(json["ok"], true);
    assert_eq!(json["id"], "req-008");
    // The payload must be an object with an "op" discriminant.
    let payload = &json["payload"];
    assert!(payload.is_object(), "payload must be a JSON object");
    assert_eq!(payload["op"], "compile", "payload op must be 'compile'");
}

// ---------------------------------------------------------------------------
// SpanDto and DiagnosticDto field shape
// ---------------------------------------------------------------------------

#[test]
fn span_dto_serialises_with_start_end_keys() {
    let span = SpanDto { start: 10, end: 20 };
    let json_str = serde_json::to_string(&span).expect("must serialise");
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(json["start"], 10);
    assert_eq!(json["end"], 20);
}

#[test]
fn diagnostic_dto_serialises_required_fields() {
    let dto = DiagnosticDto {
        code: Some("E_TEST".to_string()),
        severity: "error".to_string(),
        category: "parse-error".to_string(),
        message: "test message".to_string(),
        span: Some(SpanDto { start: 0, end: 5 }),
        line: Some(1),
        column: Some(1),
    };
    let json_str = serde_json::to_string(&dto).expect("must serialise");
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(json["code"], "E_TEST");
    assert_eq!(json["severity"], "error");
    assert_eq!(json["category"], "parse-error");
    assert_eq!(json["message"], "test message");
    assert_eq!(json["span"]["start"], 0);
    assert_eq!(json["span"]["end"], 5);
    assert_eq!(json["line"], 1);
    assert_eq!(json["column"], 1);
}

// ---------------------------------------------------------------------------
// ModelSummary contract
// ---------------------------------------------------------------------------

#[test]
fn model_summary_serialises_all_fields() {
    let summary = ModelSummary {
        kind: "sequence".to_string(),
        warning_count: 0,
        node_count: 2,
        edge_count: 1,
        title: Some("My Diagram".to_string()),
    };
    let json_str = serde_json::to_string(&summary).expect("must serialise");
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(json["kind"], "sequence");
    assert_eq!(json["warning_count"], 0);
    assert_eq!(json["node_count"], 2);
    assert_eq!(json["edge_count"], 1);
    assert_eq!(json["title"], "My Diagram");
}

#[test]
fn model_summary_default_has_zero_counts() {
    let summary = ModelSummary::default();
    assert_eq!(summary.node_count, 0);
    assert_eq!(summary.edge_count, 0);
    assert_eq!(summary.warning_count, 0);
    assert!(summary.title.is_none());
}
