fn parse_skinparam_block(
    lines: &[(&str, Span)],
    start_idx: usize,
    line: &str,
) -> Option<(Vec<StatementKind>, usize)> {
    let lower = line.to_ascii_lowercase();
    let rest = lower.strip_prefix("skinparam ")?;
    // The block form ends with `{` (possibly separated by whitespace or not).
    let rest_trimmed = rest.trim_end();
    if !rest_trimmed.ends_with('{') {
        return None;
    }
    // Extract the prefix: everything between "skinparam " and the final `{`.
    let prefix_raw = rest_trimmed.trim_end_matches('{').trim();
    if prefix_raw.is_empty() {
        return None;
    }
    // Preserve original casing from the source line for the prefix.
    let original_rest = line["skinparam ".len()..].trim_end();
    let original_prefix = original_rest.trim_end_matches('{').trim();

    // Scan for the closing `}`.
    let mut kinds: Vec<StatementKind> = Vec::new();
    let mut end_idx = start_idx;
    for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
        let inner = strip_inline_plantuml_comment(raw).trim();
        if inner == "}" {
            end_idx = idx;
            break;
        }
        if inner.is_empty() {
            continue;
        }
        // Each inner line is expected to be: `InnerKey Value` or just be ignored.
        // Split on the first whitespace to get key and value parts.
        let (inner_key, inner_value) = inner
            .split_once(|c: char| c.is_whitespace())
            .map(|(k, v)| (k.trim(), v.trim()))
            .unwrap_or((inner, ""));
        if inner_key.is_empty() {
            continue;
        }
        // Combine prefix with inner key: "class" + "BackgroundColor" → "classBackgroundColor".
        // Handle stereotype-scoped inner keys: "BackgroundColor<<Abstract>>" stays as-is after prefix.
        let combined_key = format!("{original_prefix}{inner_key}");
        kinds.push(StatementKind::SkinParam {
            key: combined_key,
            value: inner_value.to_string(),
        });
        // Track the last line we successfully read as end_idx
        end_idx = idx;
    }
    Some((kinds, end_idx))
}

/// Parse a minimal PlantUML `<style>...</style>` block and map supported style
/// rules to explicit style declarations. Normalization applies these after
/// themes and skinparams so the style cascade does not depend on source order.
///
/// Supported subset:
/// - `sequenceDiagram { ... }`
/// - optional nested selectors under sequenceDiagram:
///   - `participant { ... }`
///   - `note { ... }`
///   - `group { ... }`
/// - `classDiagram { class { ... } }`
/// - `componentDiagram { component { ... } }`
/// - `deploymentDiagram { node { ... } }`
/// - declarations in `Property Value` or `Property: Value;` form
fn parse_style_block(
    lines: &[(&str, Span)],
    start_idx: usize,
    line: &str,
) -> Result<Option<(Vec<StatementKind>, usize)>, Diagnostic> {
    if !line.eq_ignore_ascii_case("<style>") {
        return Ok(None);
    }
    let Some(target) = style_block_target(lines, start_idx) else {
        // Preserve unsupported style blocks as raw lines so family-specific
        // style handling (e.g. mindmap depth styles) can consume them without
        // generic top-level keyword parsing rewriting inner declarations.
        let mut kinds = vec![StatementKind::DeferredRaw(line.to_string())];
        for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
            kinds.push(StatementKind::DeferredRaw((*raw).to_string()));
            if strip_inline_plantuml_comment(raw)
                .trim()
                .eq_ignore_ascii_case("</style>")
            {
                return Ok(Some((kinds, idx)));
            }
        }
        return Err(Diagnostic::error(
            "[E_STYLE_BLOCK_UNCLOSED] `<style>` block is missing closing `</style>`",
        )
        .with_span(lines[start_idx].1));
    };

    let mut kinds: Vec<StatementKind> = Vec::new();
    let mut in_target = false;
    let mut nested_selector: Option<String> = None;

    for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
        let inner = strip_inline_plantuml_comment(raw).trim();
        if inner.eq_ignore_ascii_case("</style>") {
            return Ok(Some((kinds, idx)));
        }
        if inner.is_empty() {
            continue;
        }
        if target.matches_open_selector(inner) {
            in_target = true;
            nested_selector = None;
            continue;
        }
        if inner == "}" {
            if nested_selector.is_some() {
                nested_selector = None;
            } else {
                in_target = false;
            }
            continue;
        }
        if !in_target {
            continue;
        }
        if inner.ends_with('{') {
            let selector = inner.trim_end_matches('{').trim().to_ascii_lowercase();
            nested_selector = Some(selector);
            continue;
        }

        let (raw_key, raw_value) = inner
            .split_once(':')
            .or_else(|| inner.split_once(|c: char| c.is_whitespace()))
            .map(|(k, v)| (k.trim(), v.trim()))
            .unwrap_or((inner, ""));
        if raw_key.is_empty() || raw_value.is_empty() {
            continue;
        }
        let value = raw_value.trim_end_matches(';').trim();
        if value.is_empty() {
            continue;
        }

        let key = target.skinparam_key(nested_selector.as_deref(), raw_key);
        kinds.push(StatementKind::StyleParam {
            selector: nested_selector.clone(),
            property: raw_key.to_string(),
            key,
            value: value.to_string(),
        });
    }

    Err(Diagnostic::error(
        "[E_STYLE_BLOCK_UNCLOSED] `<style>` block is missing closing `</style>`",
    )
    .with_span(lines[start_idx].1))
}

#[derive(Clone, Copy)]
enum StyleBlockTarget {
    Sequence,
    Class,
    Component,
    Deployment,
    State,
    Activity,
}

impl StyleBlockTarget {
    fn matches_open_selector(self, line: &str) -> bool {
        let selector = line.trim_end_matches('{').trim();
        match self {
            Self::Sequence => selector.eq_ignore_ascii_case("sequenceDiagram"),
            Self::Class => selector.eq_ignore_ascii_case("classDiagram"),
            Self::Component => selector.eq_ignore_ascii_case("componentDiagram"),
            Self::Deployment => selector.eq_ignore_ascii_case("deploymentDiagram"),
            Self::State => selector.eq_ignore_ascii_case("stateDiagram"),
            Self::Activity => selector.eq_ignore_ascii_case("activityDiagram"),
        }
    }

    fn skinparam_key(self, nested_selector: Option<&str>, raw_key: &str) -> Option<String> {
        let key = raw_key.to_ascii_lowercase();
        match self {
            Self::Sequence => sequence_style_skinparam_key(nested_selector, &key),
            Self::Class => class_style_skinparam_key(nested_selector, &key),
            Self::Component => component_style_skinparam_key(nested_selector, &key),
            Self::Deployment => deployment_style_skinparam_key(nested_selector, &key),
            Self::State => state_style_skinparam_key(nested_selector, &key),
            Self::Activity => activity_style_skinparam_key(nested_selector, &key),
        }
    }
}

fn style_block_target(lines: &[(&str, Span)], start_idx: usize) -> Option<StyleBlockTarget> {
    for (raw, _) in lines.iter().skip(start_idx + 1) {
        let inner = strip_inline_plantuml_comment(raw).trim();
        if inner.eq_ignore_ascii_case("</style>") {
            return None;
        }
        if inner.is_empty() {
            continue;
        }
        let selector = inner.trim_end_matches('{').trim();
        if selector.eq_ignore_ascii_case("sequenceDiagram") {
            return Some(StyleBlockTarget::Sequence);
        }
        if selector.eq_ignore_ascii_case("classDiagram") {
            return Some(StyleBlockTarget::Class);
        }
        if selector.eq_ignore_ascii_case("componentDiagram") {
            return Some(StyleBlockTarget::Component);
        }
        if selector.eq_ignore_ascii_case("deploymentDiagram") {
            return Some(StyleBlockTarget::Deployment);
        }
        if selector.eq_ignore_ascii_case("stateDiagram") {
            return Some(StyleBlockTarget::State);
        }
        if selector.eq_ignore_ascii_case("activityDiagram") {
            return Some(StyleBlockTarget::Activity);
        }
        return None;
    }
    None
}

fn style_selector_stereotype(selector: &str, prefix: &str) -> Option<String> {
    let trimmed = selector.trim();
    let rest = trimmed.strip_prefix(prefix)?.trim();
    let stereotype = rest.strip_prefix("<<")?.strip_suffix(">>")?.trim();
    if stereotype.is_empty() {
        None
    } else {
        Some(stereotype.to_string())
    }
}

fn sequence_style_skinparam_key(nested_selector: Option<&str>, key: &str) -> Option<String> {
    match nested_selector {
        None => match key {
            "arrowcolor" => Some("ArrowColor".to_string()),
            "lifelinebordercolor" => Some("LifelineBorderColor".to_string()),
            "backgroundcolor" => Some("BackgroundColor".to_string()),
            _ => None,
        },
        Some("participant") => match key {
            "backgroundcolor" => Some("ParticipantBackgroundColor".to_string()),
            "bordercolor" => Some("ParticipantBorderColor".to_string()),
            "fontcolor" => Some("ParticipantFontColor".to_string()),
            _ => None,
        },
        Some("note") => match key {
            "backgroundcolor" => Some("NoteBackgroundColor".to_string()),
            "bordercolor" => Some("NoteBorderColor".to_string()),
            _ => None,
        },
        Some("group") => match key {
            "backgroundcolor" => Some("GroupBackgroundColor".to_string()),
            "bordercolor" => Some("GroupBorderColor".to_string()),
            "headerfontcolor" => Some("GroupHeaderFontColor".to_string()),
            "headerfontstyle" => Some("GroupHeaderFontStyle".to_string()),
            _ => None,
        },
        Some(_) => None,
    }
}

fn class_style_skinparam_key(nested_selector: Option<&str>, key: &str) -> Option<String> {
    let Some(selector) = nested_selector else {
        return match key {
            "arrowcolor" | "linecolor" => Some("ClassArrowColor".to_string()),
            _ => None,
        };
    };
    let selector = selector.trim();
    let scope = style_selector_stereotype(selector, "class");
    let scoped_key = |base: &str| {
        scope
            .as_ref()
            .map(|stereotype| format!("{base}<<{stereotype}>>"))
            .unwrap_or_else(|| base.to_string())
    };
    if selector == "arrow" || selector == "relation" {
        return match key {
            "linecolor" | "arrowcolor" | "bordercolor" => Some("ClassArrowColor".to_string()),
            _ => None,
        };
    }
    if selector == "class" || selector.starts_with("class<<") {
        return match key {
            "backgroundcolor" => Some(scoped_key("ClassBackgroundColor")),
            "bordercolor" | "linecolor" => Some(scoped_key("ClassBorderColor")),
            "headerbackgroundcolor" => Some(scoped_key("ClassHeaderBackgroundColor")),
            "fontcolor" => Some(scoped_key("ClassFontColor")),
            "fontsize" => Some("ClassFontSize".to_string()),
            "fontname" => Some("ClassFontName".to_string()),
            _ => None,
        };
    }
    match selector {
        "object" => match key {
            "backgroundcolor" => Some("ObjectBackgroundColor".to_string()),
            "bordercolor" | "linecolor" => Some("ObjectBorderColor".to_string()),
            "fontcolor" => Some("ObjectFontColor".to_string()),
            "fontsize" => Some("ObjectFontSize".to_string()),
            "fontname" => Some("ObjectFontName".to_string()),
            _ => None,
        },
        "usecase" => match key {
            "backgroundcolor" => Some("UseCaseBackgroundColor".to_string()),
            "bordercolor" | "linecolor" => Some("UseCaseBorderColor".to_string()),
            "fontcolor" => Some("UseCaseFontColor".to_string()),
            "fontsize" => Some("UseCaseFontSize".to_string()),
            "fontname" => Some("UseCaseFontName".to_string()),
            _ => None,
        },
        "actor" => match key {
            "backgroundcolor" => Some("ActorBackgroundColor".to_string()),
            "bordercolor" | "linecolor" => Some("ActorBorderColor".to_string()),
            "fontcolor" => Some("ActorFontColor".to_string()),
            "fontsize" => Some("ActorFontSize".to_string()),
            "fontname" => Some("ActorFontName".to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn component_style_skinparam_key(nested_selector: Option<&str>, key: &str) -> Option<String> {
    match nested_selector {
        Some("component") => match key {
            "backgroundcolor" => Some("ComponentBackgroundColor".to_string()),
            "bordercolor" | "linecolor" => Some("ComponentBorderColor".to_string()),
            "fontcolor" => Some("ComponentFontColor".to_string()),
            _ => None,
        },
        Some("interface") => match key {
            "backgroundcolor" | "color" => Some("InterfaceBackgroundColor".to_string()),
            "fontcolor" => Some("InterfaceFontColor".to_string()),
            _ => None,
        },
        Some("arrow") | Some("relation") => match key {
            "linecolor" | "arrowcolor" | "bordercolor" => Some("ComponentArrowColor".to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn deployment_style_skinparam_key(nested_selector: Option<&str>, key: &str) -> Option<String> {
    match nested_selector {
        Some("node") => match key {
            "backgroundcolor" => Some("NodeBackgroundColor".to_string()),
            "bordercolor" | "linecolor" => Some("NodeBorderColor".to_string()),
            "fontcolor" => Some("NodeFontColor".to_string()),
            _ => None,
        },
        Some("artifact") => match key {
            "backgroundcolor" => Some("ArtifactBackgroundColor".to_string()),
            "bordercolor" | "linecolor" => Some("ArtifactBorderColor".to_string()),
            "fontcolor" => Some("ArtifactFontColor".to_string()),
            _ => None,
        },
        Some("database") => match key {
            "backgroundcolor" => Some("DatabaseBackgroundColor".to_string()),
            "bordercolor" | "linecolor" => Some("DatabaseBorderColor".to_string()),
            "fontcolor" => Some("DatabaseFontColor".to_string()),
            _ => None,
        },
        Some("component") => component_style_skinparam_key(Some("component"), key),
        Some("interface") => component_style_skinparam_key(Some("interface"), key),
        Some("arrow") | Some("relation") => match key {
            "linecolor" | "arrowcolor" | "bordercolor" => {
                Some("DeploymentArrowColor".to_string())
            }
            _ => None,
        },
        _ => None,
    }
}

fn state_style_skinparam_key(nested_selector: Option<&str>, key: &str) -> Option<String> {
    match nested_selector {
        None => match key {
            "arrowcolor" => Some("StateArrowColor".to_string()),
            _ => None,
        },
        Some("state") => match key {
            "backgroundcolor" => Some("StateBackgroundColor".to_string()),
            "bordercolor" => Some("StateBorderColor".to_string()),
            "fontcolor" => Some("StateFontColor".to_string()),
            "fontsize" => Some("StateFontSize".to_string()),
            _ => None,
        },
        Some("start") => match key {
            "backgroundcolor" | "color" => Some("StateStartColor".to_string()),
            _ => None,
        },
        Some(_) => None,
    }
}

fn activity_style_skinparam_key(nested_selector: Option<&str>, key: &str) -> Option<String> {
    match nested_selector {
        None => match key {
            "arrowcolor" => Some("ActivityArrowColor".to_string()),
            _ => None,
        },
        Some("activity") => match key {
            "backgroundcolor" => Some("ActivityBackgroundColor".to_string()),
            "bordercolor" => Some("ActivityBorderColor".to_string()),
            "fontcolor" => Some("ActivityFontColor".to_string()),
            _ => None,
        },
        Some("diamond") => match key {
            "backgroundcolor" | "color" => Some("ActivityDiamondBackgroundColor".to_string()),
            _ => None,
        },
        Some("bar") | Some("fork") | Some("start") | Some("stop") => match key {
            "backgroundcolor" | "color" => Some("ActivityBarColor".to_string()),
            _ => None,
        },
        Some(_) => None,
    }
}
