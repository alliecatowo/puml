use crate::ast::{Document, ParticipantDecl, StatementKind};
use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: DocumentSymbolKind,
    pub span: Span,
    pub selection_span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentSymbolKind {
    Participant,
    Message,
}

pub fn document_symbols(document: &Document) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    for statement in &document.statements {
        match &statement.kind {
            StatementKind::Participant(ParticipantDecl { name, .. }) => {
                symbols.push(DocumentSymbol {
                    name: name.clone(),
                    kind: DocumentSymbolKind::Participant,
                    span: statement.span,
                    selection_span: statement.span,
                });
            }
            StatementKind::Message(message) => {
                symbols.push(DocumentSymbol {
                    name: format!("{} {} {}", message.from, message.arrow, message.to),
                    kind: DocumentSymbolKind::Message,
                    span: statement.span,
                    selection_span: statement.span,
                });
            }
            _ => {}
        }
    }
    symbols
}
