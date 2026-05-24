use super::document::DocumentSnapshot;
use super::formatting::TextEdit;
use super::text::{is_ident, word_range_at_pos};
use crate::ast::StatementKind;
use crate::source::Span;
use crate::Document;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationHit {
    pub span: Span,
}

pub fn definition(snapshot: &DocumentSnapshot, position: (u64, u64)) -> Option<NavigationHit> {
    let (s, e) = word_range_at_pos(&snapshot.text, position)?;
    let symbol = &snapshot.text[s..e];
    let span = find_participant_decl(snapshot.parsed.as_ref()?, symbol)?;
    Some(NavigationHit { span })
}

pub fn prepare_rename(source: &str, position: (u64, u64)) -> Option<Span> {
    let (start, end) = word_range_at_pos(source, position)?;
    Some(Span { start, end })
}

pub fn references(source: &str, position: (u64, u64)) -> Vec<NavigationHit> {
    let Some((s, e)) = word_range_at_pos(source, position) else {
        return Vec::new();
    };
    find_word_refs(source, &source[s..e])
        .into_iter()
        .map(|span| NavigationHit { span })
        .collect()
}

pub fn rename(source: &str, position: (u64, u64), new_name: &str) -> Vec<TextEdit> {
    references(source, position)
        .into_iter()
        .map(|hit| TextEdit {
            span: hit.span,
            new_text: new_name.to_string(),
        })
        .collect()
}

fn find_participant_decl(document: &Document, symbol: &str) -> Option<Span> {
    for statement in &document.statements {
        if let StatementKind::Participant(participant) = &statement.kind {
            if participant.name == symbol || participant.alias.as_deref() == Some(symbol) {
                return Some(statement.span);
            }
        }
    }
    None
}

fn find_word_refs(source: &str, symbol: &str) -> Vec<Span> {
    let mut refs = Vec::new();
    if symbol.is_empty() {
        return refs;
    }
    let bytes = source.as_bytes();
    let symbol_bytes = symbol.as_bytes();
    let mut cursor = 0;
    while cursor + symbol_bytes.len() <= bytes.len() {
        if &bytes[cursor..cursor + symbol_bytes.len()] == symbol_bytes {
            let left = cursor == 0 || !is_ident(bytes[cursor - 1] as char);
            let end = cursor + symbol_bytes.len();
            let right = end == bytes.len() || !is_ident(bytes[end] as char);
            if left && right {
                refs.push(Span { start: cursor, end });
            }
            cursor += symbol_bytes.len();
        } else {
            cursor += 1;
        }
    }
    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definition_finds_participant_declaration() {
        let source = "@startuml\nparticipant Alice\nAlice -> Bob: hi\n@enduml\n";
        let snapshot = DocumentSnapshot::new(source.to_string(), 1);

        let hit = definition(&snapshot, (2, 1)).expect("definition");

        assert_eq!(&source[hit.span.start..hit.span.end], "participant Alice");
    }

    #[test]
    fn references_and_rename_obey_word_boundaries() {
        let source = "@startuml\nAlice Alice_Bob Bob\nAlice -> Bob\n@enduml\n";

        let hits = references(source, (1, 1));
        assert_eq!(hits.len(), 2);
        assert_eq!(&source[hits[0].span.start..hits[0].span.end], "Alice");

        let edits = rename(source, (1, 1), "User");
        assert_eq!(edits.len(), 2);
        assert!(edits.iter().all(|edit| edit.new_text == "User"));
        assert!(prepare_rename(source, (1, 5)).is_none());
    }
}
