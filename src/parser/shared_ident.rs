fn clean_ident(s: &str) -> String {
    let mut out = s.trim().trim_matches('"').to_string();
    if let Some(rest) = out.strip_prefix("()") {
        out = rest.trim().to_string();
    }
    if let Some(rest) = out.strip_suffix("()") {
        out = rest.trim().to_string();
    }
    for suffix in ["++", "--", "**", "!!"] {
        out = out
            .strip_suffix(suffix)
            .map(str::trim_end)
            .unwrap_or(&out)
            .to_string();
    }
    out
}

fn strip_c4_macro_prefix(input: &str) -> &str {
    input.trim().strip_prefix('!').unwrap_or_else(|| input.trim())
}

/// Extract the class/interface/enum name from a member line inside a package/namespace block.
/// E.g. "class Service" → "Service", "interface IRepo" → "IRepo", "MyClass" → "MyClass".
fn extract_class_member_name(s: &str) -> String {
    let t = s.trim();
    let lower = t.to_ascii_lowercase();
    for kw in &[
        "abstract class ",
        "annotation ",
        "interface ",
        "abstract ",
        "enum ",
        "class ",
        "object ",
        "map ",
        "usecase ",
        "component ",
        "portin ",
        "portout ",
        "port ",
        "node ",
        "database ",
        "cloud ",
        "frame ",
        "storage ",
        "package ",
        "rectangle ",
        "folder ",
        "file ",
        "card ",
        "artifact ",
        "actor ",
    ] {
        if lower.starts_with(kw) {
            // Extract the first identifier token from the original (case-preserved) text
            let name_part = t[kw.len()..].trim();
            let name = name_part
                .split(|c: char| c.is_whitespace() || c == '{')
                .next()
                .unwrap_or("")
                .trim_matches('"');
            return clean_ident(name);
        }
    }
    // Plain identifier (like in a together block)
    clean_ident(t)
}

fn extract_component_group_member_name(s: &str) -> String {
    if let Some(StatementKind::ComponentDecl { name, alias, .. }) = parse_component_decl(s) {
        return alias.unwrap_or(name);
    }
    extract_class_member_name(s)
}
