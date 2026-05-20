use super::*;

pub(super) fn parse_keyword(line: &str) -> Option<StatementKind> {
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

pub(super) fn parse_note_head(head: &str) -> (String, Option<String>) {
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

pub(super) fn note_kind_from_keyword(keyword: &str) -> crate::ast::NoteKind {
    match keyword.to_ascii_lowercase().as_str() {
        "hnote" => crate::ast::NoteKind::Hexagonal,
        "rnote" => crate::ast::NoteKind::Rectangle,
        _ => crate::ast::NoteKind::Folded,
    }
}

pub(super) fn note_end_matches(line: &str, note_keyword: &str) -> bool {
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
