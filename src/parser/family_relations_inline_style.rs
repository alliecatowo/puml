use super::*;

/// Parse tail-form inline relation style from the RHS of a relation.
///
/// PlantUML supports the form `A --> B #line:red;line.bold;text:blue : label`
/// where the `#...` token immediately follows the target node name (before the
/// colon label, which is already stripped by `split_family_relation_label`).
///
/// This function looks for a trailing `#...` inline-style token on `rhs` and,
/// if found, returns `(clean_rhs, Some(merged_style))`. The extracted style is
/// merged into the existing bracket-style (from `[#color]` within the arrow).
///
/// Spec: PlantUML Language Reference 3.36.
pub(crate) fn parse_rhs_inline_relation_style(
    rhs: &str,
    existing: &mut ParsedFamilyRelationStyle,
) -> String {
    let trimmed = rhs.trim();
    // Look for the last `#` that is preceded by whitespace (or is at the start),
    // meaning it starts a tail style token rather than being part of the node name.
    let mut hash_pos: Option<usize> = None;
    let mut prev_was_space = true;
    for (idx, ch) in trimmed.char_indices() {
        if ch == '#' && prev_was_space {
            hash_pos = Some(idx);
        }
        prev_was_space = ch.is_ascii_whitespace();
    }
    let Some(hpos) = hash_pos else {
        return trimmed.to_string();
    };
    let candidate = &trimmed[hpos..];
    // The token must consist of `#` followed by valid inline-style chars only.
    let token_len = candidate
        .char_indices()
        .take_while(|(_, ch)| {
            ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | '-' | ':' | ';' | '.')
        })
        .map(|(i, ch)| i + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if token_len == 0 {
        return trimmed.to_string();
    }
    let token = &candidate[..token_len];
    // After the token there must be nothing (no trailing node-name chars).
    let after = candidate[token_len..].trim();
    if !after.is_empty() {
        return trimmed.to_string();
    }
    // Now parse the token into style components and merge into `existing`.
    let mut found_any = false;
    for (idx, raw_part) in token.trim_start_matches('#').split(';').enumerate() {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }
        let lower = part.to_ascii_lowercase();
        let lower_stripped = lower
            .strip_prefix("line.")
            .or_else(|| lower.strip_prefix("line:"))
            .unwrap_or(lower.as_str());
        if matches!(lower_stripped, "dashed" | "dotted" | "dash" | "dot") {
            existing.dashed = true;
            found_any = true;
        } else if matches!(lower_stripped, "bold" | "thick") {
            existing.thickness = Some(existing.thickness.unwrap_or(3).max(3));
            found_any = true;
        } else if lower_stripped == "thin" {
            existing.thickness = Some(1);
            found_any = true;
        } else if lower_stripped == "hidden" {
            existing.hidden = true;
            found_any = true;
        } else if lower_stripped == "plain" {
            existing.dashed = false;
            existing.thickness = None;
            found_any = true;
        } else if let Some(color_str) = lower_stripped
            .strip_prefix("back:")
            .or_else(|| lower_stripped.strip_prefix("color:"))
        {
            // background/fill color on a relation: treat as line color
            if let Some(color) = crate::theme::color::parse_relation_color_token(color_str) {
                existing.line_color = Some(color);
                found_any = true;
            }
        } else if lower_stripped.starts_with("text:") {
            // text: color — accepted as no-op for now (relation labels don't yet have per-label color)
            found_any = true;
        } else {
            // `lower_stripped` might be a color name (e.g. `line:green` → stripped to `green`).
            // Also handle first part as a bare color: `#red` or `#FF0000`.
            let hex_prefixed_stripped = format!("#{lower_stripped}");
            let color_opt = crate::theme::color::parse_relation_color_token(lower_stripped)
                .or_else(|| crate::theme::color::parse_relation_color_token(&hex_prefixed_stripped))
                .or_else(|| {
                    if idx == 0 {
                        let hex_prefixed = format!("#{part}");
                        crate::theme::color::parse_relation_color_token(part).or_else(|| {
                            crate::theme::color::parse_relation_color_token(&hex_prefixed)
                        })
                    } else {
                        None
                    }
                });
            if let Some(color) = color_opt {
                existing.line_color = Some(color);
                found_any = true;
            }
        }
    }
    if found_any {
        trimmed[..hpos].trim_end().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Pre-process a relation line by extracting and removing any tail-form inline
/// style token (`#color`, `#line:red;line.bold`) that appears after the RHS
/// node name.
///
/// Returns `(cleaned_line, extracted_style)`. The cleaned line has the style
/// token removed so that `split_family_relation_label` does not accidentally
/// split on the `:` inside `#line:color`.
///
/// The style token must:
/// - Start with `#` preceded by whitespace
/// - Consist entirely of `[A-Za-z0-9#:;._-]` chars (no spaces)
/// - Be followed by optional whitespace, then end-of-line or ` : <label>`
///   (i.e., it cannot be in the middle of the line)
pub(crate) fn pre_strip_inline_relation_style(
    line: &str,
) -> (String, Option<ParsedFamilyRelationStyle>) {
    let trimmed = line.trim();
    // Scan backwards from end (or from the ` : label` boundary) to find a `#` token.
    // We look for the pattern: `... <rhs_ident> <WS> #<inline_style_chars> [<WS> : <label>]`
    // Strategy: find the last `#` preceded by whitespace. Check the token after it.
    // Then verify the token is immediately followed by end-of-string or ` :` (label boundary).

    // First, find the optional label split point ` : ` or ` :`
    // We must be careful: the real label colon is ` :` where the space precedes it.
    // Our inline style contains `:` without a space before it (e.g., `#line:red`).
    // Look for the LAST ` : ` or trailing ` :` that is NOT inside the style token.
    // Since we don't know where the style token ends yet, we need a different approach.

    // Approach: scan for `#` tokens preceded by whitespace. For each candidate,
    // verify it contains no spaces (valid style token chars only).
    let mut hash_candidates: Vec<usize> = Vec::new();
    let mut prev_was_space = false;
    let mut in_quote = false;
    for (idx, ch) in trimmed.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
        }
        if !in_quote && ch == '#' && (idx == 0 || prev_was_space) {
            hash_candidates.push(idx);
        }
        prev_was_space = !in_quote && ch.is_ascii_whitespace();
    }

    // Process candidates from last to first (rightmost wins)
    for hpos in hash_candidates.into_iter().rev() {
        let candidate = &trimmed[hpos..];
        let token_len = candidate
            .char_indices()
            .take_while(|(_, ch)| {
                ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | '-' | ':' | ';' | '.')
            })
            .map(|(i, ch)| i + ch.len_utf8())
            .last()
            .unwrap_or(0);
        if token_len == 0 {
            continue;
        }
        let token = &candidate[..token_len];
        let after_token = candidate[token_len..].trim_start();
        // After the token there must be nothing, or `: <label>` (colon preceded by optional space)
        let is_at_end = after_token.is_empty();
        let is_before_label = after_token.starts_with(':') && {
            let rest = after_token[1..].trim();
            !rest.is_empty() && !suffix_has_family_relation_arrow(rest)
        };
        if !is_at_end && !is_before_label {
            continue;
        }
        // Parse the style token
        let mut style = ParsedFamilyRelationStyle::default();
        let mut found_any = false;
        for (idx2, raw_part) in token.trim_start_matches('#').split(';').enumerate() {
            let part = raw_part.trim();
            if part.is_empty() {
                continue;
            }
            let lower = part.to_ascii_lowercase();
            let lower_stripped = lower
                .strip_prefix("line.")
                .or_else(|| lower.strip_prefix("line:"))
                .unwrap_or(lower.as_str());
            if matches!(lower_stripped, "dashed" | "dotted" | "dash" | "dot") {
                style.dashed = true;
                found_any = true;
            } else if matches!(lower_stripped, "bold" | "thick") {
                style.thickness = Some(style.thickness.unwrap_or(3).max(3));
                found_any = true;
            } else if lower_stripped == "thin" {
                style.thickness = Some(1);
                found_any = true;
            } else if lower_stripped == "hidden" {
                style.hidden = true;
                found_any = true;
            } else if lower_stripped == "plain" {
                style.dashed = false;
                style.thickness = None;
                found_any = true;
            } else if let Some(color_str) = lower_stripped
                .strip_prefix("back:")
                .or_else(|| lower_stripped.strip_prefix("color:"))
            {
                if let Some(color) = crate::theme::color::parse_relation_color_token(color_str) {
                    style.line_color = Some(color);
                    found_any = true;
                }
            } else if lower_stripped.starts_with("text:") {
                // text: color — accepted as no-op
                found_any = true;
            } else {
                // `lower_stripped` might itself be a color name (e.g. `line:green` → `green`)
                // Try it as a color before falling back to the raw `part`.
                let hex_prefixed_stripped = format!("#{lower_stripped}");
                let color_opt = crate::theme::color::parse_relation_color_token(lower_stripped)
                    .or_else(|| {
                        crate::theme::color::parse_relation_color_token(&hex_prefixed_stripped)
                    })
                    .or_else(|| {
                        if idx2 == 0 {
                            let hex_prefixed = format!("#{part}");
                            crate::theme::color::parse_relation_color_token(part).or_else(|| {
                                crate::theme::color::parse_relation_color_token(&hex_prefixed)
                            })
                        } else {
                            None
                        }
                    });
                if let Some(color) = color_opt {
                    style.line_color = Some(color);
                    found_any = true;
                }
            }
        }
        if !found_any {
            continue;
        }
        // Build the cleaned line: everything before the token, then the after-token suffix
        let before = trimmed[..hpos].trim_end();
        let cleaned = if after_token.is_empty() {
            before.to_string()
        } else {
            // after_token starts with `: label`
            format!("{before} {after_token}")
        };
        return (cleaned, Some(style));
    }

    (trimmed.to_string(), None)
}
