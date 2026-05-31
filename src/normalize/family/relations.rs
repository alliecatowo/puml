use super::*;

/// Merge relations that share the same `(from, to, arrow)` triple by joining
/// their labels with `\n`.  Duplicate Rel() macro calls in C4 diagrams produce
/// overlapping arrows that visually concatenate their labels with no delimiter;
/// coalescing them into one arrow with a newline-separated label is the correct
/// PlantUML parity behaviour (#425).
///
/// Only relations that are otherwise identical (same direction, color, style)
/// are merged; differing style attributes keep the relations separate.
pub(super) fn merge_duplicate_rel_labels(
    relations: Vec<ModelFamilyRelation>,
) -> Vec<ModelFamilyRelation> {
    // Use an ordered map keyed by (from, to, arrow, direction, line_color,
    // dashed, hidden) so determinism is preserved (BTreeMap, not HashMap).
    // Value: index into `out` for the already-inserted canonical relation.
    type RelKey = (
        String,
        String,
        crate::model::FamilyRelationArrow,
        Option<crate::model::FamilyRelationDirection>,
        Option<crate::model::FamilyRelationColor>,
        bool,
        bool,
    );
    let mut seen: std::collections::BTreeMap<RelKey, usize> = std::collections::BTreeMap::new();
    let mut out: Vec<ModelFamilyRelation> = Vec::with_capacity(relations.len());

    for rel in relations {
        let key = (
            rel.from.clone(),
            rel.to.clone(),
            rel.arrow.clone(),
            rel.direction,
            rel.line_color.clone(),
            rel.dashed,
            rel.hidden,
        );
        if let Some(&idx) = seen.get(&key) {
            // Merge this relation's label into the existing one.
            if let Some(new_label) = rel.label {
                let existing = &mut out[idx].label;
                *existing = Some(match existing.take() {
                    Some(prev) => format!("{prev}\n{new_label}"),
                    None => new_label,
                });
            }
            // Merge stereotype similarly.
            if let Some(new_st) = rel.stereotype {
                let existing = &mut out[idx].stereotype;
                if existing.is_none() {
                    *existing = Some(new_st);
                }
            }
        } else {
            seen.insert(key, out.len());
            out.push(rel);
        }
    }
    out
}

pub(super) fn extract_family_heritage_relations(
    members: &mut Vec<ClassMember>,
    source_id: &str,
) -> Vec<ModelFamilyRelation> {
    let mut out = Vec::new();
    members.retain(|member| {
        let Some(rest) = member.text.strip_prefix("\x1fheritage:") else {
            return true;
        };
        if let Some((arrow, target)) = rest.split_once(':') {
            let target = target.trim();
            if !target.is_empty() {
                out.push(simple_family_relation(
                    target.to_string(),
                    source_id.to_string(),
                    arrow.to_string(),
                ));
            }
        }
        false
    });
    out
}

pub(super) fn extract_map_row_relations(
    members: &[ClassMember],
    source_id: &str,
) -> Vec<ModelFamilyRelation> {
    members
        .iter()
        .filter_map(|member| parse_map_row_relation(&member.text, source_id))
        .collect()
}

pub(super) fn parse_map_row_relation(row: &str, source_id: &str) -> Option<ModelFamilyRelation> {
    let trimmed = row.trim();
    for marker in [
        "*--->", "*-->", "*---", "*--", "*->", "-->", "---", "--", "..>", "...", "..",
    ] {
        let Some((key, target)) = trimmed.split_once(marker) else {
            continue;
        };
        let key = key.trim();
        let target = target.trim();
        if key.is_empty() || target.is_empty() {
            return None;
        }
        return Some(simple_family_relation(
            format!("{source_id}::{key}"),
            target.to_string(),
            marker.to_string(),
        ));
    }
    None
}

pub(super) fn model_relation_from_ast(
    rel: crate::ast::FamilyRelation,
) -> Result<ModelFamilyRelation, Diagnostic> {
    let arrow = crate::model::FamilyRelationArrow::parse(&rel.arrow)
        .map_err(|msg| Diagnostic::error(format!("[E_FAMILY_RELATION_ARROW] {msg}")))?;
    let direction = rel
        .direction
        .as_deref()
        .map(|direction| {
            crate::model::FamilyRelationDirection::parse(direction).ok_or_else(|| {
                Diagnostic::error(format!(
                    "[E_FAMILY_RELATION_DIRECTION] invalid relation direction `{direction}`"
                ))
            })
        })
        .transpose()?;
    let line_color = rel
        .line_color
        .as_deref()
        .map(|color| {
            crate::model::FamilyRelationColor::parse(color)
                .map_err(|msg| Diagnostic::error(format!("[E_FAMILY_RELATION_COLOR] {msg}")))
        })
        .transpose()?;
    let label_color = rel
        .label_color
        .as_deref()
        .map(|color| {
            crate::model::FamilyRelationColor::parse(color)
                .map_err(|msg| Diagnostic::error(format!("[E_FAMILY_RELATION_LABEL_COLOR] {msg}")))
        })
        .transpose()?;

    Ok(ModelFamilyRelation {
        from: rel.from,
        to: rel.to,
        arrow,
        label: rel.label,
        stereotype: rel.stereotype,
        left_cardinality: rel.left_cardinality,
        right_cardinality: rel.right_cardinality,
        left_role: rel.left_role,
        right_role: rel.right_role,
        line_color,
        label_color,
        dashed: rel.dashed,
        hidden: rel.hidden,
        thickness: rel.thickness,
        direction,
        left_lollipop: rel.left_lollipop,
        right_lollipop: rel.right_lollipop,
    })
}

pub(super) fn simple_family_relation(
    from: String,
    to: String,
    arrow: String,
) -> ModelFamilyRelation {
    ModelFamilyRelation {
        from,
        to,
        arrow: crate::model::FamilyRelationArrow::parse(&arrow)
            .expect("normalizer emits only valid relation arrow literals"),
        label: None,
        stereotype: None,
        left_cardinality: None,
        right_cardinality: None,
        left_role: None,
        right_role: None,
        line_color: None,
        label_color: None,
        dashed: false,
        hidden: false,
        thickness: None,
        direction: None,
        left_lollipop: false,
        right_lollipop: false,
    }
}

pub(super) fn relation_node_endpoint(endpoint: &str) -> String {
    let trimmed = endpoint.trim();
    if let Some((owner, member)) = trimmed.rsplit_once("::") {
        if !owner.is_empty() && !member.is_empty() {
            return owner.to_string();
        }
    }
    trimmed.to_string()
}

pub(super) fn build_family_tree_relations(
    nodes: &mut [FamilyNode],
    relations: &mut Vec<ModelFamilyRelation>,
) {
    let mut parents: Vec<usize> = Vec::new();
    for idx in 0..nodes.len() {
        let depth = nodes[idx].depth;
        while parents.len() > depth {
            parents.pop();
        }
        if let Some(parent_idx) = parents.last().copied() {
            relations.push(ModelFamilyRelation {
                from: nodes[parent_idx].name.clone(),
                to: nodes[idx].name.clone(),
                arrow: crate::model::FamilyRelationArrow::parse("->")
                    .expect("normalizer emits only valid relation arrow literals"),
                label: None,
                stereotype: None,
                left_cardinality: None,
                right_cardinality: None,
                left_role: None,
                right_role: None,
                line_color: None,
                label_color: None,
                dashed: false,
                hidden: false,
                thickness: None,
                direction: None,
                left_lollipop: false,
                right_lollipop: false,
            });
        }
        parents.push(idx);
    }
}
