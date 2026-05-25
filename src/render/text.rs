use super::text_output::{
    finish_text, optional_label, push_meta, text_value, tree_branch, tree_leaf,
};
use super::text_specialized::{
    render_archimate_text, render_chart_text, render_ditaa_text, render_ebnf_text,
    render_json_text, render_math_text, render_nwdiag_text, render_regex_text, render_sdl_text,
    render_yaml_text,
};
use super::text_timeline::render_timeline_text;
use crate::model::{
    BoardDocument, FamilyDocument, FileTreeNode, FilesDocument, NormalizedDocument,
    ParticipantRole, SequenceEventKind, SequencePage, StateDocument, StateNode, StdlibDocument,
    VirtualEndpointKind, WbsCheckbox,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOutputMode {
    Txt,
    Atxt,
    Utxt,
}

impl TextOutputMode {
    pub(super) fn unicode(self) -> bool {
        matches!(self, Self::Utxt)
    }
}

pub fn render_text_pages(model: &NormalizedDocument, mode: TextOutputMode) -> Vec<String> {
    match model {
        NormalizedDocument::Sequence(sequence) => crate::normalize::paginate(sequence)
            .iter()
            .map(|page| render_sequence_text(page, mode))
            .collect(),
        NormalizedDocument::Family(family) => vec![render_family_text(family, mode)],
        NormalizedDocument::FamilyPages(pages) => pages
            .iter()
            .map(|family| render_family_text(family, mode))
            .collect(),
        NormalizedDocument::Timeline(timeline) => vec![render_timeline_text(timeline, mode)],
        NormalizedDocument::State(state) => vec![render_state_text(state, mode)],
        NormalizedDocument::Json(doc) => vec![render_json_text(doc, mode)],
        NormalizedDocument::Yaml(doc) => vec![render_yaml_text(doc, mode)],
        NormalizedDocument::Nwdiag(doc) => vec![render_nwdiag_text(doc, mode)],
        NormalizedDocument::Archimate(doc) => vec![render_archimate_text(doc, mode)],
        NormalizedDocument::Regex(doc) => vec![render_regex_text(doc, mode)],
        NormalizedDocument::Ebnf(doc) => vec![render_ebnf_text(doc, mode)],
        NormalizedDocument::Math(doc) => vec![render_math_text(doc, mode)],
        NormalizedDocument::Sdl(doc) => vec![render_sdl_text(doc, mode)],
        NormalizedDocument::Ditaa(doc) => vec![render_ditaa_text(doc, mode)],
        NormalizedDocument::Chart(doc) => vec![render_chart_text(doc, mode)],
        NormalizedDocument::Stdlib(doc) => vec![render_stdlib_text(doc, mode)],
        NormalizedDocument::Chen(doc) => vec![render_chen_text(doc, mode)],
        NormalizedDocument::Board(doc) => vec![render_board_text(doc, mode)],
        NormalizedDocument::Files(doc) => vec![render_files_text(doc, mode)],
    }
}

fn render_board_text(doc: &BoardDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    lines.push("board".to_string());
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    for column in &doc.columns {
        lines.push(format!("column {}", text_value(&column.title, mode)));
        for card in &column.cards {
            let tags = if card.tags.is_empty() {
                String::new()
            } else {
                format!(" #{}", card.tags.join(" #"))
            };
            lines.push(format!(
                "{}{}{}",
                spaces(card.depth),
                text_value(&card.title, mode),
                tags
            ));
        }
    }
    finish_text(lines)
}

fn render_files_text(doc: &FilesDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    lines.push("files".to_string());
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    for note in &doc.top_notes {
        lines.push(format!("note {}", text_value(note, mode)));
    }
    for node in &doc.roots {
        push_file_text_node(&mut lines, node, 0, mode);
    }
    finish_text(lines)
}

fn push_file_text_node(
    lines: &mut Vec<String>,
    node: &FileTreeNode,
    depth: usize,
    mode: TextOutputMode,
) {
    let kind = if node.is_dir { "dir" } else { "file" };
    lines.push(format!(
        "{}{} {}",
        spaces(depth + 1),
        kind,
        text_value(&node.name, mode)
    ));
    for note in &node.notes {
        lines.push(format!(
            "{}note {}",
            spaces(depth + 2),
            text_value(note, mode)
        ));
    }
    for child in &node.children {
        push_file_text_node(lines, child, depth + 1, mode);
    }
}

fn render_stdlib_text(doc: &StdlibDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    lines.push("stdlib".to_string());
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    push_meta(&mut lines, "root", Some(&doc.root), mode);
    lines.push(format!("entries ({})", doc.entries.len()));
    lines.push("packs".to_string());
    for pack in &doc.packs {
        let status = match pack.status {
            crate::stdlib::StdlibPackStatus::Available => "available",
            crate::stdlib::StdlibPackStatus::Unavailable => "unavailable",
        };
        lines.push(format!(
            "  {} {} files={} aliases={}",
            status,
            text_value(&pack.name, mode),
            pack.files,
            pack.aliases
        ));
    }
    lines.push("aliases".to_string());
    for (slug, target) in &doc.aliases {
        lines.push(format!(
            "  {} -> {}",
            text_value(slug, mode),
            text_value(target, mode)
        ));
    }
    finish_text(lines)
}

fn render_chen_text(doc: &crate::model::ChenDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    lines.push("chen".to_string());
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push(format!("nodes ({})", doc.nodes.len()));
    lines.push(format!("relations ({})", doc.relations.len()));
    lines.push(format!("inheritances ({})", doc.inheritances.len()));
    finish_text(lines)
}

fn render_sequence_text(page: &SequencePage, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "header", page.header.as_deref(), mode);
    push_meta(&mut lines, "title", page.title.as_deref(), mode);
    lines.push("sequence".to_string());
    lines.push(format!("participants ({})", page.participants.len()));
    for participant in &page.participants {
        lines.push(format!(
            "  {}{}{}",
            participant_role_name(participant.role),
            text_alias_suffix(&participant.id, &participant.display, mode),
            text_value(&participant.display, mode)
        ));
    }

    lines.push("events".to_string());
    let mut indent = 1usize;
    for event in &page.events {
        match &event.kind {
            SequenceEventKind::Message {
                from,
                to,
                arrow,
                label,
                style,
                from_virtual,
                to_virtual,
            } => {
                let mut line = format!(
                    "{}{} {} {}",
                    spaces(indent),
                    endpoint_label(from, *from_virtual, mode),
                    text_value(arrow, mode),
                    endpoint_label(to, *to_virtual, mode)
                );
                if let Some(label) = label {
                    line.push_str(": ");
                    line.push_str(&text_value(label, mode));
                }
                let style_tags = sequence_style_tags(style);
                if !style_tags.is_empty() {
                    line.push_str(" [");
                    line.push_str(&style_tags.join(","));
                    line.push(']');
                }
                lines.push(line);
            }
            SequenceEventKind::Note {
                kind,
                position,
                target,
                text,
                ..
            } => {
                let target = target
                    .as_deref()
                    .map(|v| format!(" {}", text_value(v, mode)))
                    .unwrap_or_default();
                lines.push(format!(
                    "{}note {:?} {}{}: {}",
                    spaces(indent),
                    kind,
                    text_value(position, mode),
                    target,
                    text_value(text, mode)
                ));
            }
            SequenceEventKind::GroupStart { kind, label } => {
                let label = label
                    .as_deref()
                    .map(|v| format!(" {}", text_value(v, mode)))
                    .unwrap_or_default();
                lines.push(format!(
                    "{}{}{}",
                    spaces(indent),
                    text_value(kind, mode),
                    label
                ));
                indent += 1;
            }
            SequenceEventKind::GroupEnd => {
                indent = indent.saturating_sub(1).max(1);
                lines.push(format!("{}end", spaces(indent)));
            }
            SequenceEventKind::Delay(label) => {
                lines.push(format!(
                    "{}delay{}",
                    spaces(indent),
                    optional_label(label.as_deref(), mode)
                ));
            }
            SequenceEventKind::Divider(label) => {
                lines.push(format!(
                    "{}divider{}",
                    spaces(indent),
                    optional_label(label.as_deref(), mode)
                ));
            }
            SequenceEventKind::Separator(label) => {
                lines.push(format!(
                    "{}separator{}",
                    spaces(indent),
                    optional_label(label.as_deref(), mode)
                ));
            }
            SequenceEventKind::Spacer(pixels) => {
                lines.push(format!(
                    "{}spacer{}",
                    spaces(indent),
                    pixels.map(|v| format!(" {v}px")).unwrap_or_default()
                ));
            }
            SequenceEventKind::NewPage(label) => {
                lines.push(format!(
                    "{}newpage{}",
                    spaces(indent),
                    optional_label(label.as_deref(), mode)
                ));
            }
            SequenceEventKind::Autonumber(value) => {
                lines.push(format!(
                    "{}autonumber{}",
                    spaces(indent),
                    optional_label(value.as_deref(), mode)
                ));
            }
            SequenceEventKind::Activate(id) => {
                lines.push(format!(
                    "{}activate {}",
                    spaces(indent),
                    text_value(id, mode)
                ));
            }
            SequenceEventKind::Deactivate(id) => {
                lines.push(format!(
                    "{}deactivate {}",
                    spaces(indent),
                    text_value(id, mode)
                ));
            }
            SequenceEventKind::Destroy(id) => {
                lines.push(format!(
                    "{}destroy {}",
                    spaces(indent),
                    text_value(id, mode)
                ));
            }
            SequenceEventKind::Create(id) => {
                lines.push(format!("{}create {}", spaces(indent), text_value(id, mode)));
            }
            SequenceEventKind::Return { label, from, to } => {
                let route = match (from.as_deref(), to.as_deref()) {
                    (Some(from), Some(to)) => {
                        format!(" {} -> {}", text_value(from, mode), text_value(to, mode))
                    }
                    _ => String::new(),
                };
                lines.push(format!(
                    "{}return{}{}",
                    spaces(indent),
                    route,
                    optional_label(label.as_deref(), mode)
                ));
            }
            SequenceEventKind::IncludePlaceholder(value) => {
                lines.push(format!(
                    "{}include {}",
                    spaces(indent),
                    text_value(value, mode)
                ));
            }
            SequenceEventKind::DefinePlaceholder { name, value } => {
                lines.push(format!(
                    "{}define {}{}",
                    spaces(indent),
                    text_value(name, mode),
                    optional_label(value.as_deref(), mode)
                ));
            }
            SequenceEventKind::UndefPlaceholder(value) => {
                lines.push(format!(
                    "{}undef {}",
                    spaces(indent),
                    text_value(value, mode)
                ));
            }
        }
    }
    push_meta(&mut lines, "caption", page.caption.as_deref(), mode);
    push_meta(&mut lines, "legend", page.legend.as_deref(), mode);
    finish_text(lines)
}

fn render_family_text(doc: &FamilyDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "header", doc.header.as_deref(), mode);
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push(format!(
        "{:?} orientation={}",
        doc.kind,
        doc.orientation.as_str()
    ));
    if !doc.groups.is_empty() {
        lines.push("groups".to_string());
        for group in &doc.groups {
            lines.push(format!(
                "  {}{} members=[{}]",
                text_value(&group.kind, mode),
                optional_label(group.label.as_deref(), mode),
                group
                    .member_ids
                    .iter()
                    .map(|v| text_value(v, mode))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    lines.push(format!("nodes ({})", doc.nodes.len()));
    for (idx, node) in doc.nodes.iter().enumerate() {
        let branch = if idx + 1 == doc.nodes.len() {
            tree_leaf(mode)
        } else {
            tree_branch(mode)
        };
        let label = node.label.as_deref().unwrap_or(&node.name);
        let alias = node
            .alias
            .as_deref()
            .filter(|alias| *alias != node.name)
            .map(|alias| format!(" as {}", text_value(alias, mode)))
            .unwrap_or_default();
        let checkbox = node
            .wbs_checkbox
            .as_ref()
            .map(|v| format!(" {}", wbs_checkbox_text(v)))
            .unwrap_or_default();
        lines.push(format!(
            "{}{}{:?} {}{}{}",
            "  ".repeat(node.depth),
            branch,
            node.kind,
            text_value(label, mode),
            alias,
            checkbox
        ));
        for member in &node.members {
            lines.push(format!(
                "{}  member {}",
                "  ".repeat(node.depth + 1),
                text_value(&member.text, mode)
            ));
        }
    }
    if !doc.relations.is_empty() {
        lines.push(format!("relations ({})", doc.relations.len()));
        for rel in &doc.relations {
            let mut detail = String::new();
            if let Some(label) = &rel.label {
                detail.push_str(": ");
                detail.push_str(&text_value(label, mode));
            }
            if rel.dashed {
                detail.push_str(" [dashed]");
            }
            if rel.hidden {
                detail.push_str(" [hidden]");
            }
            lines.push(format!(
                "  {} {} {}{}",
                text_value(&rel.from, mode),
                text_value(&rel.arrow, mode),
                text_value(&rel.to, mode),
                detail
            ));
        }
    }
    push_meta(&mut lines, "caption", doc.caption.as_deref(), mode);
    push_meta(&mut lines, "legend", doc.legend.as_deref(), mode);
    finish_text(lines)
}

fn render_state_text(doc: &StateDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "header", doc.header.as_deref(), mode);
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("state".to_string());
    lines.push(format!("nodes ({})", doc.nodes.len()));
    for (idx, node) in doc.nodes.iter().enumerate() {
        render_state_node_text(&mut lines, node, mode, 1, idx + 1 == doc.nodes.len());
    }
    if !doc.transitions.is_empty() {
        lines.push(format!("transitions ({})", doc.transitions.len()));
        for transition in &doc.transitions {
            let mut line = format!(
                "  {} -> {}",
                text_value(&transition.from, mode),
                text_value(&transition.to, mode)
            );
            if let Some(label) = &transition.label {
                line.push_str(": ");
                line.push_str(&text_value(label, mode));
            }
            if transition.dashed {
                line.push_str(" [dashed]");
            }
            if transition.hidden {
                line.push_str(" [hidden]");
            }
            lines.push(line);
        }
    }
    push_meta(&mut lines, "caption", doc.caption.as_deref(), mode);
    push_meta(&mut lines, "legend", doc.legend.as_deref(), mode);
    finish_text(lines)
}

fn render_state_node_text(
    lines: &mut Vec<String>,
    node: &StateNode,
    mode: TextOutputMode,
    depth: usize,
    last: bool,
) {
    let branch = if last {
        tree_leaf(mode)
    } else {
        tree_branch(mode)
    };
    let display = node.display.as_deref().unwrap_or(&node.name);
    lines.push(format!(
        "{}{}{:?} {}",
        "  ".repeat(depth),
        branch,
        node.kind,
        text_value(display, mode)
    ));
    for action in &node.internal_actions {
        lines.push(format!(
            "{}  {} / {}",
            "  ".repeat(depth + 1),
            text_value(&action.kind, mode),
            text_value(&action.action, mode)
        ));
    }
    for (region_idx, region) in node.regions.iter().enumerate() {
        lines.push(format!(
            "{}region {}",
            "  ".repeat(depth + 1),
            region_idx + 1
        ));
        for (idx, child) in region.iter().enumerate() {
            render_state_node_text(lines, child, mode, depth + 2, idx + 1 == region.len());
        }
    }
}

fn text_alias_suffix(id: &str, display: &str, mode: TextOutputMode) -> String {
    if id == display {
        " ".to_string()
    } else {
        format!(" {} as ", text_value(id, mode))
    }
}

fn endpoint_label(
    id: &str,
    endpoint: Option<crate::model::VirtualEndpoint>,
    mode: TextOutputMode,
) -> String {
    if let Some(endpoint) = endpoint {
        let side = match endpoint.side {
            crate::model::VirtualEndpointSide::Left => "left",
            crate::model::VirtualEndpointSide::Right => "right",
        };
        let kind = match endpoint.kind {
            VirtualEndpointKind::Plain => "plain",
            VirtualEndpointKind::Circle => "circle",
            VirtualEndpointKind::Cross => "cross",
            VirtualEndpointKind::Filled => "filled",
            VirtualEndpointKind::Short => "short",
        };
        return format!("[{side}:{kind}]");
    }
    text_value(id, mode)
}

fn sequence_style_tags(style: &crate::model::SequenceMessageStyle) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if style.hidden {
        tags.push("hidden");
    }
    if style.dashed {
        tags.push("dashed");
    }
    if style.dotted {
        tags.push("dotted");
    }
    if style.parallel {
        tags.push("parallel");
    }
    tags
}

fn participant_role_name(role: ParticipantRole) -> &'static str {
    match role {
        ParticipantRole::Participant => "participant",
        ParticipantRole::Actor => "actor",
        ParticipantRole::Boundary => "boundary",
        ParticipantRole::Control => "control",
        ParticipantRole::Entity => "entity",
        ParticipantRole::Database => "database",
        ParticipantRole::Collections => "collections",
        ParticipantRole::Queue => "queue",
    }
}

fn wbs_checkbox_text(value: &WbsCheckbox) -> String {
    match value {
        WbsCheckbox::Checked => "[x]".to_string(),
        WbsCheckbox::Unchecked => "[ ]".to_string(),
        WbsCheckbox::Progress(value) => format!("[{value}%]"),
    }
}

fn spaces(indent: usize) -> String {
    "  ".repeat(indent)
}
