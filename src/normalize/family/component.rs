use super::*;

pub(super) fn extract_inline_stereotype_members(label: &str) -> Vec<crate::ast::ClassMember> {
    let (_, stereotypes) = strip_inline_stereotypes_with_values(label);
    declaration_stereotype_members(stereotypes)
}

pub(super) fn scoped_component_kind_hint(kind: &str) -> Option<FamilyNodeKind> {
    Some(match kind {
        "action" => FamilyNodeKind::Action,
        "agent" => FamilyNodeKind::Agent,
        "component" => FamilyNodeKind::Component,
        "interface" => FamilyNodeKind::Interface,
        "port" => FamilyNodeKind::Port,
        "node" => FamilyNodeKind::Node,
        "artifact" => FamilyNodeKind::Artifact,
        "boundary" => FamilyNodeKind::Boundary,
        "cloud" => FamilyNodeKind::Cloud,
        "circle" => FamilyNodeKind::Circle,
        "collections" => FamilyNodeKind::Collections,
        "frame" => FamilyNodeKind::Frame,
        "storage" => FamilyNodeKind::Storage,
        "container" => FamilyNodeKind::Container,
        "control" => FamilyNodeKind::Control,
        "database" => FamilyNodeKind::Database,
        "entity" => FamilyNodeKind::Entity,
        "package" => FamilyNodeKind::Package,
        "rectangle" => FamilyNodeKind::Rectangle,
        "folder" => FamilyNodeKind::Folder,
        "file" => FamilyNodeKind::File,
        "card" => FamilyNodeKind::Card,
        "actor" => FamilyNodeKind::Actor,
        "hexagon" => FamilyNodeKind::Hexagon,
        "label" => FamilyNodeKind::Label,
        "person" => FamilyNodeKind::Person,
        "process" => FamilyNodeKind::Process,
        "queue" => FamilyNodeKind::Queue,
        "stack" => FamilyNodeKind::Stack,
        "usecase" => FamilyNodeKind::UseCaseDeployment,
        _ => return None,
    })
}

pub(super) fn strip_inline_stereotypes(label: String) -> String {
    strip_inline_stereotypes_with_values(&label).0
}

pub(super) fn strip_inline_stereotypes_with_values(label: &str) -> (String, Vec<String>) {
    let mut remaining = label.trim().to_string();
    let mut stereotypes = Vec::new();
    while let Some(start) = remaining.find("<<") {
        let Some(end_rel) = remaining[start + 2..].find(">>") else {
            break;
        };
        let end = start + 2 + end_rel;
        let value = remaining[start + 2..end].trim();
        if !value.is_empty() {
            stereotypes.push(value.to_string());
        }
        remaining.replace_range(start..end + 2, "");
    }
    (remaining.trim().to_string(), stereotypes)
}

pub(super) fn declaration_stereotype_members(
    stereotypes: Vec<String>,
) -> Vec<crate::ast::ClassMember> {
    stereotypes
        .into_iter()
        .map(|stereotype| {
            // Detect spot stereotype: `(L,#color) Label` → encode as `<<spot:L:#color:Label>>`.
            let text = if let Some((letter, color, label)) =
                crate::parser::parse_spot_stereotype(&stereotype)
            {
                format!("<<spot:{letter}:{color}:{label}>>")
            } else {
                format!("<<{stereotype}>>")
            };
            crate::ast::ClassMember {
                text,
                modifier: None,
            }
        })
        .collect()
}
