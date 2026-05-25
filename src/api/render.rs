use super::pipeline::map_ast_kind_to_family;
use super::types::DiagramFamily;
use crate::diagnostic::Diagnostic;
use crate::model::{FamilyDocument, NormalizedDocument};
use crate::output::RenderArtifact;
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
        NormalizedDocument::Timeline(timeline) => vec![artifact_with_diagnostics(
            render::render_timeline_svg(timeline),
            &timeline.warnings,
        )],
        NormalizedDocument::State(state) => vec![artifact_with_diagnostics(
            render::render_state_svg(state),
            &state.warnings,
        )],
        NormalizedDocument::Json(doc) => {
            vec![artifact_with_diagnostics(
                render::render_json_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Yaml(doc) => {
            vec![artifact_with_diagnostics(
                render::render_yaml_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Nwdiag(doc) => {
            vec![render::render_nwdiag_artifact(doc).with_diagnostics(doc.warnings.clone())]
        }
        NormalizedDocument::Archimate(doc) => vec![artifact_with_diagnostics(
            render::render_archimate_svg(doc),
            &doc.warnings,
        )],
        NormalizedDocument::Regex(doc) => {
            vec![artifact_with_diagnostics(
                render::render_regex_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Ebnf(doc) => {
            vec![artifact_with_diagnostics(
                render::render_ebnf_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Math(doc) => {
            vec![artifact_with_diagnostics(
                render::render_math_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Sdl(doc) => {
            vec![artifact_with_diagnostics(
                render::render_sdl_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Ditaa(doc) => {
            vec![artifact_with_diagnostics(
                render::render_ditaa_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Chart(doc) => {
            vec![artifact_with_diagnostics(
                render::render_chart_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Stdlib(doc) => vec![artifact_with_diagnostics(
            render::render_stdlib_svg(doc),
            &doc.warnings,
        )],
        NormalizedDocument::Chen(doc) => {
            vec![artifact_with_diagnostics(
                render::render_chen_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Board(doc) => {
            vec![artifact_with_diagnostics(
                render::render_board_svg(doc),
                &doc.warnings,
            )]
        }
        NormalizedDocument::Files(doc) => {
            vec![artifact_with_diagnostics(
                render::render_files_svg(doc),
                &doc.warnings,
            )]
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

fn artifact_with_diagnostics(svg: String, diagnostics: &[Diagnostic]) -> RenderArtifact {
    RenderArtifact::svg_only(svg).with_diagnostics(diagnostics.to_vec())
}

pub fn render_family_document_svg(family: &FamilyDocument) -> String {
    render_family_document_artifact(family).svg
}

pub fn render_family_document_artifact(family: &FamilyDocument) -> RenderArtifact {
    let mut artifact = render::with_sprite_registry(&family.sprites, || {
        if family.list_sprites {
            return RenderArtifact::svg_only(render::render_sprite_sheet(&family.sprites));
        }
        match registry::family_spec_by_ast(family.kind)
            .map(|spec| spec.render_kind)
            .unwrap_or(registry::FamilyRenderKind::Unsupported)
        {
            registry::FamilyRenderKind::Salt => {
                RenderArtifact::svg_only(render::render_salt_svg(family))
            }
            registry::FamilyRenderKind::Component => render::render_component_artifact(family),
            registry::FamilyRenderKind::Deployment => render::render_deployment_artifact(family),
            registry::FamilyRenderKind::Activity => {
                RenderArtifact::svg_only(render::render_activity_svg(family))
            }
            registry::FamilyRenderKind::Timing => {
                RenderArtifact::svg_only(render::render_timing_svg(family))
            }
            registry::FamilyRenderKind::MindMap => {
                RenderArtifact::svg_only(render::render_mindmap_svg(family))
            }
            registry::FamilyRenderKind::Wbs => {
                RenderArtifact::svg_only(render::render_wbs_svg(family))
            }
            _ => render::render_family_stub_artifact(family),
        }
    });
    if let Some(title) = &family.mainframe {
        render::append_mainframe_svg(&mut artifact.svg, title);
    }
    // Render-time invariants pass: enforce structural correctness.
    // Auto-corrections (viewBox expansion, label background rects) are applied
    // in-place. Diagnostic-only violations are silently recorded.
    artifact.validate_svg(render::validate::AutoCorrect::Apply);
    if let Some(scale) = &family.scale {
        render::apply_scale_svg(&mut artifact.svg, scale);
    }
    artifact.refresh_svg_metadata();
    artifact.with_diagnostics(family.warnings.clone())
}
