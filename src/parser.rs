use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{
    DiagramKind, Document, Group, Message, Note, ParticipantDecl, ParticipantRole, Statement,
    StatementKind,
};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

const MAX_INCLUDE_DEPTH: usize = 32;

#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    pub include_root: Option<PathBuf>,
}

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parse_with_options(source, &ParseOptions::default())
}

pub fn parse_with_options(source: &str, options: &ParseOptions) -> Result<Document, Diagnostic> {
    let mut defines = BTreeMap::new();
    let mut include_stack = Vec::new();
    let mut expanded = String::new();

    preprocess_text(
        source,
        options,
        &mut defines,
        &mut include_stack,
        0,
        &mut expanded,
    )?;

    parse_preprocessed(&expanded)
}

fn preprocess_text(
    source: &str,
    options: &ParseOptions,
    defines: &mut BTreeMap<String, String>,
    include_stack: &mut Vec<PathBuf>,
    depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(Diagnostic::error(format!(
            "include depth exceeded maximum of {MAX_INCLUDE_DEPTH}"
        )));
    }

    for raw_line in source.lines() {
        let line = raw_line.trim();
        let lower = line.to_ascii_lowercase();

        if lower.starts_with("!define") {
            let body = line[7..].trim();
            let (name, value) = body.split_once(' ').unwrap_or((body, ""));
            if !name.trim().is_empty() {
                defines.insert(name.trim().to_string(), value.trim().to_string());
            }
            continue;
        }

        if lower.starts_with("!undef") {
            let name = line[6..].trim();
            if !name.is_empty() {
                defines.remove(name);
            }
            continue;
        }

        if lower.starts_with("!include") {
            let raw_target = line[8..].trim();
            if raw_target.is_empty() {
                return Err(Diagnostic::error("!include requires a relative path"));
            }

            let include_path = parse_include_target(raw_target);
            if include_path.is_absolute() {
                return Err(Diagnostic::error(format!(
                    "!include only supports relative paths: {}",
                    include_path.display()
                )));
            }

            let base_dir = if let Some(curr) = include_stack.last() {
                curr.parent().map(Path::to_path_buf)
            } else {
                options.include_root.clone()
            };

            let Some(base_dir) = base_dir else {
                return Err(Diagnostic::error(
                    "!include from stdin requires include_root option",
                ));
            };

            let resolved = normalize_path(base_dir.join(include_path));
            if include_stack.iter().any(|p| p == &resolved) {
                let mut cycle = include_stack
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>();
                cycle.push(resolved.display().to_string());
                return Err(Diagnostic::error(format!(
                    "include cycle detected: {}",
                    cycle.join(" -> ")
                )));
            }

            let content = fs::read_to_string(&resolved).map_err(|e| {
                Diagnostic::error(format!(
                    "failed to read include '{}': {e}",
                    resolved.display()
                ))
            })?;

            include_stack.push(resolved);
            preprocess_text(&content, options, defines, include_stack, depth + 1, out)?;
            include_stack.pop();
            continue;
        }

        out.push_str(&substitute_tokens(raw_line, defines));
        out.push('\n');
    }

    Ok(())
}

fn parse_include_target(raw_target: &str) -> PathBuf {
    let trimmed = raw_target.trim();
    let unwrapped = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| trimmed.strip_prefix('<').and_then(|s| s.strip_suffix('>')))
        .unwrap_or(trimmed);
    PathBuf::from(unwrapped)
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut parts = Vec::new();
    let is_abs = path.is_absolute();

    for comp in path.components() {
        use std::path::Component;
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if parts
                    .last()
                    .is_some_and(|c: &Component<'_>| !matches!(c, Component::ParentDir))
                {
                    parts.pop();
                } else if !is_abs {
                    parts.push(comp);
                }
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => parts.push(comp),
        }
    }

    let mut out = PathBuf::new();
    for c in parts {
        out.push(c.as_os_str());
    }
    out
}

fn substitute_tokens(line: &str, defines: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(line.len());
    let mut token = String::new();
    let mut in_quotes = false;

    let flush_token = |token: &mut String, out: &mut String, defines: &BTreeMap<String, String>| {
        if token.is_empty() {
            return;
        }
        if let Some(v) = defines.get(token.as_str()) {
            out.push_str(v);
        } else {
            out.push_str(token);
        }
        token.clear();
    };

    for ch in line.chars() {
        if ch == '"' {
            flush_token(&mut token, &mut out, defines);
            in_quotes = !in_quotes;
            out.push(ch);
            continue;
        }

        if !in_quotes && (ch.is_ascii_alphanumeric() || ch == '_') {
            token.push(ch);
            continue;
        }

        flush_token(&mut token, &mut out, defines);
        out.push(ch);
    }

    flush_token(&mut token, &mut out, defines);
    out
}

fn parse_preprocessed(source: &str) -> Result<Document, Diagnostic> {
    let mut statements = Vec::new();
    let mut offset = 0usize;
    let mut seen_sequence = false;

    for raw_line in source.lines() {
        let line = raw_line.trim();
        let span = Span::new(offset, offset + raw_line.len());
        offset += raw_line.len() + 1;

        if line.is_empty() || line.starts_with('"') || line.eq_ignore_ascii_case("!pragma") {
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
            if is_sequence_keyword(&kind) {
                seen_sequence = true;
            }
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
    let (core, label) = split_message_label(line);
    let (lhs_raw, arrow, rhs_raw) = split_arrow(core)?;

    let mut from = clean_ident(lhs_raw);
    let mut to = clean_ident(rhs_raw);

    if from.is_empty() && lhs_raw.contains('[') {
        from = "[*]".to_string();
    }
    if to.is_empty() && rhs_raw.contains(']') {
        to = "[*]".to_string();
    }
    if from.is_empty() || to.is_empty() {
        return None;
    }

    Some(StatementKind::Message(Message {
        from,
        to,
        arrow: arrow.to_string(),
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
    if lower.starts_with("!theme") {
        return Some(StatementKind::Theme(line[6..].trim().to_string()));
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
    let mut out = s.trim().trim_matches('"').to_string();
    for suffix in ["++", "--", "**", "!!"] {
        out = out
            .strip_suffix(suffix)
            .map(str::trim_end)
            .unwrap_or(&out)
            .to_string();
    }
    out
}

fn split_message_label(line: &str) -> (&str, Option<String>) {
    if let Some(colon) = line.find(':') {
        let text = line[colon + 1..].trim();
        (
            line[..colon].trim_end(),
            Some(text.to_string()).filter(|s| !s.is_empty()),
        )
    } else {
        (line.trim_end(), None)
    }
}

fn split_arrow(core: &str) -> Option<(&str, &str, &str)> {
    let arrow_start = core.find(['-', '<', '[']).unwrap_or(core.len());
    if arrow_start >= core.len() {
        return None;
    }
    let lhs = &core[..arrow_start];
    let arrow_bytes = core.as_bytes();
    let mut i = arrow_start;
    while i < core.len() {
        let c = arrow_bytes[i] as char;
        if c == '-' || c == '<' || c == '>' || c == '[' || c == ']' || c == 'o' || c == 'x' {
            i += 1;
            continue;
        }
        break;
    }
    if i == arrow_start {
        return None;
    }
    let arrow = core[arrow_start..i].trim();
    if !arrow.contains('-') {
        return None;
    }
    let rhs = core[i..].trim();
    Some((lhs.trim(), arrow, rhs))
}

fn is_sequence_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Note(_)
            | StatementKind::Group(_)
            | StatementKind::Footbox(_)
            | StatementKind::Delay(_)
            | StatementKind::Divider(_)
            | StatementKind::Spacer
            | StatementKind::NewPage(_)
            | StatementKind::Autonumber(_)
            | StatementKind::Activate(_)
            | StatementKind::Deactivate(_)
            | StatementKind::Destroy(_)
            | StatementKind::Create(_)
            | StatementKind::Return(_)
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_)
            | StatementKind::Theme(_)
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_with_options, ParseOptions};
    use crate::ast::StatementKind;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn define_substitution_skips_quoted_strings() {
        let doc = parse_with_options(
            "!define NAME Alice\nparticipant NAME\nnote over NAME: \"NAME\"\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::Participant(_)
        ));
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.target.as_deref(), Some("Alice"));
                assert_eq!(n.text, "\"NAME\"");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn include_resolves_relative_to_root() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B\n").unwrap();

        let doc = parse_with_options(
            "!include inc.puml",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();

        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn include_cycle_errors() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.puml"), "!include b.puml\n").unwrap();
        fs::write(dir.path().join("b.puml"), "!include a.puml\n").unwrap();

        let err = parse_with_options(
            "!include a.puml",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap_err();

        assert!(err.message.contains("include cycle detected"));
    }

    #[test]
    fn include_from_stdin_requires_root() {
        let err = parse_with_options("!include x.puml", &ParseOptions::default()).unwrap_err();
        assert!(err
            .message
            .contains("!include from stdin requires include_root option"));
    }
}
