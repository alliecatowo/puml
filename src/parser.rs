use winnow::error::ContextError;
use winnow::token::take_till;
use winnow::Parser;

use crate::ast::{
    DiagramKind, Document, Group, Message, Note, ParticipantDecl, ParticipantRole, Statement,
    StatementKind,
};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    let mut statements = Vec::new();
    let mut offset = 0usize;
    let mut seen_sequence = false;

    for raw_line in source.lines() {
        let line = raw_line.trim();
        let span = Span::new(offset, offset + raw_line.len());
        offset += raw_line.len() + 1;

        if line.is_empty() || line.starts_with('"') || line.starts_with('!') && line == "!pragma" {
            continue;
        }
        if line.eq_ignore_ascii_case("@startuml")
            || line.eq_ignore_ascii_case("@enduml")
            || line.eq_ignore_ascii_case("end")
        {
            continue;
        }

        if let Some(kind) = parse_participant(line) {
            seen_sequence = true;
            statements.push(Statement { span, kind });
            continue;
        }

        if let Some(kind) = parse_message(line) {
            seen_sequence = true;
            statements.push(Statement { span, kind });
            continue;
        }

        if let Some(kind) = parse_keyword(line) {
            seen_sequence = true;
            statements.push(Statement { span, kind });
            continue;
        }

        if line.starts_with("class ")
            || line.starts_with("usecase ")
            || line.starts_with("component ")
            || line.starts_with("state ")
            || line.starts_with("[*]")
        {
            return Ok(Document {
                kind: DiagramKind::Unknown,
                statements,
            });
        }

        statements.push(Statement {
            span,
            kind: StatementKind::Unknown(line.to_string()),
        });
    }

    Ok(Document {
        kind: if seen_sequence {
            DiagramKind::Sequence
        } else {
            DiagramKind::Unknown
        },
        statements,
    })
}

fn parse_participant(line: &str) -> Option<StatementKind> {
    let roles = [
        ("participant", ParticipantRole::Participant),
        ("actor", ParticipantRole::Actor),
        ("boundary", ParticipantRole::Boundary),
        ("control", ParticipantRole::Control),
        ("entity", ParticipantRole::Entity),
        ("database", ParticipantRole::Database),
        ("collections", ParticipantRole::Collections),
    ];

    for (kw, role) in roles {
        if !line.starts_with(kw) {
            continue;
        }
        let rest = line[kw.len()..].trim();
        if rest.is_empty() {
            return None;
        }
        let (display, rem) = if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"')?;
            (
                Some(stripped[..end].to_string()),
                stripped[end + 1..].trim(),
            )
        } else {
            (None, rest)
        };

        let mut alias = None;
        let mut name = rem.to_string();
        if let Some((lhs, rhs)) = rem.split_once(" as ") {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if display.is_none() {
                name = lhs.to_string();
            }
            alias = Some(clean_ident(rhs));
        }

        if name.is_empty() {
            name = alias.clone().unwrap_or_default();
        }
        let name = clean_ident(&name);
        let display = display.or_else(|| Some(name.clone()));

        return Some(StatementKind::Participant(ParticipantDecl {
            role,
            name,
            alias,
            display,
        }));
    }
    None
}

fn parse_message(line: &str) -> Option<StatementKind> {
    let mut input = line;
    let from_raw: &str = take_till::<_, _, ContextError>(1.., ['-', '<'])
        .parse_next(&mut input)
        .ok()?;
    let from = clean_ident(from_raw.trim());
    let arrow_start = line.len() - input.len();

    let colon_idx = line[arrow_start..]
        .find(':')
        .map(|i| i + arrow_start)
        .unwrap_or(line.len());
    let before_colon = &line[arrow_start..colon_idx];

    let arrow_end_rel = before_colon.find(|c: char| c.is_ascii_alphanumeric() || c == '"')?;
    let arrow_token = before_colon[..arrow_end_rel].trim();
    if !arrow_token.contains('-') {
        return None;
    }
    let to_raw = before_colon[arrow_end_rel..].trim();
    if to_raw.is_empty() {
        return None;
    }

    let label = if colon_idx < line.len() {
        Some(line[colon_idx + 1..].trim().to_string()).filter(|s| !s.is_empty())
    } else {
        None
    };

    Some(StatementKind::Message(Message {
        from,
        to: clean_ident(to_raw),
        arrow: arrow_token.to_string(),
        label,
    }))
}

fn parse_keyword(line: &str) -> Option<StatementKind> {
    let lower = line.to_ascii_lowercase();

    for k in ["title", "header", "footer", "caption", "legend"] {
        if lower.starts_with(&(k.to_string() + " ")) {
            let v = line[k.len()..].trim().to_string();
            return Some(match k {
                "title" => StatementKind::Title(v),
                "header" => StatementKind::Header(v),
                "footer" => StatementKind::Footer(v),
                "caption" => StatementKind::Caption(v),
                _ => StatementKind::Legend(v),
            });
        }
    }

    if lower.starts_with("skinparam ") {
        let body = line[9..].trim();
        let (key, value) = body.split_once(' ').unwrap_or((body, ""));
        return Some(StatementKind::SkinParam {
            key: key.trim().to_string(),
            value: value.trim().to_string(),
        });
    }

    if lower == "hide footbox" {
        return Some(StatementKind::Footbox(false));
    }
    if lower == "show footbox" {
        return Some(StatementKind::Footbox(true));
    }

    if lower.starts_with("note ") {
        let tail = line[5..].trim();
        let (head, text) = tail.split_once(':').unwrap_or((tail, ""));
        let mut bits = head.split_whitespace();
        let pos = bits.next().unwrap_or("over").to_string();
        let target = bits.next().map(clean_ident);
        return Some(StatementKind::Note(Note {
            position: pos,
            target,
            text: text.trim().to_string(),
        }));
    }

    for g in ["alt", "opt", "loop", "par", "critical", "break", "group"] {
        if lower == g || lower.starts_with(&(g.to_string() + " ")) {
            let label = line[g.len()..].trim();
            return Some(StatementKind::Group(Group {
                kind: g.to_string(),
                label: if label.is_empty() {
                    None
                } else {
                    Some(label.to_string())
                },
            }));
        }
    }

    if lower == "else" || lower.starts_with("else ") {
        return Some(StatementKind::Group(Group {
            kind: "else".to_string(),
            label: Some(line[4..].trim().to_string()).filter(|s| !s.is_empty()),
        }));
    }

    if lower == "end" {
        return Some(StatementKind::Group(Group {
            kind: "end".to_string(),
            label: None,
        }));
    }

    if line == "..." {
        return Some(StatementKind::Spacer);
    }
    if lower.starts_with("...") {
        return Some(StatementKind::Divider(Some(
            line.trim_matches('.').trim().to_string(),
        )));
    }
    if lower.starts_with("||") {
        return Some(StatementKind::Delay(Some(
            line.trim_matches('|').trim().to_string(),
        )));
    }
    if lower == "||" {
        return Some(StatementKind::Delay(None));
    }
    if lower.starts_with("newpage") {
        return Some(StatementKind::NewPage(line[7..].trim().to_string().into()));
    }
    if lower.starts_with("autonumber") {
        return Some(StatementKind::Autonumber(
            line[10..].trim().to_string().into(),
        ));
    }

    for (kw, ctor) in [
        (
            "activate",
            StatementKind::Activate as fn(String) -> StatementKind,
        ),
        ("deactivate", StatementKind::Deactivate),
        ("destroy", StatementKind::Destroy),
        ("create", StatementKind::Create),
    ] {
        if lower.starts_with(&(kw.to_string() + " ")) {
            return Some(ctor(clean_ident(line[kw.len()..].trim())));
        }
    }

    if lower == "return" || lower.starts_with("return ") {
        return Some(StatementKind::Return(
            Some(line[6..].trim().to_string()).filter(|s| !s.is_empty()),
        ));
    }

    if lower.starts_with("!include") {
        return Some(StatementKind::Include(line[8..].trim().to_string()));
    }
    if lower.starts_with("!define") {
        let body = line[7..].trim();
        let (name, value) = body.split_once(' ').unwrap_or((body, ""));
        return Some(StatementKind::Define {
            name: name.trim().to_string(),
            value: Some(value.trim().to_string()).filter(|s| !s.is_empty()),
        });
    }
    if lower.starts_with("!undef") {
        return Some(StatementKind::Undef(line[6..].trim().to_string()));
    }

    None
}

fn clean_ident(s: &str) -> String {
    s.trim().trim_matches('"').to_string()
}
