use super::*;
use crate::ast::RawSyntaxCategory;
use crate::normalize::common::{self, CommonDirectives, LegendTextMode, RawSyntaxContext};

mod salt_cells;
use self::salt_cells::{collect_salt_ascii_sprite_names, encode_salt_cells, salt_scan_unsupported};

pub(super) fn normalize_stub_family(document: Document) -> Result<FamilyDocument, Diagnostic> {
    let family_kind = document.kind;
    let node_kind = match family_kind {
        DiagramKind::Class => FamilyNodeKind::Class,
        DiagramKind::Object => FamilyNodeKind::Object,
        DiagramKind::UseCase => FamilyNodeKind::UseCase,
        DiagramKind::Salt => FamilyNodeKind::Salt,
        _ => {
            return Err(Diagnostic::error(
                "[E_FAMILY_STUB_INTERNAL] invalid family for stub normalization",
            ));
        }
    };

    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut groups = Vec::new();
    let mut json_projections: Vec<crate::model::JsonProjection> = Vec::new();
    let mut hide_options = std::collections::BTreeSet::new();
    let mut namespace_separator: Option<String> = None;
    let mut common = CommonDirectives::default();
    let graph_family = match family_kind {
        DiagramKind::Class => crate::theme::GraphStyleFamily::Class,
        DiagramKind::Object => crate::theme::GraphStyleFamily::Object,
        DiagramKind::UseCase => crate::theme::GraphStyleFamily::UseCase,
        _ => crate::theme::GraphStyleFamily::Class,
    };
    let mut style_cascade = crate::theme::GraphStyleCascade::new(graph_family);
    let mut salt_style = crate::theme::SaltStyle::default();
    // Phase B (#1404): accumulate `<style>` block rules into a StyleBuilder.
    let mut style_builder = crate::theme::StyleBuilder::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut note_counter: usize = 0;
    let mut sprites = crate::sprites::SpriteRegistry::new();
    let mut list_sprites = false;
    let mut last_relation: Option<(String, String)> = None;
    let mut orientation = FamilyOrientation::TopToBottom;
    let mut edge_routing = crate::render::graph_layout::EdgeRouting::default();

    // For Salt diagrams: pre-collect ASCII sprite names from `<<name\n...\n>>`
    // definitions.  These arrive as BenignPassthrough statements (the main
    // parser does not understand the salt ASCII sprite syntax); we need them
    // before we scan SaltGridRow cells for W_SALT_UNSUPPORTED_SPRITE_REF.
    let salt_ascii_sprite_names = if family_kind == DiagramKind::Salt {
        collect_salt_ascii_sprite_names(&document.statements)
    } else {
        Default::default()
    };

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::SpriteDef(sprite) => {
                sprites.insert(sprite.name.clone(), sprite);
            }
            StatementKind::ListSprites => {
                list_sprites = true;
            }
            StatementKind::SkinParam { key, value } => {
                handle_family_linetype_skinparam(&key, &value, &mut edge_routing);
                if family_kind == DiagramKind::Salt {
                    salt_style.apply_key(&key, &value);
                    continue;
                }
                style_cascade.apply_skinparam(&key, &value, stmt.span, &mut warnings);
            }
            StatementKind::JsonProjection { alias, body } => {
                json_projections.push(crate::model::JsonProjection {
                    alias,
                    body,
                    format: "json".to_string(),
                });
            }
            StatementKind::YamlProjection { alias, body } => {
                json_projections.push(crate::model::JsonProjection {
                    alias,
                    body,
                    format: "yaml".to_string(),
                });
            }
            StatementKind::ClassDecl(decl) => {
                if node_kind != FamilyNodeKind::Class {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found class declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                let mut members = decl.members;
                let fill_color = extract_family_node_fill_color(&mut members);
                let source_id = decl.alias.as_deref().unwrap_or(&decl.name).to_string();
                let heritage_relations =
                    extract_family_heritage_relations(&mut members, &source_id);
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: FamilyNodeKind::Class,
                        name: decl.name,
                        alias: decl.alias,
                        members,
                        depth: 0,
                        label: None,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color,
                    },
                );
                for rel in &heritage_relations {
                    ensure_family_class_node(&mut nodes, &rel.from);
                }
                relations.extend(heritage_relations);
            }
            StatementKind::ObjectDecl(decl) => {
                if node_kind != FamilyNodeKind::Object {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found object declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                // Detect and strip C4 stereotypes embedded in the alias
                // (e.g. `u <<person>>` → alias `u`, kind `C4Person`).
                // Non-C4 stereotypes (e.g. `<<aws-EC2>>`) are returned as
                // `extra_stereotype` and re-injected as a member so that
                // downstream renderers (e.g. cloud-icon renderer, Refs #1258)
                // can discover them via the node's member list.
                let (clean_alias, c4_kind, extra_stereotype) =
                    sequence::extract_c4_stereotype(decl.alias);
                let mut members = decl.members;
                if let Some(stereo) = extra_stereotype {
                    members.insert(
                        0,
                        crate::ast::ClassMember {
                            text: stereo,
                            modifier: None,
                        },
                    );
                }
                let resolved_kind = if members.first().is_some_and(|m| m.text.trim() == "<<map>>") {
                    let _ = members.remove(0);
                    FamilyNodeKind::Map
                } else if members
                    .first()
                    .is_some_and(|m| m.text.trim() == "<<diamond>>")
                {
                    let _ = members.remove(0);
                    FamilyNodeKind::Diamond
                } else {
                    c4_kind.unwrap_or(FamilyNodeKind::Object)
                };
                let fill_color = extract_family_node_fill_color(&mut members);
                let (name, typed_label) = if resolved_kind == FamilyNodeKind::Object {
                    split_object_instance_type(decl.name)
                } else {
                    (decl.name, None)
                };
                let node_id = clean_alias.as_deref().unwrap_or(&name).to_string();
                if resolved_kind == FamilyNodeKind::Map {
                    relations.extend(extract_map_row_relations(&members, &node_id));
                }
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: resolved_kind,
                        name,
                        alias: clean_alias,
                        members,
                        depth: 0,
                        label: typed_label,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color,
                    },
                );
            }
            StatementKind::UseCaseDecl(decl) => {
                if node_kind != FamilyNodeKind::UseCase {
                    return Err(Diagnostic::error(format!(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported: found usecase declaration in {} diagram",
                        family_kind_name(family_kind)
                    ))
                    .with_span(stmt.span));
                }
                let mut members = decl.members;
                let fill_color = extract_family_node_fill_color(&mut members);
                // An `actor` declaration becomes a UseCaseDecl with `<<actor>>` as its
                // first member. Detect this and promote it to Actor kind so the renderer
                // can draw a stick figure instead of an ellipse.
                let resolved_kind = resolve_usecase_node_kind(&mut members);
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: resolved_kind,
                        name: decl.name,
                        alias: decl.alias,
                        members,
                        depth: 0,
                        label: None,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color,
                    },
                );
            }
            StatementKind::FamilyRelation(rel) => {
                last_relation = Some((rel.from.clone(), rel.to.clone()));
                relations.push(model_relation_from_ast(rel)?);
            }
            StatementKind::AssociationClass {
                left,
                right,
                association,
                arrow,
            } => {
                upsert_family_node(
                    &mut nodes,
                    FamilyNode {
                        kind: FamilyNodeKind::Diamond,
                        name: association.clone(),
                        alias: None,
                        members: Vec::new(),
                        depth: 0,
                        label: None,
                        mindmap_side: MindMapSide::Right,
                        wbs_checkbox: None,
                        fill_color: None,
                    },
                );
                // Force the association node to Diamond kind even if a plain
                // `class Enrollment` declaration appeared earlier.
                if let Some(n) = nodes.iter_mut().find(|n| n.name == association) {
                    n.kind = FamilyNodeKind::Diamond;
                }
                relations.push(simple_family_relation(left.clone(), right.clone(), arrow));
                relations.push(simple_family_relation(
                    association.clone(),
                    left.clone(),
                    "..".to_string(),
                ));
                relations.push(simple_family_relation(association, right, "..".to_string()));
            }
            StatementKind::Note(note) => {
                note_counter += 1;
                let target = note.target.clone();
                let target_member = note.target_member.clone();
                let note_node = family_note_node(note_counter, note);
                let note_name = note_node.name.clone();
                nodes.push(note_node);
                if let Some(target) = target {
                    let target = if target.eq_ignore_ascii_case("on link") {
                        last_relation
                            .as_ref()
                            .map(|(_, to)| to.clone())
                            .unwrap_or_default()
                    } else {
                        target
                    };
                    if !target.is_empty() {
                        // When a member qualifier like `Foo::counter` is present,
                        // route the relation from the qualified endpoint so the
                        // renderer can anchor the connector at the member row.
                        let from_endpoint = if let Some(ref member) = target_member {
                            format!("{target}::{member}")
                        } else {
                            relation_node_endpoint(&target)
                        };
                        relations.push(simple_family_relation(
                            from_endpoint,
                            note_name,
                            "..".to_string(),
                        ));
                    }
                }
            }
            StatementKind::ClassGroup {
                kind,
                label,
                members,
                relations: group_relations,
                fill_color: group_fill_color,
            } => {
                // Auto-create nodes for members declared inside a package/namespace block
                // if they haven't already been declared as top-level statements.
                let mut group_member_ids = Vec::with_capacity(members.len());
                for member_id in &members {
                    let mut parts = member_id.split('\t');
                    let node_id = parts.next().unwrap_or(member_id.as_str()).to_string();
                    let mut encoded_members = parts
                        .map(|text| ClassMember {
                            text: text.to_string(),
                            modifier: None,
                        })
                        .filter(|member| {
                            let trimmed = member.text.trim();
                            if hide_options.contains("stereotype")
                                && trimmed.starts_with("<<")
                                && trimmed.ends_with(">>")
                            {
                                return false;
                            }
                            if hide_options.contains("circle") && trimmed == "()" {
                                return false;
                            }
                            if (hide_options.contains("empty members")
                                || hide_options.contains("empty methods")
                                || hide_options.contains("empty fields"))
                                && (trimmed.is_empty() || trimmed == "--" || trimmed == "..")
                            {
                                return false;
                            }
                            true
                        })
                        .collect::<Vec<_>>();
                    let fill_color = extract_family_node_fill_color(&mut encoded_members);
                    group_member_ids.push(node_id.clone());
                    let already_exists = nodes.iter().any(|n: &FamilyNode| {
                        n.name == node_id || n.alias.as_deref() == Some(node_id.as_str())
                    });
                    if !already_exists {
                        // Detect actor marker embedded by the parser for `actor`
                        // declarations inside usecase scoping blocks (e.g.
                        // `rectangle "System" { actor User }`).
                        let nk = if node_kind == FamilyNodeKind::UseCase {
                            resolve_usecase_node_kind(&mut encoded_members)
                        } else {
                            match node_kind {
                                FamilyNodeKind::Object => FamilyNodeKind::Object,
                                _ => FamilyNodeKind::Class,
                            }
                        };
                        nodes.push(FamilyNode {
                            kind: nk,
                            name: node_id,
                            alias: None,
                            members: encoded_members,
                            depth: 0,
                            label: None,
                            mindmap_side: MindMapSide::Right,
                            wbs_checkbox: None,
                            fill_color,
                        });
                    }
                }
                groups.push(FamilyGroup {
                    kind,
                    label,
                    member_ids: group_member_ids,
                    fill_color: group_fill_color,
                });
                for rel in group_relations {
                    last_relation = Some((rel.from.clone(), rel.to.clone()));
                    relations.push(model_relation_from_ast(rel)?);
                }
            }
            StatementKind::SetOption { key, value } => {
                if key.eq_ignore_ascii_case("namespaceSeparator") {
                    namespace_separator = Some(value);
                }
            }
            StatementKind::HideOption(opt) => {
                hide_options.insert(opt.to_ascii_lowercase());
            }
            StatementKind::Title(v) => common.title(v),
            StatementKind::Header(v) => common.header(v),
            StatementKind::Footer(v) => common.footer(v),
            StatementKind::Caption(v) => common.caption(v),
            StatementKind::Legend(v) => common.legend(v, LegendTextMode::ParsePackedPosition),
            StatementKind::Mainframe(v) => common.mainframe(v),
            StatementKind::Scale(body) => common.scale(&body),
            StatementKind::Theme(value) => {
                if family_kind == DiagramKind::Salt {
                    let preset = crate::theme::resolve_sequence_theme_preset(&value)
                        .map_err(|msg| Diagnostic::error(msg).with_span(stmt.span))?;
                    salt_style = crate::theme::salt_style_from_sequence_theme(&preset.style);
                    continue;
                }
                style_cascade.apply_theme(&value, stmt.span)?;
            }
            StatementKind::LegendPos(pos) => {
                common.legend_position(&pos);
            }
            StatementKind::Pragma(_)
            | StatementKind::AllowMixing
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            // Phase B (#1404): accumulate typed `<style>` block rules into the builder
            // so they can be queried per-element at cascade time.  Only Regular-scheme
            // rules are consumed; Dark rules are stored for future `--scheme dark` flag.
            // Phase E (#1417): emit W_STYLE_UNKNOWN_TAG / W_STYLE_UNKNOWN_PROPERTY /
            // E_STYLE_BAD_VALUE diagnostics via push_with_warnings.
            StatementKind::StyleBlock(block) => {
                for rule in block.rules {
                    if rule.scheme == crate::ast::style::StyleScheme::Regular {
                        style_builder.push_with_warnings(rule, &mut warnings);
                    }
                }
            }
            StatementKind::SaltGridRow { cells } => {
                if family_kind != DiagramKind::Salt {
                    return Err(Diagnostic::error(
                        "[E_FAMILY_MIXED] mixed diagram families are not supported in one document",
                    )
                    .with_span(stmt.span));
                }
                // Emit W_SALT_UNSUPPORTED_* warnings for constructs that render
                // only as placeholders (table spans, undefined sprite refs).
                salt_scan_unsupported(
                    &cells,
                    stmt.span,
                    &sprites,
                    &salt_ascii_sprite_names,
                    &mut warnings,
                );
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Salt,
                    name: encode_salt_cells(cells),
                    alias: None,
                    members: Vec::new(),
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color: None,
                });
            }
            kind if family_kind == DiagramKind::Salt && kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                match raw.category {
                    RawSyntaxCategory::Unsupported | RawSyntaxCategory::LegacyUnknown => {
                        // Graceful degradation: skip the unsupported line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        warnings.push(common::raw_syntax_feature_loss_warning(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                        continue;
                    }
                    RawSyntaxCategory::BenignPassthrough => {}
                    _ => {
                        return Err(common::raw_syntax_diagnostic(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                    }
                }
                let line = raw.line;
                if line.trim() == "---" {
                    continue;
                }
                // Treat non-row unknown lines as plain label rows
                nodes.push(FamilyNode {
                    kind: FamilyNodeKind::Salt,
                    name: format!("SALT_ROW\x1fL:{line}"),
                    alias: None,
                    members: Vec::new(),
                    depth: 0,
                    label: None,
                    mindmap_side: MindMapSide::Right,
                    wbs_checkbox: None,
                    fill_color: None,
                });
            }
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                let line = raw.line;
                // Handle `left to right direction` / `top to bottom direction`
                if raw.category == RawSyntaxCategory::BenignPassthrough {
                    if let Some(dir) = parse_family_orientation_directive(line) {
                        orientation = dir;
                        continue;
                    }
                }
                match raw.category {
                    RawSyntaxCategory::Unsupported | RawSyntaxCategory::LegacyUnknown => {
                        // Graceful degradation: skip the unsupported line and emit a
                        // non-fatal feature-loss warning so the valid remainder renders.
                        warnings.push(common::raw_syntax_feature_loss_warning(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                        continue;
                    }
                    RawSyntaxCategory::BenignPassthrough => {
                        // Unconsumed passthrough (direction not matched): still a
                        // parser-bug signal, keep as hard error below.
                    }
                    _ => {
                        return Err(common::raw_syntax_diagnostic(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(family_kind),
                        ));
                    }
                }
                if family_kind == DiagramKind::Salt {
                    let text = line.trim();
                    if !text.is_empty() {
                        nodes.push(FamilyNode {
                            kind: FamilyNodeKind::Salt,
                            name: text.to_string(),
                            alias: None,
                            members: Vec::new(),
                            depth: 0,
                            label: None,
                            mindmap_side: MindMapSide::Right,
                            wbs_checkbox: None,
                            fill_color: None,
                        });
                    }
                    continue;
                }
                return Err(common::raw_syntax_diagnostic(
                    raw,
                    stmt.span,
                    RawSyntaxContext::Family(family_kind),
                ));
            }
            _ => {
                return Err(Diagnostic::error(format!(
                    "[E_FAMILY_STUB_UNSUPPORTED] unsupported {} syntax in bootstrap slice",
                    family_kind_name(family_kind)
                ))
                .with_span(stmt.span));
            }
        }
    }

    // Merge duplicate Rel() pairs (same from→to→arrow) by joining their labels
    // with a newline so they render as stacked lines on a single arrow rather than
    // visually concatenated strings on overlapping arrows. This is the correct
    // PlantUML behaviour for C4 diagrams where Rel() is a macro that emits a
    // simple arrow — multiple calls with the same endpoints should coalesce.
    // Only applies to the stub/C4 path; the class/component paths keep separate
    // edges intentionally (e.g. bidirectional cardinality pairs). (#425)
    apply_class_visibility_controls(&mut nodes, &mut relations, &mut groups, &hide_options);
    let relations = merge_duplicate_rel_labels(relations);
    common::sort_diagnostics_by_message_and_span(&mut warnings);
    let sepia = style_cascade.sepia();
    // Phase E (#1417): apply StyleBuilder rules to the flat style structs for
    // families that read from skinparam fields rather than querying StyleBuilder
    // at render time (salt).  This replaces the former StyleParam compat shim.
    if !style_builder.is_empty() && family_kind == DiagramKind::Salt {
        apply_style_builder_to_salt(&mut salt_style, &style_builder);
    }
    let mut family_style = if family_kind == DiagramKind::Salt {
        crate::model::FamilyStyle::Salt(Box::new(salt_style))
    } else {
        style_cascade.into_family_style()
    };
    // Phase B (#1404): attach the accumulated StyleBuilder to the family style so
    // the cascade resolver can query `<style>` block rules per element at render time.
    // Phase E (#1417): also apply diagram-level arrow colour from StyleBuilder.
    if !style_builder.is_empty() {
        let boxed = Box::new(style_builder);
        if let crate::model::FamilyStyle::Class(cs) = &mut family_style {
            // Apply diagram-level arrow colour: usecaseDiagram { arrow { LineColor … } }
            // or classDiagram { arrow { LineColor … } }.
            apply_arrow_color_from_style_builder(cs, family_kind, &boxed);
            cs.style_builder = Some(boxed);
        }
    }

    Ok(FamilyDocument {
        kind: family_kind,
        nodes,
        relations,
        groups,
        json_projections,
        hide_options,
        namespace_separator,
        title: common.title,
        header: common.header,
        header_align: common.header_align,
        footer: common.footer,
        footer_align: common.footer_align,
        caption: common.caption,
        legend: common.legend,
        legend_halign: common.legend_halign,
        legend_valign: common.legend_valign,
        mainframe: common.mainframe,
        scale: common.scale,
        orientation,
        style: SequenceStyle {
            sepia,
            ..SequenceStyle::default()
        },
        family_style: Some(family_style),
        text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        maximum_width: None,
        sprites,
        list_sprites,
        edge_routing,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// StyleBlock → SaltStyle bridge (Phase E, #1417)
// ---------------------------------------------------------------------------

/// Query `builder` for `<style>` rules that target Salt widget elements and
/// apply the resulting colours to the flat `SaltStyle` fields.
///
/// This replaces the `StyleParam` compat shim path that previously translated
/// `saltDiagram { BackgroundColor … }` into `SaltBackgroundColor` skinparam
/// triples consumed by `SaltStyle::apply_key`.
fn apply_style_builder_to_salt(
    salt: &mut crate::theme::SaltStyle,
    builder: &crate::theme::StyleBuilder,
) {
    use crate::ast::style::{PName, SName};
    use crate::theme::style_builder::StyleQuery;

    let color = |query: &StyleQuery, pname: PName| -> Option<String> {
        builder.resolve(query).color(pname).map(str::to_string)
    };

    // saltDiagram { BackgroundColor / FontColor / LineColor }
    let diagram_q = StyleQuery::tags([SName::SaltDiagram]);
    if let Some(c) = color(&diagram_q, PName::BackgroundColor) {
        salt.canvas_fill = c;
    }
    if let Some(c) = color(&diagram_q, PName::FontColor) {
        salt.text_color = c;
    }
    if let Some(c) = color(&diagram_q, PName::LineColor) {
        salt.border_color = c;
    }

    // Salt widget selectors may appear as top-level selectors.

    // button { BackgroundColor / FontColor }
    let button_q = StyleQuery::tags([SName::Button]);
    if let Some(c) = color(&button_q, PName::BackgroundColor) {
        salt.button_fill = c;
    }
    if let Some(c) = color(&button_q, PName::FontColor) {
        salt.button_text_color = c;
    }

    // input / textfield / textarea { BackgroundColor / FontColor }
    let input_q = StyleQuery::tags([SName::Input]);
    if let Some(c) = color(&input_q, PName::BackgroundColor) {
        salt.input_fill = c;
    }
    if let Some(c) = color(&input_q, PName::FontColor) {
        salt.input_text_color = c;
    }

    // menu { BackgroundColor }
    let menu_q = StyleQuery::tags([SName::Menu]);
    if let Some(c) = color(&menu_q, PName::BackgroundColor) {
        salt.menu_fill = c;
    }

    // tab { BackgroundColor }
    let tab_q = StyleQuery::tags([SName::Tab]);
    if let Some(c) = color(&tab_q, PName::BackgroundColor) {
        salt.tab_fill = c;
    }

    // header { BackgroundColor / FontColor }
    let header_q = StyleQuery::tags([SName::Header]);
    if let Some(c) = color(&header_q, PName::BackgroundColor) {
        salt.header_fill = c;
    }
    if let Some(c) = color(&header_q, PName::FontColor) {
        salt.header_text_color = c;
    }
}

// ---------------------------------------------------------------------------
// StyleBlock → ClassStyle arrow-colour bridge (Phase E, #1417)
// ---------------------------------------------------------------------------

/// Apply diagram-level arrow colour from `<style>` block rules to `ClassStyle`.
///
/// The `arrow { LineColor … }` (or `arrowColor`) selector inside a family
/// style block sets the default edge/relation colour for that diagram family.
/// This was previously handled by the compat shim; now it goes through the
/// StyleBuilder.
fn apply_arrow_color_from_style_builder(
    cs: &mut crate::theme::ClassStyle,
    family_kind: DiagramKind,
    builder: &crate::theme::StyleBuilder,
) {
    use crate::ast::style::{PName, SName};
    use crate::theme::style_builder::StyleQuery;

    let diagram_sname = match family_kind {
        DiagramKind::UseCase => SName::UsecaseDiagram,
        _ => SName::ClassDiagram,
    };

    // diagram { arrow { LineColor / ArrowColor } } — "arrowcolor" is
    // now an alias for PName::LineColor in the retrieve function.
    let arrow_q = StyleQuery::tags([diagram_sname, SName::Arrow]);
    if let Some(c) = builder.resolve(&arrow_q).color(PName::LineColor) {
        cs.arrow_color = c.to_string();
    }
}
