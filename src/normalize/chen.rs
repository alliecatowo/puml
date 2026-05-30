use crate::ast::{
    ChenDeclKind as AstChenDeclKind, DiagramKind, Document, RawSyntaxCategory, StatementKind,
};
use crate::diagnostic::Diagnostic;
use crate::model::{
    ChenAttribute as ModelChenAttribute, ChenDocument, ChenInheritance as ModelChenInheritance,
    ChenNode, ChenNodeKind, ChenRelation as ModelChenRelation, FamilyOrientation,
};
use crate::normalize::common::{self, CommonDirectives, LegendTextMode, RawSyntaxContext};

pub(super) fn normalize_chen(document: Document) -> Result<ChenDocument, Diagnostic> {
    let mut nodes = Vec::new();
    let mut relations = Vec::new();
    let mut inheritances = Vec::new();
    let mut common = CommonDirectives::default();
    let mut orientation = FamilyOrientation::TopToBottom;
    let mut warnings: Vec<crate::diagnostic::Diagnostic> = Vec::new();

    for stmt in document.statements {
        match stmt.kind {
            StatementKind::ChenDecl(decl) => {
                let id = decl.alias.clone().unwrap_or_else(|| decl.name.clone());
                upsert_chen_node(
                    &mut nodes,
                    ChenNode {
                        kind: match decl.kind {
                            AstChenDeclKind::Entity | AstChenDeclKind::WeakEntity => {
                                ChenNodeKind::Entity
                            }
                            AstChenDeclKind::Relationship => ChenNodeKind::Relationship,
                        },
                        id,
                        label: decl.name,
                        weak: decl.kind == AstChenDeclKind::WeakEntity
                            || decl
                                .stereotypes
                                .iter()
                                .any(|value| value.eq_ignore_ascii_case("weak")),
                        identifying: decl
                            .stereotypes
                            .iter()
                            .any(|value| value.eq_ignore_ascii_case("identifying")),
                        attributes: decl
                            .attributes
                            .into_iter()
                            .map(normalize_chen_attribute)
                            .collect(),
                    },
                );
            }
            StatementKind::ChenRelation(rel) => {
                relations.push(ModelChenRelation {
                    from: rel.from,
                    to: rel.to,
                    cardinality: rel.cardinality,
                    total_participation: rel.total_participation,
                });
            }
            StatementKind::ChenInheritance(inheritance) => {
                inheritances.push(ModelChenInheritance {
                    parent: inheritance.parent,
                    connector: inheritance.connector,
                    discriminator: inheritance.discriminator,
                    children: inheritance.children,
                });
            }
            StatementKind::Title(value) => common.title(value),
            StatementKind::Caption(value) => common.caption(value),
            StatementKind::Legend(value) => common.legend(value, LegendTextMode::Raw),
            StatementKind::Theme(value) => {
                warnings.push(
                    Diagnostic::warning(format!(
                        "[W_THEME_UNSUPPORTED] Chen ER renderer does not apply theme `{value}`; \
                         theme directives have no effect in Chen diagrams"
                    ))
                    .with_span(stmt.span),
                );
            }
            StatementKind::Pragma(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Include(_)
            | StatementKind::Define { .. }
            | StatementKind::Undef(_) => {}
            kind if kind.raw_syntax().is_some() => {
                let raw = kind.raw_syntax().expect("raw syntax guard");
                if raw.category == RawSyntaxCategory::BenignPassthrough {
                    if let Some(dir) = parse_chen_orientation_directive(raw.line) {
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
                            RawSyntaxContext::Family(DiagramKind::Chen),
                        ));
                    }
                    _ => {
                        return Err(common::raw_syntax_diagnostic(
                            raw,
                            stmt.span,
                            RawSyntaxContext::Family(DiagramKind::Chen),
                        ));
                    }
                }
            }
            _ => {
                return Err(Diagnostic::error(
                    "[E_CHEN_UNSUPPORTED] unsupported statement in Chen diagram",
                )
                .with_span(stmt.span));
            }
        }
    }

    ensure_relation_endpoint_nodes(&mut nodes, &relations, &inheritances);

    Ok(ChenDocument {
        nodes,
        relations,
        inheritances,
        title: common.title,
        caption: common.caption,
        legend: common.legend,
        orientation,
        warnings,
    })
}

fn normalize_chen_attribute(attr: crate::ast::ChenAttribute) -> ModelChenAttribute {
    let id = attr.alias.clone().unwrap_or_else(|| attr.name.clone());
    ModelChenAttribute {
        id,
        label: attr.name,
        data_type: attr.data_type,
        key: attr.stereotypes.iter().any(|value| {
            value.eq_ignore_ascii_case("key")
                || value.eq_ignore_ascii_case("pk")
                || value.eq_ignore_ascii_case("pk-partial")
        }),
        derived: attr
            .stereotypes
            .iter()
            .any(|value| value.eq_ignore_ascii_case("derived")),
        multivalued: attr.stereotypes.iter().any(|value| {
            value.eq_ignore_ascii_case("multi") || value.eq_ignore_ascii_case("multivalued")
        }),
        children: attr
            .children
            .into_iter()
            .map(normalize_chen_attribute)
            .collect(),
    }
}

fn upsert_chen_node(nodes: &mut Vec<ChenNode>, node: ChenNode) {
    if let Some(existing) = nodes.iter_mut().find(|existing| existing.id == node.id) {
        *existing = node;
    } else {
        nodes.push(node);
    }
}

fn ensure_relation_endpoint_nodes(
    nodes: &mut Vec<ChenNode>,
    relations: &[ModelChenRelation],
    inheritances: &[ModelChenInheritance],
) {
    for rel in relations {
        for endpoint in [&rel.from, &rel.to] {
            ensure_chen_entity_node(nodes, endpoint);
        }
    }
    for inheritance in inheritances {
        ensure_chen_entity_node(nodes, &inheritance.parent);
        for child in &inheritance.children {
            ensure_chen_entity_node(nodes, child);
        }
    }
}

fn ensure_chen_entity_node(nodes: &mut Vec<ChenNode>, id: &str) {
    if id.is_empty() || nodes.iter().any(|node| node.id == id) {
        return;
    }
    nodes.push(ChenNode {
        kind: ChenNodeKind::Entity,
        id: id.to_string(),
        label: id.to_string(),
        weak: false,
        identifying: false,
        attributes: Vec::new(),
    });
}

fn parse_chen_orientation_directive(line: &str) -> Option<FamilyOrientation> {
    let tokens = line
        .split_whitespace()
        .map(|token| token.to_ascii_lowercase())
        .collect::<Vec<_>>();
    if tokens.len() != 4 || tokens[3] != "direction" {
        return None;
    }
    match [&tokens[0][..], &tokens[1][..], &tokens[2][..]]
        .join(" ")
        .as_str()
    {
        "left to right" => Some(FamilyOrientation::LeftToRight),
        "right to left" => Some(FamilyOrientation::RightToLeft),
        "top to bottom" => Some(FamilyOrientation::TopToBottom),
        "bottom to top" => Some(FamilyOrientation::BottomToTop),
        _ => None,
    }
}
