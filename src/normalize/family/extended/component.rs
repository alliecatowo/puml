use super::*;

/// Process a `ComponentDecl` statement: push the component node and any
/// named port nodes that were encoded inside a multiline `[...]` body.
pub(super) fn normalize_component_decl(
    nodes: &mut Vec<FamilyNode>,
    kind: crate::ast::ComponentNodeKind,
    name: String,
    alias: Option<String>,
    label: Option<String>,
    mut members: Vec<ClassMember>,
) {
    let node_kind = component_node_kind(kind);
    let fill_color = extract_family_node_fill_color(&mut members);
    // Extract encoded named-port declarations from multiline component blocks.
    // Lines like `port http_port` inside `component "Server" [...]` are
    // encoded as `\x1fcomponent:port:<direction>:<name>` members by the parser.
    // We emit them as separate Port nodes scoped under the component name.
    let component_id = alias.as_deref().unwrap_or(name.as_str()).to_string();
    let mut port_decls: Vec<(String, String)> = Vec::new();
    members.retain(|m| {
        if let Some(rest) = m.text.strip_prefix("\x1fcomponent:port:") {
            if let Some((direction, port_name)) = rest.split_once(':') {
                port_decls.push((direction.to_string(), port_name.to_string()));
            }
            false
        } else {
            true
        }
    });
    nodes.push(FamilyNode {
        kind: node_kind,
        name,
        alias,
        members,
        depth: 0,
        label,
        mindmap_side: MindMapSide::Right,
        wbs_checkbox: None,
        fill_color,
    });
    // Emit one Port node per declared named port, scoped as `ComponentId::port_name`.
    for (direction_hint, port_name) in port_decls {
        let scoped_name = format!("{component_id}::{port_name}");
        let port_members = if direction_hint == "portin" {
            vec![ClassMember {
                text: "<<portin>>".to_string(),
                modifier: None,
            }]
        } else if direction_hint == "portout" {
            vec![ClassMember {
                text: "<<portout>>".to_string(),
                modifier: None,
            }]
        } else {
            Vec::new()
        };
        nodes.push(FamilyNode {
            kind: FamilyNodeKind::Port,
            name: scoped_name.clone(),
            alias: Some(port_name.clone()),
            members: port_members,
            depth: 0,
            label: Some(port_name),
            mindmap_side: MindMapSide::Right,
            wbs_checkbox: None,
            fill_color: None,
        });
    }
}
