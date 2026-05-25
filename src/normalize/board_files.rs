use super::*;

const BOARD_DEPTH_LIMIT: usize = 4;
const FILE_DEPTH_WARNING: usize = 8;

pub(super) fn normalize_board(document: Document) -> Result<BoardDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut columns = Vec::new();
    let mut warnings = Vec::new();
    let mut current: Option<BoardColumn> = None;

    for line in body {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('\'') {
            continue;
        }
        if let Some(title) = parse_board_column_title(trimmed) {
            if let Some(column) = current.take() {
                columns.push(column);
            }
            current = Some(BoardColumn {
                title,
                cards: Vec::new(),
            });
            continue;
        }

        let (depth, text) = parse_board_card(trimmed);
        if current.is_none() {
            current = Some(BoardColumn {
                title: text.to_string(),
                cards: Vec::new(),
            });
            continue;
        }
        let depth = if depth > BOARD_DEPTH_LIMIT {
            warnings.push(Diagnostic::warning(format!(
                "[W_BOARD_DEPTH_LIMIT] board item depth {depth} exceeds supported depth {BOARD_DEPTH_LIMIT}; rendering at depth {BOARD_DEPTH_LIMIT}: `{text}`"
            )));
            BOARD_DEPTH_LIMIT
        } else {
            depth
        };
        let (title, tags) = split_board_tags(text);
        if let Some(column) = &mut current {
            column.cards.push(BoardCard { depth, title, tags });
        }
    }

    if let Some(column) = current {
        columns.push(column);
    }
    if columns.is_empty() {
        columns.push(BoardColumn {
            title: "Board".to_string(),
            cards: Vec::new(),
        });
    }

    Ok(BoardDocument {
        title,
        columns,
        warnings,
    })
}

fn parse_board_column_title(line: &str) -> Option<String> {
    if let Some(inner) = line.strip_prefix("==").and_then(|v| v.strip_suffix("==")) {
        let title = inner.trim();
        if !title.is_empty() {
            return Some(title.to_string());
        }
    }
    if let Some(rest) = line.strip_prefix("column ") {
        let title = rest.trim();
        if !title.is_empty() {
            return Some(title.to_string());
        }
    }
    (line.chars().next().is_some_and(|ch| ch != '+')).then(|| line.to_string())
}

fn parse_board_card(line: &str) -> (usize, &str) {
    let depth = line.chars().take_while(|ch| *ch == '+').count();
    if depth == 0 {
        (1, line)
    } else {
        (depth, line[depth..].trim())
    }
}

fn split_board_tags(text: &str) -> (String, Vec<String>) {
    let mut title = Vec::new();
    let mut tags = Vec::new();
    for token in text.split_whitespace() {
        if token.starts_with('#') && token.len() > 1 {
            tags.push(token.trim_start_matches('#').to_string());
        } else {
            title.push(token);
        }
    }
    let title = if title.is_empty() {
        text.to_string()
    } else {
        title.join(" ")
    };
    (title, tags)
}

pub(super) fn normalize_files(document: Document) -> Result<FilesDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut warnings = Vec::new();
    let mut roots = Vec::new();
    let mut top_notes = Vec::new();
    let mut note_lines: Option<Vec<String>> = None;
    let mut pending_pre_file_notes = Vec::new();
    let mut last_path: Option<String> = None;

    for line in body {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('\'') {
            continue;
        }
        if trimmed.eq_ignore_ascii_case("<note>") {
            note_lines = Some(Vec::new());
            continue;
        }
        if trimmed.eq_ignore_ascii_case("</note>") {
            if let Some(lines) = note_lines.take() {
                let note = lines.join("\n").trim().to_string();
                if note.is_empty() {
                    continue;
                }
                if let Some(path) = &last_path {
                    attach_file_note(&mut roots, path, note);
                } else {
                    pending_pre_file_notes.push(note);
                }
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_FILES_NOTE_UNMATCHED] files note end appeared without a matching <note>",
                ));
            }
            continue;
        }
        if let Some(lines) = &mut note_lines {
            lines.push(line);
            continue;
        }

        if !trimmed.starts_with('/') {
            warnings.push(Diagnostic::warning(format!(
                "[W_FILES_UNSUPPORTED_LINE] files diagram lines must begin with `/`; skipped `{trimmed}`"
            )));
            continue;
        }
        if !pending_pre_file_notes.is_empty() {
            top_notes.append(&mut pending_pre_file_notes);
        }
        let path = trimmed.to_string();
        let depth = path.split('/').filter(|part| !part.is_empty()).count();
        if depth > FILE_DEPTH_WARNING {
            warnings.push(Diagnostic::warning(format!(
                "[W_FILES_DEPTH_LIMIT] files path depth {depth} exceeds the inspected renderer depth {FILE_DEPTH_WARNING}: `{path}`"
            )));
        }
        insert_file_path(&mut roots, &path);
        last_path = Some(path);
    }

    if note_lines.is_some() {
        warnings.push(Diagnostic::warning(
            "[W_FILES_NOTE_UNCLOSED] files note block was not closed with </note>",
        ));
    }
    top_notes.append(&mut pending_pre_file_notes);

    Ok(FilesDocument {
        title,
        roots,
        top_notes,
        warnings,
    })
}

fn insert_file_path(roots: &mut Vec<FileTreeNode>, raw_path: &str) {
    let is_dir_path = raw_path.ends_with('/');
    let parts = raw_path
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let mut current = roots;
    let mut accumulated = String::new();
    for (idx, part) in parts.iter().enumerate() {
        accumulated.push('/');
        accumulated.push_str(part);
        let last = idx + 1 == parts.len();
        let is_dir = !last || is_dir_path;
        let pos = current.iter().position(|node| node.name == *part);
        let node_idx = match pos {
            Some(idx) => {
                if is_dir {
                    current[idx].is_dir = true;
                }
                idx
            }
            None => {
                current.push(FileTreeNode {
                    name: (*part).to_string(),
                    path: accumulated.clone(),
                    is_dir,
                    notes: Vec::new(),
                    children: Vec::new(),
                });
                current.len() - 1
            }
        };
        current = &mut current[node_idx].children;
    }
}

fn attach_file_note(nodes: &mut [FileTreeNode], path: &str, note: String) -> bool {
    for node in nodes {
        if node.path == path {
            node.notes.push(note);
            return true;
        }
        if attach_file_note(&mut node.children, path, note.clone()) {
            return true;
        }
    }
    false
}
