use crate::ast::DiagramKind;
use crate::registry::FamilyRenderKind;
use crate::DiagramFamily;

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

pub fn family_name_by_ast(kind: DiagramKind) -> &'static str {
    family_spec_by_ast(kind)
        .map(|spec| spec.name)
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_exposes_family_capabilities_and_render_kind() {
        let component = family_spec_by_ast(DiagramKind::Component).unwrap();
        assert_eq!(component.name, "component");
        assert_eq!(component.render_kind, FamilyRenderKind::Component);
        assert!(component.capabilities.svg);
        assert!(component.capabilities.text);
    }

    #[test]
    fn family_names_are_registry_backed() {
        assert_eq!(family_name_by_ast(DiagramKind::Sequence), "sequence");
        assert_eq!(family_name_by_ast(DiagramKind::Unknown), "unknown");
    }
}
