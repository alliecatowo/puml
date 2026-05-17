use super::*;

pub(super) fn normalize_sdl(document: Document) -> Result<SdlDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut states = Vec::new();
    let mut transitions = Vec::new();
    let warnings: Vec<Diagnostic> = Vec::new();
    let mut state_index: BTreeMap<String, usize> = BTreeMap::new();
    let record_state = |name: &str,
                        kind: SdlStateKind,
                        state_index: &mut BTreeMap<String, usize>,
                        states: &mut Vec<SdlState>| {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Some(idx) = state_index.get(name).copied() {
            if states[idx].kind == SdlStateKind::State && kind != SdlStateKind::State {
                states[idx].kind = kind;
            }
        } else {
            state_index.insert(name.to_string(), states.len());
            states.push(SdlState {
                name: name.to_string(),
                kind,
            });
        }
    };
    for line in body {
        let line = line.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
        }
        // Recognized forms:
        //   state <name>
        //   state <from> -> <to> : <signal>
        //   start/input/output/decision/stop <name>
        //   state <name> <<start|input|output|decision|stop>>
        //   <from> -> <to> : <signal>
        //   <from> -> <to>
        let lower = line.to_ascii_lowercase();
        let without_state_keyword = if lower.starts_with("state ") {
            Some(line[6..].trim())
        } else {
            None
        };
        let transition_line = without_state_keyword.unwrap_or(line);
        if let Some((from, to, signal)) = parse_sdl_transition(transition_line) {
            record_state(&from, SdlStateKind::State, &mut state_index, &mut states);
            record_state(&to, SdlStateKind::State, &mut state_index, &mut states);
            transitions.push(SdlTransition { from, to, signal });
            continue;
        }
        if let Some(raw) = without_state_keyword {
            let (name, kind) = parse_sdl_state_decl(raw, SdlStateKind::State);
            record_state(&name, kind, &mut state_index, &mut states);
            continue;
        }
        if let Some((keyword, name)) = split_sdl_keyword_decl(line) {
            let (name, kind) = parse_sdl_state_decl(name, keyword);
            record_state(&name, kind, &mut state_index, &mut states);
            continue;
        }
        // Otherwise treat as a state declaration.
        let (name, kind) = parse_sdl_state_decl(line, SdlStateKind::State);
        record_state(&name, kind, &mut state_index, &mut states);
    }
    Ok(SdlDocument {
        title,
        states,
        transitions,
        warnings,
    })
}

fn parse_sdl_transition(line: &str) -> Option<(String, String, Option<String>)> {
    let (core, signal) = if let Some((core, signal)) = line.split_once(':') {
        let signal = signal.trim();
        (
            core.trim(),
            if signal.is_empty() {
                None
            } else {
                Some(signal.to_string())
            },
        )
    } else {
        (line.trim(), None)
    };
    let (from, to) = core.split_once("->")?;
    let from = from.trim();
    let to = to.trim();
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some((from.to_string(), to.to_string(), signal))
}

fn split_sdl_keyword_decl(line: &str) -> Option<(SdlStateKind, &str)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    for (keyword, kind) in [
        ("start", SdlStateKind::Start),
        ("input", SdlStateKind::Input),
        ("output", SdlStateKind::Output),
        ("decision", SdlStateKind::Decision),
        ("stop", SdlStateKind::Stop),
        ("end", SdlStateKind::Stop),
    ] {
        if lower == keyword {
            return Some((kind, ""));
        }
        if lower.starts_with(keyword)
            && lower[keyword.len()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
        {
            return Some((kind, trimmed[keyword.len()..].trim()));
        }
    }
    None
}

fn parse_sdl_state_decl(raw: &str, default_kind: SdlStateKind) -> (String, SdlStateKind) {
    let mut text = raw.trim().to_string();
    let mut kind = default_kind;
    if let (Some(start), Some(end)) = (text.find("<<"), text.find(">>")) {
        if start < end {
            kind = match text[start + 2..end].trim().to_ascii_lowercase().as_str() {
                "start" | "*" => SdlStateKind::Start,
                "input" => SdlStateKind::Input,
                "output" => SdlStateKind::Output,
                "choice" | "decision" => SdlStateKind::Decision,
                "end" | "stop" => SdlStateKind::Stop,
                _ => default_kind,
            };
            text = format!("{}{}", text[..start].trim(), text[end + 2..].trim());
            text = text.trim().to_string();
        }
    }
    if text.is_empty() {
        text = match kind {
            SdlStateKind::Start => "Start",
            SdlStateKind::Input => "Input",
            SdlStateKind::Output => "Output",
            SdlStateKind::Decision => "Decision",
            SdlStateKind::State => "State",
            SdlStateKind::Stop => "Stop",
        }
        .to_string();
    }
    (text, kind)
}
