use super::super::arrows::ActivityArrowStyle;
use crate::model::FamilyDocument;

// ---------------------------------------------------------------------------
// Node metadata (Pass 0)
// ---------------------------------------------------------------------------

pub(in crate::render::activity) struct NodeMeta {
    pub step_kind: String,
    pub lane_name: String,
    pub fork_branch: usize,
    pub arrow_style: Option<ActivityArrowStyle>,
    /// SDL action shape (e.g. "send", "receive", "input", "output", "bar", "bracket", "brace")
    pub sdl_shape: Option<String>,
    pub note_side: Option<String>,
    pub note_floating: bool,
    /// Whether the swimlane header should be rendered in bold (from `|= Name|`)
    pub swim_bold: bool,
    /// Stereotype text for the swimlane header (from `|<<role>>Name|`)
    pub swim_stereotype: Option<String>,
}

pub(in crate::render::activity) fn parse_node_metas(doc: &FamilyDocument) -> Vec<NodeMeta> {
    doc.nodes
        .iter()
        .map(|node| {
            let mut step_kind = String::new();
            let mut lane_name = "default".to_string();
            let mut fork_branch = 0usize;
            let mut sdl_shape: Option<String> = None;
            let mut note_side: Option<String> = None;
            let mut note_floating = false;
            let mut swim_bold = false;
            let mut swim_stereotype: Option<String> = None;
            if let Some(alias) = &node.alias {
                if let Some(meta) = alias.strip_prefix("activity::") {
                    for (pi, part) in meta.split('|').enumerate() {
                        if pi == 0 {
                            step_kind = part.to_string();
                            continue;
                        }
                        if let Some(v) = part.strip_prefix("lane=") {
                            lane_name = v.to_string();
                        } else if let Some(v) = part.strip_prefix("fork_branch=") {
                            fork_branch = v.parse::<usize>().unwrap_or(0);
                        } else if let Some(v) = part.strip_prefix("sdl=") {
                            sdl_shape = Some(v.to_string());
                        } else if let Some(v) = part.strip_prefix("note_side=") {
                            note_side = Some(v.to_string());
                        } else if let Some(v) = part.strip_prefix("position=") {
                            note_side = Some(v.to_string());
                        } else if let Some(v) = part.strip_prefix("note_floating=") {
                            note_floating = v == "1" || v.eq_ignore_ascii_case("true");
                        } else if part == "swim_bold=1" {
                            swim_bold = true;
                        } else if let Some(v) = part.strip_prefix("swim_stereotype=") {
                            swim_stereotype = Some(v.to_string());
                        }
                    }
                }
            }
            let arrow_style = node.label.as_deref().and_then(parse_activity_arrow_style);
            NodeMeta {
                step_kind,
                lane_name,
                fork_branch,
                arrow_style,
                sdl_shape,
                note_side,
                note_floating,
                swim_bold,
                swim_stereotype,
            }
        })
        .collect()
}

fn parse_activity_arrow_style(label: &str) -> Option<ActivityArrowStyle> {
    let mut parts = label.split('\x1f');
    if !parts.next()?.is_empty() || parts.next()? != "activity:arrow" {
        return None;
    }
    let mut style = ActivityArrowStyle::default();
    for part in parts {
        if let Some(value) = part.strip_prefix("color:") {
            style.color = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("label:") {
            style.label = Some(value.to_string());
        } else if part == "dashed:1" {
            style.dashed = true;
        } else if part == "hidden:1" {
            style.hidden = true;
        } else if part == "bold:1" {
            style.bold = true;
        } else if part == "no_head:1" {
            style.no_head = true;
        }
    }
    Some(style)
}
