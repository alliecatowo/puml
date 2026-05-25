use super::pipeline::{map_ast_kind_to_family, normalize};
use super::types::DiagramFamily;
use crate::ast::Document;
use crate::diagnostic::Diagnostic;
use crate::model::{self, FamilyDocument, NormalizedDocument};
use crate::render::{self, TextOutputMode};
use crate::{layout, normalize as normalize_mod, parser, registry, specialized, LayoutOptions};
use serde_json::{json, Value};

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
        return result.map(|svg| vec![svg]);
    }
    let document = crate::parse(source)?;
    let family = map_ast_kind_to_family(document.kind);
    render_document_for_family(document, family)
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
    let document = crate::parse(source)?;
    let detected = map_ast_kind_to_family(document.kind);
    if family != detected {
        return Err(Diagnostic::error(format!(
            "[E_FAMILY_MISMATCH] requested diagram family `{}` but detected `{}`",
            family.as_str(),
            detected.as_str()
        )));
    }
    render_document_for_family(document, family)
}

fn render_document_for_family(
    document: Document,
    family: DiagramFamily,
) -> Result<Vec<String>, Diagnostic> {
    match family {
        DiagramFamily::Sequence => render_sequence(document),
        DiagramFamily::Class | DiagramFamily::Object | DiagramFamily::UseCase => {
            render_stub_family(document)
        }
        DiagramFamily::Salt => render_salt(document),
        DiagramFamily::Gantt | DiagramFamily::Chronology => render_timeline(document),
        DiagramFamily::State => render_state(document),
        DiagramFamily::Component => render_family_with(document, render::render_component_svg),
        DiagramFamily::Deployment => render_family_with(document, render::render_deployment_svg),
        DiagramFamily::Activity => render_family_with(document, render::render_activity_svg),
        DiagramFamily::Timing => render_family_with(document, render::render_timing_svg),
        DiagramFamily::Json => render_structured(
            document,
            "json",
            "E_FAMILY_JSON_INTERNAL",
            render::render_json_svg,
        ),
        DiagramFamily::Yaml => render_structured(
            document,
            "yaml",
            "E_FAMILY_YAML_INTERNAL",
            render::render_yaml_svg,
        ),
        DiagramFamily::Nwdiag => render_structured(
            document,
            "nwdiag",
            "E_FAMILY_NWDIAG_INTERNAL",
            render::render_nwdiag_svg,
        ),
        DiagramFamily::Archimate => render_structured(
            document,
            "archimate",
            "E_FAMILY_ARCHIMATE_INTERNAL",
            render::render_archimate_svg,
        ),
        DiagramFamily::Regex => render_structured(
            document,
            "regex",
            "E_FAMILY_STUB_INTERNAL",
            render::render_regex_svg,
        ),
        DiagramFamily::Ebnf => render_structured(
            document,
            "ebnf",
            "E_FAMILY_STUB_INTERNAL",
            render::render_ebnf_svg,
        ),
        DiagramFamily::Math => render_structured(
            document,
            "math",
            "E_FAMILY_STUB_INTERNAL",
            render::render_math_svg,
        ),
        DiagramFamily::Sdl => render_structured(
            document,
            "sdl",
            "E_FAMILY_STUB_INTERNAL",
            render::render_sdl_svg,
        ),
        DiagramFamily::Ditaa => render_structured(
            document,
            "ditaa",
            "E_FAMILY_STUB_INTERNAL",
            render::render_ditaa_svg,
        ),
        DiagramFamily::Chart => render_structured(
            document,
            "chart",
            "E_FAMILY_STUB_INTERNAL",
            render::render_chart_svg,
        ),
        DiagramFamily::Stdlib => render_stdlib(document),
        DiagramFamily::Chen => render_chen(document),
        DiagramFamily::MindMap => render_family_with(document, render::render_mindmap_svg),
        DiagramFamily::Wbs => render_family_with(document, render::render_wbs_svg),
        DiagramFamily::Unknown => Err(unsupported_render_family_diagnostic(family)),
    }
}

fn render_sequence(document: Document) -> Result<Vec<String>, Diagnostic> {
    let sequence = normalize(document)?;
    let scenes = layout::layout_pages(&sequence, LayoutOptions::default());
    Ok(render::with_sprite_registry(&sequence.sprites, || {
        if sequence.list_sprites {
            vec![render::render_sprite_sheet(&sequence.sprites)]
        } else {
            scenes.iter().map(render::render_svg).collect()
        }
    }))
}

fn render_stub_family(document: Document) -> Result<Vec<String>, Diagnostic> {
    match normalize_mod::normalize_family(document)? {
        model::NormalizedDocument::Family(family_doc) => {
            Ok(vec![render_family_document_svg(&family_doc)])
        }
        model::NormalizedDocument::FamilyPages(pages) => {
            Ok(pages.iter().map(render_family_document_svg).collect())
        }
        model::NormalizedDocument::Sequence(_)
        | model::NormalizedDocument::Timeline(_)
        | model::NormalizedDocument::State(_) => Err(Diagnostic::error(
            "[E_FAMILY_STUB_INTERNAL] unexpected model during family stub render",
        )),
        _ => Err(Diagnostic::error(
            "[E_FAMILY_STUB_INTERNAL] unexpected non-family model during family stub render",
        )),
    }
}

fn render_salt(document: Document) -> Result<Vec<String>, Diagnostic> {
    match normalize_mod::normalize_family(document)? {
        model::NormalizedDocument::Family(family_doc) => {
            Ok(vec![render_family_document_svg(&family_doc)])
        }
        _ => Err(Diagnostic::error(
            "[E_FAMILY_STUB_INTERNAL] unexpected model during salt render",
        )),
    }
}

fn render_timeline(document: Document) -> Result<Vec<String>, Diagnostic> {
    match normalize_mod::normalize_family(document)? {
        model::NormalizedDocument::Timeline(timeline) => {
            Ok(vec![render::render_timeline_svg(&timeline)])
        }
        _ => Err(Diagnostic::error(
            "[E_TIMELINE_INTERNAL] unexpected model during timeline render",
        )),
    }
}

fn render_state(document: Document) -> Result<Vec<String>, Diagnostic> {
    match normalize_mod::normalize_family(document)? {
        model::NormalizedDocument::State(state_doc) => {
            Ok(vec![render::render_state_svg(&state_doc)])
        }
        _ => Err(Diagnostic::error(
            "[E_STATE_INTERNAL] unexpected model variant during state render",
        )),
    }
}

fn render_stdlib(document: Document) -> Result<Vec<String>, Diagnostic> {
    match normalize_mod::normalize_family(document)? {
        model::NormalizedDocument::Stdlib(doc) => Ok(vec![render::render_stdlib_svg(&doc)]),
        _ => Err(Diagnostic::error(
            "[E_STDLIB_INTERNAL] unexpected model during stdlib render",
        )),
    }
}

fn render_chen(document: Document) -> Result<Vec<String>, Diagnostic> {
    match normalize_mod::normalize_family(document)? {
        model::NormalizedDocument::Chen(doc) => Ok(vec![render::render_chen_svg(&doc)]),
        _ => Err(Diagnostic::error(
            "[E_CHEN_INTERNAL] unexpected model during chen render",
        )),
    }
}

fn render_structured<T>(
    document: Document,
    family_name: &str,
    error_code: &str,
    renderer: fn(&T) -> String,
) -> Result<Vec<String>, Diagnostic>
where
    NormalizedDocument: TryIntoStructured<T>,
{
    let model = normalize_mod::normalize_family(document)?;
    model
        .try_into_structured()
        .map(|doc| vec![renderer(doc)])
        .ok_or_else(|| {
            Diagnostic::error(format!(
                "[{}] unexpected model during {} render",
                error_code, family_name
            ))
        })
}

trait TryIntoStructured<T> {
    fn try_into_structured(&self) -> Option<&T>;
}

macro_rules! structured_variant {
    ($ty:ty, $variant:ident) => {
        impl TryIntoStructured<$ty> for NormalizedDocument {
            fn try_into_structured(&self) -> Option<&$ty> {
                match self {
                    Self::$variant(doc) => Some(doc),
                    _ => None,
                }
            }
        }
    };
}

structured_variant!(model::JsonDocument, Json);
structured_variant!(model::YamlDocument, Yaml);
structured_variant!(model::NwdiagDocument, Nwdiag);
structured_variant!(model::ArchimateDocument, Archimate);
structured_variant!(model::RegexDocument, Regex);
structured_variant!(model::EbnfDocument, Ebnf);
structured_variant!(model::MathDocument, Math);
structured_variant!(model::SdlDocument, Sdl);
structured_variant!(model::DitaaDocument, Ditaa);
structured_variant!(model::ChartDocument, Chart);
structured_variant!(model::ChenDocument, Chen);

fn render_family_with(
    document: Document,
    _renderer: fn(&FamilyDocument) -> String,
) -> Result<Vec<String>, Diagnostic> {
    match normalize_mod::normalize_family(document)? {
        model::NormalizedDocument::Family(doc) => Ok(vec![render_family_document_svg(&doc)]),
        model::NormalizedDocument::Sequence(_) => Err(Diagnostic::error(
            "[E_FAMILY_INTERNAL] unexpected sequence model during extended family render",
        )),
        model::NormalizedDocument::Timeline(_) => Err(Diagnostic::error(
            "[E_FAMILY_INTERNAL] unexpected timeline model during extended family render",
        )),
        _ => Err(Diagnostic::error(
            "[E_FAMILY_INTERNAL] unexpected model during extended family render",
        )),
    }
}

fn unsupported_render_family_diagnostic(family: DiagramFamily) -> Diagnostic {
    let code = match family {
        DiagramFamily::Component => "E_RENDER_COMPONENT_UNSUPPORTED",
        DiagramFamily::Deployment => "E_RENDER_DEPLOYMENT_UNSUPPORTED",
        DiagramFamily::Activity => "E_RENDER_ACTIVITY_UNSUPPORTED",
        DiagramFamily::Timing => "E_RENDER_TIMING_UNSUPPORTED",
        DiagramFamily::MindMap => "E_RENDER_MINDMAP_UNSUPPORTED",
        DiagramFamily::Wbs => "E_RENDER_WBS_UNSUPPORTED",
        DiagramFamily::Gantt => "E_RENDER_GANTT_UNSUPPORTED",
        DiagramFamily::Chronology => "E_RENDER_CHRONOLOGY_UNSUPPORTED",
        _ => "E_RENDER_FAMILY_UNSUPPORTED",
    };
    Diagnostic::error_code(
        code,
        format!(
            "diagram family `{}` is not implemented yet; sequence is currently supported",
            family.as_str()
        ),
    )
}

pub fn render_svg_pages_from_model(model: &NormalizedDocument) -> Vec<String> {
    match model {
        NormalizedDocument::Sequence(sequence) => {
            render::with_sprite_registry(&sequence.sprites, || {
                if sequence.list_sprites {
                    vec![render::render_sprite_sheet(&sequence.sprites)]
                } else {
                    let scenes = layout::layout_pages(sequence, LayoutOptions::default());
                    scenes.iter().map(render::render_svg).collect::<Vec<_>>()
                }
            })
        }
        NormalizedDocument::Family(family) => vec![render_family_document_svg(family)],
        NormalizedDocument::FamilyPages(pages) => {
            pages.iter().map(render_family_document_svg).collect()
        }
        NormalizedDocument::Timeline(timeline) => vec![render::render_timeline_svg(timeline)],
        NormalizedDocument::State(state) => vec![render::render_state_svg(state)],
        NormalizedDocument::Json(doc) => vec![render::render_json_svg(doc)],
        NormalizedDocument::Yaml(doc) => vec![render::render_yaml_svg(doc)],
        NormalizedDocument::Nwdiag(doc) => vec![render::render_nwdiag_svg(doc)],
        NormalizedDocument::Archimate(doc) => vec![render::render_archimate_svg(doc)],
        NormalizedDocument::Regex(doc) => vec![render::render_regex_svg(doc)],
        NormalizedDocument::Ebnf(doc) => vec![render::render_ebnf_svg(doc)],
        NormalizedDocument::Math(doc) => vec![render::render_math_svg(doc)],
        NormalizedDocument::Sdl(doc) => vec![render::render_sdl_svg(doc)],
        NormalizedDocument::Ditaa(doc) => vec![render::render_ditaa_svg(doc)],
        NormalizedDocument::Chart(doc) => vec![render::render_chart_svg(doc)],
        NormalizedDocument::Stdlib(doc) => vec![render::render_stdlib_svg(doc)],
        NormalizedDocument::Chen(doc) => vec![render::render_chen_svg(doc)],
    }
}

pub fn normalized_model_summary_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => json!({
            "kind": "Sequence",
            "participants": sequence.participants.len(),
            "events": sequence.events.len(),
            "warnings": sequence.warnings.len(),
            "title": sequence.title,
            "header": sequence.header,
            "footer": sequence.footer,
            "caption": sequence.caption
        }),
        NormalizedDocument::Family(family) => family_model_summary_to_json(family),
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "pages": pages.iter().map(family_model_summary_to_json).collect::<Vec<_>>()
        }),
        NormalizedDocument::Timeline(timeline) => json!({
            "kind": "Timeline",
            "tasks": timeline.tasks.len(),
            "milestones": timeline.milestones.len(),
            "constraints": timeline.constraints.len(),
            "warnings": timeline.warnings.len(),
            "title": timeline.title
        }),
        NormalizedDocument::State(state) => json!({
            "kind": "State",
            "nodes": state.nodes.len(),
            "transitions": state.transitions.len(),
            "warnings": state.warnings.len(),
            "title": state.title
        }),
        NormalizedDocument::Json(doc) => json!({"kind": "Json", "warnings": doc.warnings.len()}),
        NormalizedDocument::Yaml(doc) => json!({"kind": "Yaml", "warnings": doc.warnings.len()}),
        NormalizedDocument::Nwdiag(doc) => {
            json!({"kind": "Nwdiag", "warnings": doc.warnings.len()})
        }
        NormalizedDocument::Archimate(doc) => {
            json!({"kind": "Archimate", "warnings": doc.warnings.len()})
        }
        NormalizedDocument::Regex(doc) => json!({"kind": "Regex", "warnings": doc.warnings.len()}),
        NormalizedDocument::Ebnf(doc) => json!({"kind": "Ebnf", "warnings": doc.warnings.len()}),
        NormalizedDocument::Math(doc) => json!({"kind": "Math", "warnings": doc.warnings.len()}),
        NormalizedDocument::Sdl(doc) => json!({"kind": "Sdl", "warnings": doc.warnings.len()}),
        NormalizedDocument::Ditaa(doc) => json!({"kind": "Ditaa", "warnings": doc.warnings.len()}),
        NormalizedDocument::Chart(doc) => json!({"kind": "Chart", "warnings": doc.warnings.len()}),
        NormalizedDocument::Stdlib(doc) => json!({
            "kind": "Stdlib",
            "entries": doc.entries.len(),
            "packs": doc.packs.len(),
            "aliases": doc.aliases.len(),
            "missing_packs": doc.missing_packs,
            "warnings": doc.warnings.len()
        }),
        NormalizedDocument::Chen(doc) => json!({
            "kind": "Chen",
            "nodes": doc.nodes.len(),
            "relations": doc.relations.len(),
            "inheritances": doc.inheritances.len(),
            "warnings": doc.warnings.len()
        }),
    }
}

pub fn normalized_scene_summary_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => {
            let pages = layout::layout_pages(sequence, LayoutOptions::default());
            json!({
                "kind": "Sequence",
                "typed": false,
                "pageCount": pages.len(),
                "pages": pages.iter().map(sequence_scene_to_json).collect::<Vec<_>>()
            })
        }
        NormalizedDocument::Family(family) => family_scene_summary_to_json(family),
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "typed": false,
            "pageCount": pages.len(),
            "pages": pages.iter().map(family_scene_summary_to_json).collect::<Vec<_>>()
        }),
        _ => {
            let summary = normalized_model_summary_to_json(model);
            json!({
                "kind": summary["kind"].clone(),
                "typed": false,
                "available": false,
                "summary": summary
            })
        }
    }
}

fn family_model_summary_to_json(family: &FamilyDocument) -> Value {
    json!({
        "kind": format!("{:?}", family.kind),
        "nodes": family.nodes.len(),
        "relations": family.relations.len(),
        "groups": family.groups.len(),
        "warnings": family.warnings.len(),
        "title": family.title
    })
}

fn sequence_scene_to_json(scene: &crate::Scene) -> Value {
    json!({
        "size": {"width": scene.width, "height": scene.height},
        "participants": scene.participants.iter().map(|participant| {
            json!({
                "id": participant.id,
                "display": participant.display_lines.join("\n"),
                "role": format!("{:?}", participant.role),
                "bounds": {
                    "x": participant.x,
                    "y": participant.y,
                    "width": participant.width,
                    "height": participant.height
                }
            })
        }).collect::<Vec<_>>(),
        "messages": scene.messages.iter().map(|message| {
            json!({
                "from": message.from_id,
                "to": message.to_id,
                "arrow": message.arrow,
                "label": message.label,
                "route": {
                    "x1": message.x1,
                    "y": message.y,
                    "routeY": message.route_y,
                    "x2": message.x2
                }
            })
        }).collect::<Vec<_>>(),
        "notes": scene.notes.len(),
        "groups": scene.groups.len(),
        "structures": scene.structures.len()
    })
}

fn family_scene_summary_to_json(family: &FamilyDocument) -> Value {
    let artifact = render_family_document_artifact(family);
    match artifact.scene {
        Some(scene) => render_core_scene_to_json(&scene),
        None => json!({
            "kind": format!("{:?}", family.kind),
            "typed": false,
            "available": false,
            "summary": family_model_summary_to_json(family)
        }),
    }
}

fn render_core_scene_to_json(scene: &crate::render_core::RenderScene) -> Value {
    json!({
        "kind": "RenderScene",
        "typed": true,
        "viewport": rect_to_json(scene.viewport),
        "nodes": scene.nodes.values().map(|node| {
            json!({
                "id": node.id,
                "bounds": rect_to_json(node.node_box.bounds),
                "ports": node.node_box.ports.len(),
                "labels": node.node_box.labels.len()
            })
        }).collect::<Vec<_>>(),
        "edges": scene.edges.values().map(|edge| {
            json!({
                "id": edge.id,
                "from": edge.from,
                "to": edge.to,
                "points": edge.route.points.iter().map(|point| {
                    json!({"x": point.x, "y": point.y})
                }).collect::<Vec<_>>(),
                "labels": edge.labels.len()
            })
        }).collect::<Vec<_>>(),
        "groups": scene.groups.values().map(|group| {
            json!({
                "id": group.id,
                "bounds": rect_to_json(group.frame.bounds),
                "children": group.frame.child_node_ids
            })
        }).collect::<Vec<_>>(),
        "lanes": scene.lanes.values().map(|lane| {
            json!({
                "id": lane.id,
                "bounds": rect_to_json(lane.bounds),
                "children": lane.child_node_ids
            })
        }).collect::<Vec<_>>(),
        "labels": scene.labels.len(),
        "routeChannels": scene.route_channels.len()
    })
}

fn rect_to_json(rect: crate::render_core::Rect) -> Value {
    json!({
        "x": rect.origin.x,
        "y": rect.origin.y,
        "width": rect.size.width,
        "height": rect.size.height
    })
}

pub fn render_family_document_svg(family: &FamilyDocument) -> String {
    render_family_document_artifact(family).svg
}

pub fn render_family_document_artifact(family: &FamilyDocument) -> render::RenderArtifact {
    let mut artifact = render::with_sprite_registry(&family.sprites, || {
        if family.list_sprites {
            return render::RenderArtifact::svg_only(render::render_sprite_sheet(&family.sprites));
        }
        match registry::family_spec_by_ast(family.kind)
            .map(|spec| spec.render_kind)
            .unwrap_or(registry::FamilyRenderKind::Unsupported)
        {
            registry::FamilyRenderKind::Salt => {
                render::RenderArtifact::svg_only(render::render_salt_svg(family))
            }
            registry::FamilyRenderKind::Component => render::render_component_artifact(family),
            registry::FamilyRenderKind::Deployment => render::render_deployment_artifact(family),
            registry::FamilyRenderKind::Activity => {
                render::RenderArtifact::svg_only(render::render_activity_svg(family))
            }
            registry::FamilyRenderKind::Timing => {
                render::RenderArtifact::svg_only(render::render_timing_svg(family))
            }
            registry::FamilyRenderKind::MindMap => {
                render::RenderArtifact::svg_only(render::render_mindmap_svg(family))
            }
            registry::FamilyRenderKind::Wbs => {
                render::RenderArtifact::svg_only(render::render_wbs_svg(family))
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
    artifact.invariant_report = Some(render::validate::run_with_scene(
        &mut artifact.svg,
        artifact.scene.as_ref(),
        render::validate::AutoCorrect::Apply,
    ));
    artifact
}
