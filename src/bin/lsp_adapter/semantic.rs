use super::Doc;
use puml::language_service::SemanticTokenKind;
use puml::language_service::{offset_to_lc, semantic_tokens as shared_semantic_tokens};
use serde_json::{json, Value};

pub fn semantic_tokens(d: &Doc) -> Value {
    let mut data = Vec::<u32>::new();
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;
    for token in shared_semantic_tokens(&d.text) {
        let (l, c) = offset_to_lc(&d.text, token.span.start);
        let dl = l as u32 - prev_line;
        let dc = if dl == 0 {
            c as u32 - prev_char
        } else {
            c as u32
        };
        data.extend([
            dl,
            dc,
            (token.span.end - token.span.start) as u32,
            lsp_semantic_token_type(token.kind),
            0,
        ]);
        prev_line = l as u32;
        prev_char = c as u32;
    }
    json!({"data":data})
}

fn lsp_semantic_token_type(kind: SemanticTokenKind) -> u32 {
    match kind {
        SemanticTokenKind::Keyword => 0,
        SemanticTokenKind::Operator => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_adapter::render::lsp_parse;

    #[test]
    fn semantic_tokens_do_not_overlap_arrow_tokens() {
        let src = "@startuml\nAlice --> Bob\n@enduml\n";
        let doc = Doc {
            text: src.to_string(),
            version: 1,
            parsed: lsp_parse(src).ok(),
        };
        let out = semantic_tokens(&doc);
        let data = out
            .get("data")
            .and_then(Value::as_array)
            .expect("semantic token data");
        // one operator token for "-->", encoded in groups of 5 u32 values.
        let token_count = data.len() / 5;
        assert_eq!(token_count, 1);
        assert_eq!(data[2].as_u64(), Some(3));
        assert_eq!(data[3].as_u64(), Some(1));
    }
}
