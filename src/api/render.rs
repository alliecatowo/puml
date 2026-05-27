use super::pipeline::map_ast_kind_to_family;
use super::types::DiagramFamily;
use crate::diagnostic::Diagnostic;
use crate::model::{FamilyDocument, NormalizedDocument};
use crate::output::{CommonCommandKind, CommonCommandPath, RenderArtifact, RenderCommonCommands};
use crate::registry::FamilyRenderKind;
use crate::render::{self, TextOutputMode};
use crate::{layout, normalize as normalize_mod, parser, registry, specialized, LayoutOptions};

pub fn render_source_to_svg(source: &str) -> Result<String, Diagnostic> {
    let pages = render_source_to_svgs(source)?;
    if pages.len() > 1 {
        return Err(Diagnostic::error(
            "multiple pages detected; use render_source_to_svgs or --multi",
        ));
    }
    Ok(pages.into_iter().next().unwrap_or_default())
}

pub fn render_source_to_svgs(source: &str) -> Result<Vec<String>, Diagnostic> {
    Ok(render_source_to_artifacts(source)?
        .into_iter()
        .map(|artifact| artifact.svg)
        .collect())
}

pub fn render_source_to_artifacts(source: &str) -> Result<Vec<RenderArtifact>, Diagnostic> {
    // Intercept specialized families before the main AST pipeline, but only
    // after applying the same preprocessing pass used by parse/check routes.
    if specialized::is_specialized_source(source) {
        let preprocessed =
            parser::preprocess_with_options(source, &parser::ParseOptions::default())?;
        let result = specialized::try_render_specialized(&preprocessed).ok_or_else(|| {
            Diagnostic::error(
                "[E_SPECIALIZED_PREPROC] preprocessed specialized source changed family",
            )
        })?;
        return result.map(|svg| vec![RenderArtifact::svg_only(svg)]);
    }
    let document = crate::parse(source)?;
    let model = normalize_mod::normalize_family(document)?;
    Ok(render_artifact_pages_from_model(&model))
}

pub fn render_source_to_text(source: &str, mode: TextOutputMode) -> Result<String, Diagnostic> {
    let pages = render_source_to_texts(source, mode)?;
    if pages.len() > 1 {
        return Err(Diagnostic::error(
            "multiple pages detected; use render_source_to_texts or --multi",
        ));
    }
    Ok(pages.into_iter().next().unwrap_or_default())
}

pub fn render_source_to_texts(
    source: &str,
    mode: TextOutputMode,
) -> Result<Vec<String>, Diagnostic> {
    let document = crate::parse(source)?;
    let model = normalize_mod::normalize_family(document)?;
    Ok(render::render_text_pages(&model, mode))
}

pub fn render_source_to_svg_for_family(
    source: &str,
    family: DiagramFamily,
) -> Result<String, Diagnostic> {
    let pages = render_source_to_svgs_for_family(source, family)?;
    if pages.len() > 1 {
        return Err(Diagnostic::error(
            "multiple pages detected; use render_source_to_svgs or --multi",
        ));
    }
    Ok(pages.into_iter().next().unwrap_or_default())
}

pub fn render_source_to_svgs_for_family(
    source: &str,
    family: DiagramFamily,
) -> Result<Vec<String>, Diagnostic> {
    Ok(render_source_to_artifacts_for_family(source, family)?
        .into_iter()
        .map(|artifact| artifact.svg)
        .collect())
}

pub fn render_source_to_artifacts_for_family(
    source: &str,
    family: DiagramFamily,
) -> Result<Vec<RenderArtifact>, Diagnostic> {
    let document = crate::parse(source)?;
    let detected = map_ast_kind_to_family(document.kind);
    if family != detected {
        return Err(Diagnostic::error(format!(
            "[E_FAMILY_MISMATCH] requested diagram family `{}` but detected `{}`",
            family.as_str(),
            detected.as_str()
        )));
    }
    let model = normalize_mod::normalize_family(document)?;
    Ok(render_artifact_pages_from_model(&model))
}

pub fn render_artifact_pages_from_model(model: &NormalizedDocument) -> Vec<RenderArtifact> {
    match model {
        NormalizedDocument::Sequence(sequence) => {
            render::with_sprite_registry(&sequence.sprites, || {
                if sequence.list_sprites {
                    vec![
                        RenderArtifact::svg_only(render::render_sprite_sheet(&sequence.sprites))
                            .with_diagnostics(sequence.warnings.clone()),
                    ]
                } else {
                    let scenes = layout::layout_pages(sequence, LayoutOptions::default());
                    scenes
                        .iter()
                        .map(|scene| {
                            render::render_sequence_artifact(scene)
                                .with_diagnostics(sequence.warnings.clone())
                        })
                        .collect::<Vec<_>>()
                }
            })
        }
        NormalizedDocument::Family(family) => vec![render_family_document_artifact(family)],
        NormalizedDocument::FamilyPages(pages) => {
            pages.iter().map(render_family_document_artifact).collect()
        }
        NormalizedDocument::Timeline(timeline) => {
            vec![render::render_timeline_artifact(timeline)
                .with_diagnostics(timeline.warnings.clone())]
        }
        NormalizedDocument::State(state) => {
            vec![render::render_state_artifact(state).with_diagnostics(state.warnings.clone())]
        }
        NormalizedDocument::Json(doc) => {
            vec![render::render_json_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Yaml(doc) => {
            vec![render::render_yaml_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Nwdiag(doc) => {
            vec![render::render_nwdiag_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Archimate(doc) => {
            vec![render::render_archimate_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Regex(doc) => {
            vec![render::render_regex_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Ebnf(doc) => {
            vec![render::render_ebnf_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Math(doc) => {
            vec![render::render_math_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Sdl(doc) => {
            vec![render::render_sdl_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Ditaa(doc) => {
            vec![render::render_ditaa_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Chart(doc) => {
            vec![render::render_chart_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Stdlib(doc) => {
            vec![render::render_stdlib_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Chen(doc) => {
            // Chen emits a typed RenderScene built from its actual drawn geometry.
            // SVG output is unchanged; the scene is attached for the typed path.
            vec![render::render_chen_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Board(doc) => {
            vec![render::render_board_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Files(doc) => {
            vec![render::render_files_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Wire(doc) => {
            vec![render::render_wire_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
    }
}

pub fn render_svg_pages_from_model(model: &NormalizedDocument) -> Vec<String> {
    render_artifact_pages_from_model(model)
        .into_iter()
        .map(|artifact| artifact.svg)
        .collect()
}

pub fn render_family_document_svg(family: &FamilyDocument) -> String {
    render_family_document_artifact(family).svg
}

pub fn render_family_document_artifact(family: &FamilyDocument) -> RenderArtifact {
    let mut artifact = render::with_sprite_registry(&family.sprites, || {
        if family.list_sprites {
            return RenderArtifact::svg_only(render::render_sprite_sheet(&family.sprites));
        }
        let render_kind = registry::family_spec_by_ast(family.kind)
            .map(|spec| spec.render_kind)
            .unwrap_or(registry::FamilyRenderKind::Unsupported);
        let mut artifact = match render_kind {
            registry::FamilyRenderKind::Salt => render::render_salt_artifact(family),
            registry::FamilyRenderKind::Component => render::render_component_artifact(family),
            registry::FamilyRenderKind::Deployment => render::render_deployment_artifact(family),
            registry::FamilyRenderKind::Activity => render::render_activity_artifact(family),
            registry::FamilyRenderKind::Timing => render::render_timing_artifact(family),
            registry::FamilyRenderKind::MindMap => render::render_mindmap_artifact(family),
            registry::FamilyRenderKind::Wbs => render::render_wbs_artifact(family),
            _ => render::render_family_stub_artifact(family),
        };
        require_migrated_family_scene_contract(&mut artifact, render_kind);
        artifact
    });
    let common_commands =
        RenderCommonCommands::from_parts(family.scale.clone(), family.mainframe.clone());
    if artifact.common_commands.is_empty() {
        artifact = artifact.with_common_commands(common_commands);
    }
    if let Some(title) = &family.mainframe {
        if !artifact.common_command_applied(CommonCommandKind::Mainframe) {
            crate::output::append_mainframe_svg(&mut artifact.svg, title);
            artifact.mark_common_command_application(
                CommonCommandKind::Mainframe,
                CommonCommandPath::SvgCompatibilityBridge,
            );
        }
    }
    // Render-time invariants pass: enforce structural correctness.
    // Auto-corrections (viewBox expansion, label background rects) are applied
    // in-place. Diagnostic-only violations are silently recorded.
    artifact.validate_svg(render::validate::AutoCorrect::Apply);
    artifact.apply_common_scale_to_svg_dimensions();
    artifact.extend_diagnostics(family.warnings.clone());
    artifact
}

fn require_migrated_family_scene_contract(
    artifact: &mut RenderArtifact,
    render_kind: FamilyRenderKind,
) {
    let owner = match render_kind {
        FamilyRenderKind::FamilyStub => "family graph renderer",
        FamilyRenderKind::Component => "component renderer",
        FamilyRenderKind::Deployment => "deployment renderer",
        _ => return,
    };
    if let Err(diagnostic) = artifact.require_typed_scene_for(owner) {
        artifact.push_diagnostic(diagnostic);
    }
}
