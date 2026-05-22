use crate::model::{
    ArchimateDocument, ChartDocument, DitaaDocument, EbnfDocument, EbnfToken, FamilyDocument,
    JsonDocument, MathDocument, NormalizedDocument, NwdiagDocument, ParticipantRole, RegexDocument,
    RegexToken, RepeatKind, SdlDocument, SequenceEventKind, SequencePage, StateDocument, StateNode,
    TimelineDocument, VirtualEndpointKind, WbsCheckbox, YamlDocument,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOutputMode {
    Txt,
    Atxt,
    Utxt,
}

impl TextOutputMode {
    fn unicode(self) -> bool {
        matches!(self, Self::Utxt)
    }

    fn tree_branch(self) -> &'static str {
        if self.unicode() {
            "\u{251c}\u{2500} "
        } else {
            "|- "
        }
    }

    fn tree_leaf(self) -> &'static str {
        if self.unicode() {
            "\u{2514}\u{2500} "
        } else {
            "`- "
        }
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
    }
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
            mode.tree_leaf()
        } else {
            mode.tree_branch()
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
        mode.tree_leaf()
    } else {
        mode.tree_branch()
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

fn render_timeline_text(doc: &TimelineDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "header", doc.header.as_deref(), mode);
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push(format!("{:?}", doc.kind));
    if let Some(start) = &doc.project_start {
        lines.push(format!("project starts {}", text_value(start, mode)));
    }
    if !doc.tasks.is_empty() {
        lines.push(format!("tasks ({})", doc.tasks.len()));
        for task in &doc.tasks {
            let critical = if task.is_critical { " critical" } else { "" };
            lines.push(format!(
                "  {} start={} duration={} workload={}{}",
                text_value(&task.name, mode),
                task.start_day,
                task.duration_days,
                task.workload_days,
                critical
            ));
        }
    }
    if !doc.milestones.is_empty() {
        lines.push(format!("milestones ({})", doc.milestones.len()));
        for milestone in &doc.milestones {
            let when = milestone
                .happens_on
                .as_deref()
                .map(|v| format!(" on {}", text_value(v, mode)))
                .unwrap_or_default();
            let critical = if milestone.is_critical {
                " critical"
            } else {
                ""
            };
            lines.push(format!(
                "  {}{}{}",
                text_value(&milestone.name, mode),
                when,
                critical
            ));
        }
    }
    if !doc.chronology_events.is_empty() {
        lines.push(format!("events ({})", doc.chronology_events.len()));
        for event in &doc.chronology_events {
            lines.push(format!(
                "  {} happens {}",
                text_value(&event.subject, mode),
                text_value(&event.when, mode)
            ));
        }
    }
    if !doc.constraints.is_empty() {
        lines.push(format!("constraints ({})", doc.constraints.len()));
        for c in &doc.constraints {
            lines.push(format!(
                "  {} {} {}",
                text_value(&c.subject, mode),
                text_value(&c.kind, mode),
                text_value(&c.target, mode)
            ));
        }
    }
    push_meta(&mut lines, "caption", doc.caption.as_deref(), mode);
    push_meta(&mut lines, "legend", doc.legend.as_deref(), mode);
    finish_text(lines)
}

fn render_json_text(doc: &JsonDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("json".to_string());
    for (idx, node) in doc.nodes.iter().enumerate() {
        let branch = if idx + 1 == doc.nodes.len() {
            mode.tree_leaf()
        } else {
            mode.tree_branch()
        };
        lines.push(format!(
            "{}{}{}",
            "  ".repeat(node.depth),
            branch,
            text_value(&node.label, mode)
        ));
    }
    finish_text(lines)
}

fn render_yaml_text(doc: &YamlDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("yaml".to_string());
    for (idx, node) in doc.nodes.iter().enumerate() {
        let branch = if idx + 1 == doc.nodes.len() {
            mode.tree_leaf()
        } else {
            mode.tree_branch()
        };
        lines.push(format!(
            "{}{}{}",
            "  ".repeat(node.depth),
            branch,
            text_value(&node.label, mode)
        ));
    }
    finish_text(lines)
}

fn render_nwdiag_text(doc: &NwdiagDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("nwdiag".to_string());
    for network in &doc.networks {
        lines.push(format!(
            "network {}{}",
            text_value(&network.name, mode),
            optional_label(network.address.as_deref(), mode)
        ));
        for node in &network.nodes {
            let address = node
                .address
                .as_deref()
                .map(|v| format!(" address={}", text_value(v, mode)))
                .unwrap_or_default();
            lines.push(format!(
                "  node {}{}",
                text_value(node.label.as_deref().unwrap_or(&node.name), mode),
                address
            ));
        }
    }
    if !doc.groups.is_empty() {
        lines.push("groups".to_string());
        for group in &doc.groups {
            lines.push(format!(
                "  {} members=[{}]",
                text_value(group.label.as_deref().unwrap_or(&group.name), mode),
                group
                    .nodes
                    .iter()
                    .map(|v| text_value(v, mode))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    finish_text(lines)
}

fn render_archimate_text(doc: &ArchimateDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("archimate".to_string());
    lines.push(format!("elements ({})", doc.elements.len()));
    for element in &doc.elements {
        lines.push(format!(
            "  {} {} {}",
            text_value(&element.layer, mode),
            text_value(&element.kind, mode),
            text_value(&element.name, mode)
        ));
    }
    if !doc.relations.is_empty() {
        lines.push(format!("relations ({})", doc.relations.len()));
        for relation in &doc.relations {
            lines.push(format!(
                "  {} -{}-> {}{}",
                text_value(&relation.from, mode),
                text_value(&relation.kind, mode),
                text_value(&relation.to, mode),
                optional_label(relation.label.as_deref(), mode)
            ));
        }
    }
    finish_text(lines)
}

fn render_regex_text(doc: &RegexDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("regex".to_string());
    for pattern in &doc.patterns {
        lines.push(format!("pattern {}", text_value(&pattern.source, mode)));
        for token in &pattern.tokens {
            lines.push(format!("  {}", regex_token_text(token, mode)));
        }
    }
    finish_text(lines)
}

fn render_ebnf_text(doc: &EbnfDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("ebnf".to_string());
    for rule in &doc.rules {
        lines.push(format!(
            "{} ::= {}",
            text_value(&rule.name, mode),
            text_value(&rule.body, mode)
        ));
        for token in &rule.tokens {
            lines.push(format!("  {}", ebnf_token_text(token, mode)));
        }
    }
    finish_text(lines)
}

fn render_math_text(doc: &MathDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("math".to_string());
    for line in doc.body.lines() {
        lines.push(format!("  {}", text_value(line, mode)));
    }
    finish_text(lines)
}

fn render_sdl_text(doc: &SdlDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("sdl".to_string());
    lines.push(format!("states ({})", doc.states.len()));
    for state in &doc.states {
        lines.push(format!(
            "  {:?} {}",
            state.kind,
            text_value(&state.name, mode)
        ));
    }
    if !doc.transitions.is_empty() {
        lines.push(format!("transitions ({})", doc.transitions.len()));
        for transition in &doc.transitions {
            lines.push(format!(
                "  {} -> {}{}",
                text_value(&transition.from, mode),
                text_value(&transition.to, mode),
                optional_label(transition.signal.as_deref(), mode)
            ));
        }
    }
    finish_text(lines)
}

fn render_ditaa_text(doc: &DitaaDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push("ditaa".to_string());
    for line in doc.body.lines() {
        lines.push(text_value(line, mode));
    }
    finish_text(lines)
}

fn render_chart_text(doc: &ChartDocument, mode: TextOutputMode) -> String {
    let mut lines = Vec::new();
    push_meta(&mut lines, "title", doc.title.as_deref(), mode);
    lines.push(format!("chart {:?}", doc.subtype));
    if !doc.data.is_empty() {
        lines.push(format!("data ({})", doc.data.len()));
        for point in &doc.data {
            lines.push(format!(
                "  {} = {}",
                text_value(&point.label, mode),
                format_number(point.value)
            ));
        }
    }
    if !doc.series.is_empty() {
        lines.push(format!("series ({})", doc.series.len()));
        for series in &doc.series {
            let values = series
                .values
                .iter()
                .map(|value| format_number(*value))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "  {} = [{}]",
                text_value(&series.name, mode),
                values
            ));
        }
    }
    push_meta(&mut lines, "caption", doc.caption.as_deref(), mode);
    finish_text(lines)
}

fn push_meta(lines: &mut Vec<String>, key: &str, value: Option<&str>, mode: TextOutputMode) {
    if let Some(value) = value {
        let value = text_value(value, mode);
        if !value.is_empty() {
            lines.push(format!("{key}: {value}"));
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

fn optional_label(value: Option<&str>, mode: TextOutputMode) -> String {
    value
        .map(|v| format!(" {}", text_value(v, mode)))
        .unwrap_or_default()
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

fn regex_token_text(token: &RegexToken, mode: TextOutputMode) -> String {
    match token {
        RegexToken::Literal(value) => format!("literal {}", text_value(value, mode)),
        RegexToken::CharClass(value) => format!("class {}", text_value(value, mode)),
        RegexToken::Group(tokens) => format!(
            "group [{}]",
            token_list_text(tokens, regex_token_text, mode)
        ),
        RegexToken::Alt(branches) => format!(
            "alt {}",
            branches
                .iter()
                .map(|branch| token_list_text(branch, regex_token_text, mode))
                .collect::<Vec<_>>()
                .join(" | ")
        ),
        RegexToken::Repeat { inner, kind } => {
            format!(
                "repeat {} {}",
                repeat_kind_text(kind),
                regex_token_text(inner, mode)
            )
        }
        RegexToken::Escape(ch) => format!("escape {}", text_value(&ch.to_string(), mode)),
        RegexToken::AnyChar => "any".to_string(),
        RegexToken::Anchor(value) => format!("anchor {}", text_value(value, mode)),
        RegexToken::Unsupported(value) => format!("unsupported {}", text_value(value, mode)),
    }
}

fn ebnf_token_text(token: &EbnfToken, mode: TextOutputMode) -> String {
    match token {
        EbnfToken::Terminal(value) => format!("terminal {}", text_value(value, mode)),
        EbnfToken::NonTerminal(value) => format!("nonterminal {}", text_value(value, mode)),
        EbnfToken::Alt(branches) => format!(
            "alt {}",
            branches
                .iter()
                .map(|branch| token_list_text(branch, ebnf_token_text, mode))
                .collect::<Vec<_>>()
                .join(" | ")
        ),
        EbnfToken::Group(tokens) => {
            format!("group [{}]", token_list_text(tokens, ebnf_token_text, mode))
        }
        EbnfToken::Optional(tokens) => {
            format!(
                "optional [{}]",
                token_list_text(tokens, ebnf_token_text, mode)
            )
        }
        EbnfToken::Repetition(tokens) => {
            format!(
                "repetition [{}]",
                token_list_text(tokens, ebnf_token_text, mode)
            )
        }
        EbnfToken::Repeat { inner, kind } => {
            format!(
                "repeat {} {}",
                repeat_kind_text(kind),
                ebnf_token_text(inner, mode)
            )
        }
        EbnfToken::Unsupported(value) => format!("unsupported {}", text_value(value, mode)),
    }
}

fn token_list_text<T>(
    tokens: &[T],
    formatter: fn(&T, TextOutputMode) -> String,
    mode: TextOutputMode,
) -> String {
    tokens
        .iter()
        .map(|token| formatter(token, mode))
        .collect::<Vec<_>>()
        .join(", ")
}

fn repeat_kind_text(kind: &RepeatKind) -> String {
    match kind {
        RepeatKind::ZeroOrOne => "?".to_string(),
        RepeatKind::ZeroOrMore => "*".to_string(),
        RepeatKind::OneOrMore => "+".to_string(),
        RepeatKind::Exact(value) => format!("{{{value}}}"),
        RepeatKind::Range { min, max } => format!(
            "{{{},{}}}",
            min.map(|v| v.to_string()).unwrap_or_default(),
            max.map(|v| v.to_string()).unwrap_or_default()
        ),
    }
}

fn spaces(indent: usize) -> String {
    "  ".repeat(indent)
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        let mut raw = format!("{value:.6}");
        while raw.contains('.') && raw.ends_with('0') {
            raw.pop();
        }
        if raw.ends_with('.') {
            raw.pop();
        }
        raw
    }
}

fn text_value(value: &str, mode: TextOutputMode) -> String {
    let single_line = value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    if mode.unicode() {
        return single_line;
    }
    single_line
        .chars()
        .map(|ch| {
            if ch.is_ascii() && !ch.is_control() {
                ch
            } else if ch == '\t' {
                ' '
            } else {
                '?'
            }
        })
        .collect()
}

fn finish_text(lines: Vec<String>) -> String {
    let mut out = lines.join("\n");
    out.push('\n');
    out
}
