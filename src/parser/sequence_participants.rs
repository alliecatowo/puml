use super::*;
pub(crate) fn parse_participant(line: &str) -> Option<StatementKind> {
    let roles = [
        ("participant", ParticipantRole::Participant),
        ("actor", ParticipantRole::Actor),
        ("boundary", ParticipantRole::Boundary),
        ("control", ParticipantRole::Control),
        ("entity", ParticipantRole::Entity),
        ("database", ParticipantRole::Database),
        ("collections", ParticipantRole::Collections),
        ("queue", ParticipantRole::Queue),
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
        let (rem, order) = split_participant_order(rem);

        let mut alias = None;
        let mut name = rem.to_string();
        if let Some(rhs) = rem.strip_prefix("as ") {
            alias = Some(clean_ident(rhs.trim()));
            name = alias.clone().unwrap_or_default();
        } else if let Some((lhs, rhs)) = rem.split_once(" as ") {
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
            order,
        }));
    }
    None
}

pub(crate) fn split_participant_order(input: &str) -> (&str, Option<i32>) {
    let trimmed = input.trim();
    let mut tokens = trimmed.rsplitn(3, char::is_whitespace);
    let value = tokens.next().unwrap_or("");
    let keyword = tokens.next().unwrap_or("");
    let before = tokens.next().unwrap_or("");
    if keyword.eq_ignore_ascii_case("order") {
        if let Ok(order) = value.parse::<i32>() {
            return (before.trim_end(), Some(order));
        }
    }
    (trimmed, None)
}
