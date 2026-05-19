fn parse_participant(line: &str) -> Option<StatementKind> {
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

fn split_participant_order(input: &str) -> (&str, Option<i32>) {
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

fn parse_message(line: &str) -> Option<StatementKind> {
    let (line, parallel) = split_parallel_message_prefix(line);
    let (core, label) = split_message_label(line);
    let (lhs_raw, arrow, rhs_raw) = split_arrow(core)?;
    let mut style = parse_arrow_style(arrow);
    style.parallel = parallel;
    let parsed_arrow = parse_arrow(arrow)?;
    let (from_id_raw, from_modifier) = split_lifecycle_modifier(lhs_raw);
    let (to_id_raw, to_modifier) = split_lifecycle_modifier(rhs_raw);

    let from = if let Some(v) = normalize_virtual_endpoint(from_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(from_id_raw) {
            return None;
        }
        clean_ident(from_id_raw)
    };
    let to = if let Some(v) = normalize_virtual_endpoint(to_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(to_id_raw) {
            return None;
        }
        clean_ident(to_id_raw)
    };

    if from.is_empty() || to.is_empty() {
        return None;
    }

    let mut arrow_encoded = parsed_arrow.to_string();
    if let Some(modifier) = from_modifier {
        arrow_encoded.push_str("@L");
        arrow_encoded.push_str(modifier);
    }
    if let Some(modifier) = to_modifier {
        arrow_encoded.push_str("@R");
        arrow_encoded.push_str(modifier);
    }

    let from_virtual = ast_virtual_endpoint_from_id(&from, true);
    let to_virtual = ast_virtual_endpoint_from_id(&to, false);
    Some(StatementKind::Message(Message {
        from,
        to,
        arrow: arrow_encoded,
        label,
        style,
        from_virtual,
        to_virtual,
    }))
}

fn split_parallel_message_prefix(line: &str) -> (&str, bool) {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix('&') {
        let rest = rest.trim_start();
        if !rest.is_empty() {
            return (rest, true);
        }
    }
    (line, false)
}

fn parse_arrow_style(arrow: &str) -> MessageStyle {
    let mut style = MessageStyle::default();
    if strip_sequence_arrow_brackets(arrow).contains('.') {
        style.dotted = true;
    }
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '[' {
            continue;
        }
        let mut body = String::new();
        for inner in chars.by_ref() {
            if inner == ']' {
                break;
            }
            body.push(inner);
        }
        for token in body
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let lower = token.to_ascii_lowercase();
            match lower.as_str() {
                "hidden" | "line.hidden" => style.hidden = true,
                "dashed" | "line.dashed" => style.dashed = true,
                "dotted" | "line.dotted" => style.dotted = true,
                "bold" | "thick" | "line.bold" | "line.thick" => style.thickness = Some(3),
                "thin" | "line.thin" => style.thickness = Some(1),
                _ if token.starts_with('#')
                    && matches!(token.len(), 4 | 5 | 7 | 9)
                    && token[1..].bytes().all(|b| b.is_ascii_hexdigit()) =>
                {
                    style.color = Some(format!("#{}", token[1..].to_ascii_lowercase()));
                }
                _ if token.starts_with('#')
                    && token[1..].bytes().all(|b| b.is_ascii_alphabetic()) =>
                {
                    style.color = Some(token[1..].to_ascii_lowercase());
                }
                _ if token.bytes().all(|b| b.is_ascii_alphabetic()) => {
                    style.color = Some(lower);
                }
                _ => {
                    if let Some(value) = lower
                        .strip_prefix("thickness=")
                        .or_else(|| lower.strip_prefix("thickness:"))
                        .or_else(|| lower.strip_prefix("thickness "))
                        .or_else(|| lower.strip_prefix("line.thickness="))
                        .or_else(|| lower.strip_prefix("line.thickness:"))
                        .or_else(|| lower.strip_prefix("line.thickness "))
                    {
                        if let Ok(n) = value.trim().parse::<u8>() {
                            style.thickness = Some(n.clamp(1, 8));
                        }
                    }
                }
            }
        }
    }
    style
}

fn ast_virtual_endpoint_from_id(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
    let (side, kind) = match id {
        "[" => (VirtualEndpointSide::Left, VirtualEndpointKind::Plain),
        "]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Plain),
        "[o" => (VirtualEndpointSide::Left, VirtualEndpointKind::Circle),
        "o]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Circle),
        "[x" => (VirtualEndpointSide::Left, VirtualEndpointKind::Cross),
        "x]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Cross),
        "[*]" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Filled,
        ),
        _ => return None,
    };
    Some(VirtualEndpoint { side, kind })
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
    if lower.starts_with("!pragma") {
        let body = line[7..].trim();
        if body.is_empty() {
            return Some(StatementKind::Unknown(
                "[E_PRAGMA_INVALID] malformed pragma syntax: missing pragma body".to_string(),
            ));
        }
        return Some(StatementKind::Pragma(body.to_string()));
    }

    if lower == "hide footbox" {
        return Some(StatementKind::Footbox(false));
    }
    if lower == "show footbox" {
        return Some(StatementKind::Footbox(true));
    }
    if lower == "hide unlinked" {
        return Some(StatementKind::HideUnlinked);
    }

    // scale directive: "scale <factor>", "scale <w>*<h>", "scale max <n>"
    if lower.starts_with("scale ") {
        let body = line[6..].trim();
        return Some(StatementKind::Scale(body.to_string()));
    }

    // Class-diagram hide options (parsed here so they work before any class decl sets detected_kind)
    if lower.starts_with("hide ") {
        let rest = lower.strip_prefix("hide ").unwrap_or("").trim();
        let class_hide_opts = [
            "circle",
            "stereotype",
            "empty members",
            "empty methods",
            "empty fields",
        ];
        for opt in class_hide_opts {
            if rest == opt {
                return Some(StatementKind::HideOption(rest.to_string()));
            }
        }
    }

    // set namespaceSeparator <sep>
    if lower.starts_with("set namespaceseparator") {
        let rest = line["set namespaceSeparator".len()..].trim();
        return Some(StatementKind::SetOption {
            key: "namespaceSeparator".to_string(),
            value: rest.to_string(),
        });
    }

    let note_kw = if lower.starts_with("note ") {
        Some("note")
    } else if lower.starts_with("hnote ") {
        Some("hnote")
    } else if lower.starts_with("rnote ") {
        Some("rnote")
    } else {
        None
    };
    if let Some(note_kw) = note_kw {
        let tail = line[note_kw.len()..].trim();
        if tail.is_empty() {
            return Some(StatementKind::Unknown(
                "[E_NOTE_INVALID] malformed note syntax: missing note head".to_string(),
            ));
        }
        let (head, text) = tail.split_once(':').unwrap_or((tail, ""));
        let (pos, target) = parse_note_head(head);
        if pos.eq_ignore_ascii_case("of") || !is_valid_note_position(&pos) {
            return Some(StatementKind::Unknown(format!(
                "[E_NOTE_INVALID] malformed note syntax: `{}`",
                line
            )));
        }
        return Some(StatementKind::Note(Note {
            kind: note_kind_from_keyword(note_kw),
            position: pos,
            target,
            text: text.trim().to_string(),
        }));
    }
    if lower.starts_with("ref ") {
        let tail = line[4..].trim();
        let (head, text) = tail.split_once(':').unwrap_or((tail, ""));
        if head.is_empty() || text.trim().is_empty() {
            return Some(StatementKind::Unknown(format!(
                "[E_REF_INVALID] malformed ref syntax: `{}`",
                line
            )));
        }
        let label = format!("{}\n{}", head.trim(), text.trim());
        return Some(StatementKind::Group(Group {
            kind: "ref".to_string(),
            label: Some(label),
        }));
    }

    for g in [
        "alt", "opt", "loop", "par", "critical", "break", "group", "box",
    ] {
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

    // `also` is the parallel-branch continuation keyword for `par` blocks,
    // analogous to `else` in `alt` blocks (PlantUML parity — fixes #780).
    if lower == "also" || lower.starts_with("also ") {
        return Some(StatementKind::Group(Group {
            kind: "also".to_string(),
            label: Some(line[4..].trim().to_string()).filter(|s| !s.is_empty()),
        }));
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
    if let Some(stripped) = lower.strip_prefix("end ") {
        let tail = stripped.trim();
        if matches!(
            tail,
            "alt" | "opt" | "loop" | "par" | "critical" | "break" | "group" | "ref" | "box"
        ) {
            return Some(StatementKind::Group(Group {
                kind: "end".to_string(),
                label: Some(tail.to_string()),
            }));
        }
    }

    if line == "..." {
        return Some(StatementKind::Spacer(None));
    }
    if lower.starts_with("...") && line.ends_with("...") && line.len() >= 6 {
        return Some(StatementKind::Divider(Some(
            line.trim_matches('.').trim().to_string(),
        )));
    }
    if lower.starts_with("|||") && line.ends_with("|||") {
        let body = line.trim_matches('|').trim();
        return Some(StatementKind::Spacer(
            body.parse::<i32>().ok().map(|n| n.clamp(1, 400)),
        ));
    }
    if lower.starts_with("||") && line.ends_with("||") && line.len() >= 4 {
        return Some(StatementKind::Delay(Some(
            line.trim_matches('|').trim().to_string(),
        )));
    }
    if lower == "||" {
        return Some(StatementKind::Delay(None));
    }
    if line.starts_with("==") && line.ends_with("==") && line.len() >= 4 {
        let label = line[2..line.len() - 2].trim().to_string();
        return Some(if label.is_empty() {
            StatementKind::Separator(None)
        } else {
            StatementKind::Separator(Some(label))
        });
    }
    if lower.starts_with("newpage") {
        return Some(StatementKind::NewPage(line[7..].trim().to_string().into()));
    }
    if lower == "ignore newpage" {
        return Some(StatementKind::IgnoreNewPage);
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

fn parse_note_head(head: &str) -> (String, Option<String>) {
    let mut bits = head.split_whitespace();
    let position = bits.next().unwrap_or("over").to_string();
    let rest = bits.collect::<Vec<_>>();
    if rest.is_empty() {
        return (position, None);
    }
    if rest[0].eq_ignore_ascii_case("of") {
        let target = rest[1..].join(" ");
        return (
            position,
            (!target.trim().is_empty()).then(|| clean_ident(target.trim())),
        );
    }
    let target = rest.join(" ");
    (
        position,
        (!target.trim().is_empty()).then(|| clean_ident(target.trim())),
    )
}

fn note_kind_from_keyword(keyword: &str) -> crate::ast::NoteKind {
    match keyword.to_ascii_lowercase().as_str() {
        "hnote" => crate::ast::NoteKind::Hexagonal,
        "rnote" => crate::ast::NoteKind::Rectangle,
        _ => crate::ast::NoteKind::Folded,
    }
}

fn note_end_matches(line: &str, note_keyword: &str) -> bool {
    line.eq_ignore_ascii_case("end note")
        || (note_keyword.eq_ignore_ascii_case("hnote") && line.eq_ignore_ascii_case("endhnote"))
        || (note_keyword.eq_ignore_ascii_case("rnote") && line.eq_ignore_ascii_case("endrnote"))
}

fn is_valid_note_position(position: &str) -> bool {
    matches!(
        position.to_ascii_lowercase().as_str(),
        "left" | "right" | "top" | "bottom" | "over" | "across"
    )
}

fn clean_ident(s: &str) -> String {
    let mut out = s.trim().trim_matches('"').to_string();
    if let Some(rest) = out.strip_prefix("()") {
        out = rest.trim().to_string();
    }
    if let Some(rest) = out.strip_suffix("()") {
        out = rest.trim().to_string();
    }
    for suffix in ["++", "--", "**", "!!"] {
        out = out
            .strip_suffix(suffix)
            .map(str::trim_end)
            .unwrap_or(&out)
            .to_string();
    }
    out
}

/// Extract the class/interface/enum name from a member line inside a package/namespace block.
/// E.g. "class Service" → "Service", "interface IRepo" → "IRepo", "MyClass" → "MyClass".
fn extract_class_member_name(s: &str) -> String {
    let t = s.trim();
    let lower = t.to_ascii_lowercase();
    for kw in &[
        "abstract class ",
        "annotation ",
        "interface ",
        "abstract ",
        "enum ",
        "class ",
        "object ",
        "map ",
        "usecase ",
        "component ",
        "portin ",
        "portout ",
        "port ",
        "node ",
        "database ",
        "cloud ",
        "frame ",
        "storage ",
        "package ",
        "rectangle ",
        "folder ",
        "file ",
        "card ",
        "artifact ",
        "actor ",
    ] {
        if lower.starts_with(kw) {
            // Extract the first identifier token from the original (case-preserved) text
            let name_part = t[kw.len()..].trim();
            let name = name_part
                .split(|c: char| c.is_whitespace() || c == '{')
                .next()
                .unwrap_or("")
                .trim_matches('"');
            return clean_ident(name);
        }
    }
    // Plain identifier (like in a together block)
    clean_ident(t)
}

fn extract_component_group_member_name(s: &str) -> String {
    if let Some(StatementKind::ComponentDecl { name, alias, .. }) = parse_component_decl(s) {
        return alias.unwrap_or(name);
    }
    extract_class_member_name(s)
}

fn split_family_relation_label(line: &str) -> (&str, Option<String>) {
    if split_family_arrow(line).is_none() {
        return split_message_label(line);
    }
    if let Some(colon) = line.rfind(" :") {
        let suffix = line[colon + 2..].trim();
        if !suffix_has_family_relation_arrow(suffix) {
            let text = line[colon + 2..].trim();
            if !text.is_empty() {
                return (line[..colon].trim_end(), Some(text.to_string()));
            }
        }
    }
    let mut in_quote = false;
    let mut last_colon = None;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if !in_quote && ch == ':' {
            last_colon = Some(idx);
        }
    }
    if let Some(colon) = last_colon {
        let prefix = line[..colon].trim_end();
        let suffix = line[colon + 1..].trim();
        if !suffix.is_empty()
            && !suffix_has_family_relation_arrow(suffix)
            && split_family_arrow(prefix).is_some()
        {
            return (prefix, Some(suffix.to_string()));
        }
    }
    (line.trim_end(), None)
}

fn suffix_has_family_relation_arrow(suffix: &str) -> bool {
    suffix.contains("--")
        || suffix.contains("..")
        || suffix.contains("->")
        || suffix.contains("<-")
        || suffix.contains("|>")
        || suffix.contains("<|")
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
    fn is_arrow_char(c: char) -> bool {
        matches!(
            c,
            '-' | '.' | '<' | '>' | '[' | ']' | 'o' | 'x' | '/' | '\\'
        )
    }

    let mut run_start: Option<usize> = None;
    let mut in_bracket = false;
    let mut skip_until = 0usize;
    for (idx, ch) in core.char_indices() {
        if idx < skip_until {
            continue;
        }
        if let Some(start) = run_start {
            if in_bracket {
                if ch == ']' {
                    in_bracket = false;
                }
                continue;
            }
            if ch == '[' {
                in_bracket = true;
                continue;
            }
            if is_arrow_char(ch) {
                continue;
            }
            let candidate = &core[start..idx];
            if !candidate.contains('-')
                && !(candidate.contains('.')
                    && (candidate.contains('<') || candidate.contains('>')))
            {
                run_start = None;
                continue;
            }
            let lhs = core[..start].trim();
            let rhs = core[idx..].trim();
            if !lhs.is_empty() && !rhs.is_empty() {
                return Some((lhs, candidate.trim(), rhs));
            }
            run_start = None;
            continue;
        }
        if ch == '[' && core[..idx].trim().is_empty() {
            let mut skipped_open_endpoint = false;
            for endpoint in ["[o", "[x"] {
                if core[idx..].starts_with(endpoint)
                    && core[idx + endpoint.len()..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    skip_until = idx + endpoint.len();
                    skipped_open_endpoint = true;
                    break;
                }
            }
            if skipped_open_endpoint {
                continue;
            }
            if let Some(close_rel) = core[idx..].find(']') {
                let bracket_body = &core[idx + ch.len_utf8()..idx + close_rel];
                if bracket_body.contains('-') {
                    continue;
                }
                let after_idx = idx + close_rel + 1;
                if core[after_idx..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
                {
                    skip_until = after_idx;
                    continue;
                }
            } else if core[idx + ch.len_utf8()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                continue;
            }
        }
        if is_arrow_char(ch) {
            if run_start.is_none() {
                run_start = Some(idx);
            }
            if ch == '[' {
                in_bracket = true;
            }
            continue;
        }
    }
    if let Some(start) = run_start {
        let candidate = &core[start..];
        if !candidate.contains('-')
            && !(candidate.contains('.') && (candidate.contains('<') || candidate.contains('>')))
        {
            return None;
        }
        let lhs = core[..start].trim();
        if lhs.is_empty() {
            return None;
        }
        return Some((lhs, candidate.trim(), ""));
    }
    None
}

fn parse_arrow(arrow: &str) -> Option<String> {
    const VALID_BASE_ARROWS: &[&str] = &[
        "->", "-->", "->>", "-->>", "<-", "<--", "<<-", "<<--", "<->", "<-->", "<<->>", "<<-->>",
    ];
    let arrow = strip_sequence_arrow_brackets(arrow);
    let mut squashed = String::with_capacity(arrow.len());
    let mut last_slash: Option<char> = None;
    let mut slash_run_len = 0usize;
    for ch in arrow.chars() {
        if matches!(ch, '/' | '\\') {
            if last_slash == Some(ch) {
                slash_run_len += 1;
            } else {
                last_slash = Some(ch);
                slash_run_len = 1;
            }
            if ch == '/' && slash_run_len > 1 {
                // Portable slash forms allow a single slash marker only.
                return None;
            }
            if slash_run_len == 1 {
                squashed.push(ch);
            }
            continue;
        }
        last_slash = None;
        slash_run_len = 0;
        squashed.push(ch);
    }

    let canonical = squashed.replace(['/', '\\'], "").replace('.', "-");
    if canonical.is_empty()
        || !canonical
            .chars()
            .all(|c| matches!(c, '-' | '<' | '>' | 'o' | 'x'))
        || !squashed
            .chars()
            .all(|c| matches!(c, '-' | '.' | '<' | '>' | 'o' | 'x' | '/' | '\\'))
    {
        return None;
    }
    let has_slash_marker = squashed.contains('/') || squashed.contains('\\');
    let has_dot_marker = squashed.contains('.');
    let expanded_marker = squashed.contains("-/") || squashed.contains("-\\");

    if has_slash_marker && matches!(canonical.as_str(), "-" | "--") {
        return Some(squashed);
    }

    if VALID_BASE_ARROWS.contains(&canonical.as_str()) {
        if has_dot_marker {
            return Some(canonical);
        }
        if has_slash_marker && !expanded_marker {
            return Some(canonical);
        }
        if expanded_marker
            && squashed.contains("-\\")
            && canonical == "-->>"
            && squashed.contains("->>")
        {
            return Some(squashed.replacen("->>", "-->>", 1));
        }
        return Some(squashed);
    }
    let with_left_trimmed = canonical
        .strip_prefix('o')
        .or_else(|| canonical.strip_prefix('x'))
        .unwrap_or(&canonical);
    let (core, right_marker_removed) = if let Some(stripped) = with_left_trimmed.strip_suffix('o') {
        (stripped, true)
    } else if let Some(stripped) = with_left_trimmed.strip_suffix('x') {
        (stripped, true)
    } else {
        (with_left_trimmed, false)
    };
    if core.is_empty() {
        return None;
    }
    if VALID_BASE_ARROWS.contains(&core) && (right_marker_removed || core != canonical) {
        if has_dot_marker {
            return Some(canonical);
        }
        if has_slash_marker && !expanded_marker {
            let mut out = core.to_string();
            if let Some(ch) = with_left_trimmed.chars().last() {
                if matches!(ch, 'o' | 'x') && right_marker_removed {
                    out.push(ch);
                }
            }
            return Some(out);
        }
        if expanded_marker && canonical.contains("-->>") && squashed.contains("->>") {
            return Some(squashed.replacen("->>", "-->>", 1));
        }
        return Some(squashed);
    }
    if let Some(stripped_core) = core.strip_prefix('-') {
        if VALID_BASE_ARROWS.contains(&stripped_core) && (right_marker_removed || core != canonical)
        {
            if has_dot_marker {
                return Some(canonical);
            }
            if has_slash_marker && !expanded_marker {
                let mut out = stripped_core.to_string();
                if let Some(ch) = with_left_trimmed.chars().last() {
                    if matches!(ch, 'o' | 'x') && right_marker_removed {
                        out.push(ch);
                    }
                }
                return Some(out);
            }
            if expanded_marker && canonical.contains("-->>") && squashed.contains("->>") {
                return Some(squashed.replacen("->>", "-->>", 1));
            }
            return Some(squashed);
        }
    }
    None
}

fn strip_sequence_arrow_brackets(arrow: &str) -> String {
    let mut out = String::with_capacity(arrow.len());
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn split_lifecycle_modifier(endpoint: &str) -> (&str, Option<&'static str>) {
    for suffix in ["++", "--", "**", "!!"] {
        if let Some(base) = endpoint.trim_end().strip_suffix(suffix) {
            return (base.trim_end(), Some(suffix));
        }
    }
    (endpoint, None)
}

fn normalize_virtual_endpoint(raw: &str) -> Option<String> {
    let t = raw.trim().trim_matches('"');
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "[*]" => Some("[*]".to_string()),
        "[" => Some("[".to_string()),
        "]" => Some("]".to_string()),
        "[o" | "o[" => Some("[o".to_string()),
        "o]" | "]o" => Some("o]".to_string()),
        "[x" | "x[" => Some("[x".to_string()),
        "x]" | "]x" => Some("x]".to_string()),
        _ => None,
    }
}

fn looks_like_virtual_endpoint_syntax(raw: &str) -> bool {
    let t = raw.trim().trim_matches('"').to_ascii_lowercase();
    t.contains('[') || t.contains(']')
}

fn looks_like_arrow_syntax(line: &str) -> bool {
    if line.starts_with('!') || line.starts_with('@') {
        return false;
    }
    line.contains("->")
        || line.contains("-->")
        || line.contains("..>")
        || line.contains("<..")
        || line.contains("<-")
        || line.contains("<--")
        || line.contains("<->")
        || line.contains("<-->")
        || line.contains("->>")
        || line.contains("-->>")
        || line.contains("-x")
        || line.contains("x-")
        || line.contains("-o")
        || line.contains("o-")
}

fn is_sequence_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Group(_)
            | StatementKind::Footbox(_)
            | StatementKind::Delay(_)
            | StatementKind::Divider(_)
            | StatementKind::Separator(_)
            | StatementKind::Spacer(_)
            | StatementKind::NewPage(_)
            | StatementKind::IgnoreNewPage
            | StatementKind::Autonumber(_)
            | StatementKind::Activate(_)
            | StatementKind::Deactivate(_)
            | StatementKind::Destroy(_)
            | StatementKind::Create(_)
            | StatementKind::Return(_)
    )
}

fn note_block_continues(lines: &[(&str, Span)], idx: usize, line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    if !(lower.starts_with("note ") || lower.starts_with("hnote ") || lower.starts_with("rnote ")) {
        return false;
    }
    for (candidate, _) in lines.iter().skip(idx + 1) {
        let trimmed = candidate.trim();
        if trimmed.eq_ignore_ascii_case("end note")
            || trimmed.eq_ignore_ascii_case("endnote")
            || trimmed.eq_ignore_ascii_case("endhnote")
            || trimmed.eq_ignore_ascii_case("endrnote")
        {
            return true;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('@') {
            return false;
        }
    }
    !line.contains(':')
}

fn text_block_continues(lines: &[(&str, Span)], idx: usize, line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    let keyword = ["title", "header", "footer", "caption", "legend"]
        .into_iter()
        .find(|keyword| lower.starts_with(&format!("{keyword} ")));
    let Some(keyword) = keyword else {
        return false;
    };
    let end_marker = format!("end {keyword}");
    for (candidate, _) in lines.iter().skip(idx + 1) {
        let trimmed = candidate.trim();
        if trimmed.eq_ignore_ascii_case(&end_marker) {
            return true;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('@') {
            return false;
        }
    }
    false
}

fn is_family_common_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Note(_)
            | StatementKind::Title(_)
            | StatementKind::Caption(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Legend(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Scale(_)
            | StatementKind::SetOption { .. }
            | StatementKind::HideOption(_)
            | StatementKind::Pragma(_)
    )
}
