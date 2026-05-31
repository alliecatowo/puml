use super::*;
pub(crate) fn parse_c4_legacy_family_relation(
    line: &str,
    family: Option<DiagramKind>,
) -> Option<Vec<StatementKind>> {
    let trimmed = line.trim();
    let open = trimmed.find('(')?;
    let macro_name = trimmed[..open].trim();
    let normalized = normalize_c4_keyword(macro_name);
    if !is_c4_legacy_relation_macro(&normalized) {
        return None;
    }

    if !matches!(
        family,
        None | Some(
            DiagramKind::Class
                | DiagramKind::Object
                | DiagramKind::UseCase
                | DiagramKind::Component
                | DiagramKind::Deployment
        )
    ) {
        return None;
    }
    let args = split_parenthesized_args(
        trimmed
            .get(open + 1..)?
            .split_once(')')
            .map(|(raw, _)| raw)
            .unwrap_or(""),
    );

    if args.len() < 3 {
        return None;
    }
    let from = clean_ident(&args[0]);
    let to = clean_ident(&args[1]);
    let label = unquote_if_quoted(args[2].trim());
    if from.is_empty() || to.is_empty() || label.is_empty() {
        return None;
    }

    match normalized.as_str() {
        "rel" | "rel_u" | "rel_d" | "rel_l" | "rel_r" | "rel_left" | "rel_right" | "rel_up"
        | "rel_down" => {
            let label = label.to_string();
            Some(vec![StatementKind::FamilyRelation(FamilyRelation {
                from,
                to,
                arrow: "-->".to_string(),
                label: Some(label.to_string()),
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
            })])
        }
        "rel_back" => Some(vec![StatementKind::FamilyRelation(FamilyRelation {
            from: to,
            to: from,
            arrow: "->".to_string(),
            label: Some(label.to_string()),
            stereotype: Some("c4-rel-back".to_string()),
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
        })]),
        "rel_dynamic" => {
            let mut c4_label = "[C4 Rel_Dynamic()]".to_string();
            let mut line_color = None;
            let mut index = None;
            for extra in args.iter().skip(3) {
                let value = unquote_if_quoted(extra.trim());
                if value.is_empty() {
                    continue;
                }
                if value.starts_with('#') {
                    line_color = Some(value);
                } else if value.parse::<u8>().is_ok() {
                    index = Some(value);
                }
            }
            if let Some(value) = index {
                c4_label = format!("[C4 Rel_Dynamic({value})]");
            }
            Some(vec![StatementKind::FamilyRelation(FamilyRelation {
                from,
                to,
                arrow: "->".to_string(),
                label: Some(format!("{c4_label} {label}")),
                stereotype: None,
                left_cardinality: None,
                right_cardinality: None,
                left_role: None,
                right_role: None,
                line_color: line_color.map(str::to_string),
                label_color: None,
                dashed: false,
                hidden: false,
                thickness: None,
                direction: None,
                left_lollipop: false,
                right_lollipop: false,
            })])
        }
        "rel_neighbor" => Some(vec![StatementKind::FamilyRelation(FamilyRelation {
            from,
            to,
            arrow: "->".to_string(),
            label: Some(format!("[C4 Rel_Neighbor] {label}")),
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
        })]),
        "rel_back_neighbor" => Some(vec![StatementKind::FamilyRelation(FamilyRelation {
            from: to,
            to: from,
            arrow: "->".to_string(),
            label: Some(format!("[C4 Rel_Back_Neighbor] {label}")),
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
        })]),
        "birel" => Some(vec![
            StatementKind::FamilyRelation(FamilyRelation {
                from: from.clone(),
                to: to.clone(),
                arrow: "->".to_string(),
                label: Some(format!("[C4 BiRel] {label}")),
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
            }),
            StatementKind::FamilyRelation(FamilyRelation {
                from: to,
                to: from,
                arrow: "->".to_string(),
                label: Some(format!("[C4 BiRel] {label}")),
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
            }),
        ]),
        _ => None,
    }
}

pub(crate) fn is_c4_legacy_relation_macro(normalized: &str) -> bool {
    matches!(
        normalized,
        "rel"
            | "rel_u"
            | "rel_d"
            | "rel_l"
            | "rel_r"
            | "rel_left"
            | "rel_right"
            | "rel_up"
            | "rel_down"
            | "rel_back"
            | "rel_dynamic"
            | "rel_neighbor"
            | "rel_back_neighbor"
            | "birel"
    )
}
