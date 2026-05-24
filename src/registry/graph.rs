use crate::ast::{ComponentNodeKind, DiagramKind};
use crate::model::FamilyNodeKind;

use super::graph_specs::GRAPH_SPECS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphElementShapeKind {
    ClassBox,
    ObjectBox,
    Map,
    Diamond,
    UseCaseOval,
    Actor,
    Component,
    Interface,
    Port,
    Node,
    Artifact,
    Cloud,
    Database,
    Package,
    Folder,
    File,
    Card,
    Hexagon,
    Label,
    Queue,
    Stack,
    C4Box,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphElementStyleHook {
    Class,
    Component,
    Deployment,
    UseCase,
    C4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphRendererShape {
    ClassNode,
    ComponentGridNode,
    DeploymentGridNode,
    C4Node,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationEndpointSupport {
    Named,
    NamedOrBracketed,
    NamedOrLollipop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphElementSpec {
    pub keyword: &'static str,
    pub aliases: &'static [&'static str],
    pub source_families: &'static [DiagramKind],
    pub component_kind: Option<ComponentNodeKind>,
    pub family_node_kind: FamilyNodeKind,
    pub shape_kind: GraphElementShapeKind,
    pub style_hook: GraphElementStyleHook,
    pub renderer_shape: GraphRendererShape,
    pub renderer_label: &'static str,
    pub relation_endpoint: RelationEndpointSupport,
}

pub fn graph_element_specs() -> &'static [GraphElementSpec] {
    GRAPH_SPECS
}

pub fn component_declaration_keywords(
) -> impl Iterator<Item = (&'static str, ComponentNodeKind)> + Clone {
    graph_element_specs().iter().flat_map(|spec| {
        spec.component_kind.into_iter().flat_map(move |kind| {
            let primary = (spec.keyword != "usecase").then_some((spec.keyword, kind));
            primary
                .into_iter()
                .chain(spec.aliases.iter().copied().map(move |alias| (alias, kind)))
        })
    })
}

pub fn graph_element_for_component_kind(
    kind: ComponentNodeKind,
) -> Option<&'static GraphElementSpec> {
    graph_element_specs()
        .iter()
        .find(|spec| spec.component_kind == Some(kind))
}

pub fn graph_element_for_family_node_kind(
    kind: FamilyNodeKind,
) -> Option<&'static GraphElementSpec> {
    graph_element_specs()
        .iter()
        .find(|spec| spec.family_node_kind == kind)
}

pub fn is_mixed_graph_family(kind: DiagramKind) -> bool {
    matches!(
        kind,
        DiagramKind::Class
            | DiagramKind::Object
            | DiagramKind::UseCase
            | DiagramKind::Component
            | DiagramKind::Deployment
    )
}

pub fn mixed_graph_family(current: DiagramKind, candidate: DiagramKind) -> Option<DiagramKind> {
    if !is_mixed_graph_family(current) || !is_mixed_graph_family(candidate) {
        return None;
    }
    Some(match (current, candidate) {
        (DiagramKind::Deployment, _) | (_, DiagramKind::Deployment) => DiagramKind::Deployment,
        (DiagramKind::Component, _) | (_, DiagramKind::Component) => DiagramKind::Component,
        _ => current,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_keywords_are_backed_by_graph_element_specs() {
        let keywords = component_declaration_keywords()
            .map(|(keyword, _)| keyword)
            .collect::<Vec<_>>();
        assert!(keywords.contains(&"component"));
        assert!(keywords.contains(&"node"));
        assert!(keywords.contains(&"usecase/"));
    }

    #[test]
    fn graph_mixing_prefers_deployment_renderer_when_needed() {
        assert_eq!(
            mixed_graph_family(DiagramKind::Component, DiagramKind::Class),
            Some(DiagramKind::Component)
        );
        assert_eq!(
            mixed_graph_family(DiagramKind::Class, DiagramKind::Deployment),
            Some(DiagramKind::Deployment)
        );
    }
}
