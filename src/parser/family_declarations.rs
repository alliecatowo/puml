#[derive(Debug, Clone)]
struct FamilyDeclParts {
    name: String,
    alias: Option<String>,
    has_block: bool,
    stereotypes: Vec<String>,
    tags: Vec<String>,
    fill_color: Option<String>,
    style_members: Vec<String>,
    business: bool,
    heritage: Vec<FamilyHeritage>,
}

#[derive(Debug, Clone)]
struct FamilyHeritage {
    arrow: String,
    target: String,
}

fn parse_named_family_decl(line: &str, keyword: &str) -> Option<FamilyDeclParts> {
    if !line.starts_with(keyword) {
        return None;
    }
    if line.len() > keyword.len()
        && !line[keyword.len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        return None;
    }
    let rest = line[keyword.len()..].trim();
    if rest.is_empty() {
        return None;
    }

    let has_block = rest.ends_with('{');
    let trimmed = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };
    let (trimmed, inline_style) = split_declaration_inline_style(trimmed);
    let supports_business = keyword
        .trim_end_matches('/')
        .eq_ignore_ascii_case("actor")
        || keyword
            .trim_end_matches('/')
            .eq_ignore_ascii_case("usecase");
    let (trimmed, business) = if supports_business {
        strip_business_suffix(trimmed.trim())
    } else {
        (trimmed.trim().to_string(), false)
    };

    let (name_raw, alias_raw, tags) = if let Some((lhs, rhs)) = trimmed.split_once(" as ") {
        let (alias_raw, tags) = split_family_decl_trailing_tags(rhs);
        (lhs.trim().to_string(), Some(alias_raw), tags)
    } else {
        let (name_raw, tags) = split_family_decl_trailing_tags(&trimmed);
        (name_raw, None, tags)
    };

    let (name_raw, heritage) = split_declaration_heritage(&name_raw);
    let (name_without_stereotypes, stereotypes) = strip_declaration_stereotypes(&name_raw);
    let name = clean_family_decl_ident(&name_without_stereotypes);
    if name.is_empty() {
        return None;
    }
    let tags = if tags.is_empty() && name.starts_with('$') {
        vec![name.clone()]
    } else {
        tags
    };
    let alias = alias_raw
        .map(|value| clean_family_decl_ident(&value))
        .filter(|v| !v.is_empty());
    Some(FamilyDeclParts {
        name,
        alias,
        has_block,
        stereotypes,
        tags,
        fill_color: inline_style.fill_color,
        style_members: inline_style.members,
        business: business || keyword.ends_with('/'),
        heritage,
    })
}

fn strip_class_declaration_visibility(line: &str) -> (Option<char>, &str) {
    let mut chars = line.chars();
    let Some(symbol @ ('+' | '-' | '#' | '~')) = chars.next() else {
        return (None, line);
    };
    let rest = chars.as_str().trim_start();
    let lower = rest.to_ascii_lowercase();
    let is_class_decl = [
        "abstract class ",
        "exception ",
        "metaclass ",
        "stereotype ",
        "interface ",
        "enum ",
        "annotation ",
        "protocol ",
        "struct ",
        "circle ",
        "abstract ",
        "class ",
        "entity ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix));
    if is_class_decl {
        (Some(symbol), rest)
    } else {
        (None, line)
    }
}

fn split_family_decl_trailing_tags(input: &str) -> (String, Vec<String>) {
    let mut rest = input.trim_end();
    let mut tags = Vec::new();
    while let Some((start, token)) = last_family_decl_token(rest) {
        if !is_family_tag_token(token) {
            break;
        }
        let before = rest[..start].trim_end();
        if before.is_empty() {
            tags.push(token.to_string());
            break;
        }
        tags.push(token.to_string());
        rest = before;
    }
    tags.reverse();
    (rest.trim().to_string(), tags)
}

fn last_family_decl_token(input: &str) -> Option<(usize, &str)> {
    let trimmed = input.trim_end();
    if trimmed.is_empty() {
        return None;
    }
    let start = trimmed
        .char_indices()
        .rev()
        .find_map(|(idx, ch)| ch.is_whitespace().then_some(idx + ch.len_utf8()))
        .unwrap_or(0);
    Some((start, &trimmed[start..]))
}

fn is_family_tag_token(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('$') else {
        return false;
    };
    !rest.is_empty()
        && rest
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
}

fn append_class_visibility_member(members: &mut Vec<ClassMember>, visibility: Option<char>) {
    if let Some(visibility) = visibility {
        members.push(ClassMember {
            text: format!("\x1fclass:visibility:{visibility}"),
            modifier: None,
        });
    }
}

fn append_family_tag_members(members: &mut Vec<ClassMember>, tags: Vec<String>) {
    for tag in tags {
        members.push(ClassMember {
            text: format!("\x1ffamily:tag:{tag}"),
            modifier: None,
        });
    }
}

fn append_heritage_members(members: &mut Vec<ClassMember>, heritage: Vec<FamilyHeritage>) {
    for item in heritage {
        members.push(ClassMember {
            text: format!("\x1fheritage:{}:{}", item.arrow, item.target),
            modifier: None,
        });
    }
}

fn split_declaration_heritage(input: &str) -> (String, Vec<FamilyHeritage>) {
    let trimmed = input.trim();
    let Some((base, clause)) = split_at_first_top_level_heritage_keyword(trimmed) else {
        return (trimmed.to_string(), Vec::new());
    };

    let mut heritage = Vec::new();
    let mut rest = clause.trim();
    loop {
        let lower = rest.to_ascii_lowercase();
        if lower.starts_with("extends ") {
            rest = rest[8..].trim_start();
            let (targets, next) = take_heritage_targets(rest);
            for target in split_heritage_targets(targets) {
                heritage.push(FamilyHeritage {
                    arrow: "<|--".to_string(),
                    target,
                });
            }
            rest = next.trim_start();
        } else if lower.starts_with("implements ") {
            rest = rest[11..].trim_start();
            let (targets, next) = take_heritage_targets(rest);
            for target in split_heritage_targets(targets) {
                heritage.push(FamilyHeritage {
                    arrow: "<|..".to_string(),
                    target,
                });
            }
            rest = next.trim_start();
        } else {
            break;
        }
        if rest.is_empty() {
            break;
        }
    }

    (base.trim().to_string(), heritage)
}

fn split_at_first_top_level_heritage_keyword(input: &str) -> Option<(&str, &str)> {
    let extends = find_top_level_keyword(input, " extends ");
    let implements = find_top_level_keyword(input, " implements ");
    let idx = match (extends, implements) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return None,
    };
    Some((&input[..idx], input[idx + 1..].trim_start()))
}

fn take_heritage_targets(input: &str) -> (&str, &str) {
    let extends = find_top_level_keyword(input, " extends ");
    let implements = find_top_level_keyword(input, " implements ");
    let idx = match (extends, implements) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => return (input, ""),
    };
    (&input[..idx], input[idx + 1..].trim_start())
}

fn split_heritage_targets(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut angle_depth = 0i32;
    let mut in_quote = false;
    for (idx, ch) in input.char_indices() {
        match ch {
            '"' => in_quote = !in_quote,
            '<' if !in_quote => angle_depth += 1,
            '>' if !in_quote => angle_depth = angle_depth.saturating_sub(1),
            ',' if !in_quote && angle_depth == 0 => {
                let target = clean_ident(&input[start..idx]);
                if !target.is_empty() {
                    out.push(target);
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    let target = clean_ident(&input[start..]);
    if !target.is_empty() {
        out.push(target);
    }
    out
}

fn find_top_level_keyword(input: &str, keyword: &str) -> Option<usize> {
    let lower = input.to_ascii_lowercase();
    let needle = keyword.to_ascii_lowercase();
    let mut search_from = 0usize;
    while let Some(rel) = lower[search_from..].find(&needle) {
        let idx = search_from + rel;
        if is_top_level_span(input, idx) {
            return Some(idx);
        }
        search_from = idx + needle.len();
    }
    None
}

fn is_top_level_span(input: &str, byte_idx: usize) -> bool {
    let mut angle_depth = 0i32;
    let mut in_quote = false;
    for (idx, ch) in input.char_indices() {
        if idx >= byte_idx {
            break;
        }
        match ch {
            '"' => in_quote = !in_quote,
            '<' if !in_quote => angle_depth += 1,
            '>' if !in_quote => angle_depth = angle_depth.saturating_sub(1),
            _ => {}
        }
    }
    !in_quote && angle_depth == 0
}

fn append_inline_fill_member(members: &mut Vec<ClassMember>, fill_color: Option<String>) {
    if let Some(color) = fill_color {
        members.push(ClassMember {
            text: format!("\x1fstyle:fill:{color}"),
            modifier: None,
        });
    }
}

fn append_inline_style_members(members: &mut Vec<ClassMember>, style_members: Vec<String>) {
    members.extend(style_members.into_iter().map(|text| ClassMember {
        text,
        modifier: None,
    }));
}

fn append_business_member(members: &mut Vec<ClassMember>, business: bool) {
    if business {
        members.push(ClassMember {
            text: "<<business>>".to_string(),
            modifier: None,
        });
    }
}

fn strip_business_suffix(input: &str) -> (String, bool) {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_suffix('/') {
        (rest.trim_end().to_string(), true)
    } else {
        (trimmed.to_string(), false)
    }
}

fn clean_family_decl_ident(input: &str) -> String {
    let cleaned = clean_ident(input);
    cleaned
        .strip_prefix(':')
        .and_then(|value| value.strip_suffix(':'))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(clean_ident)
        .unwrap_or(cleaned)
}

fn split_declaration_inline_fill(input: &str) -> (String, Option<String>) {
    let (cleaned, style) = split_declaration_inline_style(input);
    (cleaned, style.fill_color)
}

#[derive(Debug, Clone, Default)]
struct FamilyInlineStyle {
    fill_color: Option<String>,
    members: Vec<String>,
}

fn split_declaration_inline_style(input: &str) -> (String, FamilyInlineStyle) {
    let trimmed = input.trim();
    let mut in_quote = false;
    let mut last_hash: Option<usize> = None;
    for (idx, ch) in trimmed.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if !in_quote && ch == '#' {
            last_hash = Some(idx);
        }
    }
    let Some(hash_idx) = last_hash else {
        return (trimmed.to_string(), FamilyInlineStyle::default());
    };
    if hash_idx > 0
        && !trimmed[..hash_idx]
            .chars()
            .last()
            .is_some_and(char::is_whitespace)
    {
        return (trimmed.to_string(), FamilyInlineStyle::default());
    }
    let after = &trimmed[hash_idx..];
    let token_len = after
        .char_indices()
        .take_while(|(_, ch)| {
            ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | '-' | ':' | ';' | '.')
        })
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if token_len == 0 {
        return (trimmed.to_string(), FamilyInlineStyle::default());
    }
    let token = &after[..token_len];
    let Some(style) = parse_family_decl_inline_style_token(token) else {
        return (trimmed.to_string(), FamilyInlineStyle::default());
    };
    let before = trimmed[..hash_idx].trim_end();
    let suffix = after[token_len..].trim_start();
    let mut cleaned = before.to_string();
    if !suffix.is_empty() {
        if !cleaned.is_empty() {
            cleaned.push(' ');
        }
        cleaned.push_str(suffix);
    }
    (cleaned, style)
}

fn parse_family_decl_inline_style_token(token: &str) -> Option<FamilyInlineStyle> {
    let mut style = FamilyInlineStyle::default();
    for (idx, raw_part) in token.trim_start_matches('#').split(';').enumerate() {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }
        let lower = part.to_ascii_lowercase();
        if let Some(color) = lower
            .strip_prefix("back:")
            .and_then(crate::theme::color::parse_relation_color_token)
        {
            style.fill_color = Some(color);
        } else if let Some(color) = lower
            .strip_prefix("line:")
            .and_then(crate::theme::color::parse_relation_color_token)
        {
            style.members.push(format!("\x1fstyle:border:{color}"));
        } else if let Some(color) = lower
            .strip_prefix("text:")
            .and_then(crate::theme::color::parse_relation_color_token)
        {
            style.members.push(format!("\x1fstyle:text:{color}"));
        } else if matches!(lower.as_str(), "line.dashed" | "line.dotted" | "dashed" | "dotted")
        {
            style.members.push("\x1fstyle:border-dashed".to_string());
        } else if matches!(lower.as_str(), "line.bold" | "line.thick" | "bold" | "thick") {
            style.members.push("\x1fstyle:border-thickness:3".to_string());
        } else if matches!(lower.as_str(), "line.thin" | "thin") {
            style.members.push("\x1fstyle:border-thickness:1".to_string());
        } else if idx == 0 {
            let hex_prefixed = format!("#{part}");
            if let Some(color) = crate::theme::color::parse_relation_color_token(part)
                .or_else(|| crate::theme::color::parse_relation_color_token(&hex_prefixed))
            {
                style.fill_color = Some(color);
            }
        }
    }
    (style.fill_color.is_some() || !style.members.is_empty()).then_some(style)
}

fn declaration_marker_members(marker: Option<&str>, stereotypes: Vec<String>) -> Vec<ClassMember> {
    let mut members = Vec::new();
    if let Some(marker) = marker {
        members.push(ClassMember {
            text: marker.to_string(),
            modifier: None,
        });
    }
    for stereotype in stereotypes {
        members.push(ClassMember {
            text: format!("<<{stereotype}>>"),
            modifier: None,
        });
    }
    members
}

fn strip_declaration_stereotypes(input: &str) -> (String, Vec<String>) {
    let mut remaining = input.trim().to_string();
    let mut stereotypes = Vec::new();
    while let Some(start) = remaining.find("<<") {
        let Some(end_rel) = remaining[start + 2..].find(">>") else {
            break;
        };
        let end = start + 2 + end_rel;
        let value = remaining[start + 2..end].trim();
        if !value.is_empty() {
            stereotypes.push(value.to_string());
        }
        remaining.replace_range(start..end + 2, "");
    }
    (remaining.trim().to_string(), stereotypes)
}

fn parse_parenthesized_usecase_decl(line: &str) -> Option<FamilyDeclParts> {
    let trimmed = line.trim();
    let (trimmed, keyword_business) = if let Some(rest) = trimmed.strip_prefix("usecase/") {
        (rest.trim(), true)
    } else {
        (
            trimmed.strip_prefix("usecase ").unwrap_or(trimmed).trim(),
            false,
        )
    };
    if !trimmed.starts_with('(') {
        return None;
    }
    let close = trimmed.find(')')?;
    let name_raw = trimmed[1..close].trim();
    if name_raw.is_empty() {
        return None;
    }
    let rest = trimmed[close + 1..].trim();
    let has_block = rest.ends_with('{');
    let rest = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };
    let (rest, inline_style) = split_declaration_inline_style(rest);
    let (rest, suffix_business) = if let Some(after) = rest.trim().strip_prefix('/') {
        (after.trim_start().to_string(), true)
    } else {
        strip_business_suffix(rest.trim())
    };
    let rest = rest.trim();
    if !rest.is_empty() && !rest.starts_with("as ") {
        return None;
    }
    let alias = rest
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_family_decl_ident)
        .filter(|v| !v.is_empty());
    Some(FamilyDeclParts {
        name: clean_family_decl_ident(name_raw),
        alias,
        has_block,
        stereotypes: Vec::new(),
        tags: Vec::new(),
        fill_color: inline_style.fill_color,
        style_members: inline_style.members,
        business: keyword_business || suffix_business,
        heritage: Vec::new(),
    })
}

fn parse_colon_actor_decl(line: &str) -> Option<FamilyDeclParts> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix(':')?;
    let close = rest.find(':')?;
    let name_raw = rest[..close].trim();
    if name_raw.is_empty() {
        return None;
    }
    let rest = rest[close + 1..].trim();
    let (rest, inline_style) = split_declaration_inline_style(rest);
    let (rest, business) = if let Some(after) = rest.trim().strip_prefix('/') {
        (after.trim_start().to_string(), true)
    } else {
        strip_business_suffix(rest.trim())
    };
    let rest = rest.trim();
    let alias = rest
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_family_decl_ident)
        .filter(|v| !v.is_empty());
    if !rest.is_empty() && alias.is_none() {
        return None;
    }
    Some(FamilyDeclParts {
        name: clean_family_decl_ident(name_raw),
        alias,
        has_block: false,
        stereotypes: Vec::new(),
        tags: Vec::new(),
        fill_color: inline_style.fill_color,
        style_members: inline_style.members,
        business,
        heritage: Vec::new(),
    })
}
