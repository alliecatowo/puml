use super::*;
pub(crate) fn parse_skinparam_block(
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

    // Check if the block prefix itself carries a stereotype scope, e.g.
    // `skinparam class<<entity>> { ... }`.  In that case we need to reorder
    // the key so the stereotype suffix lands at the *end*, because the
    // classifier in `classify_class_skinparam` (and friends) calls
    // `split_stereotype_scope` which expects `ClassBackgroundColor<<entity>>`
    // not `class<<entity>>BackgroundColor`.
    //
    // Detect: prefix ends with `>>` and contains `<<`.
    let (base_prefix, block_stereotype) = {
        let p = original_prefix;
        if let Some(stripped) = p.strip_suffix(">>") {
            if let Some(start) = stripped.rfind("<<") {
                let base = stripped[..start].trim();
                let stereo = stripped[start + 2..].trim();
                if !base.is_empty() && !stereo.is_empty() {
                    (base, Some(stereo.to_string()))
                } else {
                    (p, None)
                }
            } else {
                (p, None)
            }
        } else {
            (p, None)
        }
    };

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
        // When the block selector carries a stereotype (e.g. `class<<entity>>`), move
        // the stereotype tag to the end of the combined key so the skinparam classifier
        // can peel it with `split_stereotype_scope`:
        //   "class" + "BackgroundColor" + "<<entity>>" → "classBackgroundColor<<entity>>"
        let combined_key = if let Some(ref stereo) = block_stereotype {
            format!("{base_prefix}{inner_key}<<{stereo}>>")
        } else {
            format!("{original_prefix}{inner_key}")
        };
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
/// - `usecaseDiagram { usecase { ... } actor { ... } }`
/// - `componentDiagram { component { ... } }`
/// - `deploymentDiagram { node { ... } }`
/// - `saltDiagram { button/input/menu/tab/... { ... } }`
/// - declarations in `Property Value` or `Property: Value;` form
pub(crate) fn parse_style_block(
    lines: &[(&str, Span)],
    start_idx: usize,
    line: &str,
) -> Result<Option<(Vec<StatementKind>, usize)>, Diagnostic> {
    if !line.eq_ignore_ascii_case("<style>") {
        return Ok(None);
    }

    // Collect the raw body text between <style> and </style>, then run the
    // recursive-descent parser to produce a typed StyleBlock AST.
    let mut body_lines: Vec<&str> = Vec::new();
    let mut close_idx: Option<usize> = None;
    for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
        if strip_inline_plantuml_comment(raw)
            .trim()
            .eq_ignore_ascii_case("</style>")
        {
            close_idx = Some(idx);
            break;
        }
        body_lines.push(raw);
    }
    let body_text = body_lines.join("\n");
    let (style_block_ast, _compat_triples) =
        crate::parser::style_block::parse_style_block_body(&body_text);
    // _compat_triples: the legacy StyleParam compat shim was removed in Phase E (#1417).

    if !has_known_style_target(lines, start_idx) {
        // Preserve unrecognised style blocks as DeferredRaw lines so that
        // family-specific raw handlers (e.g. mindmap depth styles) can still
        // consume them. Also emit the typed StyleBlock for cascade resolution.
        let mut kinds = vec![
            StatementKind::StyleBlock(style_block_ast),
            StatementKind::DeferredRaw(line.to_string()),
        ];
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
    }

    // Phase E (#1417): The legacy flat-triple compat shim is removed.
    // Emit only the typed StyleBlock AST; the cascade resolver handles all families.
    let kinds: Vec<StatementKind> = vec![StatementKind::StyleBlock(style_block_ast)];

    // Scan to the closing </style> tag (we already found it above).
    if let Some(end_idx) = close_idx {
        return Ok(Some((kinds, end_idx)));
    }

    // Fall through: close_idx was None (close not found during look-ahead), scan again.
    for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
        let inner = strip_inline_plantuml_comment(raw).trim();
        if inner.eq_ignore_ascii_case("</style>") {
            return Ok(Some((kinds, idx)));
        }
    }

    Err(
        Diagnostic::error("[E_STYLE_BLOCK_UNCLOSED] `<style>` block is missing closing `</style>`")
            .with_span(lines[start_idx].1),
    )
}

/// Returns `true` when the `<style>` block at `start_idx` in `lines` opens with
/// a known top-level diagram selector (e.g. `sequenceDiagram { … }`).
///
/// Unknown selectors (e.g. mindmap-depth styles) are handled via the `DeferredRaw`
/// path so that family-specific raw handlers can consume them.
fn has_known_style_target(lines: &[(&str, Span)], start_idx: usize) -> bool {
    style_block_has_target(lines, start_idx)
}

fn style_block_has_target(lines: &[(&str, Span)], start_idx: usize) -> bool {
    for (raw, _) in lines.iter().skip(start_idx + 1) {
        let inner = strip_inline_plantuml_comment(raw).trim();
        if inner.eq_ignore_ascii_case("</style>") {
            return false;
        }
        if inner.is_empty() {
            continue;
        }
        let selector = inner.trim_end_matches('{').trim();
        return matches!(
            selector.to_ascii_lowercase().as_str(),
            "sequencediagram"
                | "classdiagram"
                | "usecasediagram"
                | "componentdiagram"
                | "deploymentdiagram"
                | "statediagram"
                | "activitydiagram"
                | "saltdiagram"
        );
    }
    false
}
