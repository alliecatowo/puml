use crate::ast::{ComponentNodeKind, DiagramKind};
use crate::model::FamilyNodeKind;
use crate::DiagramFamily;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyRenderKind {
    Sequence,
    FamilyStub,
    Salt,
    Component,
    Deployment,
    Activity,
    Timing,
    MindMap,
    Wbs,
    Timeline,
    State,
    Json,
    Yaml,
    Nwdiag,
    Archimate,
    Regex,
    Ebnf,
    Math,
    Sdl,
    Ditaa,
    Chart,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagramFamilyCapabilities {
    pub svg: bool,
    pub text: bool,
    pub metadata: bool,
    pub language_service: bool,
    pub plantuml_frontend: bool,
    pub mermaid_frontend: bool,
    pub picouml_frontend: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagramFamilySpec {
    pub ast_kind: DiagramKind,
    pub public_family: DiagramFamily,
    pub name: &'static str,
    pub render_kind: FamilyRenderKind,
    pub capabilities: DiagramFamilyCapabilities,
}

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

const ALL_FRONTENDS: DiagramFamilyCapabilities = DiagramFamilyCapabilities {
    svg: true,
    text: true,
    metadata: true,
    language_service: true,
    plantuml_frontend: true,
    mermaid_frontend: true,
    picouml_frontend: true,
};

const PLANTUML_ONLY: DiagramFamilyCapabilities = DiagramFamilyCapabilities {
    svg: true,
    text: true,
    metadata: true,
    language_service: true,
    plantuml_frontend: true,
    mermaid_frontend: false,
    picouml_frontend: false,
};

const FAMILY_SPECS: &[DiagramFamilySpec] = &[
    family(
        DiagramKind::Sequence,
        DiagramFamily::Sequence,
        "sequence",
        FamilyRenderKind::Sequence,
        ALL_FRONTENDS,
    ),
    family(
        DiagramKind::Class,
        DiagramFamily::Class,
        "class",
        FamilyRenderKind::FamilyStub,
        ALL_FRONTENDS,
    ),
    family(
        DiagramKind::Object,
        DiagramFamily::Object,
        "object",
        FamilyRenderKind::FamilyStub,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::UseCase,
        DiagramFamily::UseCase,
        "usecase",
        FamilyRenderKind::FamilyStub,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Salt,
        DiagramFamily::Salt,
        "salt",
        FamilyRenderKind::Salt,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::MindMap,
        DiagramFamily::MindMap,
        "mindmap",
        FamilyRenderKind::MindMap,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Wbs,
        DiagramFamily::Wbs,
        "wbs",
        FamilyRenderKind::Wbs,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Gantt,
        DiagramFamily::Gantt,
        "gantt",
        FamilyRenderKind::Timeline,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Chronology,
        DiagramFamily::Chronology,
        "chronology",
        FamilyRenderKind::Timeline,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Component,
        DiagramFamily::Component,
        "component",
        FamilyRenderKind::Component,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Deployment,
        DiagramFamily::Deployment,
        "deployment",
        FamilyRenderKind::Deployment,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::State,
        DiagramFamily::State,
        "state",
        FamilyRenderKind::State,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Activity,
        DiagramFamily::Activity,
        "activity",
        FamilyRenderKind::Activity,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Timing,
        DiagramFamily::Timing,
        "timing",
        FamilyRenderKind::Timing,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Json,
        DiagramFamily::Json,
        "json",
        FamilyRenderKind::Json,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Yaml,
        DiagramFamily::Yaml,
        "yaml",
        FamilyRenderKind::Yaml,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Nwdiag,
        DiagramFamily::Nwdiag,
        "nwdiag",
        FamilyRenderKind::Nwdiag,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Archimate,
        DiagramFamily::Archimate,
        "archimate",
        FamilyRenderKind::Archimate,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Regex,
        DiagramFamily::Regex,
        "regex",
        FamilyRenderKind::Regex,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Ebnf,
        DiagramFamily::Ebnf,
        "ebnf",
        FamilyRenderKind::Ebnf,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Math,
        DiagramFamily::Math,
        "math",
        FamilyRenderKind::Math,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Sdl,
        DiagramFamily::Sdl,
        "sdl",
        FamilyRenderKind::Sdl,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Ditaa,
        DiagramFamily::Ditaa,
        "ditaa",
        FamilyRenderKind::Ditaa,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Chart,
        DiagramFamily::Chart,
        "chart",
        FamilyRenderKind::Chart,
        PLANTUML_ONLY,
    ),
    family(
        DiagramKind::Unknown,
        DiagramFamily::Unknown,
        "unknown",
        FamilyRenderKind::Unsupported,
        DiagramFamilyCapabilities {
            svg: false,
            text: false,
            metadata: false,
            language_service: false,
            plantuml_frontend: false,
            mermaid_frontend: false,
            picouml_frontend: false,
        },
    ),
];

macro_rules! graph {
    (
        $keyword:expr,
        $aliases:expr,
        $source_families:expr,
        $component_kind:expr,
        $family_node_kind:expr,
        $shape_kind:expr,
        $style_hook:expr,
        $renderer_shape:expr,
        $renderer_label:expr,
        $relation_endpoint:expr $(,)?
    ) => {
        GraphElementSpec {
            keyword: $keyword,
            aliases: $aliases,
            source_families: $source_families,
            component_kind: $component_kind,
            family_node_kind: $family_node_kind,
            shape_kind: $shape_kind,
            style_hook: $style_hook,
            renderer_shape: $renderer_shape,
            renderer_label: $renderer_label,
            relation_endpoint: $relation_endpoint,
        }
    };
}

const GRAPH_SPECS: &[GraphElementSpec] = &[
    graph!(
        "class",
        &[],
        &[
            DiagramKind::Class,
            DiagramKind::Component,
            DiagramKind::Deployment,
        ],
        None,
        FamilyNodeKind::Class,
        GraphElementShapeKind::ClassBox,
        GraphElementStyleHook::Class,
        GraphRendererShape::ClassNode,
        "class",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "abstract class",
        &["abstract"],
        &[DiagramKind::Class],
        None,
        FamilyNodeKind::Class,
        GraphElementShapeKind::ClassBox,
        GraphElementStyleHook::Class,
        GraphRendererShape::ClassNode,
        "class",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "interface",
        &[],
        &[DiagramKind::Class, DiagramKind::Component],
        Some(ComponentNodeKind::Interface),
        FamilyNodeKind::Interface,
        GraphElementShapeKind::Interface,
        GraphElementStyleHook::Component,
        GraphRendererShape::ComponentGridNode,
        "interface",
        RelationEndpointSupport::NamedOrLollipop,
    ),
    graph!(
        "enum",
        &[],
        &[DiagramKind::Class],
        None,
        FamilyNodeKind::Class,
        GraphElementShapeKind::ClassBox,
        GraphElementStyleHook::Class,
        GraphRendererShape::ClassNode,
        "class",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "annotation",
        &[],
        &[DiagramKind::Class],
        None,
        FamilyNodeKind::Class,
        GraphElementShapeKind::ClassBox,
        GraphElementStyleHook::Class,
        GraphRendererShape::ClassNode,
        "class",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "object",
        &[],
        &[
            DiagramKind::Object,
            DiagramKind::Component,
            DiagramKind::Deployment,
        ],
        None,
        FamilyNodeKind::Object,
        GraphElementShapeKind::ObjectBox,
        GraphElementStyleHook::Class,
        GraphRendererShape::ClassNode,
        "object",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "map",
        &[],
        &[DiagramKind::Object],
        None,
        FamilyNodeKind::Map,
        GraphElementShapeKind::Map,
        GraphElementStyleHook::Class,
        GraphRendererShape::ClassNode,
        "map",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "diamond",
        &[],
        &[DiagramKind::Object],
        None,
        FamilyNodeKind::Diamond,
        GraphElementShapeKind::Diamond,
        GraphElementStyleHook::Class,
        GraphRendererShape::ClassNode,
        "diamond",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "usecase",
        &["usecase/"],
        &[DiagramKind::UseCase, DiagramKind::Deployment],
        Some(ComponentNodeKind::UseCase),
        FamilyNodeKind::UseCaseDeployment,
        GraphElementShapeKind::UseCaseOval,
        GraphElementStyleHook::UseCase,
        GraphRendererShape::DeploymentGridNode,
        "usecase",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "actor",
        &["actor/"],
        &[
            DiagramKind::UseCase,
            DiagramKind::Component,
            DiagramKind::Deployment,
        ],
        Some(ComponentNodeKind::Actor),
        FamilyNodeKind::Actor,
        GraphElementShapeKind::Actor,
        GraphElementStyleHook::UseCase,
        GraphRendererShape::DeploymentGridNode,
        "actor",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "component",
        &[],
        &[DiagramKind::Component],
        Some(ComponentNodeKind::Component),
        FamilyNodeKind::Component,
        GraphElementShapeKind::Component,
        GraphElementStyleHook::Component,
        GraphRendererShape::ComponentGridNode,
        "component",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "port",
        &["portin", "portout"],
        &[DiagramKind::Component],
        Some(ComponentNodeKind::Port),
        FamilyNodeKind::Port,
        GraphElementShapeKind::Port,
        GraphElementStyleHook::Component,
        GraphRendererShape::ComponentGridNode,
        "port",
        RelationEndpointSupport::NamedOrLollipop,
    ),
    graph!(
        "node",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Node),
        FamilyNodeKind::Node,
        GraphElementShapeKind::Node,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "node",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "artifact",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Artifact),
        FamilyNodeKind::Artifact,
        GraphElementShapeKind::Artifact,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "artifact",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "cloud",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Cloud),
        FamilyNodeKind::Cloud,
        GraphElementShapeKind::Cloud,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "cloud",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "database",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Database),
        FamilyNodeKind::Database,
        GraphElementShapeKind::Database,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "database",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "package",
        &[],
        &[DiagramKind::Component, DiagramKind::Deployment],
        Some(ComponentNodeKind::Package),
        FamilyNodeKind::Package,
        GraphElementShapeKind::Package,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "package",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "rectangle",
        &[],
        &[DiagramKind::Component, DiagramKind::Deployment],
        Some(ComponentNodeKind::Rectangle),
        FamilyNodeKind::Rectangle,
        GraphElementShapeKind::Package,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "rectangle",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "folder",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Folder),
        FamilyNodeKind::Folder,
        GraphElementShapeKind::Folder,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "folder",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "file",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::File),
        FamilyNodeKind::File,
        GraphElementShapeKind::File,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "file",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "card",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Card),
        FamilyNodeKind::Card,
        GraphElementShapeKind::Card,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "card",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "frame",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Frame),
        FamilyNodeKind::Frame,
        GraphElementShapeKind::Node,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "frame",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "storage",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Storage),
        FamilyNodeKind::Storage,
        GraphElementShapeKind::Database,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "storage",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "queue",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Queue),
        FamilyNodeKind::Queue,
        GraphElementShapeKind::Queue,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "queue",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "stack",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Stack),
        FamilyNodeKind::Stack,
        GraphElementShapeKind::Stack,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "stack",
        RelationEndpointSupport::NamedOrBracketed,
    ),
    graph!(
        "agent",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Agent),
        FamilyNodeKind::Agent,
        GraphElementShapeKind::Component,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "agent",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "boundary",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Boundary),
        FamilyNodeKind::Boundary,
        GraphElementShapeKind::Component,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "boundary",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "control",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Control),
        FamilyNodeKind::Control,
        GraphElementShapeKind::Component,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "control",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "entity",
        &[],
        &[DiagramKind::Class, DiagramKind::Deployment],
        Some(ComponentNodeKind::Entity),
        FamilyNodeKind::Entity,
        GraphElementShapeKind::Component,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "entity",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "person",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Person),
        FamilyNodeKind::Person,
        GraphElementShapeKind::Actor,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "person",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "process",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Process),
        FamilyNodeKind::Process,
        GraphElementShapeKind::Component,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "process",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "action",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Action),
        FamilyNodeKind::Action,
        GraphElementShapeKind::Component,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "action",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "circle",
        &[],
        &[DiagramKind::Class, DiagramKind::Deployment],
        Some(ComponentNodeKind::Circle),
        FamilyNodeKind::Circle,
        GraphElementShapeKind::Diamond,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "circle",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "collections",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Collections),
        FamilyNodeKind::Collections,
        GraphElementShapeKind::Stack,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "collections",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "container",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Container),
        FamilyNodeKind::Container,
        GraphElementShapeKind::Component,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "container",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "hexagon",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Hexagon),
        FamilyNodeKind::Hexagon,
        GraphElementShapeKind::Hexagon,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "hexagon",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "label",
        &[],
        &[DiagramKind::Deployment],
        Some(ComponentNodeKind::Label),
        FamilyNodeKind::Label,
        GraphElementShapeKind::Label,
        GraphElementStyleHook::Deployment,
        GraphRendererShape::DeploymentGridNode,
        "label",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "c4 person",
        &["Person"],
        &[
            DiagramKind::Object,
            DiagramKind::Component,
            DiagramKind::Deployment,
        ],
        None,
        FamilyNodeKind::C4Person,
        GraphElementShapeKind::C4Box,
        GraphElementStyleHook::C4,
        GraphRendererShape::C4Node,
        "person",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "c4 system",
        &["System"],
        &[
            DiagramKind::Object,
            DiagramKind::Component,
            DiagramKind::Deployment,
        ],
        None,
        FamilyNodeKind::C4System,
        GraphElementShapeKind::C4Box,
        GraphElementStyleHook::C4,
        GraphRendererShape::C4Node,
        "system",
        RelationEndpointSupport::Named,
    ),
    graph!(
        "c4 component",
        &["Component"],
        &[
            DiagramKind::Object,
            DiagramKind::Component,
            DiagramKind::Deployment,
        ],
        None,
        FamilyNodeKind::C4Component,
        GraphElementShapeKind::C4Box,
        GraphElementStyleHook::C4,
        GraphRendererShape::C4Node,
        "component",
        RelationEndpointSupport::Named,
    ),
];

const fn family(
    ast_kind: DiagramKind,
    public_family: DiagramFamily,
    name: &'static str,
    render_kind: FamilyRenderKind,
    capabilities: DiagramFamilyCapabilities,
) -> DiagramFamilySpec {
    DiagramFamilySpec {
        ast_kind,
        public_family,
        name,
        render_kind,
        capabilities,
    }
}

pub fn diagram_family_specs() -> &'static [DiagramFamilySpec] {
    FAMILY_SPECS
}

pub fn diagram_family_for_ast(kind: DiagramKind) -> DiagramFamily {
    diagram_family_specs()
        .iter()
        .find(|spec| spec.ast_kind == kind)
        .map(|spec| spec.public_family)
        .unwrap_or(DiagramFamily::Unknown)
}

pub fn family_spec_by_ast(kind: DiagramKind) -> Option<&'static DiagramFamilySpec> {
    diagram_family_specs()
        .iter()
        .find(|spec| spec.ast_kind == kind)
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
    fn registry_exposes_family_capabilities_and_render_kind() {
        let component = family_spec_by_ast(DiagramKind::Component).unwrap();
        assert_eq!(component.name, "component");
        assert_eq!(component.render_kind, FamilyRenderKind::Component);
        assert!(component.capabilities.svg);
        assert!(component.capabilities.text);
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
