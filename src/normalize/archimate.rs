use super::*;

pub(super) fn normalize_archimate_document(
    document: Document,
) -> Result<ArchimateDocument, Diagnostic> {
    let (raw, title) = collect_raw_block(&document);
    let mut elements: Vec<ArchimateElement> = Vec::new();
    let mut relations: Vec<ArchimateRelation> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('\'') {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("archimate ") {
            // archimate "Name" as alias <<layer>>
            if let Some(elem) = parse_archimate_element(rest) {
                elements.push(elem);
                continue;
            }
        }
        // ArchiMate stdlib-style declarations:
        // Business_Actor(customer, "Customer")
        // Application_Component(service, "Order Service")
        // Technology_Node(host, "Runtime")
        if let Some(elem) = parse_archimate_macro_element(trimmed) {
            elements.push(elem);
            continue;
        }
        // Relation macros: Rel_Association(a, b, "label"), Rel_Realization(a, b)
        if let Some(open) = trimmed.find('(') {
            let macro_name = trimmed[..open].trim();
            if let Some(kind) = archimate_rel_kind_from_macro(macro_name) {
                let inside = trimmed[open + 1..].trim_end_matches([')', ' ', '\t']);
                let args: Vec<String> = split_csv_args(inside);
                if args.len() >= 2 {
                    let from = args[0].trim().trim_matches('"').to_string();
                    let to = args[1].trim().trim_matches('"').to_string();
                    let label = args
                        .get(2)
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .filter(|s| !s.is_empty());
                    relations.push(ArchimateRelation {
                        from,
                        to,
                        kind: kind.to_string(),
                        label,
                        direction: archimate_rel_direction_from_macro(macro_name),
                        style: args.iter().skip(3).find_map(|arg| {
                            let value = arg.trim().trim_matches('"');
                            if value.contains("dashed") || value.contains("bold") {
                                Some(value.to_string())
                            } else {
                                parse_archimate_color_arg(arg)
                            }
                        }),
                    });
                    continue;
                }
            }
        }
        // Plain arrow: a --> b : label
        if let Some(rel) = parse_archimate_arrow(trimmed) {
            relations.push(rel);
            continue;
        }
    }

    Ok(ArchimateDocument {
        elements,
        relations,
        title,
        warnings: Vec::new(),
    })
}

fn parse_archimate_element(rest: &str) -> Option<ArchimateElement> {
    // expect: "Name" as alias <<layer>>  OR  Name <<layer>>  OR  "Name" <<layer>>
    let mut s = rest.trim().to_string();
    let mut stereotype = "business".to_string();
    if let Some(open) = s.find("<<") {
        if let Some(close) = s[open + 2..].find(">>") {
            stereotype = s[open + 2..open + 2 + close].trim().to_string();
            s = format!("{} {}", &s[..open], &s[open + 2 + close + 2..]);
        }
    }
    let s = s.trim();
    let (name, alias) = if let Some(stripped) = s.strip_prefix('"') {
        let close = stripped.find('"')?;
        let name = stripped[..close].to_string();
        let rest = stripped[close + 1..].trim();
        let alias = rest.strip_prefix("as ").map(|a| a.trim().to_string());
        (name, alias)
    } else {
        let mut parts = s.split_whitespace();
        let name = parts.next()?.to_string();
        let alias = if parts.next() == Some("as") {
            parts.next().map(|s| s.to_string())
        } else {
            None
        };
        (name, alias)
    };
    let (name, fill) = split_archimate_inline_color(name);
    let layer = archimate_layer_from_stereotype(&stereotype);
    let kind = archimate_kind_from_stereotype(&stereotype);
    Some(ArchimateElement {
        name,
        alias,
        layer: layer.to_string(),
        kind,
        fill,
        stroke: None,
    })
}

fn parse_archimate_macro_element(line: &str) -> Option<ArchimateElement> {
    let open = line.find('(')?;
    let macro_name = line[..open].trim();
    let (layer, kind) = archimate_layer_and_kind_from_macro(macro_name)?;
    let inside = line[open + 1..].trim_end_matches([')', ' ', '\t']);
    let args = split_csv_args(inside);
    let alias = args.first()?.trim().trim_matches('"').to_string();
    if alias.is_empty() {
        return None;
    }
    let name = args
        .get(1)
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| alias.clone());
    let fill = args
        .iter()
        .skip(2)
        .find_map(|arg| parse_archimate_color_arg(arg));
    Some(ArchimateElement {
        name,
        alias: Some(alias),
        layer: layer.to_string(),
        kind,
        fill,
        stroke: None,
    })
}

fn archimate_layer_and_kind_from_macro(name: &str) -> Option<(&'static str, String)> {
    let lower = name.to_ascii_lowercase();
    let layer = if lower.starts_with("strategy_") {
        "strategy"
    } else if lower.starts_with("business_") {
        "business"
    } else if lower.starts_with("application_") || lower.starts_with("data_") {
        "application"
    } else if lower.starts_with("technology_") || lower.starts_with("physical_") {
        "technology"
    } else if lower.starts_with("motivation_") {
        "motivation"
    } else if lower.starts_with("junction_") {
        "junction"
    } else if lower.starts_with("implementation_") || lower.starts_with("migration_") {
        "strategy"
    } else {
        return None;
    };
    Some((layer, archimate_kind_from_macro_name(name)))
}

fn archimate_kind_from_macro_name(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("data_") {
        return lower.replace('_', "-");
    }
    name.split_once('_')
        .map(|(_, suffix)| suffix)
        .unwrap_or(name)
        .trim_matches('_')
        .replace('_', "-")
        .to_ascii_lowercase()
}

fn archimate_layer_from_stereotype(stereotype: &str) -> String {
    let lower = stereotype.to_ascii_lowercase().replace('_', "-");
    if lower.starts_with("strategy")
        || lower.starts_with("implementation")
        || lower.starts_with("migration")
    {
        "strategy".to_string()
    } else if lower.starts_with("business") {
        "business".to_string()
    } else if lower.starts_with("application") || lower.starts_with("data") {
        "application".to_string()
    } else if lower.starts_with("technology") || lower.starts_with("physical") {
        "technology".to_string()
    } else if lower.starts_with("motivation") {
        "motivation".to_string()
    } else if lower.starts_with("junction") {
        "junction".to_string()
    } else {
        lower
    }
}

fn archimate_kind_from_stereotype(stereotype: &str) -> String {
    stereotype.trim().replace('_', "-").to_ascii_lowercase()
}

fn archimate_rel_kind_from_macro(name: &str) -> Option<&'static str> {
    let base = archimate_rel_macro_base(name);
    match base.as_str() {
        "Rel_Access" => Some("access"),
        "Rel_Aggregation" => Some("aggregation"),
        "Rel_Association" => Some("association"),
        "Rel_Assignment" => Some("assignment"),
        "Rel_Composition" => Some("composition"),
        "Rel_Flow" => Some("flow"),
        "Rel_Influence" => Some("influence"),
        "Rel_Realization" => Some("realization"),
        "Rel_Serving" => Some("serving"),
        "Rel_Specialization" => Some("specialization"),
        "Rel_Triggering" => Some("triggering"),
        "Rel_Used_By" => Some("used_by"),
        _ => None,
    }
}

fn archimate_rel_macro_base(name: &str) -> String {
    let mut base = name.to_string();
    for suffix in [
        "_Down", "_Up", "_Left", "_Right", "_D", "_U", "_L", "_R", "_d", "_u", "_l", "_r",
    ] {
        if base.ends_with(suffix) {
            base.truncate(base.len().saturating_sub(suffix.len()));
            break;
        }
    }
    base
}

fn archimate_rel_direction_from_macro(name: &str) -> Option<String> {
    for (suffix, direction) in [
        ("_Down", "down"),
        ("_D", "down"),
        ("_d", "down"),
        ("_Up", "up"),
        ("_U", "up"),
        ("_u", "up"),
        ("_Left", "left"),
        ("_L", "left"),
        ("_l", "left"),
        ("_Right", "right"),
        ("_R", "right"),
        ("_r", "right"),
    ] {
        if name.ends_with(suffix) {
            return Some(direction.to_string());
        }
    }
    None
}

fn parse_archimate_color_arg(arg: &str) -> Option<String> {
    let value = arg.trim().trim_matches('"');
    if value.starts_with('#') || value.starts_with("$") {
        Some(value.to_string())
    } else {
        None
    }
}

fn split_archimate_inline_color(name: String) -> (String, Option<String>) {
    let mut parts = name.split_whitespace().collect::<Vec<_>>();
    let fill = parts
        .last()
        .copied()
        .filter(|part| part.starts_with('#') || part.starts_with("$"))
        .map(str::to_string);
    if fill.is_some() {
        parts.pop();
        (parts.join(" "), fill)
    } else {
        (name, None)
    }
}

pub(super) fn split_csv_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for ch in s.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            cur.push(ch);
        } else if ch == ',' && !in_quotes {
            out.push(std::mem::take(&mut cur));
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn parse_archimate_arrow(line: &str) -> Option<ArchimateRelation> {
    for arrow in ["-->", "->", "<--", "<-"] {
        if let Some(ix) = line.find(arrow) {
            let lhs = line[..ix].trim();
            let rhs_full = line[ix + arrow.len()..].trim();
            if lhs.is_empty() || rhs_full.is_empty() {
                return None;
            }
            let (rhs, label) = match rhs_full.split_once(':') {
                Some((r, l)) => (r.trim(), Some(l.trim().to_string())),
                None => (rhs_full, None),
            };
            return Some(ArchimateRelation {
                from: lhs.to_string(),
                to: rhs.to_string(),
                kind: "uses".to_string(),
                label,
                direction: match arrow {
                    "<--" | "<-" => Some("left".to_string()),
                    _ => Some("right".to_string()),
                },
                style: if arrow.contains("--") {
                    Some("dashed".to_string())
                } else {
                    None
                },
            });
        }
    }
    None
}
